//! Provider trait and implementations

use async_trait::async_trait;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::{LanguageModel, Model};
use crate::auth::AuthCredentials;

/// Provider trait for LLM service providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Unique identifier for the provider
    fn id(&self) -> &str;
    
    /// Display name for the provider
    fn name(&self) -> &str;
    
    /// Base API endpoint (if applicable)
    fn api_endpoint(&self) -> Option<&str>;
    
    /// Environment variables used for authentication
    fn env_vars(&self) -> &[String];
    
    /// NPM package name (for Node.js SDK)
    fn npm_package(&self) -> Option<&str>;
    
    /// Get available models
    fn models(&self) -> &HashMap<String, Box<dyn Model>>;
    
    /// Authenticate with the provider
    async fn authenticate(&self) -> crate::Result<AuthCredentials>;
    
    /// Get a specific model by ID
    async fn get_model(&self, model_id: &str) -> crate::Result<Box<dyn LanguageModel>>;
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_endpoint: Option<String>,
    pub env_vars: Vec<String>,
    pub npm_package: Option<String>,
    pub models: HashMap<String, ModelConfig>,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub release_date: String,
    pub capabilities: ModelCapabilities,
    pub cost: Cost,
    pub limits: Limits,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub attachment: bool,
    pub reasoning: bool,
    pub temperature: bool,
    pub tool_call: bool,
    pub vision: bool,
    pub caching: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cost {
    pub input: f64,
    pub output: f64,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Limits {
    pub context: u32,
    pub output: u32,
}

/// Provider registry for managing multiple providers
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }
    
    pub fn get(&self, id: &str) -> Option<&Box<dyn Provider>> {
        self.providers.get(id)
    }
    
    pub fn list(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}