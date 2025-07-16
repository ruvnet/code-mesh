//! OpenAI provider implementation

use super::{LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ProviderError, ProviderType, Usage};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

/// OpenAI API provider
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    config: HashMap<String, serde_json::Value>,
}

/// OpenAI API request format
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

/// OpenAI message format
#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

/// OpenAI API response format
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

/// OpenAI choice object
#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIResponseMessage,
    finish_reason: Option<String>,
}

/// OpenAI response message
#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    role: String,
    content: String,
}

/// OpenAI usage object
#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI streaming response chunk
#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIStreamChoice>,
}

/// OpenAI streaming choice
#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    index: u32,
    delta: OpenAIStreamDelta,
    finish_reason: Option<String>,
}

/// OpenAI streaming delta
#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    role: Option<String>,
    content: Option<String>,
}

/// OpenAI models response
#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    object: String,
    data: Vec<OpenAIModel>,
}

/// OpenAI model object
#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    object: String,
    created: u64,
    owned_by: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self, ProviderError> {
        let client = Client::new();
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        
        Ok(OpenAIProvider {
            client,
            api_key,
            base_url,
            config: HashMap::new(),
        })
    }
    
    /// Convert internal message to OpenAI format
    fn convert_message(&self, message: &Message) -> OpenAIMessage {
        let role = match message.role {
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::Tool => "tool".to_string(),
        };
        
        OpenAIMessage {
            role,
            content: message.content.clone(),
        }
    }
    
    /// Convert OpenAI response to internal format
    fn convert_response(&self, response: OpenAIResponse) -> Result<LLMResponse, ProviderError> {
        let choice = response.choices.into_iter().next()
            .ok_or_else(|| ProviderError::InvalidRequest("No choices in response".to_string()))?;
        
        let usage = response.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        
        Ok(LLMResponse {
            content: choice.message.content,
            model: response.model,
            usage,
            finish_reason: choice.finish_reason,
            metadata: HashMap::new(),
        })
    }
    
    /// Create request headers
    fn create_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.api_key)).unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }
    
    fn name(&self) -> String {
        "OpenAI".to_string()
    }
    
    async fn is_available(&self) -> bool {
        // Try to fetch models to check if the API is available
        self.get_models().await.is_ok()
    }
    
    async fn get_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/models", self.base_url);
        let headers = self.create_headers();
        
        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Network(
                reqwest::Error::from(reqwest::StatusCode::from_u16(response.status().as_u16()).unwrap())
            ));
        }
        
        let models_response: OpenAIModelsResponse = response.json().await?;
        Ok(models_response.data.into_iter().map(|m| m.id).collect())
    }
    
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let headers = self.create_headers();
        
        let openai_request = OpenAIRequest {
            model: request.model.unwrap_or_else(|| self.default_model()),
            messages: request.messages.iter().map(|m| self.convert_message(m)).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            stream: false,
            tools: request.tools.map(|tools| {
                tools.iter().map(|t| serde_json::to_value(t).unwrap()).collect()
            }),
            tool_choice: request.tool_choice.map(|tc| serde_json::to_value(tc).unwrap()),
        };
        
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&openai_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Generic(error_text));
        }
        
        let openai_response: OpenAIResponse = response.json().await?;
        self.convert_response(openai_response)
    }
    
    async fn complete_stream(&self, request: LLMRequest) -> Result<Box<dyn Stream<Item = Result<LLMResponse, ProviderError>> + Send + Unpin>, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let headers = self.create_headers();
        
        let openai_request = OpenAIRequest {
            model: request.model.unwrap_or_else(|| self.default_model()),
            messages: request.messages.iter().map(|m| self.convert_message(m)).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            stream: true,
            tools: request.tools.map(|tools| {
                tools.iter().map(|t| serde_json::to_value(t).unwrap()).collect()
            }),
            tool_choice: request.tool_choice.map(|tc| serde_json::to_value(tc).unwrap()),
        };
        
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&openai_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Generic(error_text));
        }
        
        let stream = OpenAIStreamWrapper::new(response.bytes_stream());
        Ok(Box::new(stream))
    }
    
    fn default_model(&self) -> String {
        "gpt-3.5-turbo".to_string()
    }
    
    fn get_config(&self) -> HashMap<String, serde_json::Value> {
        self.config.clone()
    }
    
    fn update_config(&mut self, config: HashMap<String, serde_json::Value>) -> Result<(), ProviderError> {
        self.config = config;
        Ok(())
    }
    
    async fn validate_config(&self) -> Result<(), ProviderError> {
        // Try to fetch models to validate the API key
        self.get_models().await?;
        Ok(())
    }
}

/// Stream wrapper for OpenAI streaming responses
struct OpenAIStreamWrapper {
    stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: String,
}

impl OpenAIStreamWrapper {
    fn new(stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static) -> Self {
        Self {
            stream: Box::pin(stream),
            buffer: String::new(),
        }
    }
    
    fn parse_chunk(&mut self, chunk: &str) -> Option<Result<LLMResponse, ProviderError>> {
        // OpenAI sends data in Server-Sent Events format
        if chunk.starts_with("data: ") {
            let data = &chunk[6..];
            if data.trim() == "[DONE]" {
                return None;
            }
            
            match serde_json::from_str::<OpenAIStreamChunk>(data) {
                Ok(stream_chunk) => {
                    if let Some(choice) = stream_chunk.choices.into_iter().next() {
                        if let Some(content) = choice.delta.content {
                            return Some(Ok(LLMResponse {
                                content,
                                model: stream_chunk.model,
                                usage: None,
                                finish_reason: choice.finish_reason,
                                metadata: HashMap::new(),
                            }));
                        }
                    }
                }
                Err(e) => {
                    return Some(Err(ProviderError::Serialization(e)));
                }
            }
        }
        None
    }
}

impl Stream for OpenAIStreamWrapper {
    type Item = Result<LLMResponse, ProviderError>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    let chunk = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&chunk);
                    
                    // Process complete lines
                    while let Some(line_end) = self.buffer.find('\n') {
                        let line = self.buffer[..line_end].to_string();
                        self.buffer = self.buffer[line_end + 1..].to_string();
                        
                        if let Some(result) = self.parse_chunk(&line) {
                            return Poll::Ready(Some(result));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(ProviderError::Network(e))));
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{Message, MessageRole};
    
    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAIProvider::new("test_key".to_string(), None).unwrap();
        assert_eq!(provider.provider_type(), ProviderType::OpenAI);
        assert_eq!(provider.name(), "OpenAI");
        assert_eq!(provider.default_model(), "gpt-3.5-turbo");
    }
    
    #[test]
    fn test_message_conversion() {
        let provider = OpenAIProvider::new("test_key".to_string(), None).unwrap();
        
        let message = Message::user("Hello, world!");
        let openai_message = provider.convert_message(&message);
        
        assert_eq!(openai_message.role, "user");
        assert_eq!(openai_message.content, "Hello, world!");
    }
    
    #[test]
    fn test_config_operations() {
        let mut provider = OpenAIProvider::new("test_key".to_string(), None).unwrap();
        
        let mut config = HashMap::new();
        config.insert("temperature".to_string(), serde_json::json!(0.8));
        
        provider.update_config(config.clone()).unwrap();
        assert_eq!(provider.get_config(), config);
    }
}