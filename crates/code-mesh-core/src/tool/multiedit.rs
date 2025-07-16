//! Multi-edit tool implementation for batch file editing with atomic transactions

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;
use similar::TextDiff;
use uuid::Uuid;

use super::{Tool, ToolContext, ToolResult, ToolError};
use super::edit::{SimpleReplacer, LineTrimmedReplacer, WhitespaceNormalizedReplacer, IndentationFlexibleReplacer, ReplacementStrategy};

/// Tool for performing multiple file edits in a single atomic operation
pub struct MultiEditTool;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EditOperation {
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

#[derive(Debug, Deserialize)]
struct MultiEditParams {
    file_path: String,
    edits: Vec<EditOperation>,
}

#[derive(Debug)]
struct FileBackup {
    backup_id: String,
    original_content: String,
    backup_path: PathBuf,
}

#[derive(Debug)]
struct EditResult {
    operation_index: usize,
    replacements: usize,
    strategy_used: String,
    content_after: String,
}

#[async_trait]
impl Tool for MultiEditTool {
    fn id(&self) -> &str {
        "multiedit"
    }
    
    fn description(&self) -> &str {
        "Perform multiple file edits in a single atomic operation with rollback support"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "edits": {
                    "type": "array",
                    "description": "Array of edit operations to perform sequentially",
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_string": {
                                "type": "string",
                                "description": "Text to find and replace"
                            },
                            "new_string": {
                                "type": "string",
                                "description": "Replacement text"
                            },
                            "replace_all": {
                                "type": "boolean",
                                "description": "Replace all occurrences (default: false)",
                                "default": false
                            }
                        },
                        "required": ["old_string", "new_string"]
                    },
                    "minItems": 1
                }
            },
            "required": ["file_path", "edits"]
        })
    }
    
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: MultiEditParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Validate parameters
        if params.edits.is_empty() {
            return Err(ToolError::InvalidParameters(
                "At least one edit operation is required".to_string()
            ));
        }
        
        // Check for invalid edits
        for (i, edit) in params.edits.iter().enumerate() {
            if edit.old_string == edit.new_string {
                return Err(ToolError::InvalidParameters(format!(
                    "Edit operation {} has identical old_string and new_string", i
                )));
            }
        }
        
        // Resolve file path
        let path = if PathBuf::from(&params.file_path).is_absolute() {
            PathBuf::from(&params.file_path)
        } else {
            ctx.working_directory.join(&params.file_path)
        };
        
        // Create backup before any modifications
        let backup = self.create_backup(&path).await?;
        
        // Attempt to apply all edits
        match self.apply_edits_atomic(&path, &params.edits, &ctx).await {
            Ok(results) => {
                // Success - clean up backup and return results
                self.cleanup_backup(&backup).await.ok(); // Don't fail if cleanup fails
                self.format_success_result(&params.file_path, &backup.original_content, &path, results).await
            }
            Err(error) => {
                // Failure - restore from backup
                if let Err(restore_error) = self.restore_backup(&backup, &path).await {
                    return Err(ToolError::ExecutionFailed(format!(
                        "Edit failed: {}. Backup restoration also failed: {}", 
                        error, restore_error
                    )));
                }
                self.cleanup_backup(&backup).await.ok();
                Err(error)
            }
        }
    }
}

impl MultiEditTool {
    /// Create a backup of the file before modifications
    async fn create_backup(&self, path: &PathBuf) -> Result<FileBackup, ToolError> {
        let original_content = fs::read_to_string(path).await?;
        let backup_id = Uuid::new_v4().to_string();
        let backup_path = path.with_extension(format!("backup.{}", backup_id));
        
        // Write backup file
        fs::write(&backup_path, &original_content).await?;
        
        Ok(FileBackup {
            backup_id,
            original_content,
            backup_path,
        })
    }
    
    /// Apply all edits atomically - if any fail, return error
    async fn apply_edits_atomic(
        &self,
        path: &PathBuf,
        edits: &[EditOperation],
        ctx: &ToolContext,
    ) -> Result<Vec<EditResult>, ToolError> {
        let mut current_content = fs::read_to_string(path).await?;
        let mut results = Vec::new();
        
        // Replacement strategies to try
        let strategies: Vec<(&str, Box<dyn ReplacementStrategy + Send + Sync>)> = vec![
            ("simple", Box::new(SimpleReplacer)),
            ("line_trimmed", Box::new(LineTrimmedReplacer)),
            ("whitespace_normalized", Box::new(WhitespaceNormalizedReplacer)),
            ("indentation_flexible", Box::new(IndentationFlexibleReplacer)),
        ];
        
        // Apply each edit sequentially
        for (i, edit) in edits.iter().enumerate() {
            // Check for cancellation
            if *ctx.abort_signal.borrow() {
                return Err(ToolError::Aborted);
            }
            
            let mut found_replacement = false;
            let mut replacements = 0;
            let mut strategy_used = String::new();
            
            // Try each replacement strategy
            for (strategy_name, strategy) in &strategies {
                let result = strategy.replace(&current_content, &edit.old_string, &edit.new_string, edit.replace_all);
                if result.count > 0 {
                    current_content = result.content;
                    replacements = result.count;
                    strategy_used = strategy_name.to_string();
                    found_replacement = true;
                    break;
                }
            }
            
            if !found_replacement {
                return Err(ToolError::ExecutionFailed(format!(
                    "Edit operation {} failed: Could not find '{}' in file after {} previous edit(s)",
                    i,
                    edit.old_string.chars().take(100).collect::<String>(),
                    i
                )));
            }
            
            results.push(EditResult {
                operation_index: i,
                replacements,
                strategy_used,
                content_after: current_content.clone(),
            });
        }
        
        // All edits successful - write final content
        fs::write(path, &current_content).await?;
        
        Ok(results)
    }
    
    /// Restore file from backup
    async fn restore_backup(&self, backup: &FileBackup, path: &PathBuf) -> Result<(), ToolError> {
        fs::write(path, &backup.original_content).await?;
        Ok(())
    }
    
    /// Clean up backup file
    async fn cleanup_backup(&self, backup: &FileBackup) -> Result<(), ToolError> {
        if backup.backup_path.exists() {
            fs::remove_file(&backup.backup_path).await?;
        }
        Ok(())
    }
    
    /// Format successful result with comprehensive information
    async fn format_success_result(
        &self,
        file_path: &str,
        original_content: &str,
        final_path: &PathBuf,
        results: Vec<EditResult>,
    ) -> Result<ToolResult, ToolError> {
        let final_content = fs::read_to_string(final_path).await?;
        
        // Calculate total replacements
        let total_replacements: usize = results.iter().map(|r| r.replacements).sum();
        
        // Generate comprehensive diff
        let diff = TextDiff::from_lines(original_content, &final_content);
        let mut diff_output = String::new();
        let mut changes_count = 0;
        
        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Delete => {
                    diff_output.push_str(&format!("- {}", change));
                    changes_count += 1;
                }
                similar::ChangeTag::Insert => {
                    diff_output.push_str(&format!("+ {}", change));
                    changes_count += 1;
                }
                similar::ChangeTag::Equal => {},
            }
        }
        
        // Create detailed metadata
        let edit_details: Vec<Value> = results.iter().map(|result| {
            json!({
                "operation_index": result.operation_index,
                "replacements": result.replacements,
                "strategy_used": result.strategy_used
            })
        }).collect();
        
        let metadata = json!({
            "path": final_path.to_string_lossy(),
            "total_operations": results.len(),
            "total_replacements": total_replacements,
            "operations_details": edit_details,
            "diff": diff_output,
            "diff_changes": changes_count,
            "atomic_transaction": true
        });
        
        let operations_summary = results.iter()
            .map(|r| format!("Op {}: {} replacement{} ({})", 
                r.operation_index, 
                r.replacements,
                if r.replacements == 1 { "" } else { "s" },
                r.strategy_used
            ))
            .collect::<Vec<_>>()
            .join(", ");
        
        Ok(ToolResult {
            title: format!(
                "Successfully completed {} edit operation{} with {} total replacement{} in {}",
                results.len(),
                if results.len() == 1 { "" } else { "s" },
                total_replacements,
                if total_replacements == 1 { "" } else { "s" },
                file_path
            ),
            metadata,
            output: format!(
                "Multi-edit completed successfully:\n\
                - File: {}\n\
                - Total operations: {}\n\
                - Total replacements: {}\n\
                - Operations: {}\n\
                - Atomic transaction: All edits applied successfully or rolled back on failure",
                file_path,
                results.len(),
                total_replacements,
                operations_summary
            ),
        })
    }
}

// Note: Replacement strategies are imported at the top of the file

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_multiedit_atomic_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello world\nThis is a test\nEnd of file").unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        let tool = MultiEditTool;
        let params = json!({
            "file_path": temp_path.to_string_lossy(),
            "edits": [
                {
                    "old_string": "Hello",
                    "new_string": "Hi",
                    "replace_all": false
                },
                {
                    "old_string": "test",
                    "new_string": "example",
                    "replace_all": false
                }
            ]
        });
        
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "test".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.title.contains("2 edit operation"));
        assert!(result.title.contains("2 total replacement"));
        
        let content = fs::read_to_string(&temp_path).await.unwrap();
        assert!(content.contains("Hi world"));
        assert!(content.contains("This is a example"));
    }
    
    #[tokio::test]
    async fn test_multiedit_atomic_failure_rollback() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello world\nThis is a test\nEnd of file").unwrap();
        let temp_path = temp_file.path().to_path_buf();
        let original_content = fs::read_to_string(&temp_path).await.unwrap();
        
        let tool = MultiEditTool;
        let params = json!({
            "file_path": temp_path.to_string_lossy(),
            "edits": [
                {
                    "old_string": "Hello",
                    "new_string": "Hi",
                    "replace_all": false
                },
                {
                    "old_string": "nonexistent",
                    "new_string": "replacement",
                    "replace_all": false
                }
            ]
        });
        
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "test".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        
        // Verify rollback - content should be unchanged
        let final_content = fs::read_to_string(&temp_path).await.unwrap();
        assert_eq!(original_content, final_content);
    }
    
    #[tokio::test]
    async fn test_multiedit_replace_all() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test test test\nAnother test line").unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        let tool = MultiEditTool;
        let params = json!({
            "file_path": temp_path.to_string_lossy(),
            "edits": [
                {
                    "old_string": "test",
                    "new_string": "example",
                    "replace_all": true
                }
            ]
        });
        
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "test".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.title.contains("4 total replacement"));
        
        let content = fs::read_to_string(&temp_path).await.unwrap();
        assert_eq!(content, "example example example\nAnother example line\n");
    }
}