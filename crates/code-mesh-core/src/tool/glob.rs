//! Glob tool implementation for file pattern matching

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

use super::{Tool, ToolContext, ToolResult, ToolError};

/// Tool for finding files using glob patterns
pub struct GlobTool;

#[derive(Debug, Deserialize)]
struct GlobParams {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    max_results: Option<usize>,
}

#[async_trait]
impl Tool for GlobTool {
    fn id(&self) -> &str {
        "glob"
    }
    
    fn description(&self) -> &str {
        "Find files matching glob patterns"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files (e.g., '*.rs', '**/*.js', 'src/**/*.{ts,tsx}')"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current directory)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return"
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
        let params: GlobParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Determine search directory
        let search_dir = if let Some(path) = &params.path {
            if PathBuf::from(path).is_absolute() {
                PathBuf::from(path)
            } else {
                ctx.working_directory.join(path)
            }
        } else {
            ctx.working_directory.clone()
        };
        
        // Validate directory exists
        if !search_dir.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Directory not found: {}",
                search_dir.display()
            )));
        }
        
        // Build full pattern
        let full_pattern = search_dir.join(&params.pattern).to_string_lossy().to_string();
        
        // Use glob crate to find matches
        let matches: Result<Vec<_>, _> = glob::glob(&full_pattern)
            .map_err(|e| ToolError::InvalidParameters(format!("Invalid glob pattern: {}", e)))?
            .collect();
        
        let paths = matches.map_err(|e| ToolError::ExecutionFailed(format!("Glob error: {}", e)))?;
        
        // Filter and limit results
        let mut results: Vec<PathBuf> = paths.into_iter()
            .filter(|path| path.is_file())
            .collect();
        
        // Sort results for consistent output
        results.sort();
        
        // Apply limit
        if let Some(max) = params.max_results {
            results.truncate(max);
        }
        
        // Convert to relative paths for cleaner output
        let relative_results: Vec<String> = results.iter()
            .filter_map(|path| {
                path.strip_prefix(&ctx.working_directory)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .or_else(|| Some(path.to_string_lossy().to_string()))
            })
            .collect();
        
        // Create output
        let output = if relative_results.is_empty() {
            "No files found matching pattern".to_string()
        } else {
            relative_results.join("\n")
        };
        
        let metadata = json!({
            "pattern": params.pattern,
            "search_directory": search_dir.to_string_lossy(),
            "matches": relative_results.len(),
            "files": relative_results,
        });
        
        Ok(ToolResult {
            title: format!("Found {} file{} matching '{}'", 
                relative_results.len(),
                if relative_results.len() == 1 { "" } else { "s" },
                params.pattern
            ),
            metadata,
            output,
        })
    }
}

/// Enhanced glob tool with more sophisticated patterns
pub struct GlobAdvancedTool;

#[derive(Debug, Deserialize)]
struct AdvancedGlobParams {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    include_dirs: bool,
    #[serde(default)]
    case_insensitive: bool,
    #[serde(default)]
    follow_symlinks: bool,
}

#[async_trait]
impl Tool for GlobAdvancedTool {
    fn id(&self) -> &str {
        "glob_advanced"
    }
    
    fn description(&self) -> &str {
        "Advanced file pattern matching with additional options"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results"
                },
                "include_dirs": {
                    "type": "boolean",
                    "description": "Include directories in results",
                    "default": false
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive matching",
                    "default": false
                },
                "follow_symlinks": {
                    "type": "boolean",
                    "description": "Follow symbolic links",
                    "default": false
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
        let params: AdvancedGlobParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Use walkdir for more advanced traversal
        let search_dir = if let Some(path) = &params.path {
            if PathBuf::from(path).is_absolute() {
                PathBuf::from(path)
            } else {
                ctx.working_directory.join(path)
            }
        } else {
            ctx.working_directory.clone()
        };
        
        let walker = walkdir::WalkDir::new(&search_dir)
            .follow_links(params.follow_symlinks)
            .max_depth(100); // Reasonable limit
        
        let pattern = if params.case_insensitive {
            params.pattern.to_lowercase()
        } else {
            params.pattern.clone()
        };
        
        let mut matches = Vec::new();
        
        for entry in walker {
            if *ctx.abort_signal.borrow() {
                return Err(ToolError::Aborted);
            }
            
            let entry = entry.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            let path = entry.path();
            
            // Filter by type
            if !params.include_dirs && path.is_dir() {
                continue;
            }
            
            // Check pattern match
            let path_str = path.to_string_lossy();
            let check_str = if params.case_insensitive {
                path_str.to_lowercase()
            } else {
                path_str.to_string()
            };
            
            if glob::Pattern::new(&pattern)
                .map_err(|e| ToolError::InvalidParameters(e.to_string()))?
                .matches(&check_str) 
            {
                matches.push(path.to_path_buf());
                
                if let Some(max) = params.max_results {
                    if matches.len() >= max {
                        break;
                    }
                }
            }
        }
        
        // Convert to relative paths
        let relative_matches: Vec<String> = matches.iter()
            .filter_map(|path| {
                path.strip_prefix(&ctx.working_directory)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .or_else(|| Some(path.to_string_lossy().to_string()))
            })
            .collect();
        
        let output = if relative_matches.is_empty() {
            "No matches found".to_string()
        } else {
            relative_matches.join("\n")
        };
        
        let metadata = json!({
            "pattern": params.pattern,
            "search_directory": search_dir.to_string_lossy(),
            "matches": relative_matches.len(),
            "include_dirs": params.include_dirs,
            "case_insensitive": params.case_insensitive,
        });
        
        Ok(ToolResult {
            title: format!("Found {} match{}", 
                relative_matches.len(),
                if relative_matches.len() == 1 { "" } else { "es" }
            ),
            metadata,
            output,
        })
    }
}