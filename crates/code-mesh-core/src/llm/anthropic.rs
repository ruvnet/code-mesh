//! Anthropic provider implementation with full streaming support

use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use futures_util::{Stream, StreamExt};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use parking_lot::Mutex;
use std::sync::Arc;
use futures::future;

use super::{
    Provider, Model, LanguageModel, Message, MessageRole, MessageContent,
    GenerateOptions, GenerateResult, StreamOptions, StreamChunk,
    Usage, FinishReason, ToolCall, ToolDefinition,
};
use crate::auth::{Auth, AuthCredentials};

/// Simple auth implementation for models
struct SimpleAnthropicAuth;

impl SimpleAnthropicAuth {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Auth for SimpleAnthropicAuth {
    fn provider_id(&self) -> &str {
        "anthropic"
    }
    
    async fn get_credentials(&self) -> crate::Result<AuthCredentials> {
        // Try to get API key from environment
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            Ok(AuthCredentials::ApiKey { key: api_key })
        } else {
            Err(crate::Error::AuthenticationFailed(
                "No ANTHROPIC_API_KEY environment variable found".to_string()
            ))
        }
    }
    
    async fn set_credentials(&self, _credentials: AuthCredentials) -> crate::Result<()> {
        // Simple auth doesn't support setting credentials
        Err(crate::Error::AuthenticationFailed(
            "Setting credentials not supported in simple auth".to_string()
        ))
    }
    
    async fn remove_credentials(&self) -> crate::Result<()> {
        // Simple auth doesn't support removing credentials
        Ok(())
    }
    
    async fn has_credentials(&self) -> bool {
        std::env::var("ANTHROPIC_API_KEY").is_ok()
    }
}

/// Anthropic provider with rate limiting and retry logic
pub struct AnthropicProvider {
    client: Client,
    auth: Box<dyn Auth>,
    rate_limiter: Arc<RateLimiter>,
}

/// Rate limiter for API requests
pub(crate) struct RateLimiter {
    pub(crate) last_request: Mutex<Option<Instant>>,
    pub(crate) min_interval: Duration,
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        Self {
            last_request: Mutex::new(None),
            min_interval: Duration::from_millis(100), // 10 RPS max
        }
    }
    
    pub(crate) async fn acquire(&self) {
        let should_wait = {
            let mut last = self.last_request.lock();
            if let Some(last_time) = *last {
                let elapsed = last_time.elapsed();
                if elapsed < self.min_interval {
                    Some(self.min_interval - elapsed)
                } else {
                    *last = Some(Instant::now());
                    None
                }
            } else {
                *last = Some(Instant::now());
                None
            }
        };
        
        if let Some(wait_time) = should_wait {
            sleep(wait_time).await;
            self.last_request.lock().replace(Instant::now());
        }
    }
}

impl AnthropicProvider {
    pub fn new(auth: Box<dyn Auth>) -> Self {
        let client = Client::builder()
            .default_headers({
                let mut headers = header::HeaderMap::new();
                headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
                headers.insert("accept", "application/json".parse().unwrap());
                headers.insert("content-type", "application/json".parse().unwrap());
                headers.insert("user-agent", "code-mesh/0.1.0".parse().unwrap());
                headers
            })
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap();
        
        Self { 
            client, 
            auth,
            rate_limiter: Arc::new(RateLimiter::new()),
        }
    }
    
    /// Execute request with retry logic
    async fn execute_with_retry<F, T>(&self, operation: F) -> crate::Result<T>
    where
        F: Fn() -> future::BoxFuture<'static, crate::Result<T>>,
    {
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            self.rate_limiter.acquire().await;
            
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    
                    if attempts >= max_attempts {
                        return Err(e);
                    }
                    
                    // Check if error is retryable
                    let should_retry = match &e {
                        crate::Error::Network(req_err) => {
                            req_err.status().map_or(true, |status| {
                                status.as_u16() >= 500 || status.as_u16() == 429
                            })
                        },
                        crate::Error::Provider(msg) => {
                            msg.contains("rate_limit") || msg.contains("timeout")
                        },
                        _ => false,
                    };
                    
                    if !should_retry {
                        return Err(e);
                    }
                    
                    // Exponential backoff
                    let delay = Duration::from_millis(1000 * (2_u64.pow(attempts - 1)));
                    sleep(delay).await;
                }
            }
        }
    }
    
    /// Validate and refresh credentials
    pub(crate) async fn validate_and_refresh_credentials(&self) -> crate::Result<String> {
        let credentials = self.auth.get_credentials().await?;
        
        match credentials {
            AuthCredentials::ApiKey { key } => {
                // Validate API key format
                if !key.starts_with("sk-ant-") {
                    return Err(crate::Error::AuthenticationFailed(
                        "Invalid Anthropic API key format".to_string()
                    ));
                }
                Ok(key)
            },
            AuthCredentials::OAuth { access_token, refresh_token, expires_at } => {
                // Check if token is expired
                if let Some(expires_at) = expires_at {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    
                    if now >= expires_at {
                        // Try to refresh token
                        if let Some(refresh_token) = refresh_token {
                            return self.refresh_oauth_token(refresh_token).await;
                        } else {
                            return Err(crate::Error::AuthenticationFailed(
                                "OAuth token expired and no refresh token available".to_string()
                            ));
                        }
                    }
                }
                Ok(access_token)
            },
            _ => Err(crate::Error::AuthenticationFailed(
                "Unsupported credential type for Anthropic".to_string()
            )),
        }
    }
    
    async fn refresh_oauth_token(&self, refresh_token: String) -> crate::Result<String> {
        // Implement OAuth token refresh
        let refresh_request = self.client
            .post("https://api.anthropic.com/oauth/token")
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": refresh_token
            }))
            .send()
            .await?;
        
        if !refresh_request.status().is_success() {
            return Err(crate::Error::AuthenticationFailed(
                "Failed to refresh OAuth token".to_string()
            ));
        }
        
        let refresh_response: serde_json::Value = refresh_request.json().await?;
        
        let new_access_token = refresh_response["access_token"]
            .as_str()
            .ok_or_else(|| crate::Error::AuthenticationFailed(
                "Invalid refresh response".to_string()
            ))?
            .to_string();
        
        let new_refresh_token = refresh_response["refresh_token"]
            .as_str()
            .map(|s| s.to_string());
        
        let expires_in = refresh_response["expires_in"]
            .as_u64()
            .unwrap_or(3600);
        
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + expires_in;
        
        // Store updated credentials
        let new_credentials = AuthCredentials::OAuth {
            access_token: new_access_token.clone(),
            refresh_token: new_refresh_token,
            expires_at: Some(expires_at),
        };
        
        self.auth.set_credentials(new_credentials).await?;
        
        Ok(new_access_token)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
    }
    
    fn name(&self) -> &str {
        "Anthropic"
    }
    
    fn api_endpoint(&self) -> Option<&str> {
        Some("https://api.anthropic.com/v1")
    }
    
    fn env_vars(&self) -> &[String] {
        static ENV_VARS: Lazy<Vec<String>> = Lazy::new(|| {
            vec!["ANTHROPIC_API_KEY".to_string()]
        });
        &ENV_VARS
    }
    
    fn npm_package(&self) -> Option<&str> {
        Some("@anthropic-ai/sdk")
    }
    
    fn models(&self) -> &HashMap<String, Box<dyn Model>> {
        static MODELS: Lazy<HashMap<String, Box<dyn Model>>> = Lazy::new(|| {
            let mut models = HashMap::new();
            
            // Claude 3.5 Sonnet - Latest flagship model
            models.insert("claude-3-5-sonnet-20241022".to_string(), Box::new(AnthropicModelInfo {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: "Claude 3.5 Sonnet".to_string(),
                release_date: "2024-10-22".to_string(),
                capabilities: super::provider::ModelCapabilities {
                    attachment: true,
                    reasoning: true,
                    temperature: true,
                    tool_call: true,
                    vision: true,
                    caching: true,
                },
                cost: super::provider::Cost {
                    input: 3.0,
                    output: 15.0,
                    cache_read: Some(0.3),
                    cache_write: Some(3.75),
                },
                limits: super::provider::Limits {
                    context: 200000,
                    output: 8192,
                },
                options: HashMap::new(),
            }) as Box<dyn Model>);
            
            // Claude 3.5 Haiku - Fast and efficient
            models.insert("claude-3-5-haiku-20241022".to_string(), Box::new(AnthropicModelInfo {
                id: "claude-3-5-haiku-20241022".to_string(),
                name: "Claude 3.5 Haiku".to_string(),
                release_date: "2024-10-22".to_string(),
                capabilities: super::provider::ModelCapabilities {
                    attachment: true,
                    reasoning: true,
                    temperature: true,
                    tool_call: true,
                    vision: true,
                    caching: false,
                },
                cost: super::provider::Cost {
                    input: 1.0,
                    output: 5.0,
                    cache_read: None,
                    cache_write: None,
                },
                limits: super::provider::Limits {
                    context: 200000,
                    output: 8192,
                },
                options: HashMap::new(),
            }) as Box<dyn Model>);
            
            // Claude 3 Opus - Most capable
            models.insert("claude-3-opus-20240229".to_string(), Box::new(AnthropicModelInfo {
                id: "claude-3-opus-20240229".to_string(),
                name: "Claude 3 Opus".to_string(),
                release_date: "2024-02-29".to_string(),
                capabilities: super::provider::ModelCapabilities {
                    attachment: true,
                    reasoning: true,
                    temperature: true,
                    tool_call: true,
                    vision: true,
                    caching: true,
                },
                cost: super::provider::Cost {
                    input: 15.0,
                    output: 75.0,
                    cache_read: Some(1.5),
                    cache_write: Some(18.75),
                },
                limits: super::provider::Limits {
                    context: 200000,
                    output: 4096,
                },
                options: HashMap::new(),
            }) as Box<dyn Model>);
            
            models
        });
        &MODELS
    }
    
    async fn authenticate(&self) -> crate::Result<AuthCredentials> {
        self.auth.get_credentials().await
    }
    
    async fn get_model(&self, model_id: &str) -> crate::Result<Box<dyn LanguageModel>> {
        // Create a simple API key auth for the model  
        let auth: Box<dyn Auth> = Box::new(SimpleAnthropicAuth::new());
        
        Ok(Box::new(AnthropicModel {
            client: self.client.clone(),
            model_id: model_id.to_string(),
            auth,
            rate_limiter: self.rate_limiter.clone(),
        }))
    }
}

/// Anthropic model implementation
struct AnthropicModel {
    client: Client,
    model_id: String,
    auth: Box<dyn Auth>,
    rate_limiter: Arc<RateLimiter>,
}

impl AnthropicModel {
    /// Simple retry logic for requests  
    async fn execute_with_retry_simple<F, Fut, T>(&self, mut operation: F) -> crate::Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = crate::Result<T>>,
    {
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            self.rate_limiter.acquire().await;
            
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    
                    if attempts >= max_attempts {
                        return Err(e);
                    }
                    
                    // Check if error is retryable
                    let should_retry = match &e {
                        crate::Error::Network(req_err) => {
                            req_err.status().map_or(true, |status| {
                                status.as_u16() >= 500 || status.as_u16() == 429
                            })
                        },
                        crate::Error::Provider(msg) => {
                            msg.contains("rate_limit") || msg.contains("timeout")
                        },
                        _ => false,
                    };
                    
                    if !should_retry {
                        return Err(e);
                    }
                    
                    // Exponential backoff
                    let delay = Duration::from_millis(1000 * (2_u64.pow(attempts - 1)));
                    sleep(delay).await;
                }
            }
        }
    }
}

#[async_trait]
impl LanguageModel for AnthropicModel {
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        // Get credentials
        let credentials = self.auth.get_credentials().await?;
        let api_key = match credentials {
            AuthCredentials::ApiKey { key } => {
                if !key.starts_with("sk-ant-") {
                    return Err(crate::Error::AuthenticationFailed(
                        "Invalid Anthropic API key format".to_string()
                    ));
                }
                key
            },
            AuthCredentials::OAuth { access_token, .. } => access_token,
            _ => return Err(crate::Error::AuthenticationFailed(
                "Unsupported credential type for Anthropic".to_string()
            )),
        };
        
        // Convert messages to Anthropic format
        let (system_prompt, anthropic_messages) = convert_messages_with_system(messages)?;
        
        // Build request
        let mut request_body = serde_json::json!({
            "model": self.model_id,
            "messages": anthropic_messages,
            "max_tokens": options.max_tokens.unwrap_or(4096),
        });
        
        if let Some(system) = system_prompt {
            request_body["system"] = serde_json::json!(system);
        }
        
        if let Some(temp) = options.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }
        
        if !options.stop_sequences.is_empty() {
            request_body["stop_sequences"] = serde_json::json!(options.stop_sequences);
        }
        
        if !options.tools.is_empty() {
            request_body["tools"] = serde_json::json!(convert_tools_to_anthropic(options.tools));
        }
        
        // Send request with retry logic
        let client = self.client.clone();
        let response = self.execute_with_retry_simple(|| {
            let client = client.clone();
            let api_key = api_key.clone();
            let request_body = request_body.clone();
            
            Box::pin(async move {
                let response = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", api_key)
                    .json(&request_body)
                    .send()
                    .await?;
                
                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    
                    return Err(crate::Error::Provider(format!(
                        "Anthropic API error ({}): {}", 
                        status.as_u16(), 
                        error_text
                    )));
                }
                
                Ok(response)
            })
        }).await?;
        
        let api_response: AnthropicResponse = response.json().await?;
        
        // Convert response
        Ok(GenerateResult {
            content: extract_content(&api_response),
            tool_calls: extract_tool_calls(&api_response),
            usage: Usage {
                prompt_tokens: api_response.usage.input_tokens,
                completion_tokens: api_response.usage.output_tokens,
                total_tokens: api_response.usage.input_tokens + api_response.usage.output_tokens,
            },
            finish_reason: convert_finish_reason(&api_response.stop_reason),
        })
    }
    
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: StreamOptions,
    ) -> crate::Result<Box<dyn futures::Stream<Item = crate::Result<StreamChunk>> + Send + Unpin>> {
        // Get credentials
        let credentials = self.auth.get_credentials().await?;
        let api_key = match credentials {
            AuthCredentials::ApiKey { key } => {
                if !key.starts_with("sk-ant-") {
                    return Err(crate::Error::AuthenticationFailed(
                        "Invalid Anthropic API key format".to_string()
                    ));
                }
                key
            },
            AuthCredentials::OAuth { access_token, .. } => access_token,
            _ => return Err(crate::Error::AuthenticationFailed(
                "Unsupported credential type for Anthropic".to_string()
            )),
        };
        
        // Convert messages to Anthropic format
        let (system_prompt, anthropic_messages) = convert_messages_with_system(messages)?;
        
        // Build request
        let mut request_body = serde_json::json!({
            "model": self.model_id,
            "messages": anthropic_messages,
            "max_tokens": options.max_tokens.unwrap_or(4096),
            "stream": true
        });
        
        if let Some(system) = system_prompt {
            request_body["system"] = serde_json::json!(system);
        }
        
        if let Some(temp) = options.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }
        
        if !options.stop_sequences.is_empty() {
            request_body["stop_sequences"] = serde_json::json!(options.stop_sequences);
        }
        
        if !options.tools.is_empty() {
            request_body["tools"] = serde_json::json!(convert_tools_to_anthropic(options.tools));
        }
        
        // Simple request for streaming (no complex retry for streams)
        self.rate_limiter.acquire().await;
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("accept", "text/event-stream")
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            return Err(crate::Error::Provider(format!(
                "Anthropic streaming API error ({}): {}", 
                status.as_u16(), 
                error_text
            )));
        }
        
        // Create SSE stream
        let stream = AnthropicStream::new(response.bytes_stream());
        Ok(Box::new(stream))
    }
    
    fn supports_tools(&self) -> bool {
        matches!(self.model_id.as_str(), 
            "claude-3-5-sonnet-20241022" | 
            "claude-3-5-haiku-20241022" | 
            "claude-3-opus-20240229" |
            "claude-3-sonnet-20240229" |
            "claude-3-haiku-20240307"
        )
    }
    
    fn supports_vision(&self) -> bool {
        self.model_id.contains("claude-3")
    }
    
    fn supports_caching(&self) -> bool {
        matches!(self.model_id.as_str(), 
            "claude-3-5-sonnet-20241022" | 
            "claude-3-opus-20240229"
        )
    }
}

// Model info implementation
#[derive(Debug, Clone)]
pub(crate) struct AnthropicModelInfo {
    id: String,
    name: String,
    release_date: String,
    capabilities: super::provider::ModelCapabilities,
    cost: super::provider::Cost,
    limits: super::provider::Limits,
    options: HashMap<String, serde_json::Value>,
}

impl Model for AnthropicModelInfo {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn release_date(&self) -> &str { &self.release_date }
    fn capabilities(&self) -> &super::provider::ModelCapabilities { &self.capabilities }
    fn cost(&self) -> &super::provider::Cost { &self.cost }
    fn limits(&self) -> &super::provider::Limits { &self.limits }
    fn options(&self) -> &HashMap<String, serde_json::Value> { &self.options }
}

// Response types
#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// Streaming types
#[derive(Debug, Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct MessageStart {
    message: MessageStartData,
}

#[derive(Debug, Deserialize)]
struct MessageStartData {
    id: String,
    #[serde(rename = "type")]
    message_type: String,
    role: String,
    model: String,
    content: Vec<serde_json::Value>,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlockStart {
    index: u32,
    content_block: ContentBlock,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ContentBlockDelta {
    index: u32,
    delta: ContentDelta,
}

#[derive(Debug, Deserialize)]
struct ContentDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
    partial_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageDelta {
    delta: MessageDeltaData,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaData {
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
}

// Streaming implementation
#[pin_project]
struct AnthropicStream {
    #[pin]
    inner: futures_util::stream::BoxStream<'static, Result<Bytes, reqwest::Error>>,
    buffer: String,
    current_tool_calls: Vec<ToolCall>,
    tool_call_buffer: HashMap<u32, (String, String)>, // index -> (name, partial_json)
    finished: bool,
}

impl AnthropicStream {
    fn new(stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static) -> Self {
        Self {
            inner: stream.boxed(),
            buffer: String::new(),
            current_tool_calls: Vec::new(),
            tool_call_buffer: HashMap::new(),
            finished: false,
        }
    }
    
    fn parse_sse_line(&mut self, line: &str) -> Option<crate::Result<StreamChunk>> {
        if line.is_empty() || line.starts_with(':') {
            return None;
        }
        
        if !line.starts_with("data: ") {
            return None;
        }
        
        let data = &line[6..]; // Remove "data: " prefix
        
        if data == "[DONE]" {
            self.finished = true;
            return Some(Ok(StreamChunk {
                delta: String::new(),
                tool_calls: Vec::new(),
                finish_reason: Some(FinishReason::Stop),
            }));
        }
        
        let event: StreamEvent = match serde_json::from_str(data) {
            Ok(event) => event,
            Err(e) => {
                return Some(Err(crate::Error::Provider(
                    format!("Failed to parse SSE event: {}", e)
                )));
            }
        };
        
        self.process_stream_event(event)
    }
    
    fn process_stream_event(&mut self, event: StreamEvent) -> Option<crate::Result<StreamChunk>> {
        process_stream_event_static(event, &mut self.tool_call_buffer, &mut self.current_tool_calls, &mut self.finished)
    }
}

/// Static function to process SSE lines
fn process_sse_line_static(
    line: &str,
    tool_call_buffer: &mut HashMap<u32, (String, String)>,
    _current_tool_calls: &mut Vec<ToolCall>,
    finished: &mut bool,
) -> Option<crate::Result<StreamChunk>> {
    if line.is_empty() || line.starts_with(':') {
        return None;
    }
    
    if !line.starts_with("data: ") {
        return None;
    }
    
    let data = &line[6..]; // Remove "data: " prefix
    
    if data == "[DONE]" {
        *finished = true;
        return Some(Ok(StreamChunk {
            delta: String::new(),
            tool_calls: Vec::new(),
            finish_reason: Some(FinishReason::Stop),
        }));
    }
    
    let event: StreamEvent = match serde_json::from_str(data) {
        Ok(event) => event,
        Err(e) => {
            return Some(Err(crate::Error::Provider(
                format!("Failed to parse SSE event: {}", e)
            )));
        }
    };
    
    process_stream_event_static(event, tool_call_buffer, _current_tool_calls, finished)
}

/// Static function to process stream events
fn process_stream_event_static(
    event: StreamEvent,
    tool_call_buffer: &mut HashMap<u32, (String, String)>,
    current_tool_calls: &mut Vec<ToolCall>,
    finished: &mut bool,
) -> Option<crate::Result<StreamChunk>> {
        match event.event_type.as_str() {
            "message_start" => {
                // Message initialization - no output yet
                None
            }
            "content_block_start" => {
                if let Ok(block_start) = serde_json::from_value::<ContentBlockStart>(event.data) {
                    if block_start.content_block.block_type == "tool_use" {
                        if let (Some(name), Some(_)) = (&block_start.content_block.name, &block_start.content_block.input) {
                            tool_call_buffer.insert(block_start.index, (name.clone(), String::new()));
                        }
                    }
                }
                None
            }
            "content_block_delta" => {
                if let Ok(delta) = serde_json::from_value::<ContentBlockDelta>(event.data) {
                    match delta.delta.delta_type.as_str() {
                        "text_delta" => {
                            if let Some(text) = delta.delta.text {
                                return Some(Ok(StreamChunk {
                                    delta: text,
                                    tool_calls: Vec::new(),
                                    finish_reason: None,
                                }));
                            }
                        }
                        "input_json_delta" => {
                            if let Some(partial_json) = delta.delta.partial_json {
                                if let Some((_name, existing_json)) = tool_call_buffer.get_mut(&delta.index) {
                                    existing_json.push_str(&partial_json);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            "content_block_stop" => {
                if let Ok(block_stop) = serde_json::from_value::<serde_json::Value>(event.data) {
                    if let Some(index) = block_stop.get("index").and_then(|i| i.as_u64()) {
                        if let Some((name, json_str)) = tool_call_buffer.remove(&(index as u32)) {
                            if let Ok(arguments) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                let tool_call = ToolCall {
                                    id: format!("call_{}", uuid::Uuid::new_v4()),
                                    name,
                                    arguments,
                                };
                                current_tool_calls.push(tool_call.clone());
                                return Some(Ok(StreamChunk {
                                    delta: String::new(),
                                    tool_calls: vec![tool_call],
                                    finish_reason: None,
                                }));
                            }
                        }
                    }
                }
                None
            }
            "message_delta" => {
                if let Ok(msg_delta) = serde_json::from_value::<MessageDelta>(event.data) {
                    if let Some(stop_reason) = msg_delta.delta.stop_reason {
                        let finish_reason = match stop_reason.as_str() {
                            "end_turn" => FinishReason::Stop,
                            "max_tokens" => FinishReason::Length,
                            "tool_use" => FinishReason::ToolCalls,
                            "stop_sequence" => FinishReason::Stop,
                            _ => FinishReason::Stop,
                        };
                        return Some(Ok(StreamChunk {
                            delta: String::new(),
                            tool_calls: Vec::new(),
                            finish_reason: Some(finish_reason),
                        }));
                    }
                }
                None
            }
            "message_stop" => {
                *finished = true;
                Some(Ok(StreamChunk {
                    delta: String::new(),
                    tool_calls: Vec::new(),
                    finish_reason: Some(FinishReason::Stop),
                }))
            }
            "error" => {
                if let Some(error_msg) = event.data.get("error").and_then(|e| e.as_str()) {
                    Some(Err(crate::Error::Provider(format!("Anthropic streaming error: {}", error_msg))))
                } else {
                    Some(Err(crate::Error::Provider("Unknown Anthropic streaming error".to_string())))
                }
            }
            _ => None, // Ignore unknown event types
        }
}

impl Stream for AnthropicStream {
    type Item = crate::Result<StreamChunk>;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        
        if *this.finished {
            return Poll::Ready(None);
        }
        
        loop {
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    this.buffer.push_str(&chunk_str);
                    
                    // Process complete lines
                    while let Some(line_end) = this.buffer.find('\n') {
                        let line = this.buffer[..line_end].trim_end_matches('\r').to_string();
                        this.buffer.drain(..=line_end);
                        
                        if let Some(result) = process_sse_line_static(&line, &mut this.tool_call_buffer, &mut this.current_tool_calls, &mut this.finished) {
                            return Poll::Ready(Some(result));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(crate::Error::Other(e.into()))));
                }
                Poll::Ready(None) => {
                    *this.finished = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// Helper functions
fn convert_messages(messages: Vec<Message>) -> Vec<serde_json::Value> {
    messages.into_iter().filter_map(|msg| {
        // Skip system messages - they're handled separately in Anthropic API
        if matches!(msg.role, MessageRole::System) {
            return None;
        }
        
        let role = match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "user", // Tool results are sent as user messages in Anthropic
            MessageRole::System => return None, // Already filtered above
        };
        
        let content = match msg.content {
            MessageContent::Text(text) => {
                if matches!(msg.role, MessageRole::Tool) {
                    // Format tool result
                    serde_json::json!([{
                        "type": "tool_result",
                        "tool_use_id": msg.tool_call_id.unwrap_or_else(|| "unknown".to_string()),
                        "content": text
                    }])
                } else {
                    serde_json::json!(text)
                }
            }
            MessageContent::Parts(parts) => {
                let anthropic_content: Vec<serde_json::Value> = parts.into_iter().map(|part| {
                    match part {
                        super::MessagePart::Text { text } => serde_json::json!({
                            "type": "text",
                            "text": text
                        }),
                        super::MessagePart::Image { image } => {
                            if let Some(base64) = image.base64 {
                                serde_json::json!({
                                    "type": "image",
                                    "source": {
                                        "type": "base64",
                                        "media_type": image.mime_type,
                                        "data": base64
                                    }
                                })
                            } else if let Some(url) = image.url {
                                serde_json::json!({
                                    "type": "image",
                                    "source": {
                                        "type": "url",
                                        "url": url
                                    }
                                })
                            } else {
                                serde_json::json!({
                                    "type": "text",
                                    "text": "[Invalid image data]"
                                })
                            }
                        }
                    }
                }).collect();
                serde_json::json!(anthropic_content)
            }
        };
        
        let mut obj = serde_json::json!({
            "role": role,
            "content": content,
        });
        
        // Add tool calls if present
        if let Some(tool_calls) = &msg.tool_calls {
            if !tool_calls.is_empty() {
                let mut content_array = vec![];
                
                // Add existing content if it's text
                if let serde_json::Value::String(text) = &content {
                    if !text.trim().is_empty() {
                        content_array.push(serde_json::json!({
                            "type": "text",
                            "text": text
                        }));
                    }
                }
                
                // Add tool calls
                for tool_call in tool_calls {
                    content_array.push(serde_json::json!({
                        "type": "tool_use",
                        "id": tool_call.id,
                        "name": tool_call.name,
                        "input": tool_call.arguments
                    }));
                }
                
                obj["content"] = serde_json::json!(content_array);
            }
        }
        
        Some(obj)
    }).collect()
}

/// Convert messages and extract system prompt separately
pub(crate) fn convert_messages_with_system(messages: Vec<Message>) -> crate::Result<(Option<String>, Vec<serde_json::Value>)> {
    let mut system_prompt = None;
    let mut filtered_messages = Vec::new();
    
    for msg in messages {
        match msg.role {
            MessageRole::System => {
                match msg.content {
                    MessageContent::Text(text) => {
                        if system_prompt.is_some() {
                            // Combine multiple system messages
                            let existing = system_prompt.take().unwrap();
                            system_prompt = Some(format!("{} {}", existing, text));
                        } else {
                            system_prompt = Some(text);
                        }
                    }
                    MessageContent::Parts(_) => {
                        return Err(crate::Error::Other(anyhow::anyhow!(
                            "System messages with parts are not supported by Anthropic API"
                        )));
                    }
                }
            }
            _ => {
                filtered_messages.push(msg);
            }
        }
    }
    
    Ok((system_prompt, convert_messages(filtered_messages)))
}

fn convert_tools_to_anthropic(tools: Vec<ToolDefinition>) -> Vec<serde_json::Value> {
    tools.into_iter().map(|tool| {
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.parameters,
        })
    }).collect()
}

fn extract_content(response: &AnthropicResponse) -> String {
    response.content.iter()
        .filter_map(|c| c.text.as_ref())
        .cloned()
        .collect::<Vec<_>>()
        .join("")
}

fn extract_tool_calls(response: &AnthropicResponse) -> Vec<ToolCall> {
    response.content.iter()
        .filter(|c| c.content_type == "tool_use")
        .filter_map(|c| {
            Some(ToolCall {
                id: uuid::Uuid::new_v4().to_string(),
                name: c.name.clone()?,
                arguments: c.input.clone()?,
            })
        })
        .collect()
}

fn convert_finish_reason(stop_reason: &Option<String>) -> FinishReason {
    match stop_reason.as_deref() {
        Some("end_turn") => FinishReason::Stop,
        Some("max_tokens") => FinishReason::Length,
        Some("tool_use") => FinishReason::ToolCalls,
        _ => FinishReason::Stop,
    }
}

