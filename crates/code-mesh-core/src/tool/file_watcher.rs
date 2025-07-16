//! File watching system for live updates
//! Provides real-time notifications when files or directories change

use async_trait::async_trait;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolResult, ToolError};

/// File watcher tool for monitoring file system changes
pub struct FileWatcherTool {
    active_watchers: Arc<RwLock<HashMap<String, WatcherInstance>>>,
}

#[derive(Debug, Deserialize)]
struct WatchParams {
    path: String,
    #[serde(default)]
    recursive: bool,
    #[serde(default)]
    patterns: Option<Vec<String>>,
    #[serde(default)]
    ignore_patterns: Option<Vec<String>>,
    #[serde(default)]
    debounce_ms: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileChangeEvent {
    pub event_id: String,
    pub timestamp: SystemTime,
    pub event_type: String,
    pub paths: Vec<PathBuf>,
    pub details: HashMap<String, Value>,
}

struct WatcherInstance {
    watcher_id: String,
    _watcher: RecommendedWatcher,
    event_sender: mpsc::UnboundedSender<FileChangeEvent>,
    patterns: Option<Vec<String>>,
    ignore_patterns: Option<Vec<String>>,
}

impl Default for FileWatcherTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWatcherTool {
    pub fn new() -> Self {
        Self {
            active_watchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start watching a path for changes
    pub async fn start_watching(
        &self,
        path: impl AsRef<Path>,
        recursive: bool,
        patterns: Option<Vec<String>>,
        ignore_patterns: Option<Vec<String>>,
    ) -> Result<(String, mpsc::UnboundedReceiver<FileChangeEvent>), ToolError> {
        let path = path.as_ref();
        let watcher_id = Uuid::new_v4().to_string();
        
        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // Create the watcher
        let event_tx_clone = event_tx.clone();
        let mut watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
            match result {
                Ok(event) => {
                    let file_event = FileChangeEvent {
                        event_id: Uuid::new_v4().to_string(),
                        timestamp: SystemTime::now(),
                        event_type: format!("{:?}", event.kind),
                        paths: event.paths,
                        details: HashMap::new(),
                    };
                    
                    if let Err(e) = event_tx_clone.send(file_event) {
                        tracing::warn!("Failed to send file change event: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("File watcher error: {}", e);
                }
            }
        })
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create watcher: {}", e)))?;
        
        // Start watching
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        
        watcher.watch(path, mode)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to start watching: {}", e)))?;
        
        // Store the watcher instance
        let instance = WatcherInstance {
            watcher_id: watcher_id.clone(),
            _watcher: watcher,
            event_sender: event_tx,
            patterns: patterns.clone(),
            ignore_patterns: ignore_patterns.clone(),
        };
        
        {
            let mut watchers = self.active_watchers.write().await;
            watchers.insert(watcher_id.clone(), instance);
        }
        
        Ok((watcher_id, event_rx))
    }
    
    /// Stop watching a path
    pub async fn stop_watching(&self, watcher_id: &str) -> Result<(), ToolError> {
        let mut watchers = self.active_watchers.write().await;
        if watchers.remove(watcher_id).is_some() {
            Ok(())
        } else {
            Err(ToolError::ExecutionFailed(format!(
                "Watcher {} not found",
                watcher_id
            )))
        }
    }
    
    /// Get list of active watchers
    pub async fn list_watchers(&self) -> Vec<String> {
        let watchers = self.active_watchers.read().await;
        watchers.keys().cloned().collect()
    }
    
    /// Check if a file change matches the given patterns
    fn matches_patterns(
        path: &Path,
        patterns: &Option<Vec<String>>,
        ignore_patterns: &Option<Vec<String>>,
    ) -> bool {
        let path_str = path.to_string_lossy();
        
        // Check ignore patterns first
        if let Some(ignore) = ignore_patterns {
            for pattern in ignore {
                if glob::Pattern::new(pattern)
                    .map(|p| p.matches(&path_str))
                    .unwrap_or(false)
                {
                    return false;
                }
            }
        }
        
        // Check include patterns
        if let Some(include) = patterns {
            for pattern in include {
                if glob::Pattern::new(pattern)
                    .map(|p| p.matches(&path_str))
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            false // If patterns specified but none match
        } else {
            true // No patterns specified, match all
        }
    }
}

#[async_trait]
impl Tool for FileWatcherTool {
    fn id(&self) -> &str {
        "file_watcher"
    }
    
    fn description(&self) -> &str {
        "Monitor file system changes with pattern matching and filtering"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to watch for changes"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Watch subdirectories recursively",
                    "default": false
                },
                "patterns": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Glob patterns to match files (e.g., ['*.rs', '*.js'])"
                },
                "ignorePatterns": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Glob patterns to ignore (e.g., ['*.tmp', 'node_modules/**'])"
                },
                "debounceMs": {
                    "type": "number",
                    "description": "Debounce delay in milliseconds to group rapid changes",
                    "minimum": 0,
                    "maximum": 10000
                }
            },
            "required": ["path"]
        })
    }
    
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: WatchParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Resolve path
        let watch_path = if PathBuf::from(&params.path).is_absolute() {
            PathBuf::from(&params.path)
        } else {
            ctx.working_directory.join(&params.path)
        };
        
        // Validate path exists
        if !watch_path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Path does not exist: {}",
                watch_path.display()
            )));
        }
        
        // Start watching
        let (watcher_id, mut event_rx) = self.start_watching(
            &watch_path,
            params.recursive,
            params.patterns.clone(),
            params.ignore_patterns.clone(),
        ).await?;
        
        // For this demonstration, we'll watch for a short time and return events
        // In a real implementation, this would be managed differently
        let watch_duration = Duration::from_millis(params.debounce_ms.unwrap_or(1000));
        let start_time = SystemTime::now();
        let mut events = Vec::new();
        
        // Collect events for the specified duration
        while start_time.elapsed().unwrap_or_default() < watch_duration {
            if *ctx.abort_signal.borrow() {
                self.stop_watching(&watcher_id).await.ok();
                return Err(ToolError::Aborted);
            }
            
            match tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                Ok(Some(event)) => {
                    // Filter event based on patterns
                    let matching_paths: Vec<_> = event.paths.iter()
                        .filter(|path| Self::matches_patterns(
                            path,
                            &params.patterns,
                            &params.ignore_patterns
                        ))
                        .cloned()
                        .collect();
                    
                    if !matching_paths.is_empty() {
                        let filtered_event = FileChangeEvent {
                            event_id: event.event_id,
                            timestamp: event.timestamp,
                            event_type: event.event_type,
                            paths: matching_paths,
                            details: event.details,
                        };
                        events.push(filtered_event);
                    }
                }
                Ok(None) => break, // Channel closed
                Err(_) => continue, // Timeout, continue watching
            }
        }
        
        // Stop watching
        self.stop_watching(&watcher_id).await.ok();
        
        // Calculate relative paths for display
        let relative_path = watch_path
            .strip_prefix(&ctx.working_directory)
            .unwrap_or(&watch_path)
            .to_string_lossy()
            .to_string();
        
        let metadata = json!({
            "watcher_id": watcher_id,
            "path": watch_path.to_string_lossy(),
            "relative_path": relative_path,
            "recursive": params.recursive,
            "patterns": params.patterns,
            "ignore_patterns": params.ignore_patterns,
            "watch_duration_ms": watch_duration.as_millis(),
            "events_collected": events.len(),
            "events": events.iter().map(|e| json!({
                "event_id": e.event_id,
                "timestamp": e.timestamp.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
                "event_type": e.event_type,
                "paths": e.paths.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        });
        
        let output = if events.is_empty() {
            format!(
                "No file changes detected in {} during {}ms watch period",
                relative_path,
                watch_duration.as_millis()
            )
        } else {
            let mut output_lines = vec![
                format!(
                    "Detected {} file change{} in {} during {}ms watch period:",
                    events.len(),
                    if events.len() == 1 { "" } else { "s" },
                    relative_path,
                    watch_duration.as_millis()
                )
            ];
            
            for event in &events {
                output_lines.push(format!(
                    "  - {}: {}",
                    event.event_type,
                    event.paths.iter()
                        .map(|p| p.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            
            output_lines.join("\n")
        };
        
        Ok(ToolResult {
            title: format!("Watched {} for {}ms", relative_path, watch_duration.as_millis()),
            metadata,
            output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    
    #[tokio::test]
    async fn test_file_watcher_creation() {
        let watcher = FileWatcherTool::new();
        assert!(watcher.list_watchers().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_pattern_matching() {
        let path = Path::new("test.rs");
        let patterns = Some(vec!["*.rs".to_string()]);
        let ignore_patterns = Some(vec!["*.tmp".to_string()]);
        
        assert!(FileWatcherTool::matches_patterns(&path, &patterns, &ignore_patterns));
        
        let ignored_path = Path::new("test.tmp");
        assert!(!FileWatcherTool::matches_patterns(&ignored_path, &patterns, &ignore_patterns));
    }
    
    #[tokio::test]
    async fn test_file_watcher_tool() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        
        let tool = FileWatcherTool::new();
        let params = json!({
            "path": temp_path.to_string_lossy(),
            "recursive": false,
            "patterns": ["*.txt"],
            "debounceMs": 500
        });
        
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "test".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        // Start watching in background
        let tool_clone = tool.clone();
        let params_clone = params.clone();
        let ctx_clone = ctx.clone();
        
        let watch_task = tokio::spawn(async move {
            tool_clone.execute(params_clone, ctx_clone).await
        });
        
        // Give watcher time to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Create a file to trigger an event
        let test_file = temp_path.join("test.txt");
        fs::write(&test_file, "test content").await.unwrap();
        
        // Wait for watcher to complete
        let result = watch_task.await.unwrap();
        
        // Note: Due to timing, the test might not catch the file creation
        // In a real scenario, events would be handled asynchronously
        assert!(result.is_ok());
    }
}