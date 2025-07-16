//! Agent orchestration for Code Mesh

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Agent trait for AI agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent ID
    fn id(&self) -> &str;
    
    /// Get agent name
    fn name(&self) -> &str;
    
    /// Get agent capabilities
    fn capabilities(&self) -> &[String];
    
    /// Execute a task
    async fn execute(&self, task: Task) -> crate::Result<TaskResult>;
}

/// Task for an agent to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub context: serde_json::Value,
    pub dependencies: Vec<String>,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}