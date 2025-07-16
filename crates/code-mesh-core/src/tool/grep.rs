//! Grep tool implementation using ripgrep

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolResult, ToolError};

/// Tool for searching file contents using ripgrep
pub struct GrepTool;

#[derive(Debug, Deserialize)]
struct GrepParams {
    pattern: String,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default = "default_output_mode")]
    output_mode: String,
    #[serde(default)]
    case_insensitive: bool,
    #[serde(default)]
    line_numbers: bool,
    #[serde(default)]
    context_before: Option<usize>,
    #[serde(default)]
    context_after: Option<usize>,
    #[serde(default)]
    max_count: Option<usize>,
}

fn default_output_mode() -> String {
    "files_with_matches".to_string()
}

#[async_trait]
impl Tool for GrepTool {
    fn id(&self) -> &str {
        "grep"
    }
    
    fn description(&self) -> &str {
        "Search for patterns in files using ripgrep"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression pattern to search for"
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g., '*.rs', '*.{js,ts}')"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: current directory)"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output format",
                    "default": "files_with_matches"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive search",
                    "default": false
                },
                "line_numbers": {
                    "type": "boolean",
                    "description": "Show line numbers",
                    "default": false
                },
                "context_before": {
                    "type": "integer",
                    "description": "Lines of context before matches"
                },
                "context_after": {
                    "type": "integer", 
                    "description": "Lines of context after matches"
                },
                "max_count": {
                    "type": "integer",
                    "description": "Maximum number of results"
                }
            },
            "required": ["pattern"]
        })
    }
    
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: GrepParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Check if ripgrep is available
        let rg_path = which::which("rg").or_else(|_| which::which("ripgrep"))
            .map_err(|_| ToolError::ExecutionFailed("ripgrep not found. Please install ripgrep.".to_string()))?;
        
        // Build command
        let mut cmd = Command::new(rg_path);
        
        // Basic options
        cmd.arg("--no-heading")
           .arg("--no-config");
        
        // Output mode
        match params.output_mode.as_str() {
            "content" => {
                if params.line_numbers {
                    cmd.arg("--line-number");
                }
            },
            "files_with_matches" => {
                cmd.arg("--files-with-matches");
            },
            "count" => {
                cmd.arg("--count");
            },
            _ => return Err(ToolError::InvalidParameters("Invalid output_mode".to_string())),
        }
        
        // Case sensitivity
        if params.case_insensitive {
            cmd.arg("--ignore-case");
        }
        
        // Context
        if let Some(before) = params.context_before {
            cmd.arg("--before-context").arg(before.to_string());
        }
        if let Some(after) = params.context_after {
            cmd.arg("--after-context").arg(after.to_string());
        }
        
        // Max count
        if let Some(max) = params.max_count {
            cmd.arg("--max-count").arg(max.to_string());
        }
        
        // Glob pattern
        if let Some(glob) = &params.glob {
            cmd.arg("--glob").arg(glob);
        }
        
        // Pattern
        cmd.arg(&params.pattern);
        
        // Search path
        let search_path = if let Some(path) = &params.path {
            if PathBuf::from(path).is_absolute() {
                PathBuf::from(path)
            } else {
                ctx.working_directory.join(path)
            }
        } else {
            ctx.working_directory.clone()
        };
        cmd.arg(&search_path);
        
        // Execute
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute ripgrep: {}", e)))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !output.status.success() && !stdout.is_empty() {
            // ripgrep returns non-zero when no matches found, but that's not an error
            if output.status.code() == Some(1) {
                // No matches found
                return Ok(ToolResult {
                    title: format!("No matches found for '{}'", params.pattern),
                    metadata: json!({
                        "pattern": params.pattern,
                        "matches": 0,
                        "output_mode": params.output_mode,
                    }),
                    output: "No matches found".to_string(),
                });
            } else {
                return Err(ToolError::ExecutionFailed(format!("ripgrep error: {}", stderr)));
            }
        }
        
        // Count results
        let result_count = match params.output_mode.as_str() {
            "content" => stdout.lines().count(),
            "files_with_matches" => stdout.lines().filter(|line| !line.trim().is_empty()).count(),
            "count" => stdout.lines()
                .filter_map(|line| line.split(':').last()?.parse::<usize>().ok())
                .sum(),
            _ => 0,
        };
        
        // Truncate if too long
        let truncated = stdout.len() > 10000;
        let display_output = if truncated {
            format!("{}... (truncated, {} total results)", &stdout[..10000], result_count)
        } else {
            stdout.to_string()
        };
        
        let metadata = json!({
            "pattern": params.pattern,
            "glob": params.glob,
            "path": search_path.to_string_lossy(),
            "output_mode": params.output_mode,
            "matches": result_count,
            "truncated": truncated,
        });
        
        Ok(ToolResult {
            title: format!("Found {} match{} for '{}'", 
                result_count, 
                if result_count == 1 { "" } else { "es" },
                params.pattern
            ),
            metadata,
            output: display_output,
        })
    }
}