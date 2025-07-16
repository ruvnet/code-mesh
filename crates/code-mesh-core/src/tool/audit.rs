//! Audit logging system for tool execution
//! Provides comprehensive logging and monitoring of all tool operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{ToolContext, ToolResult, ToolError};
use super::permission::RiskLevel;

/// Audit log entry for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub entry_id: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub message_id: String,
    pub tool_id: String,
    pub operation_type: OperationType,
    pub status: ExecutionStatus,
    pub risk_level: Option<RiskLevel>,
    pub parameters: Value,
    pub result_metadata: Option<Value>,
    pub error_details: Option<String>,
    pub execution_time_ms: Option<u64>,
    pub user_context: HashMap<String, Value>,
    pub system_context: SystemContext,
}

/// Type of operation performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    FileRead,
    FileWrite,
    FileEdit,
    FileDelete,
    CommandExecution,
    NetworkRequest,
    SystemQuery,
    ProcessSpawn,
    Other(String),
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Started,
    Completed,
    Failed,
    Aborted,
    PermissionDenied,
}

/// System context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub working_directory: PathBuf,
    pub platform: String,
    pub hostname: Option<String>,
    pub process_id: u32,
    pub environment_hash: Option<String>,
}

/// Audit logger implementation
pub struct AuditLogger {
    log_file_path: Option<PathBuf>,
    in_memory_logs: Arc<RwLock<Vec<AuditLogEntry>>>,
    max_memory_entries: usize,
    enabled: bool,
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new() -> Self {
        Self {
            log_file_path: None,
            in_memory_logs: Arc::new(RwLock::new(Vec::new())),
            max_memory_entries: 10000,
            enabled: true,
        }
    }
    
    /// Create audit logger with file output
    pub fn with_file(log_file_path: PathBuf) -> Self {
        Self {
            log_file_path: Some(log_file_path),
            in_memory_logs: Arc::new(RwLock::new(Vec::new())),
            max_memory_entries: 10000,
            enabled: true,
        }
    }
    
    /// Enable or disable audit logging
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Log the start of a tool execution
    pub async fn log_tool_start(
        &self,
        tool_id: &str,
        operation_type: OperationType,
        ctx: &ToolContext,
        parameters: Value,
        risk_level: Option<RiskLevel>,
    ) -> Result<String, ToolError> {
        if !self.enabled {
            return Ok(String::new());
        }
        
        let entry_id = Uuid::new_v4().to_string();
        let entry = AuditLogEntry {
            entry_id: entry_id.clone(),
            timestamp: Utc::now(),
            session_id: ctx.session_id.clone(),
            message_id: ctx.message_id.clone(),
            tool_id: tool_id.to_string(),
            operation_type,
            status: ExecutionStatus::Started,
            risk_level,
            parameters,
            result_metadata: None,
            error_details: None,
            execution_time_ms: None,
            user_context: HashMap::new(),
            system_context: self.create_system_context(ctx).await,
        };\n        \n        self.write_log_entry(&entry).await?;\n        Ok(entry_id)\n    }\n    \n    /// Log the completion of a tool execution\n    pub async fn log_tool_completion(\n        &self,\n        entry_id: &str,\n        result: &ToolResult,\n        execution_time_ms: u64,\n    ) -> Result<(), ToolError> {\n        if !self.enabled {\n            return Ok(());\n        }\n        \n        self.update_log_entry(\n            entry_id,\n            ExecutionStatus::Completed,\n            Some(result.metadata.clone()),\n            None,\n            Some(execution_time_ms),\n        ).await\n    }\n    \n    /// Log a tool execution failure\n    pub async fn log_tool_failure(\n        &self,\n        entry_id: &str,\n        error: &ToolError,\n        execution_time_ms: u64,\n    ) -> Result<(), ToolError> {\n        if !self.enabled {\n            return Ok(());\n        }\n        \n        let status = match error {\n            ToolError::Aborted => ExecutionStatus::Aborted,\n            ToolError::PermissionDenied(_) => ExecutionStatus::PermissionDenied,\n            _ => ExecutionStatus::Failed,\n        };\n        \n        self.update_log_entry(\n            entry_id,\n            status,\n            None,\n            Some(error.to_string()),\n            Some(execution_time_ms),\n        ).await\n    }\n    \n    /// Get audit logs matching criteria\n    pub async fn get_logs(\n        &self,\n        session_id: Option<&str>,\n        tool_id: Option<&str>,\n        start_time: Option<DateTime<Utc>>,\n        end_time: Option<DateTime<Utc>>,\n        limit: Option<usize>,\n    ) -> Vec<AuditLogEntry> {\n        let logs = self.in_memory_logs.read().await;\n        \n        logs.iter()\n            .filter(|entry| {\n                if let Some(sid) = session_id {\n                    if entry.session_id != sid {\n                        return false;\n                    }\n                }\n                \n                if let Some(tid) = tool_id {\n                    if entry.tool_id != tid {\n                        return false;\n                    }\n                }\n                \n                if let Some(start) = start_time {\n                    if entry.timestamp < start {\n                        return false;\n                    }\n                }\n                \n                if let Some(end) = end_time {\n                    if entry.timestamp > end {\n                        return false;\n                    }\n                }\n                \n                true\n            })\n            .take(limit.unwrap_or(usize::MAX))\n            .cloned()\n            .collect()\n    }\n    \n    /// Get audit statistics\n    pub async fn get_statistics(&self) -> AuditStatistics {\n        let logs = self.in_memory_logs.read().await;\n        \n        let mut stats = AuditStatistics {\n            total_entries: logs.len(),\n            by_tool: HashMap::new(),\n            by_status: HashMap::new(),\n            by_risk_level: HashMap::new(),\n            average_execution_time_ms: 0.0,\n            total_execution_time_ms: 0,\n        };\n        \n        let mut total_time = 0u64;\n        let mut completed_count = 0;\n        \n        for entry in logs.iter() {\n            // Count by tool\n            *stats.by_tool.entry(entry.tool_id.clone()).or_insert(0) += 1;\n            \n            // Count by status\n            let status_key = format!(\"{:?}\", entry.status);\n            *stats.by_status.entry(status_key).or_insert(0) += 1;\n            \n            // Count by risk level\n            if let Some(risk) = &entry.risk_level {\n                let risk_key = format!(\"{:?}\", risk);\n                *stats.by_risk_level.entry(risk_key).or_insert(0) += 1;\n            }\n            \n            // Calculate execution time\n            if let Some(time) = entry.execution_time_ms {\n                total_time += time;\n                completed_count += 1;\n            }\n        }\n        \n        stats.total_execution_time_ms = total_time;\n        if completed_count > 0 {\n            stats.average_execution_time_ms = total_time as f64 / completed_count as f64;\n        }\n        \n        stats\n    }\n    \n    /// Clear old audit logs\n    pub async fn cleanup_old_logs(&self, older_than: DateTime<Utc>) -> usize {\n        let mut logs = self.in_memory_logs.write().await;\n        let original_count = logs.len();\n        \n        logs.retain(|entry| entry.timestamp >= older_than);\n        \n        original_count - logs.len()\n    }\n    \n    /// Create system context information\n    async fn create_system_context(&self, ctx: &ToolContext) -> SystemContext {\n        SystemContext {\n            working_directory: ctx.working_directory.clone(),\n            platform: std::env::consts::OS.to_string(),\n            hostname: hostname::get().ok().and_then(|h| h.into_string().ok()),\n            process_id: std::process::id(),\n            environment_hash: self.hash_environment(),\n        }\n    }\n    \n    /// Create a hash of relevant environment variables\n    fn hash_environment(&self) -> Option<String> {\n        use std::collections::BTreeMap;\n        use sha2::{Sha256, Digest};\n        \n        let relevant_vars = [\"PATH\", \"HOME\", \"USER\", \"USERNAME\", \"SHELL\"];\n        let mut env_map = BTreeMap::new();\n        \n        for var in &relevant_vars {\n            if let Ok(value) = std::env::var(var) {\n                env_map.insert(*var, value);\n            }\n        }\n        \n        if env_map.is_empty() {\n            return None;\n        }\n        \n        let serialized = serde_json::to_string(&env_map).ok()?;\n        let mut hasher = Sha256::new();\n        hasher.update(serialized.as_bytes());\n        Some(format!(\"{:x}\", hasher.finalize()))\n    }\n    \n    /// Write a log entry to storage\n    async fn write_log_entry(&self, entry: &AuditLogEntry) -> Result<(), ToolError> {\n        // Add to in-memory storage\n        {\n            let mut logs = self.in_memory_logs.write().await;\n            logs.push(entry.clone());\n            \n            // Trim if over limit\n            if logs.len() > self.max_memory_entries {\n                logs.remove(0);\n            }\n        }\n        \n        // Write to file if configured\n        if let Some(log_path) = &self.log_file_path {\n            let log_line = serde_json::to_string(entry)\n                .map_err(|e| ToolError::ExecutionFailed(format!(\"Failed to serialize log entry: {}\", e)))?;\n            \n            let mut file = OpenOptions::new()\n                .create(true)\n                .append(true)\n                .open(log_path)\n                .await\n                .map_err(|e| ToolError::ExecutionFailed(format!(\"Failed to open audit log file: {}\", e)))?;\n            \n            file.write_all(format!(\"{}\n\", log_line).as_bytes())\n                .await\n                .map_err(|e| ToolError::ExecutionFailed(format!(\"Failed to write to audit log: {}\", e)))?;\n            \n            file.flush().await\n                .map_err(|e| ToolError::ExecutionFailed(format!(\"Failed to flush audit log: {}\", e)))?;\n        }\n        \n        Ok(())\n    }\n    \n    /// Update an existing log entry\n    async fn update_log_entry(\n        &self,\n        entry_id: &str,\n        status: ExecutionStatus,\n        result_metadata: Option<Value>,\n        error_details: Option<String>,\n        execution_time_ms: Option<u64>,\n    ) -> Result<(), ToolError> {\n        let mut logs = self.in_memory_logs.write().await;\n        \n        if let Some(entry) = logs.iter_mut().find(|e| e.entry_id == entry_id) {\n            entry.status = status;\n            entry.result_metadata = result_metadata;\n            entry.error_details = error_details;\n            entry.execution_time_ms = execution_time_ms;\n            \n            // Write updated entry to file if configured\n            if self.log_file_path.is_some() {\n                self.write_log_entry(entry).await?;\n            }\n        }\n        \n        Ok(())\n    }\n}\n\n/// Audit statistics summary\n#[derive(Debug, Clone, Serialize)]\npub struct AuditStatistics {\n    pub total_entries: usize,\n    pub by_tool: HashMap<String, usize>,\n    pub by_status: HashMap<String, usize>,\n    pub by_risk_level: HashMap<String, usize>,\n    pub average_execution_time_ms: f64,\n    pub total_execution_time_ms: u64,\n}\n\n/// Helper function to determine operation type from tool ID\npub fn operation_type_from_tool(tool_id: &str) -> OperationType {\n    match tool_id {\n        \"read\" => OperationType::FileRead,\n        \"write\" => OperationType::FileWrite,\n        \"edit\" | \"multiedit\" => OperationType::FileEdit,\n        \"bash\" => OperationType::CommandExecution,\n        \"web_fetch\" | \"web_search\" => OperationType::NetworkRequest,\n        \"grep\" | \"glob\" => OperationType::SystemQuery,\n        _ => OperationType::Other(tool_id.to_string()),\n    }\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    use tempfile::NamedTempFile;\n    \n    #[tokio::test]\n    async fn test_audit_logger() {\n        let logger = AuditLogger::new();\n        \n        let ctx = ToolContext {\n            session_id: \"test_session\".to_string(),\n            message_id: \"test_message\".to_string(),\n            abort_signal: tokio::sync::watch::channel(false).1,\n            working_directory: std::env::current_dir().unwrap(),\n        };\n        \n        // Log tool start\n        let entry_id = logger.log_tool_start(\n            \"test_tool\",\n            OperationType::FileRead,\n            &ctx,\n            serde_json::json!({\"test\": \"value\"}),\n            Some(RiskLevel::Low),\n        ).await.unwrap();\n        \n        // Log completion\n        let result = ToolResult {\n            title: \"Test\".to_string(),\n            metadata: serde_json::json!({\"result\": \"success\"}),\n            output: \"Test output\".to_string(),\n        };\n        \n        logger.log_tool_completion(&entry_id, &result, 100).await.unwrap();\n        \n        // Check logs\n        let logs = logger.get_logs(Some(\"test_session\"), None, None, None, None).await;\n        assert_eq!(logs.len(), 1);\n        assert_eq!(logs[0].tool_id, \"test_tool\");\n        assert!(matches!(logs[0].status, ExecutionStatus::Completed));\n    }\n    \n    #[tokio::test]\n    async fn test_audit_statistics() {\n        let logger = AuditLogger::new();\n        \n        let ctx = ToolContext {\n            session_id: \"test_session\".to_string(),\n            message_id: \"test_message\".to_string(),\n            abort_signal: tokio::sync::watch::channel(false).1,\n            working_directory: std::env::current_dir().unwrap(),\n        };\n        \n        // Create multiple log entries\n        for i in 0..3 {\n            let entry_id = logger.log_tool_start(\n                \"test_tool\",\n                OperationType::FileRead,\n                &ctx,\n                serde_json::json!({\"test\": i}),\n                Some(RiskLevel::Low),\n            ).await.unwrap();\n            \n            let result = ToolResult {\n                title: \"Test\".to_string(),\n                metadata: serde_json::json!({\"result\": \"success\"}),\n                output: \"Test output\".to_string(),\n            };\n            \n            logger.log_tool_completion(&entry_id, &result, 100 + i * 50).await.unwrap();\n        }\n        \n        let stats = logger.get_statistics().await;\n        assert_eq!(stats.total_entries, 3);\n        assert_eq!(stats.by_tool.get(\"test_tool\"), Some(&3));\n        assert!(stats.average_execution_time_ms > 0.0);\n    }\n    \n    #[tokio::test]\n    async fn test_file_logging() {\n        let temp_file = NamedTempFile::new().unwrap();\n        let log_path = temp_file.path().to_path_buf();\n        \n        let logger = AuditLogger::with_file(log_path.clone());\n        \n        let ctx = ToolContext {\n            session_id: \"test_session\".to_string(),\n            message_id: \"test_message\".to_string(),\n            abort_signal: tokio::sync::watch::channel(false).1,\n            working_directory: std::env::current_dir().unwrap(),\n        };\n        \n        logger.log_tool_start(\n            \"test_tool\",\n            OperationType::FileRead,\n            &ctx,\n            serde_json::json!({\"test\": \"value\"}),\n            Some(RiskLevel::Low),\n        ).await.unwrap();\n        \n        // Check that file was written\n        let content = tokio::fs::read_to_string(&log_path).await.unwrap();\n        assert!(content.contains(\"test_tool\"));\n        assert!(content.contains(\"test_session\"));\n    }\n}