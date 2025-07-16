//! Enhanced Write tool implementation
//! Features atomic writes, backup creation, permission system, and comprehensive validation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;
use chrono::Utc;

use super::{Tool, ToolContext, ToolResult, ToolError};
use super::permission::{PermissionRequest, RiskLevel, create_permission_request};

/// Tool for writing content to files
pub struct WriteTool;

#[derive(Debug, Deserialize)]
struct WriteParams {
    #[serde(rename = "filePath")]
    file_path: String,
    content: String,
    #[serde(default)]
    create_backup: Option<bool>,
    #[serde(default)]
    force_overwrite: Option<bool>,
}

#[async_trait]
impl Tool for WriteTool {
    fn id(&self) -> &str {
        "write"
    }
    
    fn description(&self) -> &str {
        "Write content to a file with atomic operations and backup support"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "The absolute path to the file to write (must be absolute, not relative)"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                },
                "createBackup": {
                    "type": "boolean",
                    "description": "Create a backup of existing file before overwriting",
                    "default": true
                },
                "forceOverwrite": {
                    "type": "boolean",
                    "description": "Force overwrite without confirmation for existing files",
                    "default": false
                }
            },
            "required": ["filePath", "content"]
        })
    }
    
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WriteParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Validate file path is absolute
        let path = if PathBuf::from(&params.file_path).is_absolute() {
            PathBuf::from(&params.file_path)
        } else {
            return Err(ToolError::InvalidParameters(
                "filePath must be absolute, not relative".to_string()
            ));
        };
        
        // Check if file already exists
        let file_exists = path.exists();
        let is_new_file = !file_exists;
        
        // Validate write permissions and safety
        self.validate_write_operation(&path, &params, &ctx).await?;
        
        // Request permission for the operation
        let risk_level = if is_new_file {
            RiskLevel::Low
        } else {
            RiskLevel::Medium
        };
        
        let permission_request = create_permission_request(
            Uuid::new_v4().to_string(),
            ctx.session_id.clone(),
            if is_new_file {
                format!("Create new file: {}", path.display())
            } else {
                format!("Overwrite existing file: {}", path.display())
            },
            risk_level,
            json!({
                "filePath": path.to_string_lossy(),
                "content": params.content.chars().take(200).collect::<String>(),
                "contentLength": params.content.len(),
                "isNewFile": is_new_file,
                "createBackup": params.create_backup.unwrap_or(true)
            }),
        );
        
        // For now, we'll skip permission checking in the basic implementation
        // In a full implementation, this would integrate with the permission system
        
        // Create backup if requested and file exists
        let backup_path = if file_exists && params.create_backup.unwrap_or(true) {
            Some(self.create_backup(&path).await?)
        } else {
            None
        };
        
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create parent directories: {}", e))
            })?;
        }
        
        // Perform atomic write
        let write_result = self.atomic_write(&path, &params.content).await;
        
        match write_result {
            Ok(()) => {
                // Success - clean up any temporary backup if we don't need it
                // (In a full implementation, we might keep backups for rollback)
                
                let line_count = params.content.lines().count();
                let byte_count = params.content.len();
                
                // Calculate relative path for display
                let relative_path = path
                    .strip_prefix(&ctx.working_directory)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                
                // Prepare comprehensive metadata
                let metadata = json!({
                    "path": path.to_string_lossy(),
                    "relative_path": relative_path,
                    "lines_written": line_count,
                    "bytes_written": byte_count,
                    "was_new_file": is_new_file,
                    "backup_created": backup_path.is_some(),
                    "backup_path": backup_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    "timestamp": Utc::now().to_rfc3339(),
                    "content_preview": params.content.lines().take(3).collect::<Vec<_>>().join("\n")
                });
                
                let action = if is_new_file { "Created" } else { "Updated" };
                
                Ok(ToolResult {
                    title: relative_path,
                    metadata,
                    output: format!(
                        "{} file with {} bytes ({} lines){}.",
                        action,
                        byte_count,
                        line_count,
                        if backup_path.is_some() { " (backup created)" } else { "" }
                    ),
                })
            }
            Err(e) => {
                // Restore from backup if write failed and we have one
                if let Some(backup) = &backup_path {
                    if let Err(restore_err) = fs::rename(backup, &path).await {
                        tracing::warn!("Failed to restore backup after write failure: {}", restore_err);
                    }
                }
                Err(e)
            }
        }
    }
}

impl WriteTool {
    /// Validate that the write operation is safe and allowed
    async fn validate_write_operation(
        &self,
        path: &Path,
        params: &WriteParams,
        ctx: &ToolContext,
    ) -> Result<(), ToolError> {
        // Check if path is within allowed directories (security check)
        if !self.is_path_allowed(path, &ctx.working_directory) {
            return Err(ToolError::PermissionDenied(format!(
                "Writing outside of working directory is not allowed: {}",
                path.display()
            )));
        }
        
        // Check for dangerous file extensions
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "exe" | "bat" | "cmd" | "com" | "scr" | "msi" => {
                    return Err(ToolError::PermissionDenied(
                        "Writing executable files is not allowed for security reasons".to_string()
                    ));
                }
                _ => {}
            }
        }
        
        // Check content size limits
        if params.content.len() > 50_000_000 { // 50MB limit
            return Err(ToolError::InvalidParameters(
                "Content too large (>50MB). Consider breaking into smaller files.".to_string()
            ));
        }
        
        // Validate content is valid UTF-8 (already ensured by serde, but good to be explicit)
        if !params.content.is_ascii() && params.content.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
            return Err(ToolError::InvalidParameters(
                "Content contains invalid control characters".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Check if a path is allowed for writing
    fn is_path_allowed(&self, target_path: &Path, working_dir: &Path) -> bool {
        // Must be within or equal to working directory
        target_path.starts_with(working_dir) || target_path == working_dir
    }
    
    /// Create a backup of an existing file
    async fn create_backup(&self, original_path: &Path) -> Result<PathBuf, ToolError> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!(
            "{}.backup.{}",
            original_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
            timestamp
        );
        
        let backup_path = original_path.parent()
            .unwrap_or(Path::new("."))
            .join(backup_name);
        
        fs::copy(original_path, &backup_path).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to create backup: {}", e))
        })?;
        
        Ok(backup_path)
    }
    
    /// Perform atomic write operation
    async fn atomic_write(&self, target_path: &Path, content: &str) -> Result<(), ToolError> {
        // Create temporary file in the same directory
        let temp_name = format!(
            ".{}.tmp.{}",
            target_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
            Uuid::new_v4().simple()
        );
        
        let temp_path = target_path.parent()
            .unwrap_or(Path::new("."))
            .join(temp_name);
        
        // Write to temporary file first
        fs::write(&temp_path, content).await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to write temporary file: {}", e))
        })?;
        
        // Atomically move temporary file to target location
        fs::rename(&temp_path, target_path).await.map_err(|e| {
            // Clean up temporary file on failure
            if let Err(cleanup_err) = std::fs::remove_file(&temp_path) {
                tracing::warn!("Failed to clean up temporary file {}: {}", temp_path.display(), cleanup_err);
            }
            ToolError::ExecutionFailed(format!("Failed to move temporary file to target: {}", e))
        })?;
        
        Ok(())
    }
}