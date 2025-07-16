//! LLM Provider implementations for OpenCode
//!
//! This module provides a unified interface for interacting with different
//! Large Language Model providers like OpenAI, Anthropic, and local models.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

pub mod openai;
pub mod anthropic;
pub mod local;

pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use local::LocalProvider;

/// Supported LLM provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Local,
    Custom(String),
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Local => write!(f, "local"),
            ProviderType::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl From<&str> for ProviderType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => ProviderType::OpenAI,
            "anthropic" => ProviderType::Anthropic,
            "local" => ProviderType::Local,
            _ => ProviderType::Custom(s.to_string()),
        }
    }
}

/// Message role in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Response from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Request parameters for LLM calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub stream: bool,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

/// Function definition for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool choice configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolChoice {
    None,
    Auto,
    Required,
    Function(String),
}

/// Provider-specific errors
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Main trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider type
    fn provider_type(&self) -> ProviderType;
    
    /// Get the provider name
    fn name(&self) -> String;
    
    /// Check if the provider is available
    async fn is_available(&self) -> bool;
    
    /// Get available models
    async fn get_models(&self) -> Result<Vec<String>, ProviderError>;
    
    /// Send a completion request
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, ProviderError>;
    
    /// Send a streaming completion request
    async fn complete_stream(&self, request: LLMRequest) -> Result<Box<dyn futures::Stream<Item = Result<LLMResponse, ProviderError>> + Send + Unpin>, ProviderError>;
    
    /// Get the default model for this provider
    fn default_model(&self) -> String;
    
    /// Get provider-specific configuration
    fn get_config(&self) -> HashMap<String, serde_json::Value>;
    
    /// Update provider configuration
    fn update_config(&mut self, config: HashMap<String, serde_json::Value>) -> Result<(), ProviderError>;
    
    /// Validate an API key or configuration
    async fn validate_config(&self) -> Result<(), ProviderError>;
}

/// Factory for creating LLM providers
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from configuration
    pub fn create_provider(
        provider_type: ProviderType,
        config: &crate::config::ProviderConfig,
    ) -> Result<Box<dyn LLMProvider>, ProviderError> {
        match provider_type {
            ProviderType::OpenAI => {
                let api_key = config.api_key.as_ref()
                    .ok_or_else(|| ProviderError::Configuration("OpenAI API key is required".to_string()))?;
                let provider = OpenAIProvider::new(api_key.clone(), config.base_url.clone())?;
                Ok(Box::new(provider))
            }
            ProviderType::Anthropic => {
                let api_key = config.api_key.as_ref()
                    .ok_or_else(|| ProviderError::Configuration("Anthropic API key is required".to_string()))?;
                let provider = AnthropicProvider::new(api_key.clone(), config.base_url.clone())?;
                Ok(Box::new(provider))
            }
            ProviderType::Local => {
                let base_url = config.base_url.as_ref()
                    .ok_or_else(|| ProviderError::Configuration("Local provider base URL is required".to_string()))?;
                let provider = LocalProvider::new(base_url.clone())?;
                Ok(Box::new(provider))
            }
            ProviderType::Custom(name) => {
                Err(ProviderError::Configuration(format!("Custom provider '{}' not implemented", name)))
            }
        }
    }
    
    /// Get all available provider types
    pub fn available_providers() -> Vec<ProviderType> {
        vec![
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Local,
        ]
    }
}

impl Default for LLMRequest {
    fn default() -> Self {
        LLMRequest {
            messages: Vec::new(),
            model: None,
            temperature: Some(0.7),
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: false,
            tools: None,
            tool_choice: None,
        }
    }
}

impl Message {
    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Message {
            role: MessageRole::System,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Message {
            role: MessageRole::User,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Message {
            role: MessageRole::Assistant,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// Create a new tool message
    pub fn tool(content: impl Into<String>) -> Self {
        Message {
            role: MessageRole::Tool,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }
    
    /// Add metadata to the message
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_provider_type_conversion() {
        assert_eq!(ProviderType::from("openai"), ProviderType::OpenAI);
        assert_eq!(ProviderType::from("anthropic"), ProviderType::Anthropic);
        assert_eq!(ProviderType::from("local"), ProviderType::Local);
        assert_eq!(ProviderType::from("custom"), ProviderType::Custom("custom".to_string()));
    }
    
    #[test]
    fn test_provider_type_display() {
        assert_eq!(ProviderType::OpenAI.to_string(), "openai");
        assert_eq!(ProviderType::Anthropic.to_string(), "anthropic");
        assert_eq!(ProviderType::Local.to_string(), "local");
        assert_eq!(ProviderType::Custom("test".to_string()).to_string(), "test");
    }
    
    #[test]
    fn test_message_creation() {
        let msg = Message::system("You are a helpful assistant");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "You are a helpful assistant");
        assert!(msg.metadata.is_empty());
        
        let msg = Message::user("Hello").with_metadata("timestamp", serde_json::json!(1234567890));
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.metadata.len(), 1);
    }
    
    #[test]
    fn test_llm_request_default() {
        let request = LLMRequest::default();
        assert!(request.messages.is_empty());
        assert_eq!(request.temperature, Some(0.7));
        assert!(!request.stream);
    }
    
    #[test]
    fn test_available_providers() {
        let providers = ProviderFactory::available_providers();
        assert!(providers.contains(&ProviderType::OpenAI));
        assert!(providers.contains(&ProviderType::Anthropic));
        assert!(providers.contains(&ProviderType::Local));
    }
}