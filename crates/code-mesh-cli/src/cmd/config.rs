//! Configuration management for the CLI

use crate::cmd::{CliError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub profiles: HashMap<String, Profile>,
    pub default_profile: String,
    pub global: GlobalConfig,
}

/// Profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub default_model: Option<String>,
    pub default_provider: Option<String>,
    pub api_base_url: Option<String>,
    pub timeout: Option<u64>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Global configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub auto_save_sessions: bool,
    pub session_history_limit: usize,
    pub log_level: String,
    pub enable_telemetry: bool,
    pub check_for_updates: bool,
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("default".to_string(), Profile::default());

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            profiles,
            default_profile: "default".to_string(),
            global: GlobalConfig::default(),
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            default_model: None,
            default_provider: None,
            api_base_url: None,
            timeout: Some(30),
            max_tokens: Some(4000),
            temperature: Some(0.7),
            settings: HashMap::new(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            auto_save_sessions: true,
            session_history_limit: 100,
            log_level: "info".to_string(),
            enable_telemetry: false,
            check_for_updates: true,
            theme: "default".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            // Create default configuration
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| CliError::Config(format!("Failed to read config file: {}", e)))?;

        let mut config: Config = serde_json::from_str(&content)
            .map_err(|e| CliError::Config(format!("Invalid config format: {}", e)))?;

        // Migrate config if needed
        config.migrate()?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| CliError::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| CliError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&config_path, content)
            .map_err(|e| CliError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Get configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CliError::Config("Unable to determine config directory".to_string()))?;
        
        Ok(config_dir.join("code-mesh").join("config.json"))
    }

    /// Get data directory path
    pub fn data_dir() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| CliError::Config("Unable to determine data directory".to_string()))?;
        
        Ok(data_dir.join("code-mesh"))
    }

    /// Get cache directory path
    pub fn cache_dir() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| CliError::Config("Unable to determine cache directory".to_string()))?;
        
        Ok(cache_dir.join("code-mesh"))
    }

    /// Get current profile
    pub fn current_profile(&self) -> Result<&Profile> {
        self.profiles
            .get(&self.default_profile)
            .ok_or_else(|| CliError::Config(format!("Profile '{}' not found", self.default_profile)))
    }

    /// Get mutable reference to current profile
    pub fn current_profile_mut(&mut self) -> Result<&mut Profile> {
        let profile_name = self.default_profile.clone();
        self.profiles
            .get_mut(&profile_name)
            .ok_or_else(|| CliError::Config(format!("Profile '{}' not found", profile_name)))
    }

    /// Add or update a profile
    pub fn set_profile(&mut self, name: String, profile: Profile) {
        self.profiles.insert(name, profile);
    }

    /// Remove a profile
    pub fn remove_profile(&mut self, name: &str) -> Result<()> {
        if name == "default" {
            return Err(CliError::Config("Cannot remove default profile".to_string()));
        }
        
        if name == self.default_profile {
            self.default_profile = "default".to_string();
        }
        
        self.profiles.remove(name);
        Ok(())
    }

    /// Switch to a different profile
    pub fn switch_profile(&mut self, name: &str) -> Result<()> {
        if !self.profiles.contains_key(name) {
            return Err(CliError::Config(format!("Profile '{}' not found", name)));
        }
        
        self.default_profile = name.to_string();
        Ok(())
    }

    /// Migrate configuration to newer version
    fn migrate(&mut self) -> Result<()> {
        let current_version = env!("CARGO_PKG_VERSION");
        
        if self.version != current_version {
            // Perform migration based on version differences
            // For now, just update the version
            self.version = current_version.to_string();
        }
        
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Check if default profile exists
        if !self.profiles.contains_key(&self.default_profile) {
            return Err(CliError::Config(format!(
                "Default profile '{}' not found",
                self.default_profile
            )));
        }

        // Validate each profile
        for (name, profile) in &self.profiles {
            profile.validate().map_err(|e| {
                CliError::Config(format!("Profile '{}' is invalid: {}", name, e))
            })?;
        }

        Ok(())
    }

    /// Reset configuration to defaults
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl Profile {
    /// Validate profile settings
    pub fn validate(&self) -> Result<()> {
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(CliError::Config(
                    "Temperature must be between 0.0 and 2.0".to_string(),
                ));
            }
        }

        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 {
                return Err(CliError::Config(
                    "Max tokens must be greater than 0".to_string(),
                ));
            }
        }

        if let Some(timeout) = self.timeout {
            if timeout == 0 {
                return Err(CliError::Config(
                    "Timeout must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Get effective model (with fallback)
    pub fn effective_model(&self) -> Option<String> {
        self.default_model.clone()
            .or_else(|| Some("anthropic/claude-3-sonnet-20240229".to_string()))
    }

    /// Get effective provider (with fallback)
    pub fn effective_provider(&self) -> Option<String> {
        self.default_provider.clone()
            .or_else(|| Some("anthropic".to_string()))
    }
}

/// Configuration manager
pub struct ConfigManager {
    config: Config,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self { config })
    }

    /// Get immutable reference to config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get mutable reference to config
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Save the current configuration
    pub fn save(&self) -> Result<()> {
        self.config.save()
    }

    /// Reload configuration from file
    pub fn reload(&mut self) -> Result<()> {
        self.config = Config::load()?;
        Ok(())
    }

    /// Initialize configuration for a new installation
    pub fn init(force: bool) -> Result<()> {
        let config_path = Config::config_path()?;
        
        if config_path.exists() && !force {
            return Err(CliError::Config(
                "Configuration already exists. Use --force to overwrite".to_string(),
            ));
        }

        let config = Config::default();
        config.save()?;

        // Create necessary directories
        let data_dir = Config::data_dir()?;
        let cache_dir = Config::cache_dir()?;
        
        fs::create_dir_all(&data_dir)
            .map_err(|e| CliError::Config(format!("Failed to create data directory: {}", e)))?;
        
        fs::create_dir_all(&cache_dir)
            .map_err(|e| CliError::Config(format!("Failed to create cache directory: {}", e)))?;

        Ok(())
    }
}