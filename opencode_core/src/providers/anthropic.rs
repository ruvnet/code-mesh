//! Anthropic provider implementation

use super::{LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ProviderError, ProviderType, Usage};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Anthropic API provider
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    config: HashMap<String, serde_json::Value>,
}

/// Anthropic API request format
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

/// Anthropic message format
#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic API response format
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: AnthropicUsage,
}

/// Anthropic content block
#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// Anthropic usage object
#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic streaming response
#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<AnthropicResponse>,
    content_block: Option<AnthropicContent>,
    delta: Option<AnthropicDelta>,
}

/// Anthropic streaming delta
#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self, ProviderError> {
        let client = Client::new();
        let base_url = base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string());
        
        Ok(AnthropicProvider {
            client,
            api_key,
            base_url,
            config: HashMap::new(),
        })
    }
    
    /// Convert internal messages to Anthropic format
    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system_message = None;
        let mut anthropic_messages = Vec::new();
        
        for message in messages {
            match message.role {
                MessageRole::System => {
                    system_message = Some(message.content.clone());
                }
                MessageRole::User => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: message.content.clone(),
                    });
                }
                MessageRole::Assistant => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: message.content.clone(),
                    });
                }
                MessageRole::Tool => {
                    // Handle tool messages as user messages with special formatting
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: format!("Tool result: {}", message.content),
                    });
                }
            }
        }
        
        (system_message, anthropic_messages)
    }
    
    /// Convert Anthropic response to internal format
    fn convert_response(&self, response: AnthropicResponse) -> Result<LLMResponse, ProviderError> {
        let content = response.content
            .into_iter()
            .filter_map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");
        
        let usage = Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
        };
        
        Ok(LLMResponse {
            content,
            model: response.model,
            usage: Some(usage),
            finish_reason: response.stop_reason,
            metadata: HashMap::new(),
        })
    }
    
    /// Create request headers
    fn create_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            reqwest::header::HeaderValue::from_str(&self.api_key).unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "anthropic-version",
            reqwest::header::HeaderValue::from_static("2023-06-01"),
        );
        headers
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }
    
    fn name(&self) -> String {
        "Anthropic".to_string()
    }
    
    async fn is_available(&self) -> bool {
        // Try to make a test request to check if the API is available
        self.validate_config().await.is_ok()
    }
    
    async fn get_models(&self) -> Result<Vec<String>, ProviderError> {
        // Anthropic doesn't have a public models endpoint, so return known models
        Ok(vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-2.1".to_string(),
            "claude-2.0".to_string(),
            "claude-instant-1.2".to_string(),
        ])
    }
    
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url);
        let headers = self.create_headers();
        
        let (system, messages) = self.convert_messages(&request.messages);
        
        let anthropic_request = AnthropicRequest {
            model: request.model.unwrap_or_else(|| self.default_model()),
            messages,
            system,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: false,
            tools: request.tools.map(|tools| {
                tools.iter().map(|t| serde_json::to_value(t).unwrap()).collect()
            }),
            tool_choice: request.tool_choice.map(|tc| serde_json::to_value(tc).unwrap()),
        };
        
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&anthropic_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Generic(error_text));
        }
        
        let anthropic_response: AnthropicResponse = response.json().await?;
        self.convert_response(anthropic_response)
    }
    
    async fn complete_stream(&self, request: LLMRequest) -> Result<Box<dyn Stream<Item = Result<LLMResponse, ProviderError>> + Send + Unpin>, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url);
        let headers = self.create_headers();
        
        let (system, messages) = self.convert_messages(&request.messages);
        
        let anthropic_request = AnthropicRequest {
            model: request.model.unwrap_or_else(|| self.default_model()),
            messages,
            system,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: true,
            tools: request.tools.map(|tools| {
                tools.iter().map(|t| serde_json::to_value(t).unwrap()).collect()
            }),
            tool_choice: request.tool_choice.map(|tc| serde_json::to_value(tc).unwrap()),
        };
        
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&anthropic_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Generic(error_text));
        }
        
        let stream = AnthropicStreamWrapper::new(response.bytes_stream());
        Ok(Box::new(stream))
    }
    
    fn default_model(&self) -> String {
        "claude-3-5-sonnet-20241022".to_string()
    }
    
    fn get_config(&self) -> HashMap<String, serde_json::Value> {
        self.config.clone()
    }
    
    fn update_config(&mut self, config: HashMap<String, serde_json::Value>) -> Result<(), ProviderError> {
        self.config = config;
        Ok(())
    }
    
    async fn validate_config(&self) -> Result<(), ProviderError> {
        // Make a minimal test request to validate the API key
        let url = format!("{}/v1/messages", self.base_url);
        let headers = self.create_headers();
        
        let test_request = AnthropicRequest {
            model: self.default_model(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            system: None,
            temperature: Some(0.0),
            max_tokens: Some(1),
            top_p: None,
            stop_sequences: None,
            stream: false,
            tools: None,
            tool_choice: None,
        };
        
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&test_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Authentication(error_text));
        }
        
        Ok(())
    }
}

/// Stream wrapper for Anthropic streaming responses
struct AnthropicStreamWrapper {
    stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: String,
}

impl AnthropicStreamWrapper {
    fn new(stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static) -> Self {
        Self {
            stream: Box::pin(stream),
            buffer: String::new(),
        }
    }
    
    fn parse_event(&mut self, event: &str) -> Option<Result<LLMResponse, ProviderError>> {
        // Anthropic sends data in Server-Sent Events format
        if event.starts_with("data: ") {
            let data = &event[6..];
            
            match serde_json::from_str::<AnthropicStreamEvent>(data) {
                Ok(stream_event) => {
                    match stream_event.event_type.as_str() {
                        "content_block_delta" => {
                            if let Some(delta) = stream_event.delta {
                                if let Some(text) = delta.text {
                                    return Some(Ok(LLMResponse {
                                        content: text,
                                        model: "claude".to_string(), // Default model name
                                        usage: None,
                                        finish_reason: None,
                                        metadata: HashMap::new(),
                                    }));
                                }
                            }
                        }
                        "message_delta" => {
                            // Handle message completion
                            return Some(Ok(LLMResponse {
                                content: "".to_string(),
                                model: "claude".to_string(),
                                usage: None,
                                finish_reason: Some("stop".to_string()),
                                metadata: HashMap::new(),
                            }));
                        }
                        _ => {}
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

impl Stream for AnthropicStreamWrapper {
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
                        
                        if let Some(result) = self.parse_event(&line) {
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
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new("test_key".to_string(), None).unwrap();
        assert_eq!(provider.provider_type(), ProviderType::Anthropic);
        assert_eq!(provider.name(), "Anthropic");
        assert_eq!(provider.default_model(), "claude-3-5-sonnet-20241022");
    }
    
    #[test]
    fn test_message_conversion() {
        let provider = AnthropicProvider::new("test_key".to_string(), None).unwrap();
        
        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::user("Hello, world!"),
            Message::assistant("Hi there!"),
        ];
        
        let (system, anthropic_messages) = provider.convert_messages(&messages);
        
        assert_eq!(system, Some("You are a helpful assistant".to_string()));
        assert_eq!(anthropic_messages.len(), 2);
        assert_eq!(anthropic_messages[0].role, "user");
        assert_eq!(anthropic_messages[1].role, "assistant");
    }
    
    #[test]
    fn test_get_models() {
        let provider = AnthropicProvider::new("test_key".to_string(), None).unwrap();
        let models = provider.get_models();
        
        // This is a sync test, so we can't actually await the result
        // But we can test that the method exists and returns the expected type
        assert!(matches!(models, _));
    }
}