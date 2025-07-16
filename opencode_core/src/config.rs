//! Configuration management for OpenCode
//!
//! This module handles loading and managing configuration from various sources
//! including files, environment variables, and browser storage (in WASM).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration structure for OpenCode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// LLM provider configurations
    pub providers: HashMap<String, ProviderConfig>,
    
    /// Default provider to use
    pub default_provider: Option<String>,
    
    /// UI and display preferences
    pub ui: UiConfig,
    
    /// Agent configuration
    pub agents: AgentDefaultConfig,
    
    /// Session management settings
    pub session: SessionConfig,
    
    /// File system and project settings
    pub filesystem: FileSystemConfig,
    
    /// Memory and context settings
    pub memory: MemoryConfig,
}

/// Configuration for LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type (openai, anthropic, local, etc.)
    pub provider_type: String,
    
    /// API key or authentication token
    pub api_key: Option<String>,
    
    /// Base URL for the API
    pub base_url: Option<String>,
    
    /// Model name to use
    pub model: Option<String>,
    
    /// Additional provider-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Theme settings
    pub theme: String,
    
    /// Terminal settings
    pub terminal: TerminalConfig,
    
    /// Code display settings
    pub code_display: CodeDisplayConfig,
}

/// Terminal-specific UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    
    /// Terminal width preference
    pub width: Option<u16>,
    
    /// Terminal height preference
    pub height: Option<u16>,
    
    /// Enable mouse support
    pub mouse_support: bool,
}

/// Code display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDisplayConfig {
    /// Show line numbers
    pub line_numbers: bool,
    
    /// Diff display style
    pub diff_style: String,
    
    /// Tab width
    pub tab_width: usize,
    
    /// Word wrap
    pub word_wrap: bool,
}

/// Default agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaultConfig {
    /// Default agent system prompt
    pub system_prompt: Option<String>,
    
    /// Default temperature
    pub temperature: f32,
    
    /// Default max tokens
    pub max_tokens: Option<u32>,
    
    /// Default timeout in seconds
    pub timeout: u64,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    
    /// Maximum session history length
    pub max_history_length: usize,
    
    /// Session storage directory
    pub storage_dir: Option<PathBuf>,
}

/// File system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemConfig {
    /// Project root directory
    pub project_root: Option<PathBuf>,
    
    /// Files and directories to ignore
    pub ignore_patterns: Vec<String>,
    
    /// Enable file watching
    pub enable_watching: bool,
    
    /// Maximum file size to process (in bytes)
    pub max_file_size: usize,
}

/// Memory and context configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum memory entries
    pub max_entries: usize,
    
    /// Memory expiration time in seconds
    pub expiration_seconds: u64,
    
    /// Enable persistent memory
    pub persistent: bool,
    
    /// Memory storage directory
    pub storage_dir: Option<PathBuf>,
}

/// Configuration loading and management errors
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),
    
    #[error("Configuration parse error: {0}")]
    ParseError(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Environment variable error: {0}")]
    EnvironmentError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl Default for Config {
    fn default() -> Self {
        Config {
            providers: HashMap::new(),
            default_provider: None,
            ui: UiConfig::default(),
            agents: AgentDefaultConfig::default(),
            session: SessionConfig::default(),
            filesystem: FileSystemConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            theme: "default".to_string(),
            terminal: TerminalConfig::default(),
            code_display: CodeDisplayConfig::default(),
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            syntax_highlighting: true,
            width: None,
            height: None,
            mouse_support: true,
        }
    }
}

impl Default for CodeDisplayConfig {
    fn default() -> Self {
        CodeDisplayConfig {
            line_numbers: true,
            diff_style: "unified".to_string(),
            tab_width: 4,
            word_wrap: false,
        }
    }
}

impl Default for AgentDefaultConfig {
    fn default() -> Self {
        AgentDefaultConfig {
            system_prompt: None,
            temperature: 0.7,
            max_tokens: None,
            timeout: 30,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            auto_save_interval: 300, // 5 minutes
            max_history_length: 1000,
            storage_dir: None,
        }
    }
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        FileSystemConfig {
            project_root: None,
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".env".to_string(),
            ],
            enable_watching: true,
            max_file_size: 1024 * 1024, // 1MB
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        MemoryConfig {
            max_entries: 10000,
            expiration_seconds: 24 * 60 * 60, // 24 hours
            persistent: true,
            storage_dir: None,
        }
    }
}

impl Config {
    /// Load configuration from the default location
    pub async fn load() -> Result<Self, ConfigError> {
        #[cfg(feature = "native-runtime")]
        {
            Self::load_native().await
        }
        
        #[cfg(feature = "wasm-runtime")]
        {
            Self::load_wasm().await
        }
        
        #[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
        {
            Ok(Self::default())
        }
    }
    
    /// Load configuration from a specific file
    pub async fn load_from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        #[cfg(feature = "native-runtime")]
        {
            let content = tokio::fs::read_to_string(path).await?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        }
        
        #[cfg(not(feature = "native-runtime"))]
        {
            Err(ConfigError::InvalidConfig("File loading not supported in WASM".to_string()))
        }
    }
    
    /// Save configuration to the default location
    pub async fn save(&self) -> Result<(), ConfigError> {
        #[cfg(feature = "native-runtime")]
        {
            self.save_native().await
        }
        
        #[cfg(feature = "wasm-runtime")]
        {
            self.save_wasm().await
        }
        
        #[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
        {
            Ok(())
        }
    }
    
    /// Get a provider configuration by name
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }
    
    /// Add or update a provider configuration
    pub fn set_provider(&mut self, name: String, config: ProviderConfig) {
        self.providers.insert(name, config);
    }
    
    /// Get the default provider configuration
    pub fn get_default_provider(&self) -> Option<&ProviderConfig> {
        self.default_provider.as_ref()
            .and_then(|name| self.providers.get(name))
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check if default provider exists
        if let Some(default) = &self.default_provider {
            if !self.providers.contains_key(default) {
                return Err(ConfigError::InvalidConfig(
                    format!("Default provider '{}' not found in providers", default)
                ));
            }
        }
        
        // Validate provider configurations
        for (name, provider) in &self.providers {
            if provider.provider_type.is_empty() {
                return Err(ConfigError::InvalidConfig(
                    format!("Provider '{}' has empty provider_type", name)
                ));
            }
        }
        
        Ok(())
    }
}

// Platform-specific implementations
#[cfg(feature = "native-runtime")]
impl Config {
    async fn load_native() -> Result<Self, ConfigError> {
        use std::path::Path;
        
        // Try to load from various locations
        let config_paths = [
            std::env::var("OPENCODE_CONFIG").ok().map(PathBuf::from),
            dirs::config_dir().map(|d| d.join("opencode").join("config.json")),
            Some(PathBuf::from("opencode.json")),
            Some(PathBuf::from(".opencode.json")),
        ];
        
        for path_opt in config_paths.iter().flatten() {
            if path_opt.exists() {
                return Self::load_from_file(path_opt).await;
            }
        }
        
        // If no config file found, create default config
        let mut config = Self::default();
        
        // Load environment variables
        config.load_env_vars()?;
        
        Ok(config)
    }
    
    async fn save_native(&self) -> Result<(), ConfigError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| ConfigError::StorageError("Cannot determine config directory".to_string()))?
            .join("opencode");
        
        tokio::fs::create_dir_all(&config_dir).await?;
        
        let config_path = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(config_path, content).await?;
        
        Ok(())
    }
    
    fn load_env_vars(&mut self) -> Result<(), ConfigError> {
        // Load OpenAI configuration
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.providers.insert("openai".to_string(), ProviderConfig {
                provider_type: "openai".to_string(),
                api_key: Some(api_key),
                base_url: std::env::var("OPENAI_BASE_URL").ok(),
                model: std::env::var("OPENAI_MODEL").ok(),
                settings: HashMap::new(),
            });
            
            if self.default_provider.is_none() {
                self.default_provider = Some("openai".to_string());
            }
        }
        
        // Load Anthropic configuration
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            self.providers.insert("anthropic".to_string(), ProviderConfig {
                provider_type: "anthropic".to_string(),
                api_key: Some(api_key),
                base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
                model: std::env::var("ANTHROPIC_MODEL").ok(),
                settings: HashMap::new(),
            });
            
            if self.default_provider.is_none() {
                self.default_provider = Some("anthropic".to_string());
            }
        }
        
        Ok(())
    }
}

#[cfg(feature = "wasm-runtime")]
impl Config {
    async fn load_wasm() -> Result<Self, ConfigError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| 
            ConfigError::StorageError("No window object available".to_string()))?;
        
        let storage = window.local_storage()
            .map_err(|_| ConfigError::StorageError("Cannot access localStorage".to_string()))?
            .ok_or_else(|| ConfigError::StorageError("localStorage not available".to_string()))?;
        
        match storage.get_item("opencode_config") {
            Ok(Some(config_str)) => {
                let config: Config = serde_json::from_str(&config_str)?;
                Ok(config)
            }
            Ok(None) => Ok(Self::default()),
            Err(_) => Err(ConfigError::StorageError("Failed to read from localStorage".to_string())),
        }
    }
    
    async fn save_wasm(&self) -> Result<(), ConfigError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| 
            ConfigError::StorageError("No window object available".to_string()))?;
        
        let storage = window.local_storage()
            .map_err(|_| ConfigError::StorageError("Cannot access localStorage".to_string()))?
            .ok_or_else(|| ConfigError::StorageError("localStorage not available".to_string()))?;
        
        let config_str = serde_json::to_string(self)?;
        storage.set_item("opencode_config", &config_str)
            .map_err(|_| ConfigError::StorageError("Failed to write to localStorage".to_string()))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.providers.is_empty());
        assert!(config.default_provider.is_none());
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        config.default_provider = Some("nonexistent".to_string());
        
        assert!(config.validate().is_err());
        
        config.providers.insert("nonexistent".to_string(), ProviderConfig {
            provider_type: "test".to_string(),
            api_key: None,
            base_url: None,
            model: None,
            settings: HashMap::new(),
        });
        
        assert!(config.validate().is_ok());
    }
    
    #[cfg(feature = "native-runtime")]
    #[tokio::test]
    async fn test_config_file_operations() {
        let mut config = Config::default();
        config.providers.insert("test".to_string(), ProviderConfig {
            provider_type: "test".to_string(),
            api_key: Some("test_key".to_string()),
            base_url: None,
            model: None,
            settings: HashMap::new(),
        });
        
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();
        
        // Save config
        let content = serde_json::to_string_pretty(&config).unwrap();
        tokio::fs::write(temp_path, content).await.unwrap();
        
        // Load config
        let loaded_config = Config::load_from_file(temp_path).await.unwrap();
        assert_eq!(loaded_config.providers.len(), 1);
        assert!(loaded_config.providers.contains_key("test"));
    }
}