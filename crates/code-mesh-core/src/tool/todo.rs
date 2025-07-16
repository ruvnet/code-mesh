//! Todo management tool for task tracking and dependency management

use super::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Task status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Blocked,
}

/// Task priority enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Task dependency type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Task must complete before this task can start
    BlocksStart,
    /// Task must complete before this task can complete
    BlocksCompletion,
    /// Tasks should be worked on together
    Related,
}

/// Task dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub task_id: String,
    pub dependency_type: DependencyType,
    pub description: Option<String>,
}

/// Task metadata for analytics and tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub estimated_duration: Option<chrono::Duration>,
    pub actual_duration: Option<chrono::Duration>,
    pub tags: Vec<String>,
    pub assignee: Option<String>,
    pub project: Option<String>,
    pub milestone: Option<String>,
}

/// Individual task item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub content: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub due_date: Option<DateTime<Utc>>,
    pub dependencies: Vec<TaskDependency>,
    pub metadata: TaskMetadata,
    pub progress: f32, // 0.0 to 1.0
    pub notes: Vec<String>,
}

impl Task {
    pub fn new(id: String, content: String, priority: TaskPriority) -> Self {
        let now = Utc::now();
        Self {
            id,
            content,
            status: TaskStatus::Pending,
            priority,
            created_at: now,
            updated_at: now,
            completed_at: None,
            due_date: None,
            dependencies: Vec::new(),
            metadata: TaskMetadata {
                estimated_duration: None,
                actual_duration: None,
                tags: Vec::new(),
                assignee: None,
                project: None,
                milestone: None,
            },
            progress: 0.0,
            notes: Vec::new(),
        }
    }

    pub fn update_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = Utc::now();
        
        if self.status == TaskStatus::Completed {
            self.completed_at = Some(Utc::now());
            self.progress = 1.0;
            
            // Calculate actual duration if we have a created_at time
            self.metadata.actual_duration = Some(
                self.updated_at.signed_duration_since(self.created_at)
            );
        }
    }

    pub fn add_note(&mut self, note: String) {
        self.notes.push(note);
        self.updated_at = Utc::now();
    }

    pub fn add_dependency(&mut self, dependency: TaskDependency) {
        self.dependencies.push(dependency);
        self.updated_at = Utc::now();
    }

    pub fn is_blocked_by(&self, other_task: &Task) -> bool {
        self.dependencies.iter().any(|dep| {
            dep.task_id == other_task.id && 
            matches!(dep.dependency_type, DependencyType::BlocksStart) &&
            other_task.status != TaskStatus::Completed
        })
    }

    pub fn can_start(&self, all_tasks: &HashMap<String, Task>) -> bool {
        // Check if all blocking dependencies are completed
        for dep in &self.dependencies {
            if matches!(dep.dependency_type, DependencyType::BlocksStart) {
                if let Some(blocking_task) = all_tasks.get(&dep.task_id) {
                    if blocking_task.status != TaskStatus::Completed {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Task list for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    pub session_id: String,
    pub tasks: HashMap<String, Task>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TaskList {
    pub fn new(session_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            tasks: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(task.id.clone(), task);
        self.updated_at = Utc::now();
    }

    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut Task> {
        if self.tasks.contains_key(id) {
            self.updated_at = Utc::now();
        }
        self.tasks.get_mut(id)
    }

    pub fn remove_task(&mut self, id: &str) -> Option<Task> {
        let task = self.tasks.remove(id);
        if task.is_some() {
            self.updated_at = Utc::now();
        }
        task
    }

    pub fn get_tasks_by_status(&self, status: &TaskStatus) -> Vec<&Task> {
        self.tasks.values().filter(|t| &t.status == status).collect()
    }

    pub fn get_tasks_by_priority(&self, priority: &TaskPriority) -> Vec<&Task> {
        self.tasks.values().filter(|t| &t.priority == priority).collect()
    }

    pub fn get_available_tasks(&self) -> Vec<&Task> {
        self.tasks.values()
            .filter(|t| t.status == TaskStatus::Pending && t.can_start(&self.tasks))
            .collect()
    }

    pub fn get_blocked_tasks(&self) -> Vec<&Task> {
        self.tasks.values()
            .filter(|t| t.status == TaskStatus::Pending && !t.can_start(&self.tasks))
            .collect()
    }

    pub fn get_completion_stats(&self) -> TaskStats {
        let total = self.tasks.len();
        let completed = self.get_tasks_by_status(&TaskStatus::Completed).len();
        let in_progress = self.get_tasks_by_status(&TaskStatus::InProgress).len();
        let pending = self.get_tasks_by_status(&TaskStatus::Pending).len();
        let blocked = self.get_blocked_tasks().len();
        let cancelled = self.get_tasks_by_status(&TaskStatus::Cancelled).len();

        TaskStats {
            total,
            completed,
            in_progress,
            pending,
            blocked,
            cancelled,
            completion_rate: if total > 0 { completed as f32 / total as f32 } else { 0.0 },
        }
    }
}

/// Task statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskStats {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub pending: usize,
    pub blocked: usize,
    pub cancelled: usize,
    pub completion_rate: f32,
}

/// In-memory storage for task lists
#[derive(Debug)]
pub struct TaskStorage {
    task_lists: Arc<RwLock<HashMap<String, TaskList>>>,
}

impl TaskStorage {
    pub fn new() -> Self {
        Self {
            task_lists: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_or_create_list(&self, session_id: &str) -> TaskList {
        let mut lists = self.task_lists.write().await;
        lists.entry(session_id.to_string())
            .or_insert_with(|| TaskList::new(session_id.to_string()))
            .clone()
    }

    pub async fn save_list(&self, list: TaskList) {
        let mut lists = self.task_lists.write().await;
        lists.insert(list.session_id.clone(), list);
    }

    pub async fn get_all_sessions(&self) -> Vec<String> {
        let lists = self.task_lists.read().await;
        lists.keys().cloned().collect()
    }
}

/// Todo management tool
pub struct TodoTool {
    storage: TaskStorage,
}

impl TodoTool {
    pub fn new() -> Self {
        Self {
            storage: TaskStorage::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "snake_case")]
enum TodoAction {
    List,
    Add { content: String, priority: Option<TaskPriority> },
    Update { 
        id: String, 
        status: Option<TaskStatus>, 
        priority: Option<TaskPriority>,
        content: Option<String>,
        progress: Option<f32>,
    },
    Remove { id: String },
    AddDependency { 
        task_id: String, 
        depends_on: String, 
        dependency_type: DependencyType,
        description: Option<String>,
    },
    AddNote { id: String, note: String },
    Stats,
    Export { format: Option<String> },
}

#[async_trait]
impl Tool for TodoTool {
    fn id(&self) -> &str {
        "todo"
    }
    
    fn description(&self) -> &str {
        "Comprehensive task management tool with dependency tracking, progress reporting, and analytics"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "add", "update", "remove", "add_dependency", "add_note", "stats", "export"],
                    "description": "The action to perform"
                },
                "content": {
                    "type": "string",
                    "description": "Task content (for add action)"
                },
                "priority": {
                    "type": "string",
                    "enum": ["low", "medium", "high", "critical"],
                    "description": "Task priority"
                },
                "id": {
                    "type": "string",
                    "description": "Task ID (for update, remove, add_note actions)"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "in_progress", "completed", "cancelled", "blocked"],
                    "description": "Task status (for update action)"
                },
                "progress": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": "Task progress from 0.0 to 1.0 (for update action)"
                },
                "depends_on": {
                    "type": "string",
                    "description": "ID of task this depends on (for add_dependency action)"
                },
                "dependency_type": {
                    "type": "string",
                    "enum": ["blocks_start", "blocks_completion", "related"],
                    "description": "Type of dependency (for add_dependency action)"
                },
                "note": {
                    "type": "string",
                    "description": "Note to add to task (for add_note action)"
                },
                "format": {
                    "type": "string",
                    "enum": ["json", "markdown", "csv"],
                    "description": "Export format (for export action)"
                },
                "description": {
                    "type": "string",
                    "description": "Description for dependency (for add_dependency action)"
                }
            },
            "required": ["action"]
        })
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult, ToolError> {
        let action: TodoAction = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        let mut task_list = self.storage.get_or_create_list(&ctx.session_id).await;

        match action {
            TodoAction::List => {
                let output = format_task_list(&task_list);
                let stats = task_list.get_completion_stats();
                
                Ok(ToolResult {
                    title: format!("Task List ({} tasks)", task_list.tasks.len()),
                    output,
                    metadata: json!({
                        "stats": stats,
                        "session_id": ctx.session_id,
                        "task_count": task_list.tasks.len()
                    }),
                })
            },

            TodoAction::Add { content, priority } => {
                let task_id = Uuid::new_v4().to_string();
                let priority_value = priority.unwrap_or(TaskPriority::Medium);
                let task = Task::new(task_id.clone(), content.clone(), priority_value.clone());
                
                task_list.add_task(task);
                self.storage.save_list(task_list.clone()).await;

                Ok(ToolResult {
                    title: "Task Added".to_string(),
                    output: format!("Added task: {} (ID: {})", content, task_id),
                    metadata: json!({
                        "task_id": task_id,
                        "content": content,
                        "priority": priority_value
                    }),
                })
            },

            TodoAction::Update { id, status, priority, content, progress } => {
                if let Some(task) = task_list.get_task_mut(&id) {
                    if let Some(new_status) = status {
                        task.update_status(new_status);
                    }
                    if let Some(new_priority) = priority {
                        task.priority = new_priority;
                        task.updated_at = Utc::now();
                    }
                    if let Some(new_content) = content {
                        task.content = new_content;
                        task.updated_at = Utc::now();
                    }
                    if let Some(new_progress) = progress {
                        task.progress = new_progress.clamp(0.0, 1.0);
                        task.updated_at = Utc::now();
                    }
                    
                    let task_content = task.content.clone();
                    let task_status = task.status.clone();
                    let task_priority = task.priority.clone();
                    let task_progress = task.progress;

                    self.storage.save_list(task_list).await;

                    Ok(ToolResult {
                        title: "Task Updated".to_string(),
                        output: format!("Updated task: {}", task_content),
                        metadata: json!({
                            "task_id": id,
                            "status": task_status,
                            "priority": task_priority,
                            "progress": task_progress
                        }),
                    })
                } else {
                    Err(ToolError::InvalidParameters(format!("Task not found: {}", id)))
                }
            },

            TodoAction::Remove { id } => {
                if let Some(task) = task_list.remove_task(&id) {
                    self.storage.save_list(task_list).await;

                    Ok(ToolResult {
                        title: "Task Removed".to_string(),
                        output: format!("Removed task: {}", task.content),
                        metadata: json!({
                            "task_id": id,
                            "content": task.content
                        }),
                    })
                } else {
                    Err(ToolError::InvalidParameters(format!("Task not found: {}", id)))
                }
            },

            TodoAction::AddDependency { task_id, depends_on, dependency_type, description } => {
                // Verify both tasks exist
                if !task_list.tasks.contains_key(&task_id) {
                    return Err(ToolError::InvalidParameters(format!("Task not found: {}", task_id)));
                }
                if !task_list.tasks.contains_key(&depends_on) {
                    return Err(ToolError::InvalidParameters(format!("Dependency task not found: {}", depends_on)));
                }

                // Check for circular dependencies
                if would_create_cycle(&task_list, &task_id, &depends_on) {
                    return Err(ToolError::InvalidParameters("Adding this dependency would create a circular dependency".to_string()));
                }

                if let Some(task) = task_list.get_task_mut(&task_id) {
                    let dependency = TaskDependency {
                        task_id: depends_on.clone(),
                        dependency_type: dependency_type.clone(),
                        description,
                    };
                    task.add_dependency(dependency);
                    self.storage.save_list(task_list).await;

                    Ok(ToolResult {
                        title: "Dependency Added".to_string(),
                        output: format!("Added dependency: {} depends on {}", task_id, depends_on),
                        metadata: json!({
                            "task_id": task_id,
                            "depends_on": depends_on,
                            "dependency_type": dependency_type
                        }),
                    })
                } else {
                    Err(ToolError::InvalidParameters(format!("Task not found: {}", task_id)))
                }
            },

            TodoAction::AddNote { id, note } => {
                if let Some(task) = task_list.get_task_mut(&id) {
                    task.add_note(note.clone());
                    let task_content = task.content.clone();
                    self.storage.save_list(task_list).await;

                    Ok(ToolResult {
                        title: "Note Added".to_string(),
                        output: format!("Added note to task {}: {}", task_content, note),
                        metadata: json!({
                            "task_id": id,
                            "note": note
                        }),
                    })
                } else {
                    Err(ToolError::InvalidParameters(format!("Task not found: {}", id)))
                }
            },

            TodoAction::Stats => {
                let stats = task_list.get_completion_stats();
                let available_tasks = task_list.get_available_tasks();
                let blocked_tasks = task_list.get_blocked_tasks();

                let output = format!(
                    "Task Statistics:\n\
                     Total tasks: {}\n\
                     Completed: {}\n\
                     In progress: {}\n\
                     Pending: {}\n\
                     Blocked: {}\n\
                     Cancelled: {}\n\
                     Completion rate: {:.1}%\n\n\
                     Available tasks (can start now): {}\n\
                     Blocked tasks (waiting on dependencies): {}",
                    stats.total,
                    stats.completed,
                    stats.in_progress,
                    stats.pending,
                    stats.blocked,
                    stats.cancelled,
                    stats.completion_rate * 100.0,
                    available_tasks.len(),
                    blocked_tasks.len()
                );

                Ok(ToolResult {
                    title: "Task Statistics".to_string(),
                    output,
                    metadata: json!({
                        "stats": stats,
                        "available_tasks": available_tasks.len(),
                        "blocked_tasks": blocked_tasks.len()
                    }),
                })
            },

            TodoAction::Export { format } => {
                let format = format.as_deref().unwrap_or("json");
                let output = match format {
                    "json" => serde_json::to_string_pretty(&task_list.tasks)
                        .map_err(|e| ToolError::ExecutionFailed(format!("JSON serialization failed: {}", e)))?,
                    "markdown" => export_to_markdown(&task_list),
                    "csv" => export_to_csv(&task_list)?,
                    _ => return Err(ToolError::InvalidParameters("Unsupported export format".to_string())),
                };

                Ok(ToolResult {
                    title: format!("Task Export ({})", format),
                    output,
                    metadata: json!({
                        "format": format,
                        "task_count": task_list.tasks.len(),
                        "exported_at": Utc::now()
                    }),
                })
            },
        }
    }
}

/// Format task list for display
fn format_task_list(task_list: &TaskList) -> String {
    if task_list.tasks.is_empty() {
        return "No tasks found.".to_string();
    }

    let mut output = String::new();
    let stats = task_list.get_completion_stats();
    
    output.push_str(&format!(
        "Task List ({}% complete)\n\n",
        (stats.completion_rate * 100.0) as u32
    ));

    // Group by status
    let statuses = [
        TaskStatus::InProgress,
        TaskStatus::Pending,
        TaskStatus::Blocked,
        TaskStatus::Completed,
        TaskStatus::Cancelled,
    ];

    for status in &statuses {
        let tasks = task_list.get_tasks_by_status(status);
        if !tasks.is_empty() {
            output.push_str(&format!("## {:?} ({})\n", status, tasks.len()));
            
            for task in tasks {
                let progress_bar = create_progress_bar(task.progress);
                let dependencies = if task.dependencies.is_empty() {
                    String::new()
                } else {
                    format!(" (depends on: {})", 
                        task.dependencies.iter()
                            .map(|d| &d.task_id[..8])
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };

                output.push_str(&format!(
                    "- [{}] {} {} {}{}\n",
                    if task.status == TaskStatus::Completed { "x" } else { " " },
                    task.content,
                    format!("({:?})", task.priority),
                    progress_bar,
                    dependencies
                ));

                if !task.notes.is_empty() {
                    for note in &task.notes {
                        output.push_str(&format!("  📝 {}\n", note));
                    }
                }
            }
            output.push('\n');
        }
    }

    output
}

/// Create a simple progress bar
fn create_progress_bar(progress: f32) -> String {
    let width = 10;
    let filled = (progress * width as f32) as usize;
    let empty = width - filled;
    format!("[{}{}] {:.0}%", "█".repeat(filled), "░".repeat(empty), progress * 100.0)
}

/// Export task list to markdown format
fn export_to_markdown(task_list: &TaskList) -> String {
    let mut output = format!("# Task List - {}\n\n", task_list.session_id);
    
    for task in task_list.tasks.values() {
        output.push_str(&format!(
            "## {} ({})\n\n",
            task.content,
            task.id
        ));
        
        output.push_str(&format!("- **Status**: {:?}\n", task.status));
        output.push_str(&format!("- **Priority**: {:?}\n", task.priority));
        output.push_str(&format!("- **Progress**: {:.1}%\n", task.progress * 100.0));
        output.push_str(&format!("- **Created**: {}\n", task.created_at.format("%Y-%m-%d %H:%M:%S")));
        
        if let Some(completed_at) = task.completed_at {
            output.push_str(&format!("- **Completed**: {}\n", completed_at.format("%Y-%m-%d %H:%M:%S")));
        }
        
        if !task.dependencies.is_empty() {
            output.push_str("- **Dependencies**:\n");
            for dep in &task.dependencies {
                output.push_str(&format!("  - {} ({:?})\n", dep.task_id, dep.dependency_type));
            }
        }
        
        if !task.notes.is_empty() {
            output.push_str("- **Notes**:\n");
            for note in &task.notes {
                output.push_str(&format!("  - {}\n", note));
            }
        }
        
        output.push('\n');
    }
    
    output
}

/// Export task list to CSV format
fn export_to_csv(task_list: &TaskList) -> Result<String, ToolError> {
    let mut output = "ID,Content,Status,Priority,Progress,Created,Updated,Completed,Dependencies,Notes\n".to_string();
    
    for task in task_list.tasks.values() {
        let dependencies = task.dependencies.iter()
            .map(|d| format!("{}:{:?}", d.task_id, d.dependency_type))
            .collect::<Vec<_>>()
            .join(";");
        
        let notes = task.notes.join(";");
        
        let completed = task.completed_at
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        
        output.push_str(&format!(
            "{},{},{:?},{:?},{:.2},{},{},{},{},{}\n",
            task.id,
            task.content,
            task.status,
            task.priority,
            task.progress,
            task.created_at.format("%Y-%m-%d %H:%M:%S"),
            task.updated_at.format("%Y-%m-%d %H:%M:%S"),
            completed,
            dependencies,
            notes
        ));
    }
    
    Ok(output)
}

/// Check if adding a dependency would create a circular dependency
fn would_create_cycle(task_list: &TaskList, from_task: &str, to_task: &str) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![to_task];
    
    while let Some(current) = stack.pop() {
        if current == from_task {
            return true; // Cycle detected
        }
        
        if visited.contains(current) {
            continue;
        }
        visited.insert(current);
        
        if let Some(task) = task_list.tasks.get(current) {
            for dep in &task.dependencies {
                if matches!(dep.dependency_type, DependencyType::BlocksStart | DependencyType::BlocksCompletion) {
                    stack.push(&dep.task_id);
                }
            }
        }
    }
    
    false
}

impl Default for TodoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("test-1".to_string(), "Test task".to_string(), TaskPriority::High);
        assert_eq!(task.id, "test-1");
        assert_eq!(task.content, "Test task");
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.progress, 0.0);
    }

    #[test]
    fn test_task_status_update() {
        let mut task = Task::new("test-1".to_string(), "Test task".to_string(), TaskPriority::Medium);
        task.update_status(TaskStatus::Completed);
        
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.progress, 1.0);
        assert!(task.completed_at.is_some());
        assert!(task.metadata.actual_duration.is_some());
    }

    #[test]
    fn test_task_dependencies() {
        let mut task1 = Task::new("task-1".to_string(), "First task".to_string(), TaskPriority::High);
        let task2 = Task::new("task-2".to_string(), "Second task".to_string(), TaskPriority::Medium);
        
        let dependency = TaskDependency {
            task_id: task2.id.clone(),
            dependency_type: DependencyType::BlocksStart,
            description: Some("Must complete first".to_string()),
        };
        
        task1.add_dependency(dependency);
        assert_eq!(task1.dependencies.len(), 1);
        assert!(task1.is_blocked_by(&task2));
    }

    #[test]
    fn test_cycle_detection() {
        let mut task_list = TaskList::new("test-session".to_string());
        
        let task1 = Task::new("task-1".to_string(), "Task 1".to_string(), TaskPriority::Medium);
        let mut task2 = Task::new("task-2".to_string(), "Task 2".to_string(), TaskPriority::Medium);
        
        // task2 depends on task1
        task2.add_dependency(TaskDependency {
            task_id: "task-1".to_string(),
            dependency_type: DependencyType::BlocksStart,
            description: None,
        });
        
        task_list.add_task(task1);
        task_list.add_task(task2);
        
        // Check if task1 depending on task2 would create a cycle
        assert!(would_create_cycle(&task_list, "task-1", "task-2"));
        assert!(!would_create_cycle(&task_list, "task-2", "task-1"));
    }
}