//! Edit tool implementation with multiple replacement strategies

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;
use similar::TextDiff;

use super::{Tool, ToolContext, ToolResult, ToolError};

/// Tool for editing files with smart replacement strategies
pub struct EditTool;

#[derive(Debug, Deserialize)]
struct EditParams {
    file_path: String,
    old_string: String,
    new_string: String,
    #[serde(default)]
    replace_all: bool,
}

#[async_trait]
impl Tool for EditTool {
    fn id(&self) -> &str {
        "edit"
    }
    
    fn description(&self) -> &str {
        "Edit files using find-and-replace with smart matching strategies"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
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
            "required": ["file_path", "old_string", "new_string"]
        })
    }
    
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: EditParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        if params.old_string == params.new_string {
            return Err(ToolError::InvalidParameters(
                "old_string and new_string cannot be the same".to_string()
            ));
        }
        
        // Resolve path
        let path = if PathBuf::from(&params.file_path).is_absolute() {
            PathBuf::from(&params.file_path)
        } else {
            ctx.working_directory.join(&params.file_path)
        };
        
        // Read file
        let content = fs::read_to_string(&path).await?;
        
        // Try different replacement strategies
        let strategies: [Box<dyn ReplacementStrategy>; 4] = [
            Box::new(SimpleReplacer),
            Box::new(LineTrimmedReplacer),
            Box::new(WhitespaceNormalizedReplacer),
            Box::new(IndentationFlexibleReplacer),
        ];
        
        let mut replacements = 0;
        let mut new_content = content.clone();
        
        for strategy in &strategies {
            let result = strategy.replace(&content, &params.old_string, &params.new_string, params.replace_all);
            if result.count > 0 {
                new_content = result.content;
                replacements = result.count;
                break;
            }
        }
        
        if replacements == 0 {
            return Err(ToolError::ExecutionFailed(format!(
                "Could not find '{}' in {}. The file might have been modified since you last read it.",
                params.old_string.chars().take(100).collect::<String>(),
                params.file_path
            )));
        }
        
        // Write updated content
        fs::write(&path, &new_content).await?;
        
        // Generate diff
        let diff = TextDiff::from_lines(&content, &new_content);
        let mut diff_output = String::new();
        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Delete => diff_output.push_str(&format!("- {}", change)),
                similar::ChangeTag::Insert => diff_output.push_str(&format!("+ {}", change)),
                similar::ChangeTag::Equal => {},
            }
        }
        
        let metadata = json!({
            "path": path.to_string_lossy(),
            "replacements": replacements,
            "replace_all": params.replace_all,
            "diff": diff_output,
        });
        
        Ok(ToolResult {
            title: format!("Made {} replacement{} in {}", 
                replacements, 
                if replacements == 1 { "" } else { "s" },
                params.file_path
            ),
            metadata,
            output: format!(
                "Successfully replaced {} occurrence{} of '{}' with '{}' in {}",
                replacements,
                if replacements == 1 { "" } else { "s" },
                params.old_string.chars().take(50).collect::<String>(),
                params.new_string.chars().take(50).collect::<String>(),
                params.file_path
            ),
        })
    }
}

/// Replacement strategy trait
pub trait ReplacementStrategy: Send + Sync {
    fn replace(&self, content: &str, old: &str, new: &str, replace_all: bool) -> ReplaceResult;
}

pub struct ReplaceResult {
    pub content: String,
    pub count: usize,
}

/// Simple exact string replacement
pub struct SimpleReplacer;

impl ReplacementStrategy for SimpleReplacer {
    fn replace(&self, content: &str, old: &str, new: &str, replace_all: bool) -> ReplaceResult {
        if replace_all {
            let count = content.matches(old).count();
            ReplaceResult {
                content: content.replace(old, new),
                count,
            }
        } else {
            if let Some(pos) = content.find(old) {
                let mut result = content.to_string();
                result.replace_range(pos..pos + old.len(), new);
                ReplaceResult { content: result, count: 1 }
            } else {
                ReplaceResult { content: content.to_string(), count: 0 }
            }
        }
    }
}

/// Replacement with trimmed line matching
pub struct LineTrimmedReplacer;

impl ReplacementStrategy for LineTrimmedReplacer {
    fn replace(&self, content: &str, old: &str, new: &str, replace_all: bool) -> ReplaceResult {
        let old_lines: Vec<&str> = old.lines().collect();
        let content_lines: Vec<&str> = content.lines().collect();
        
        if old_lines.is_empty() {
            return ReplaceResult { content: content.to_string(), count: 0 };
        }
        
        let mut result_lines: Vec<String> = Vec::new();
        let mut i = 0;
        let mut count = 0;
        
        while i < content_lines.len() {
            let mut matched = true;
            
            // Check if we have enough lines to match
            if i + old_lines.len() > content_lines.len() {
                result_lines.push(content_lines[i].to_string());
                i += 1;
                continue;
            }
            
            // Try to match trimmed lines
            for (j, old_line) in old_lines.iter().enumerate() {
                if content_lines[i + j].trim() != old_line.trim() {
                    matched = false;
                    break;
                }
            }
            
            if matched {
                // Replace with new content
                for new_line in new.lines() {
                    result_lines.push(new_line.to_string());
                }
                i += old_lines.len();
                count += 1;
                
                if !replace_all {
                    // Copy remaining lines
                    result_lines.extend(content_lines[i..].iter().map(|s| s.to_string()));
                    break;
                }
            } else {
                result_lines.push(content_lines[i].to_string());
                i += 1;
            }
        }
        
        ReplaceResult {
            content: result_lines.join("\n"),
            count,
        }
    }
}

/// Replacement with normalized whitespace
pub struct WhitespaceNormalizedReplacer;

impl ReplacementStrategy for WhitespaceNormalizedReplacer {
    fn replace(&self, content: &str, old: &str, new: &str, replace_all: bool) -> ReplaceResult {
        let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let old_normalized = normalize(old);
        
        // Simple implementation - could be more sophisticated
        let content_normalized = normalize(content);
        if let Some(_) = content_normalized.find(&old_normalized) {
            // For now, fall back to simple replacement
            SimpleReplacer.replace(content, old, new, replace_all)
        } else {
            ReplaceResult { content: content.to_string(), count: 0 }
        }
    }
}

/// Replacement with flexible indentation
pub struct IndentationFlexibleReplacer;

impl ReplacementStrategy for IndentationFlexibleReplacer {
    fn replace(&self, content: &str, old: &str, new: &str, replace_all: bool) -> ReplaceResult {
        // Detect common indentation in old string
        let old_lines: Vec<&str> = old.lines().collect();
        if old_lines.is_empty() {
            return ReplaceResult { content: content.to_string(), count: 0 };
        }
        
        // Find minimum indentation in old string
        let min_indent = old_lines.iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);
        
        // Strip common indentation from old string
        let stripped_old: Vec<String> = old_lines.iter()
            .map(|line| {
                if line.trim().is_empty() {
                    line.to_string()
                } else {
                    line.chars().skip(min_indent).collect()
                }
            })
            .collect();
        
        // Try to find this pattern in content with any indentation
        let content_lines: Vec<&str> = content.lines().collect();
        let mut result_lines: Vec<String> = Vec::new();
        let mut i = 0;
        let mut count = 0;
        
        while i < content_lines.len() {
            let mut matched = true;
            let mut found_indent = 0;
            
            if i + stripped_old.len() > content_lines.len() {
                result_lines.push(content_lines[i].to_string());
                i += 1;
                continue;
            }
            
            // Try to match with flexible indentation
            for (j, stripped_line) in stripped_old.iter().enumerate() {
                let content_line = content_lines[i + j];
                
                if stripped_line.trim().is_empty() {
                    if !content_line.trim().is_empty() {
                        matched = false;
                        break;
                    }
                } else {
                    if j == 0 {
                        // Determine indentation from first line
                        found_indent = content_line.len() - content_line.trim_start().len();
                    }
                    
                    let expected_content = if stripped_line.trim().is_empty() {
                        ""
                    } else {
                        &format!("{}{}", " ".repeat(found_indent), stripped_line.trim_start())
                    };
                    
                    if content_line != expected_content {
                        matched = false;
                        break;
                    }
                }
            }
            
            if matched {
                // Replace with new content, maintaining indentation
                let indent_str = " ".repeat(found_indent);
                for new_line in new.lines() {
                    if new_line.trim().is_empty() {
                        result_lines.push("".to_string());
                    } else {
                        result_lines.push(format!("{}{}", indent_str, new_line.trim_start()));
                    }
                }
                i += stripped_old.len();
                count += 1;
                
                if !replace_all {
                    result_lines.extend(content_lines[i..].iter().map(|s| s.to_string()));
                    break;
                }
            } else {
                result_lines.push(content_lines[i].to_string());
                i += 1;
            }
        }
        
        ReplaceResult {
            content: result_lines.join("\n"),
            count,
        }
    }
}