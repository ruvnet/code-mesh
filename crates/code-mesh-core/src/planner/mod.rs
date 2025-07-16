//! Task planning for Code Mesh

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Planner trait for task decomposition and planning
#[async_trait]
pub trait Planner: Send + Sync {
    /// Create a plan from a high-level task description
    async fn plan(&self, task: &str, context: PlanContext) -> crate::Result<Plan>;
    
    /// Update a plan based on progress
    async fn update_plan(&self, plan: &mut Plan, progress: &Progress) -> crate::Result<()>;
}

/// Planning context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanContext {
    pub available_tools: Vec<String>,
    pub constraints: Vec<String>,
    pub preferences: serde_json::Value,
}

/// Execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub steps: Vec<Step>,
    pub dependencies: Vec<Dependency>,
}

/// Plan step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub description: String,
    pub tool: Option<String>,
    pub parameters: serde_json::Value,
    pub expected_outcome: String,
}

/// Step dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Sequential,
    Parallel,
    Conditional,
}

/// Progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub completed_steps: Vec<String>,
    pub failed_steps: Vec<(String, String)>, // (step_id, error)
    pub current_step: Option<String>,
}