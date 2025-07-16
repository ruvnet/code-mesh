//! LLM provider abstractions and implementations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod provider;
pub mod model;
pub mod anthropic;
pub mod openai;
pub mod github_copilot;
pub mod registry;

#[cfg(test)]
mod anthropic_test;

pub use provider::{
    Provider, ProviderConfig, ProviderRegistry, ModelConfig, ModelCapabilities, 
    Cost, Limits, ProviderSource, ProviderStatus, RetryConfig, retry_with_backoff,
    ModelInfo, ModelLimits, ModelPricing, ModelStatus, ProviderHealth, RateLimitInfo, UsageStats,
    Model
};
pub use anthropic::{AnthropicProvider, AnthropicModel, AnthropicModelWithProvider};
pub use openai::{OpenAIProvider, OpenAIModel, OpenAIModelWithProvider, AzureOpenAIProvider, AzureOpenAIModelWithProvider};
pub use github_copilot::{GitHubCopilotProvider, GitHubCopilotModel, GitHubCopilotModelWithProvider};
pub use registry::{LLMRegistry, create_default_registry, create_registry_with_models_dev};

/// Language model trait for interacting with LLM providers
#[async_trait]
pub trait LanguageModel: Send + Sync {
    /// Generate text from a list of messages
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult>;
    
    /// Stream text generation
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: StreamOptions,
    ) -> crate::Result<Box<dyn futures::Stream<Item = crate::Result<StreamChunk>> + Send + Unpin>>;
    
    /// Check if the model supports tool calling
    fn supports_tools(&self) -> bool;
    
    /// Check if the model supports vision/images
    fn supports_vision(&self) -> bool;
    
    /// Check if the model supports caching
    fn supports_caching(&self) -> bool;
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<MessagePart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePart {
    Text { text: String },
    Image { image: ImageData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub url: Option<String>,
    pub base64: Option<String>,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct GenerateOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Vec<ToolDefinition>,
    pub stop_sequences: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Vec<ToolDefinition>,
    pub stop_sequences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct GenerateResult {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub delta: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: Option<FinishReason>,
}

#[derive(Debug, Clone, Copy)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
}