//! Tool system for Code Mesh
//! Comprehensive tool system with file operations, process execution, monitoring, and security

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// Core modules
pub mod audit;
pub mod file_watcher;
pub mod permission;

// File operation tools
pub mod read;
pub mod write;
pub mod edit;
pub mod multiedit;

// Process and system tools
pub mod bash;

// Search and pattern matching tools
pub mod grep;
pub mod glob;

// Task management tools
pub mod task;
pub mod todo;

// Web and HTTP tools
pub mod http;
pub mod web;

// Re-exports for easy access
pub use audit::{AuditLogger, AuditLogEntry, OperationType, ExecutionStatus, AuditStatistics, operation_type_from_tool};
pub use file_watcher::{FileWatcherTool, FileChangeEvent};
pub use permission::{PermissionManager, PermissionProvider, PermissionRequest, RiskLevel, 
                     InteractivePermissionProvider, AutoApprovePermissionProvider, 
                     create_permission_request, PermissionResult};

// Tool implementations
pub use read::ReadTool;
pub use write::WriteTool;
pub use edit::EditTool;
pub use multiedit::MultiEditTool;
pub use bash::BashTool;
pub use grep::GrepTool;
pub use glob::{GlobTool, GlobAdvancedTool};
pub use task::TaskTool;
pub use web::{WebFetchTool, WebSearchTool};
pub use todo::TodoTool;

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
    audit_logger: Option<AuditLogger>,
    permission_manager: Option<PermissionManager>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            audit_logger: None,
            permission_manager: None,
        }
    }

    /// Create a registry with all default tools registered
    pub fn with_defaults() -> Result<Self, ToolError> {
        let mut registry = Self::new();
        
        // Register core file operation tools
        registry.register(Box::new(ReadTool));
        registry.register(Box::new(WriteTool));
        registry.register(Box::new(EditTool));
        registry.register(Box::new(MultiEditTool));
        
        // Register search and pattern matching tools
        registry.register(Box::new(GrepTool));
        registry.register(Box::new(GlobTool));
        registry.register(Box::new(GlobAdvancedTool));
        
        // Register process execution tools
        registry.register(Box::new(BashTool));
        
        // Register monitoring tools
        registry.register(Box::new(FileWatcherTool::new()));
        
        // Register web tools (conditionally)
        #[cfg(not(feature = "wasm"))]
        {
            if let Ok(web_fetch) = WebFetchTool::new() {
                registry.register(Box::new(web_fetch));
            }
            if let Ok(web_search) = WebSearchTool::new() {
                registry.register(Box::new(web_search));
            }
        }
        
        // Register task and todo tools
        registry.register(Box::new(TaskTool::new()));
        registry.register(Box::new(TodoTool::new()));
        
        Ok(registry)
    }
    
    /// Create a registry with minimal tools for WASM environments
    #[cfg(feature = "wasm")]
    pub fn with_wasm_tools() -> Result<Self, ToolError> {
        let mut registry = Self::new();
        
        // Only register tools that work in WASM
        registry.register(Box::new(ReadTool));
        registry.register(Box::new(WriteTool));
        registry.register(Box::new(EditTool));
        registry.register(Box::new(MultiEditTool));
        registry.register(Box::new(GrepTool));
        registry.register(Box::new(GlobTool));
        
        // Web tools can work in WASM
        if let Ok(web_fetch) = WebFetchTool::new() {
            registry.register(Box::new(web_fetch));
        }
        if let Ok(web_search) = WebSearchTool::new() {
            registry.register(Box::new(web_search));
        }
        
        Ok(registry)
    }
    
    /// Set an audit logger for the registry
    pub fn with_audit_logger(mut self, logger: AuditLogger) -> Self {
        self.audit_logger = Some(logger);
        self
    }
    
    /// Set a permission manager for the registry
    pub fn with_permission_manager(mut self, manager: PermissionManager) -> Self {
        self.permission_manager = Some(manager);
        self
    }
    
    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.id().to_string(), tool);
    }
    
    /// Get a tool by ID
    pub fn get(&self, id: &str) -> Option<&Box<dyn Tool>> {
        self.tools.get(id)
    }
    
    /// List all tool IDs
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get tool definitions for LLM function calling
    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| {
            ToolDefinition {
                name: tool.id().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters_schema(),
            }
        }).collect()
    }
    
    /// Execute a tool with full audit and permission support
    pub async fn execute_tool(
        &self,
        tool_id: &str,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let tool = self.get(tool_id)
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Tool '{}' not found", tool_id)))?;
        
        let start_time = std::time::Instant::now();
        
        // Log tool start if audit logger is available
        let audit_entry_id = if let Some(logger) = &self.audit_logger {
            let operation_type = operation_type_from_tool(tool_id);
            let risk_level = self.assess_tool_risk(tool_id, &args);
            
            Some(logger.log_tool_start(
                tool_id,
                operation_type,
                &ctx,
                args.clone(),
                risk_level,
            ).await?)
        } else {
            None
        };
        
        // Execute the tool
        let result = tool.execute(args, ctx).await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // Log result if audit logger is available
        if let Some(logger) = &self.audit_logger {
            if let Some(entry_id) = audit_entry_id {
                match &result {
                    Ok(tool_result) => {
                        logger.log_tool_completion(&entry_id, tool_result, execution_time).await?;
                    }
                    Err(error) => {
                        logger.log_tool_failure(&entry_id, error, execution_time).await?;
                    }
                }
            }
        }
        
        result
    }
    
    /// Assess the risk level of a tool operation
    fn assess_tool_risk(&self, tool_id: &str, _args: &Value) -> Option<RiskLevel> {
        match tool_id {
            "read" | "grep" | "glob" => Some(RiskLevel::Low),
            "write" | "edit" | "multiedit" => Some(RiskLevel::Medium),
            "bash" => Some(RiskLevel::High),
            "web_fetch" | "web_search" => Some(RiskLevel::Medium),
            _ => Some(RiskLevel::Low),
        }
    }
    
    /// Get audit statistics if audit logger is available
    pub async fn get_audit_statistics(&self) -> Option<AuditStatistics> {
        if let Some(logger) = &self.audit_logger {
            Some(logger.get_statistics().await)
        } else {
            None
        }
    }
}

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Configuration for tool system
#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub enable_audit_logging: bool,
    pub audit_log_path: Option<std::path::PathBuf>,
    pub permission_provider: PermissionProviderConfig,
    pub security_mode: SecurityMode,
}

#[derive(Debug, Clone)]
pub enum PermissionProviderConfig {
    AutoApprove,
    Interactive { auto_approve_low_risk: bool },
    Disabled,
}

#[derive(Debug, Clone)]
pub enum SecurityMode {
    Strict,    // High security, limited tool access
    Balanced,  // Moderate security with user prompts
    Permissive, // Low security, most operations allowed
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enable_audit_logging: true,
            audit_log_path: None,
            permission_provider: PermissionProviderConfig::Interactive { 
                auto_approve_low_risk: true 
            },
            security_mode: SecurityMode::Balanced,
        }
    }
}

/// Factory for creating configured tool registries
pub struct ToolRegistryFactory;

impl ToolRegistryFactory {
    /// Create a tool registry with the given configuration
    pub fn create_with_config(config: ToolConfig) -> Result<ToolRegistry, ToolError> {
        let mut registry = ToolRegistry::with_defaults()?;
        
        // Configure audit logging
        if config.enable_audit_logging {
            let logger = if let Some(log_path) = config.audit_log_path {
                AuditLogger::with_file(log_path)
            } else {
                AuditLogger::new()
            };
            registry = registry.with_audit_logger(logger);
        }
        
        // Configure permission management
        let permission_manager = match config.permission_provider {
            PermissionProviderConfig::AutoApprove => {
                PermissionManager::new(Box::new(AutoApprovePermissionProvider))
            }
            PermissionProviderConfig::Interactive { auto_approve_low_risk } => {
                PermissionManager::new(Box::new(
                    InteractivePermissionProvider::new(auto_approve_low_risk)
                ))
            }
            PermissionProviderConfig::Disabled => {
                PermissionManager::new(Box::new(AutoApprovePermissionProvider))
            }
        };
        
        registry = registry.with_permission_manager(permission_manager);
        
        Ok(registry)
    }
    
    /// Create a development-friendly tool registry
    pub fn create_for_development() -> Result<ToolRegistry, ToolError> {
        let config = ToolConfig {
            enable_audit_logging: true,
            audit_log_path: None,
            permission_provider: PermissionProviderConfig::AutoApprove,
            security_mode: SecurityMode::Permissive,
        };
        
        Self::create_with_config(config)
    }
    
    /// Create a production-ready tool registry
    pub fn create_for_production(audit_log_path: std::path::PathBuf) -> Result<ToolRegistry, ToolError> {
        let config = ToolConfig {
            enable_audit_logging: true,
            audit_log_path: Some(audit_log_path),
            permission_provider: PermissionProviderConfig::Interactive { 
                auto_approve_low_risk: false 
            },
            security_mode: SecurityMode::Strict,
        };
        
        Self::create_with_config(config)
    }
    
    /// Create a WASM-compatible tool registry
    #[cfg(feature = "wasm")]
    pub fn create_for_wasm() -> Result<ToolRegistry, ToolError> {
        let mut registry = ToolRegistry::with_wasm_tools()?;
        
        // Minimal configuration for WASM
        let logger = AuditLogger::new();
        let permission_manager = PermissionManager::new(Box::new(AutoApprovePermissionProvider));
        
        registry = registry
            .with_audit_logger(logger)
            .with_permission_manager(permission_manager);
        
        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_tool_registry_creation() {
        let registry = ToolRegistry::with_defaults().unwrap();
        let tools = registry.list();
        
        // Should have all basic tools
        assert!(tools.contains(&"read"));
        assert!(tools.contains(&"write"));
        assert!(tools.contains(&"edit"));
        assert!(tools.contains(&"bash"));
        assert!(tools.contains(&"grep"));
        assert!(tools.contains(&"glob"));
    }
    
    #[tokio::test]
    async fn test_tool_execution() {
        let registry = ToolRegistry::with_defaults().unwrap();
        
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "test".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: std::env::current_dir().unwrap(),
        };
        
        // Test read tool with current directory
        let args = serde_json::json!({
            "filePath": std::env::current_dir().unwrap().join("Cargo.toml").to_string_lossy()
        });
        
        let result = registry.execute_tool("read", args, ctx).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_factory_configurations() {
        // Test development configuration
        let dev_registry = ToolRegistryFactory::create_for_development();
        assert!(dev_registry.is_ok());
        
        // Test production configuration
        let temp_path = std::env::temp_dir().join("test_audit.log");
        let prod_registry = ToolRegistryFactory::create_for_production(temp_path);
        assert!(prod_registry.is_ok());
    }
    
    #[test]
    fn test_tool_definitions() {
        let registry = ToolRegistry::with_defaults().unwrap();
        let definitions = registry.get_definitions();
        
        assert!(!definitions.is_empty());
        
        // Check that all definitions have required fields
        for def in definitions {
            assert!(!def.name.is_empty());
            assert!(!def.description.is_empty());
            assert!(def.parameters.is_object());
        }
    }
}