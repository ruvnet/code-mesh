//! Configuration management for Code Mesh Core
//!
//! This module provides a comprehensive configuration system that supports:
//! - Multiple configuration sources (files, environment, programmatic)
//! - Provider-specific configurations
//! - Tool configurations and permissions
//! - Runtime feature detection
//! - Configuration validation and defaults

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// General application settings
    pub app: AppConfig,
    
    /// Provider configurations
    pub providers: HashMap<String, ProviderConfig>,
    
    /// Tool configurations
    pub tools: ToolConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Authentication configuration
    pub auth: AuthConfig,
    
    /// Session configuration
    pub session: SessionConfig,
    
    /// Memory configuration
    pub memory: MemoryConfig,
    
    /// Agent configuration
    pub agent: AgentConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            providers: HashMap::new(),
            tools: ToolConfig::default(),
            storage: StorageConfig::default(),
            auth: AuthConfig::default(),
            session: SessionConfig::default(),
            memory: MemoryConfig::default(),
            agent: AgentConfig::default(),
        }
    }
}

/// Application-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application name
    pub name: String,
    
    /// Version information
    pub version: String,
    
    /// Data directory path
    pub data_dir: Option<PathBuf>,
    
    /// Configuration directory path
    pub config_dir: Option<PathBuf>,
    
    /// Log level
    pub log_level: String,
    
    /// Feature flags
    pub features: FeatureConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "code-mesh".to_string(),
            version: crate::VERSION.to_string(),
            data_dir: None,
            config_dir: None,
            log_level: "info".to_string(),
            features: FeatureConfig::default(),
        }
    }
}

/// Feature configuration flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Enable compression for storage
    pub compression: bool,
    
    /// Enable file watching
    pub file_watching: bool,
    
    /// Enable advanced cryptography
    pub advanced_crypto: bool,
    
    /// Enable telemetry
    pub telemetry: bool,
    
    /// Enable experimental features
    pub experimental: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            compression: crate::features::HAS_COMPRESSION,
            file_watching: crate::features::HAS_FILE_WATCHING,
            advanced_crypto: crate::features::HAS_ADVANCED_CRYPTO,
            telemetry: false,
            experimental: false,
        }
    }
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider ID
    pub id: String,
    
    /// Provider name
    pub name: String,
    
    /// Base URL for API
    pub base_url: Option<String>,
    
    /// API version
    pub api_version: Option<String>,
    
    /// Default model to use
    pub default_model: Option<String>,
    
    /// Model configurations
    pub models: HashMap<String, ModelConfig>,
    
    /// Provider-specific options
    pub options: HashMap<String, serde_json::Value>,
    
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    
    /// Timeout configuration
    pub timeout: TimeoutConfig,
    
    /// Retry configuration
    pub retry: RetryConfig,
    
    /// Whether this provider is enabled
    pub enabled: bool,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            base_url: None,
            api_version: None,
            default_model: None,
            models: HashMap::new(),
            options: HashMap::new(),
            rate_limit: RateLimitConfig::default(),
            timeout: TimeoutConfig::default(),
            retry: RetryConfig::default(),
            enabled: true,
        }
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model ID
    pub id: String,
    
    /// Model name
    pub name: String,
    
    /// Maximum context length
    pub max_context: Option<u32>,
    
    /// Maximum output tokens
    pub max_output: Option<u32>,
    
    /// Default temperature
    pub temperature: Option<f32>,
    
    /// Supports tool calling
    pub supports_tools: bool,
    
    /// Supports vision/images
    pub supports_vision: bool,
    
    /// Supports streaming
    pub supports_streaming: bool,
    
    /// Supports caching
    pub supports_caching: bool,
    
    /// Cost information
    pub cost: CostConfig,
    
    /// Model-specific options
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            max_context: None,
            max_output: None,
            temperature: None,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            supports_caching: false,
            cost: CostConfig::default(),
            options: HashMap::new(),
        }
    }
}

/// Cost configuration for models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Input tokens cost per 1K tokens
    pub input_cost_per_1k: f64,
    
    /// Output tokens cost per 1K tokens
    pub output_cost_per_1k: f64,
    
    /// Cache read cost per 1K tokens
    pub cache_read_cost_per_1k: Option<f64>,
    
    /// Cache write cost per 1K tokens
    pub cache_write_cost_per_1k: Option<f64>,
    
    /// Currency for costs
    pub currency: String,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            input_cost_per_1k: 0.0,
            output_cost_per_1k: 0.0,
            cache_read_cost_per_1k: None,
            cache_write_cost_per_1k: None,
            currency: "USD".to_string(),
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute
    pub requests_per_minute: Option<u32>,
    
    /// Tokens per minute
    pub tokens_per_minute: Option<u32>,
    
    /// Concurrent requests
    pub concurrent_requests: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: None,
            tokens_per_minute: None,
            concurrent_requests: Some(4),
        }
    }
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Connect timeout in seconds
    pub connect: u64,
    
    /// Request timeout in seconds
    pub request: u64,
    
    /// Stream timeout in seconds
    pub stream: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect: 10,
            request: 300,
            stream: 600,
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    
    /// Base delay in milliseconds
    pub base_delay: u64,
    
    /// Maximum delay in milliseconds
    pub max_delay: u64,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: 1000,
            max_delay: 60000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Tool system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Enabled tools
    pub enabled: Vec<String>,
    
    /// Disabled tools
    pub disabled: Vec<String>,
    
    /// Tool-specific configurations
    pub tool_configs: HashMap<String, ToolSpecificConfig>,
    
    /// Default permission level
    pub default_permission: String,
    
    /// Sandbox configuration
    pub sandbox: SandboxConfig,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled: vec!["read".to_string(), "write".to_string(), "edit".to_string()],
            disabled: Vec::new(),
            tool_configs: HashMap::new(),
            default_permission: "restricted".to_string(),
            sandbox: SandboxConfig::default(),
        }
    }
}

/// Tool-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpecificConfig {
    /// Permission level for this tool
    pub permission: String,
    
    /// Allowed file patterns
    pub allowed_patterns: Vec<String>,
    
    /// Denied file patterns
    pub denied_patterns: Vec<String>,
    
    /// Maximum file size in bytes
    pub max_file_size: Option<u64>,
    
    /// Tool-specific options
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for ToolSpecificConfig {
    fn default() -> Self {
        Self {
            permission: "restricted".to_string(),
            allowed_patterns: Vec::new(),
            denied_patterns: Vec::new(),
            max_file_size: None,
            options: HashMap::new(),
        }
    }
}

/// Sandbox configuration for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing
    pub enabled: bool,
    
    /// Allowed directories
    pub allowed_dirs: Vec<PathBuf>,
    
    /// Denied directories
    pub denied_dirs: Vec<PathBuf>,
    
    /// Network access allowed
    pub network_access: bool,
    
    /// Maximum execution time in seconds
    pub max_execution_time: u64,
    
    /// Maximum memory usage in MB
    pub max_memory_mb: Option<u64>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_dirs: Vec::new(),
            denied_dirs: Vec::new(),
            network_access: false,
            max_execution_time: 30,
            max_memory_mb: Some(512),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: String,
    
    /// Connection string or path
    pub connection: String,
    
    /// Enable compression
    pub compression: bool,
    
    /// Encryption configuration
    pub encryption: EncryptionConfig,
    
    /// Backup configuration
    pub backup: BackupConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "file".to_string(),
            connection: "data/storage".to_string(),
            compression: crate::features::HAS_COMPRESSION,
            encryption: EncryptionConfig::default(),
            backup: BackupConfig::default(),
        }
    }
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,
    
    /// Encryption algorithm
    pub algorithm: String,
    
    /// Key derivation method
    pub key_derivation: String,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "aes-256-gcm".to_string(),
            key_derivation: "pbkdf2".to_string(),
        }
    }
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,
    
    /// Backup interval in hours
    pub interval_hours: u64,
    
    /// Maximum number of backups to keep
    pub max_backups: u32,
    
    /// Backup directory
    pub backup_dir: Option<PathBuf>,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_hours: 24,
            max_backups: 7,
            backup_dir: None,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Storage backend for auth data
    pub storage_backend: String,
    
    /// Token refresh interval in minutes
    pub refresh_interval: u64,
    
    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            storage_backend: "encrypted_file".to_string(),
            refresh_interval: 60,
            security: SecurityConfig::default(),
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable key rotation
    pub key_rotation: bool,
    
    /// Key rotation interval in days
    pub rotation_interval_days: u64,
    
    /// Enable audit logging
    pub audit_logging: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            key_rotation: true,
            rotation_interval_days: 30,
            audit_logging: true,
        }
    }
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum number of messages per session
    pub max_messages: u32,
    
    /// Maximum session age in hours
    pub max_age_hours: u64,
    
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    
    /// Context window configuration
    pub context_window: ContextWindowConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_messages: 1000,
            max_age_hours: 24 * 7, // 1 week
            auto_save_interval: 30,
            context_window: ContextWindowConfig::default(),
        }
    }
}

/// Context window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowConfig {
    /// Maximum tokens in context
    pub max_tokens: u32,
    
    /// Context trimming strategy
    pub trim_strategy: String,
    
    /// Preserve system messages
    pub preserve_system: bool,
    
    /// Preserve recent messages count
    pub preserve_recent: u32,
}

impl Default for ContextWindowConfig {
    fn default() -> Self {
        Self {
            max_tokens: 100000,
            trim_strategy: "preserve_recent".to_string(),
            preserve_system: true,
            preserve_recent: 10,
        }
    }
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Memory backend type
    pub backend: String,
    
    /// Maximum memory entries
    pub max_entries: u32,
    
    /// Memory TTL in hours
    pub ttl_hours: u64,
    
    /// Enable memory compression
    pub compression: bool,
    
    /// Memory indexing configuration
    pub indexing: IndexingConfig,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: "hybrid".to_string(),
            max_entries: 10000,
            ttl_hours: 24 * 30, // 30 days
            compression: crate::features::HAS_COMPRESSION,
            indexing: IndexingConfig::default(),
        }
    }
}

/// Memory indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Enable semantic search
    pub semantic_search: bool,
    
    /// Enable full-text search
    pub full_text_search: bool,
    
    /// Embedding model for semantic search
    pub embedding_model: Option<String>,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            semantic_search: false,
            full_text_search: true,
            embedding_model: None,
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum concurrent agents
    pub max_concurrent: u32,
    
    /// Agent timeout in seconds
    pub timeout_seconds: u64,
    
    /// Enable agent collaboration
    pub collaboration: bool,
    
    /// Default agent capabilities
    pub default_capabilities: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            timeout_seconds: 300,
            collaboration: true,
            default_capabilities: vec![
                "read".to_string(),
                "write".to_string(),
                "execute".to_string(),
            ],
        }
    }
}

/// Configuration manager for loading and managing configurations
pub struct ConfigManager {
    config: Config,
    config_path: Option<PathBuf>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            config_path: None,
        }
    }

    /// Load configuration from file
    pub async fn load_from_file<P: Into<PathBuf>>(&mut self, path: P) -> Result<()> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::Io(e))?;
        
        let config: Config = match path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::from_str(&content)?,
            Some("toml") => toml::from_str(&content)
                .map_err(|e| Error::Other(anyhow::anyhow!("TOML parse error: {}", e)))?,
            Some("yaml") | Some("yml") => {
                return Err(Error::Other(anyhow::anyhow!("YAML support not implemented")));
            }
            _ => return Err(Error::Other(anyhow::anyhow!("Unsupported config file format"))),
        };

        self.config = config;
        self.config_path = Some(path);
        Ok(())
    }

    /// Load configuration from environment variables
    pub fn load_from_env(&mut self) -> Result<()> {
        // Load environment variables with CODE_MESH_ prefix
        let mut config = self.config.clone();
        
        if let Ok(log_level) = std::env::var("CODE_MESH_LOG_LEVEL") {
            config.app.log_level = log_level;
        }
        
        if let Ok(data_dir) = std::env::var("CODE_MESH_DATA_DIR") {
            config.app.data_dir = Some(PathBuf::from(data_dir));
        }
        
        // Add more environment variable mappings as needed
        
        self.config = config;
        Ok(())
    }

    /// Save configuration to file
    pub async fn save_to_file<P: Into<PathBuf>>(&self, path: P) -> Result<()> {
        let path = path.into();
        
        let content = match path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::to_string_pretty(&self.config)?,
            Some("toml") => toml::to_string_pretty(&self.config)
                .map_err(|e| Error::Other(anyhow::anyhow!("TOML serialize error: {}", e)))?,
            _ => return Err(Error::Other(anyhow::anyhow!("Unsupported config file format"))),
        };
        
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get mutable access to the configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Validate the current configuration
    pub fn validate(&self) -> Result<()> {
        // Validate provider configurations
        for (id, provider) in &self.config.providers {
            if provider.id != *id {
                return Err(Error::Other(anyhow::anyhow!(
                    "Provider ID mismatch: {} != {}",
                    provider.id,
                    id
                )));
            }
        }

        // Validate tool configurations
        if self.config.tools.default_permission.is_empty() {
            return Err(Error::Other(anyhow::anyhow!(
                "Default permission cannot be empty"
            )));
        }

        // Add more validation rules as needed

        Ok(())
    }

    /// Get configuration for a specific provider
    pub fn get_provider_config(&self, provider_id: &str) -> Option<&ProviderConfig> {
        self.config.providers.get(provider_id)
    }

    /// Get configuration for a specific tool
    pub fn get_tool_config(&self, tool_id: &str) -> Option<&ToolSpecificConfig> {
        self.config.tools.tool_configs.get(tool_id)
    }

    /// Merge another configuration into the current one
    pub fn merge(&mut self, other: Config) -> Result<()> {
        // Deep merge configurations
        // This is a simplified version - in practice, you'd want more sophisticated merging
        for (id, provider) in other.providers {
            self.config.providers.insert(id, provider);
        }

        for (id, tool_config) in other.tools.tool_configs {
            self.config.tools.tool_configs.insert(id, tool_config);
        }

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}