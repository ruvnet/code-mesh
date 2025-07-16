//! Enhanced Read tool implementation
//! Features chunked reading, image detection, file suggestions, and comprehensive metadata

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use mime_guess::MimeGuess;
use std::collections::VecDeque;

use super::{Tool, ToolContext, ToolResult, ToolError};

const DEFAULT_READ_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

/// Tool for reading file contents
pub struct ReadTool;

#[derive(Debug, Deserialize)]
struct ReadParams {
    #[serde(rename = "filePath")]
    file_path: String,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for ReadTool {
    fn id(&self) -> &str {
        "read"
    }
    
    fn description(&self) -> &str {
        "Read contents of a file with optional line offset and limit"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "The absolute path to the file to read"
                },
                "offset": {
                    "type": "number",
                    "description": "The line number to start reading from (0-based)"
                },
                "limit": {
                    "type": "number",
                    "description": "The number of lines to read. Only provide if the file is too large to read at once."
                }
            },
            "required": ["filePath"]
        })
    }
    
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ReadParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Resolve path relative to working directory
        let path = if PathBuf::from(&params.file_path).is_absolute() {
            PathBuf::from(&params.file_path)
        } else {
            ctx.working_directory.join(&params.file_path)
        };
        
        // Check if file exists and provide suggestions if not
        if !path.exists() {
            let suggestions = self.suggest_similar_files(&path).await;
            let error_msg = if suggestions.is_empty() {
                format!("File not found: {}", path.display())
            } else {
                format!(
                    "File not found: {}\n\nDid you mean one of these?\n{}",
                    path.display(),
                    suggestions.join("\n")
                )
            };
            return Err(ToolError::ExecutionFailed(error_msg));
        }
        
        // Check if it's an image file
        if let Some(image_type) = self.detect_image_type(&path) {
            return Err(ToolError::ExecutionFailed(format!(
                "This is an image file of type: {}\nUse a different tool to process images",
                image_type
            )));
        }
        
        // Read file contents with chunking for large files
        let content = match self.read_file_contents(&path).await {
            Ok(content) => content,
            Err(e) => return Err(ToolError::ExecutionFailed(format!("Failed to read file: {}", e))),
        };
        
        // Process lines with offset and limit
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        
        let limit = params.limit.unwrap_or(DEFAULT_READ_LIMIT);
        let offset = params.offset.unwrap_or(0);
        
        let start = offset.min(total_lines);
        let end = (start + limit).min(total_lines);
        
        // Format output with line numbers, truncating long lines
        let mut output_lines = Vec::new();
        output_lines.push("<file>".to_string());
        
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_num = start + i + 1;
            let truncated_line = if line.len() > MAX_LINE_LENGTH {
                format!("{}...", &line[..MAX_LINE_LENGTH])
            } else {
                line.to_string()
            };
            output_lines.push(format!("{:05}| {}", line_num, truncated_line));
        }
        
        if total_lines > end {
            output_lines.push(format!(
                "\n(File has more lines. Use 'offset' parameter to read beyond line {})",
                end
            ));
        }
        
        output_lines.push("</file>".to_string());
        
        // Generate preview (first 20 lines for metadata)
        let preview = lines
            .iter()
            .take(20)
            .map(|line| {
                if line.len() > 100 {
                    format!("{}...", &line[..100])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        // Calculate relative path for title
        let title = path
            .strip_prefix(&ctx.working_directory)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        
        // Prepare comprehensive metadata
        let metadata = json!({
            "path": path.to_string_lossy(),
            "relative_path": title,
            "total_lines": total_lines,
            "lines_read": end - start,
            "offset": start,
            "limit": limit,
            "encoding": "utf-8",
            "file_size": content.len(),
            "preview": preview,
            "truncated_lines": lines[start..end].iter().any(|line| line.len() > MAX_LINE_LENGTH)
        });
        
        Ok(ToolResult {
            title,
            metadata,
            output: output_lines.join("\n"),
        })
    }
}

impl ReadTool {
    /// Detect if a file is an image based on its extension
    fn detect_image_type(&self, path: &Path) -> Option<&'static str> {
        let extension = path.extension()?.to_str()?.to_lowercase();
        match extension.as_str() {
            "jpg" | "jpeg" => Some("JPEG"),
            "png" => Some("PNG"),
            "gif" => Some("GIF"),
            "bmp" => Some("BMP"),
            "svg" => Some("SVG"),
            "webp" => Some("WebP"),
            "tiff" | "tif" => Some("TIFF"),
            "ico" => Some("ICO"),
            _ => {
                // Also check MIME type as fallback
                let mime = MimeGuess::from_path(path).first();
                if let Some(mime) = mime {
                    if mime.type_() == mime_guess::mime::IMAGE {
                        Some("Image")
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
    
    /// Read file contents with proper error handling
    async fn read_file_contents(&self, path: &Path) -> Result<String, std::io::Error> {
        // Check file metadata first
        let metadata = fs::metadata(path).await?;
        
        if metadata.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path is a directory, not a file"
            ));
        }
        
        // For very large files, we might want to limit reading
        if metadata.len() > 100_000_000 { // 100MB limit
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File too large to read (>100MB). Consider using offset and limit parameters."
            ));
        }
        
        fs::read_to_string(path).await
    }
    
    /// Suggest similar files when target file is not found
    async fn suggest_similar_files(&self, target_path: &Path) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        let Some(parent_dir) = target_path.parent() else {
            return suggestions;
        };
        
        let Some(target_name) = target_path.file_name().and_then(|n| n.to_str()) else {
            return suggestions;
        };
        
        // Read directory entries
        let Ok(mut entries) = fs::read_dir(parent_dir).await else {
            return suggestions;
        };
        
        let target_lower = target_name.to_lowercase();
        
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                let name_lower = name.to_lowercase();
                
                // Check for similar names (contains or is contained)
                if name_lower.contains(&target_lower) || target_lower.contains(&name_lower) {
                    if let Some(full_path) = parent_dir.join(&name).to_str() {
                        suggestions.push(full_path.to_string());
                        
                        // Limit suggestions to avoid overwhelming output
                        if suggestions.len() >= 3 {
                            break;
                        }
                    }
                }
            }
        }
        
        suggestions
    }
}