//! LLM provider abstractions and implementations
//!
//! This module defines the core traits and types for integrating with various
//! Language Model providers like Anthropic, OpenAI, Google, and others.

use crate::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    Message, GenerateOptions, GenerateResult, StreamChunk, FinishReason,
    Usage, ToolDefinition, MessageRole, MessageContent
};

/// Provider trait for LLM providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Unique identifier for this provider
    fn id(&self) -> &str;
    
    /// Human-readable name of the provider
    fn name(&self) -> &str;
    
    /// Base URL for the provider's API
    fn base_url(&self) -> &str;
    
    /// API version being used
    fn api_version(&self) -> &str;
    
    /// List of available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    
    /// Get a specific model by ID
    async fn get_model(&self, model_id: &str) -> Result<Arc<dyn Model>>;
    
    /// Check if the provider is available (API reachable, credentials valid)
    async fn health_check(&self) -> Result<ProviderHealth>;
    
    /// Get provider-specific configuration
    fn get_config(&self) -> &ProviderConfig;
    
    /// Update provider configuration
    async fn update_config(&mut self, config: ProviderConfig) -> Result<()>;
    
    /// Get rate limiting information
    async fn get_rate_limits(&self) -> Result<RateLimitInfo>;
    
    /// Get current usage statistics
    async fn get_usage(&self) -> Result<UsageStats>;
}

/// Model trait for individual language models
#[async_trait]
pub trait Model: Send + Sync {
    /// Unique identifier for this model
    fn id(&self) -> &str;
    
    /// Human-readable name of the model
    fn name(&self) -> &str;
    
    /// Provider that owns this model
    fn provider_id(&self) -> &str;
    
    /// Model capabilities
    fn capabilities(&self) -> &ModelCapabilities;
    
    /// Model configuration
    fn config(&self) -> &ModelConfig;
    
    /// Generate a response from messages
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> Result<GenerateResult>;
    
    /// Stream response generation
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;
    
    /// Count tokens in messages
    async fn count_tokens(&self, messages: &[Message]) -> Result<u32>;
    
    /// Estimate cost for a request
    async fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> Result<f64>;
    
    /// Get model-specific metadata
    fn metadata(&self) -> &ModelMetadata;
}

use futures::Stream;
use std::pin::Pin;

/// Information about an available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub capabilities: ModelCapabilities,
    pub limits: ModelLimits,
    pub pricing: ModelPricing,
    pub release_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ModelStatus,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Supports text generation
    pub text_generation: bool,
    
    /// Supports tool/function calling
    pub tool_calling: bool,
    
    /// Supports vision/image inputs
    pub vision: bool,
    
    /// Supports streaming responses
    pub streaming: bool,
    
    /// Supports response caching
    pub caching: bool,
    
    /// Supports JSON mode
    pub json_mode: bool,
    
    /// Supports reasoning/chain-of-thought
    pub reasoning: bool,
    
    /// Supports code generation
    pub code_generation: bool,
    
    /// Supports multiple languages
    pub multilingual: bool,
    
    /// Custom capabilities
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            text_generation: true,
            tool_calling: false,
            vision: false,
            streaming: true,
            caching: false,
            json_mode: false,
            reasoning: false,
            code_generation: false,
            multilingual: false,
            custom: HashMap::new(),
        }
    }
}

/// Model limits and constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimits {
    /// Maximum context length in tokens
    pub max_context_tokens: u32,
    
    /// Maximum output tokens per request
    pub max_output_tokens: u32,
    
    /// Maximum image size in bytes (if vision is supported)
    pub max_image_size_bytes: Option<u64>,
    
    /// Maximum number of images per request
    pub max_images_per_request: Option<u32>,
    
    /// Maximum number of tool calls per request
    pub max_tool_calls: Option<u32>,
    
    /// Rate limits
    pub rate_limits: RateLimitInfo,
}

/// Model pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per 1K input tokens
    pub input_cost_per_1k: f64,
    
    /// Cost per 1K output tokens
    pub output_cost_per_1k: f64,
    
    /// Cost per 1K cached input tokens
    pub cache_read_cost_per_1k: Option<f64>,
    
    /// Cost per 1K cache write tokens
    pub cache_write_cost_per_1k: Option<f64>,
    
    /// Currency code (e.g., "USD")
    pub currency: String,
}

impl Default for ModelPricing {
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

/// Model status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    /// Model is available and fully functional
    Active,
    
    /// Model is available but deprecated
    Deprecated,
    
    /// Model is in beta/preview
    Beta,
    
    /// Model is temporarily unavailable
    Unavailable,
    
    /// Model is permanently discontinued
    Discontinued,
}

/// Rate limiting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Requests per minute
    pub requests_per_minute: Option<u32>,
    
    /// Tokens per minute
    pub tokens_per_minute: Option<u32>,
    
    /// Tokens per day
    pub tokens_per_day: Option<u32>,
    
    /// Concurrent requests
    pub concurrent_requests: Option<u32>,
    
    /// Current usage counts
    pub current_usage: Option<CurrentUsage>,
}

/// Current usage against rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUsage {
    /// Requests used in current minute
    pub requests_this_minute: u32,
    
    /// Tokens used in current minute
    pub tokens_this_minute: u32,
    
    /// Tokens used today
    pub tokens_today: u32,
    
    /// Currently active requests
    pub active_requests: u32,
}

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Whether the provider is available
    pub available: bool,
    
    /// Latency in milliseconds
    pub latency_ms: Option<u64>,
    
    /// Any error messages
    pub error: Option<String>,
    
    /// Timestamp of last check
    pub last_check: chrono::DateTime<chrono::Utc>,
    
    /// Additional status information
    pub details: HashMap<String, serde_json::Value>,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider ID
    pub provider_id: String,
    
    /// API key or token
    pub api_key: Option<String>,
    
    /// Base URL override
    pub base_url_override: Option<String>,
    
    /// API version override
    pub api_version_override: Option<String>,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum retries
    pub max_retries: u32,
    
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    
    /// Custom headers
    pub custom_headers: HashMap<String, String>,
    
    /// Organization ID (for providers that support it)
    pub organization_id: Option<String>,
    
    /// Project ID (for providers that support it)
    pub project_id: Option<String>,
    
    /// Additional configuration
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_id: String::new(),
            api_key: None,
            base_url_override: None,
            api_version_override: None,
            timeout_seconds: 60,
            max_retries: 3,
            retry_delay_ms: 1000,
            custom_headers: HashMap::new(),
            organization_id: None,
            project_id: None,
            extra: HashMap::new(),
        }
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model ID
    pub model_id: String,
    
    /// Default temperature
    pub default_temperature: Option<f32>,
    
    /// Default max tokens
    pub default_max_tokens: Option<u32>,
    
    /// Default top-p
    pub default_top_p: Option<f32>,
    
    /// Default stop sequences
    pub default_stop_sequences: Vec<String>,
    
    /// Whether to use caching by default
    pub use_caching: bool,
    
    /// Model-specific options
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_id: String::new(),
            default_temperature: None,
            default_max_tokens: None,
            default_top_p: None,
            default_stop_sequences: Vec::new(),
            use_caching: false,
            options: HashMap::new(),
        }
    }
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model family (e.g., "gpt-4", "claude-3")
    pub family: String,
    
    /// Model size/parameters (if known)
    pub parameters: Option<String>,
    
    /// Training data cutoff
    pub training_cutoff: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Model version
    pub version: Option<String>,
    
    /// Additional metadata
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for ModelMetadata {
    fn default() -> Self {
        Self {
            family: String::new(),
            parameters: None,
            training_cutoff: None,
            version: None,
            extra: HashMap::new(),
        }
    }
}

/// Usage statistics for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Total requests made
    pub total_requests: u64,
    
    /// Total tokens consumed
    pub total_tokens: u64,
    
    /// Total cost incurred
    pub total_cost: f64,
    
    /// Currency for cost
    pub currency: String,
    
    /// Usage by model
    pub by_model: HashMap<String, ModelUsage>,
    
    /// Usage by time period
    pub by_period: HashMap<String, PeriodUsage>,
}

/// Usage statistics for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    /// Number of requests
    pub requests: u64,
    
    /// Input tokens used
    pub input_tokens: u64,
    
    /// Output tokens generated
    pub output_tokens: u64,
    
    /// Cache hits
    pub cache_hits: u64,
    
    /// Total cost
    pub cost: f64,
    
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

/// Usage statistics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodUsage {
    /// Start of period
    pub start: chrono::DateTime<chrono::Utc>,
    
    /// End of period
    pub end: chrono::DateTime<chrono::Utc>,
    
    /// Total requests in period
    pub requests: u64,
    
    /// Total tokens in period
    pub tokens: u64,
    
    /// Total cost in period
    pub cost: f64,
}

/// Registry for managing LLM providers
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
    models: HashMap<String, Arc<dyn Model>>,
    default_provider: Option<String>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            models: HashMap::new(),
            default_provider: None,
        }
    }

    /// Register a provider
    pub fn register_provider(&mut self, provider: Arc<dyn Provider>) -> Result<()> {
        let provider_id = provider.id().to_string();
        
        if self.providers.contains_key(&provider_id) {
            return Err(Error::Other(anyhow::anyhow!(
                "Provider {} is already registered",
                provider_id
            )));
        }
        
        self.providers.insert(provider_id, provider);
        Ok(())
    }

    /// Get a provider by ID
    pub fn get_provider(&self, provider_id: &str) -> Result<Arc<dyn Provider>> {
        self.providers
            .get(provider_id)
            .cloned()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Provider {} not found", provider_id)))
    }

    /// List all registered providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Get a model by provider and model ID
    pub async fn get_model(&mut self, provider_id: &str, model_id: &str) -> Result<Arc<dyn Model>> {
        let key = format!("{}/{}", provider_id, model_id);
        
        // Check cache first
        if let Some(model) = self.models.get(&key) {
            return Ok(model.clone());
        }
        
        // Get provider and fetch model
        let provider = self.get_provider(provider_id)?;
        let model = provider.get_model(model_id).await?;
        
        // Cache the model
        self.models.insert(key, model.clone());
        
        Ok(model)
    }

    /// Parse a model string (format: "provider/model" or "provider:model")
    pub fn parse_model_string(&self, model_string: &str) -> Result<(String, String)> {
        if let Some((provider, model)) = model_string.split_once('/') {
            Ok((provider.to_string(), model.to_string()))
        } else if let Some((provider, model)) = model_string.split_once(':') {
            Ok((provider.to_string(), model.to_string()))
        } else {
            // If no separator, use default provider
            if let Some(default_provider) = &self.default_provider {
                Ok((default_provider.clone(), model_string.to_string()))
            } else {
                Err(Error::Other(anyhow::anyhow!(
                    "Invalid model string format: {}. Expected 'provider/model' or 'provider:model'",
                    model_string
                )))
            }
        }
    }

    /// Set default provider
    pub fn set_default_provider(&mut self, provider_id: &str) -> Result<()> {
        if !self.providers.contains_key(provider_id) {
            return Err(Error::Other(anyhow::anyhow!(
                "Provider {} is not registered",
                provider_id
            )));
        }
        
        self.default_provider = Some(provider_id.to_string());
        Ok(())
    }

    /// Get default provider
    pub fn get_default_provider(&self) -> Option<&str> {
        self.default_provider.as_deref()
    }

    /// List all available models
    pub async fn list_all_models(&self) -> Result<Vec<ModelInfo>> {
        let mut all_models = Vec::new();
        
        for provider in self.providers.values() {
            match provider.list_models().await {
                Ok(models) => all_models.extend(models),
                Err(e) => {
                    tracing::warn!("Failed to list models for provider {}: {}", provider.id(), e);
                }
            }
        }
        
        Ok(all_models)
    }

    /// Get provider health for all providers
    pub async fn get_all_provider_health(&self) -> HashMap<String, ProviderHealth> {
        let mut health_status = HashMap::new();
        
        for (id, provider) in &self.providers {
            match provider.health_check().await {
                Ok(health) => {
                    health_status.insert(id.clone(), health);
                }
                Err(e) => {
                    health_status.insert(
                        id.clone(),
                        ProviderHealth {
                            available: false,
                            latency_ms: None,
                            error: Some(e.to_string()),
                            last_check: chrono::Utc::now(),
                            details: HashMap::new(),
                        },
                    );
                }
            }
        }
        
        health_status
    }

    /// Clear cached models
    pub fn clear_model_cache(&mut self) {
        self.models.clear();
    }

    /// Remove a provider
    pub fn remove_provider(&mut self, provider_id: &str) -> Result<()> {
        if !self.providers.contains_key(provider_id) {
            return Err(Error::Other(anyhow::anyhow!(
                "Provider {} is not registered",
                provider_id
            )));
        }
        
        // Remove provider
        self.providers.remove(provider_id);
        
        // Remove cached models for this provider
        self.models.retain(|key, _| !key.starts_with(&format!("{}/", provider_id)));
        
        // Clear default provider if it was this one
        if self.default_provider.as_deref() == Some(provider_id) {
            self.default_provider = None;
        }
        
        Ok(())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_string() {
        let registry = ProviderRegistry::new();
        
        // Test with slash separator
        let (provider, model) = registry.parse_model_string("anthropic/claude-3-opus").unwrap();
        assert_eq!(provider, "anthropic");
        assert_eq!(model, "claude-3-opus");
        
        // Test with colon separator
        let (provider, model) = registry.parse_model_string("openai:gpt-4").unwrap();
        assert_eq!(provider, "openai");
        assert_eq!(model, "gpt-4");
        
        // Test without separator (should fail without default)
        assert!(registry.parse_model_string("claude-3-opus").is_err());
    }

    #[test]
    fn test_model_capabilities_default() {
        let caps = ModelCapabilities::default();
        assert!(caps.text_generation);
        assert!(!caps.tool_calling);
        assert!(!caps.vision);
        assert!(caps.streaming);
    }

    #[test]
    fn test_provider_config_default() {
        let config = ProviderConfig::default();
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 1000);
    }
}