use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::{
    FinishReason, GenerateOptions, GenerateResult, LanguageModel, Message, MessageContent,
    MessagePart, MessageRole, StreamChunk, StreamOptions, ToolCall, ToolDefinition, Usage,
};
use crate::auth::{Auth, AuthCredentials};

/// OpenAI provider implementation
pub struct OpenAIProvider {
    auth: Box<dyn Auth>,
    client: Client,
    models: HashMap<String, OpenAIModel>,
    api_base: String,
}

#[derive(Debug, Clone)]
pub struct OpenAIModel {
    pub id: String,
    pub name: String,
    pub max_tokens: u32,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_caching: bool,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OpenAITool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: OpenAIContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum OpenAIContent {
    Text(String),
    Parts(Vec<OpenAIContentPart>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum OpenAIContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OpenAIImageUrl },
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIImageUrl {
    url: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Serialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl OpenAIProvider {
    const API_BASE: &'static str = "https://api.openai.com";
    
    pub fn new(auth: Box<dyn Auth>) -> Self {
        Self::with_api_base(auth, Self::API_BASE.to_string())
    }
    
    pub fn with_api_base(auth: Box<dyn Auth>, api_base: String) -> Self {
        let client = Client::new();
        let models = Self::default_models();
        
        Self {
            auth,
            client,
            models,
            api_base,
        }
    }
    
    fn default_models() -> HashMap<String, OpenAIModel> {
        let mut models = HashMap::new();
        
        models.insert(
            "gpt-4o".to_string(),
            OpenAIModel {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                max_tokens: 4096,
                supports_tools: true,
                supports_vision: true,
                supports_caching: false,
            },
        );
        
        models.insert(
            "gpt-4o-mini".to_string(),
            OpenAIModel {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o Mini".to_string(),
                max_tokens: 4096,
                supports_tools: true,
                supports_vision: true,
                supports_caching: false,
            },
        );
        
        models.insert(
            "gpt-4-turbo".to_string(),
            OpenAIModel {
                id: "gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                max_tokens: 4096,
                supports_tools: true,
                supports_vision: true,
                supports_caching: false,
            },
        );
        
        models.insert(
            "gpt-3.5-turbo".to_string(),
            OpenAIModel {
                id: "gpt-3.5-turbo".to_string(),
                name: "GPT-3.5 Turbo".to_string(),
                max_tokens: 4096,
                supports_tools: true,
                supports_vision: false,
                supports_caching: false,
            },
        );
        
        models.insert(
            "o1-preview".to_string(),
            OpenAIModel {
                id: "o1-preview".to_string(),
                name: "OpenAI o1 Preview".to_string(),
                max_tokens: 32768,
                supports_tools: false,
                supports_vision: false,
                supports_caching: false,
            },
        );
        
        models.insert(
            "o1-mini".to_string(),
            OpenAIModel {
                id: "o1-mini".to_string(),
                name: "OpenAI o1 Mini".to_string(),
                max_tokens: 65536,
                supports_tools: false,
                supports_vision: false,
                supports_caching: false,
            },
        );
        
        models
    }
    
    async fn get_auth_header(&self) -> crate::Result<String> {
        let credentials = self.auth.get_credentials().await?;
        
        match credentials {
            AuthCredentials::ApiKey { key } => Ok(format!("Bearer {}", key)),
            _ => Err(crate::Error::Other(anyhow::anyhow!(
                "Invalid credentials for OpenAI (API key required)"
            ))),
        }
    }
    
    fn convert_messages(&self, messages: Vec<Message>) -> Vec<OpenAIMessage> {
        messages
            .into_iter()
            .map(|msg| self.convert_message(msg))
            .collect()
    }
    
    fn convert_message(&self, message: Message) -> OpenAIMessage {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
        .to_string();
        
        let content = match message.content {
            MessageContent::Text(text) => OpenAIContent::Text(text),
            MessageContent::Parts(parts) => {
                let openai_parts: Vec<OpenAIContentPart> = parts
                    .into_iter()
                    .filter_map(|part| match part {
                        MessagePart::Text { text } => Some(OpenAIContentPart::Text { text }),
                        MessagePart::Image { image } => {
                            if let Some(url) = image.url {
                                Some(OpenAIContentPart::ImageUrl {
                                    image_url: OpenAIImageUrl {
                                        url,
                                        detail: Some("auto".to_string()),
                                    },
                                })
                            } else if let Some(base64) = image.base64 {
                                Some(OpenAIContentPart::ImageUrl {
                                    image_url: OpenAIImageUrl {
                                        url: format!("data:{};base64,{}", image.mime_type, base64),
                                        detail: Some("auto".to_string()),
                                    },
                                })
                            } else {
                                None
                            }
                        }
                    })
                    .collect();
                OpenAIContent::Parts(openai_parts)
            }
        };
        
        let tool_calls = message.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|call| OpenAIToolCall {
                    id: call.id,
                    tool_type: "function".to_string(),
                    function: OpenAIFunctionCall {
                        name: call.name,
                        arguments: call.arguments.to_string(),
                    },
                })
                .collect()
        });
        
        OpenAIMessage {
            role,
            content,
            name: message.name,
            tool_calls,
            tool_call_id: message.tool_call_id,
        }
    }
    
    fn convert_tools(&self, tools: Vec<ToolDefinition>) -> Vec<OpenAITool> {
        tools
            .into_iter()
            .map(|tool| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                },
            })
            .collect()
    }
    
    fn parse_finish_reason(&self, reason: Option<String>) -> FinishReason {
        match reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        }
    }
}

pub struct OpenAIModelWithProvider {
    model: OpenAIModel,
    provider: OpenAIProvider,
}

impl OpenAIModelWithProvider {
    pub fn new(model: OpenAIModel, provider: OpenAIProvider) -> Self {
        Self { model, provider }
    }
}

#[async_trait]
impl LanguageModel for OpenAIModelWithProvider {
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        let auth_header = self.provider.get_auth_header().await?;
        let openai_messages = self.provider.convert_messages(messages);
        let tools = self.provider.convert_tools(options.tools);
        
        let request = OpenAIRequest {
            model: self.model.id.clone(),
            messages: openai_messages,
            max_tokens: options.max_tokens.unwrap_or(self.model.max_tokens),
            temperature: options.temperature,
            tools,
            stop: options.stop_sequences,
            stream: Some(false),
        };
        
        let response = self
            .provider
            .client
            .post(&format!("{}/v1/chat/completions", self.provider.api_base))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(crate::Error::Other(anyhow::anyhow!(
                "API request failed with status {}: {}",
                status,
                body
            )));
        }
        
        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Failed to parse response: {}", e)))?;
            
        let choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| crate::Error::Other(anyhow::anyhow!("No choices in response")))?;
            
        let content = match choice.message.content {
            OpenAIContent::Text(text) => text,
            OpenAIContent::Parts(parts) => {
                parts
                    .into_iter()
                    .filter_map(|part| match part {
                        OpenAIContentPart::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }
        };
        
        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|call| ToolCall {
                id: call.id,
                name: call.function.name,
                arguments: serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            })
            .collect();
            
        Ok(GenerateResult {
            content,
            tool_calls,
            usage: Usage {
                prompt_tokens: openai_response.usage.prompt_tokens,
                completion_tokens: openai_response.usage.completion_tokens,
                total_tokens: openai_response.usage.total_tokens,
            },
            finish_reason: self.provider.parse_finish_reason(choice.finish_reason),
        })
    }
    
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: StreamOptions,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<StreamChunk>> + Send + Unpin>> {
        // Similar to generate but with stream: true
        // Implementation would handle SSE stream parsing
        Err(crate::Error::Other(anyhow::anyhow!(
            "Streaming not yet implemented for OpenAI"
        )))
    }
    
    fn supports_tools(&self) -> bool {
        self.model.supports_tools
    }
    
    fn supports_vision(&self) -> bool {
        self.model.supports_vision
    }
    
    fn supports_caching(&self) -> bool {
        self.model.supports_caching
    }
}

/// Azure OpenAI provider implementation
pub struct AzureOpenAIProvider {
    base_provider: OpenAIProvider,
    deployment_name: String,
    api_version: String,
}

impl AzureOpenAIProvider {
    pub fn new(
        auth: Box<dyn Auth>,
        endpoint: String,
        deployment_name: String,
        api_version: String,
    ) -> Self {
        let base_provider = OpenAIProvider::with_api_base(auth, endpoint);
        
        Self {
            base_provider,
            deployment_name,
            api_version,
        }
    }
    
    pub fn default_api_version() -> String {
        "2024-02-15-preview".to_string()
    }
    
    fn get_endpoint(&self) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.base_provider.api_base, self.deployment_name, self.api_version
        )
    }
}

pub struct AzureOpenAIModelWithProvider {
    model: OpenAIModel,
    provider: AzureOpenAIProvider,
}

impl AzureOpenAIModelWithProvider {
    pub fn new(model: OpenAIModel, provider: AzureOpenAIProvider) -> Self {
        Self { model, provider }
    }
}

#[async_trait]
impl LanguageModel for AzureOpenAIModelWithProvider {
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        let auth_header = self.provider.base_provider.get_auth_header().await?;
        let openai_messages = self.provider.base_provider.convert_messages(messages);
        let tools = self.provider.base_provider.convert_tools(options.tools);
        
        // Azure uses deployment name instead of model in URL
        let request = OpenAIRequest {
            model: self.model.id.clone(), // Still include model in body for compatibility
            messages: openai_messages,
            max_tokens: options.max_tokens.unwrap_or(self.model.max_tokens),
            temperature: options.temperature,
            tools,
            stop: options.stop_sequences,
            stream: Some(false),
        };
        
        let response = self
            .provider
            .base_provider
            .client
            .post(&self.provider.get_endpoint())
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(crate::Error::Other(anyhow::anyhow!(
                "API request failed with status {}: {}",
                status,
                body
            )));
        }
        
        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Failed to parse response: {}", e)))?;
            
        let choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| crate::Error::Other(anyhow::anyhow!("No choices in response")))?;
            
        let content = match choice.message.content {
            OpenAIContent::Text(text) => text,
            OpenAIContent::Parts(parts) => {
                parts
                    .into_iter()
                    .filter_map(|part| match part {
                        OpenAIContentPart::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }
        };
        
        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|call| ToolCall {
                id: call.id,
                name: call.function.name,
                arguments: serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            })
            .collect();
            
        Ok(GenerateResult {
            content,
            tool_calls,
            usage: Usage {
                prompt_tokens: openai_response.usage.prompt_tokens,
                completion_tokens: openai_response.usage.completion_tokens,
                total_tokens: openai_response.usage.total_tokens,
            },
            finish_reason: self.provider.base_provider.parse_finish_reason(choice.finish_reason),
        })
    }
    
    async fn stream(
        &self,
        _messages: Vec<Message>,
        _options: StreamOptions,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<StreamChunk>> + Send + Unpin>> {
        Err(crate::Error::Other(anyhow::anyhow!(
            "Streaming not yet implemented for Azure OpenAI"
        )))
    }
    
    fn supports_tools(&self) -> bool {
        self.model.supports_tools
    }
    
    fn supports_vision(&self) -> bool {
        self.model.supports_vision
    }
    
    fn supports_caching(&self) -> bool {
        self.model.supports_caching
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_models() {
        let models = OpenAIProvider::default_models();
        assert!(!models.is_empty());
        assert!(models.contains_key("gpt-4o"));
        assert!(models.contains_key("gpt-4o-mini"));
        assert!(models.contains_key("o1-preview"));
    }
    
    #[test]
    fn test_model_capabilities() {
        let models = OpenAIProvider::default_models();
        let gpt4o = models.get("gpt-4o").unwrap();
        assert!(gpt4o.supports_tools);
        assert!(gpt4o.supports_vision);
        
        let o1 = models.get("o1-preview").unwrap();
        assert!(!o1.supports_tools);
        assert!(!o1.supports_vision);
        
        let gpt35 = models.get("gpt-3.5-turbo").unwrap();
        assert!(gpt35.supports_tools);
        assert!(!gpt35.supports_vision);
    }
    
    #[test]
    fn test_azure_endpoint() {
        use crate::auth::FileAuthStorage;
        use tempfile::tempdir;
        
        let temp_dir = tempdir().unwrap();
        let auth_path = temp_dir.path().join("auth.json");
        let storage = FileAuthStorage::new(auth_path);
        let auth = Box::new(crate::auth::AnthropicAuth::new(Box::new(storage))); // Dummy auth
        
        let provider = AzureOpenAIProvider::new(
            auth,
            "https://test.openai.azure.com".to_string(),
            "gpt-4".to_string(),
            "2024-02-15-preview".to_string(),
        );
        
        let endpoint = provider.get_endpoint();
        assert!(endpoint.contains("openai/deployments/gpt-4"));
        assert!(endpoint.contains("api-version=2024-02-15-preview"));
    }
}