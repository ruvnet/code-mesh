//! Tool system for Code Mesh

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Tool trait for implementing various tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique identifier for the tool
    fn id(&self) -> &str;
    
    /// Human-readable description of the tool
    fn description(&self) -> &str;
    
    /// JSON Schema for the tool's parameters
    fn parameters_schema(&self) -> Value;
    
    /// Execute the tool with given parameters and context
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError>;
}

/// Context provided to tools during execution
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub message_id: String,
    pub abort_signal: tokio::sync::watch::Receiver<bool>,
    pub working_directory: std::path::PathBuf,
}

/// Result returned by tool execution
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub title: String,
    pub metadata: Value,
    pub output: String,
}

/// Tool execution errors
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Operation aborted")]
    Aborted,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Tool registry for managing available tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.id().to_string(), tool);
    }
    
    pub fn get(&self, id: &str) -> Option<&Box<dyn Tool>> {
        self.tools.get(id)
    }
    
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
    
    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| {
            ToolDefinition {
                name: tool.id().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters_schema(),
            }
        }).collect()
    }
}

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}