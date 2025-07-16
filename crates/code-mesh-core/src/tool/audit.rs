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
        };
        
        self.write_log_entry(&entry).await?;
        Ok(entry_id)
    }
    
    /// Log the completion of a tool execution
    pub async fn log_tool_completion(
        &self,
        entry_id: &str,
        result: &ToolResult,
        execution_time_ms: u64,
    ) -> Result<(), ToolError> {
        if !self.enabled {
            return Ok(());
        }
        
        self.update_log_entry(
            entry_id,
            ExecutionStatus::Completed,
            Some(result.metadata.clone()),
            None,
            Some(execution_time_ms),
        ).await
    }
    
    /// Log a tool execution failure
    pub async fn log_tool_failure(
        &self,
        entry_id: &str,
        error: &ToolError,
        execution_time_ms: u64,
    ) -> Result<(), ToolError> {
        if !self.enabled {
            return Ok(());
        }
        
        let status = match error {
            ToolError::Aborted => ExecutionStatus::Aborted,
            ToolError::PermissionDenied(_) => ExecutionStatus::PermissionDenied,
            _ => ExecutionStatus::Failed,
        };
        
        self.update_log_entry(
            entry_id,
            status,
            None,
            Some(error.to_string()),
            Some(execution_time_ms),
        ).await
    }
    
    /// Get audit logs matching criteria
    pub async fn get_logs(
        &self,
        session_id: Option<&str>,
        tool_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Vec<AuditLogEntry> {
        let logs = self.in_memory_logs.read().await;
        
        logs.iter()
            .filter(|entry| {
                if let Some(sid) = session_id {
                    if entry.session_id != sid {
                        return false;
                    }
                }
                
                if let Some(tid) = tool_id {
                    if entry.tool_id != tid {
                        return false;
                    }
                }
                
                if let Some(start) = start_time {
                    if entry.timestamp < start {
                        return false;
                    }
                }
                
                if let Some(end) = end_time {
                    if entry.timestamp > end {
                        return false;
                    }
                }
                
                true
            })
            .take(limit.unwrap_or(usize::MAX))
            .cloned()
            .collect()
    }
    
    /// Get audit statistics
    pub async fn get_statistics(&self) -> AuditStatistics {
        let logs = self.in_memory_logs.read().await;
        
        let mut stats = AuditStatistics {
            total_entries: logs.len(),
            by_tool: HashMap::new(),
            by_status: HashMap::new(),
            by_risk_level: HashMap::new(),
            average_execution_time_ms: 0.0,
            total_execution_time_ms: 0,
        };
        
        let mut total_time = 0u64;
        let mut completed_count = 0;
        
        for entry in logs.iter() {
            // Count by tool
            *stats.by_tool.entry(entry.tool_id.clone()).or_insert(0) += 1;
            
            // Count by status
            let status_key = format!("{:?}", entry.status);
            *stats.by_status.entry(status_key).or_insert(0) += 1;
            
            // Count by risk level
            if let Some(risk) = &entry.risk_level {
                let risk_key = format!("{:?}", risk);
                *stats.by_risk_level.entry(risk_key).or_insert(0) += 1;
            }
            
            // Calculate execution time
            if let Some(time) = entry.execution_time_ms {
                total_time += time;
                completed_count += 1;
            }
        }
        
        stats.total_execution_time_ms = total_time;
        if completed_count > 0 {
            stats.average_execution_time_ms = total_time as f64 / completed_count as f64;
        }
        
        stats
    }
    
    /// Clear old audit logs
    pub async fn cleanup_old_logs(&self, older_than: DateTime<Utc>) -> usize {
        let mut logs = self.in_memory_logs.write().await;
        let original_count = logs.len();
        
        logs.retain(|entry| entry.timestamp >= older_than);
        
        original_count - logs.len()
    }
    
    /// Create system context information
    async fn create_system_context(&self, ctx: &ToolContext) -> SystemContext {
        SystemContext {
            working_directory: ctx.working_directory.clone(),
            platform: std::env::consts::OS.to_string(),
            hostname: hostname::get().ok().and_then(|h| h.into_string().ok()),
            process_id: std::process::id(),
            environment_hash: self.hash_environment(),
        }
    }
    
    /// Create a hash of relevant environment variables
    fn hash_environment(&self) -> Option<String> {
        use std::collections::BTreeMap;
        use sha2::{Sha256, Digest};
        
        let relevant_vars = ["PATH", "HOME", "USER", "USERNAME", "SHELL"];
        let mut env_map = BTreeMap::new();
        
        for var in &relevant_vars {
            if let Ok(value) = std::env::var(var) {
                env_map.insert(*var, value);
            }
        }
        
        if env_map.is_empty() {
            return None;
        }
        
        let serialized = serde_json::to_string(&env_map).ok()?;
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        Some(format!("{:x}", hasher.finalize()))
    }
    
    /// Write a log entry to storage
    async fn write_log_entry(&self, entry: &AuditLogEntry) -> Result<(), ToolError> {
        // Add to in-memory storage
        {
            let mut logs = self.in_memory_logs.write().await;
            logs.push(entry.clone());
            
            // Trim if over limit
            if logs.len() > self.max_memory_entries {
                logs.remove(0);
            }
        }
        
        // Write to file if configured
        if let Some(log_path) = &self.log_file_path {
            let log_line = serde_json::to_string(entry)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize log entry: {}", e)))?;
            
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to open audit log file: {}", e)))?;
            
            file.write_all(format!("{}\n", log_line).as_bytes())
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write to audit log: {}", e)))?;
            
            file.flush().await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to flush audit log: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Update an existing log entry
    async fn update_log_entry(
        &self,
        entry_id: &str,
        status: ExecutionStatus,
        result_metadata: Option<Value>,
        error_details: Option<String>,
        execution_time_ms: Option<u64>,
    ) -> Result<(), ToolError> {
        let mut logs = self.in_memory_logs.write().await;
        
        if let Some(entry) = logs.iter_mut().find(|e| e.entry_id == entry_id) {
            entry.status = status;
            entry.result_metadata = result_metadata;
            entry.error_details = error_details;
            entry.execution_time_ms = execution_time_ms;
            
            // Write updated entry to file if configured
            if self.log_file_path.is_some() {
                self.write_log_entry(entry).await?;
            }
        }
        
        Ok(())
    }
}

/// Audit statistics summary
#[derive(Debug, Clone, Serialize)]
pub struct AuditStatistics {
    pub total_entries: usize,
    pub by_tool: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
    pub by_risk_level: HashMap<String, usize>,
    pub average_execution_time_ms: f64,
    pub total_execution_time_ms: u64,
}

/// Helper function to determine operation type from tool ID
pub fn operation_type_from_tool(tool_id: &str) -> OperationType {
    match tool_id {
        "read" => OperationType::FileRead,
        "write" => OperationType::FileWrite,
        "edit" | "multiedit" => OperationType::FileEdit,
        "bash" => OperationType::CommandExecution,
        "web_fetch" | "web_search" => OperationType::NetworkRequest,
        "grep" | "glob" => OperationType::SystemQuery,
        _ => OperationType::Other(tool_id.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AuditLogger::new();
        
        let ctx = ToolContext {
            session_id: "test_session".to_string(),
            message_id: "test_message".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        // Log tool start
        let entry_id = logger.log_tool_start(
            "test_tool",
            OperationType::FileRead,
            &ctx,
            serde_json::json!({"test": "value"}),
            Some(RiskLevel::Low),
        ).await.unwrap();
        
        // Log completion
        let result = ToolResult {
            title: "Test".to_string(),
            metadata: serde_json::json!({"result": "success"}),
            output: "Test output".to_string(),
        };
        
        logger.log_tool_completion(&entry_id, &result, 100).await.unwrap();
        
        // Check logs
        let logs = logger.get_logs(Some("test_session"), None, None, None, None).await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].tool_id, "test_tool");
        assert!(matches!(logs[0].status, ExecutionStatus::Completed));
    }
    
    #[tokio::test]
    async fn test_audit_statistics() {
        let logger = AuditLogger::new();
        
        let ctx = ToolContext {
            session_id: "test_session".to_string(),
            message_id: "test_message".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        // Create multiple log entries
        for i in 0..3 {
            let entry_id = logger.log_tool_start(
                "test_tool",
                OperationType::FileRead,
                &ctx,
                serde_json::json!({"test": i}),
                Some(RiskLevel::Low),
            ).await.unwrap();
            
            let result = ToolResult {
                title: "Test".to_string(),
                metadata: serde_json::json!({"result": "success"}),
                output: "Test output".to_string(),
            };
            
            logger.log_tool_completion(&entry_id, &result, 100 + i * 50).await.unwrap();
        }
        
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.by_tool.get("test_tool"), Some(&3));
        assert!(stats.average_execution_time_ms > 0.0);
    }
    
    #[tokio::test]
    async fn test_file_logging() {
        let temp_file = NamedTempFile::new().unwrap();
        let log_path = temp_file.path().to_path_buf();
        
        let logger = AuditLogger::with_file(log_path.clone());
        
        let ctx = ToolContext {
            session_id: "test_session".to_string(),
            message_id: "test_message".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        logger.log_tool_start(
            "test_tool",
            OperationType::FileRead,
            &ctx,
            serde_json::json!({"test": "value"}),
            Some(RiskLevel::Low),
        ).await.unwrap();
        
        // Check that file was written
        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(content.contains("test_tool"));
        assert!(content.contains("test_session"));
    }
}