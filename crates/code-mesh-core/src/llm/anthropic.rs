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
    Provider, Model, Message, MessageRole, MessageContent, MessagePart,
    GenerateOptions, GenerateResult, StreamChunk, StreamOptions,
    ToolCall, ToolDefinition, Usage, FinishReason, LanguageModel,
    ModelInfo, ModelCapabilities, ModelLimits, ModelPricing, ModelStatus,
    ProviderHealth, ProviderConfig, RateLimitInfo, UsageStats,
    ModelConfig,
};
use super::provider::ModelMetadata;
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

/// Anthropic provider operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnthropicMode {
    /// Standard API mode
    Standard,
    /// Batch processing mode (lower cost, higher latency)
    Batch,
    /// Real-time mode (optimized for low latency)
    RealTime,
}

impl Default for AnthropicMode {
    fn default() -> Self {
        Self::Standard
    }
}

/// Anthropic provider with rate limiting and retry logic
#[derive(Clone)]
pub struct AnthropicProvider {
    client: Client,
    auth: Arc<dyn Auth>,
    rate_limiter: Arc<RateLimiter>,
    config: ProviderConfig,
    mode: AnthropicMode,
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
            auth: Arc::from(auth),
            rate_limiter: Arc::new(RateLimiter::new()),
            config: ProviderConfig {
                provider_id: "anthropic".to_string(),
                ..Default::default()
            },
            mode: AnthropicMode::default(),
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
    
    fn base_url(&self) -> &str {
        "https://api.anthropic.com"
    }
    
    fn api_version(&self) -> &str {
        "2023-06-01"
    }
    
    async fn list_models(&self) -> crate::Result<Vec<ModelInfo>> {
        // Return hardcoded list of available Anthropic models
        Ok(vec![
            ModelInfo {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: "Claude 3.5 Sonnet".to_string(),
                description: Some("Latest flagship model with improved performance".to_string()),
                capabilities: ModelCapabilities {
                    text_generation: true,
                    tool_calling: true,
                    vision: true,
                    streaming: true,
                    caching: true,
                    json_mode: true,
                    reasoning: true,
                    code_generation: true,
                    multilingual: true,
                    custom: HashMap::new(),
                },
                limits: ModelLimits {
                    max_context_tokens: 200000,
                    max_output_tokens: 8192,
                    max_image_size_bytes: Some(5 * 1024 * 1024),
                    max_images_per_request: Some(20),
                    max_tool_calls: Some(20),
                    rate_limits: RateLimitInfo {
                        requests_per_minute: Some(100),
                        tokens_per_minute: Some(40000),
                        tokens_per_day: None,
                        concurrent_requests: Some(10),
                        current_usage: None,
                    },
                },
                pricing: ModelPricing {
                    input_cost_per_1k: 3.0,
                    output_cost_per_1k: 15.0,
                    cache_read_cost_per_1k: Some(0.3),
                    cache_write_cost_per_1k: Some(3.75),
                    currency: "USD".to_string(),
                },
                release_date: Some(chrono::DateTime::parse_from_rfc3339("2024-10-22T00:00:00Z").unwrap().with_timezone(&chrono::Utc)),
                status: ModelStatus::Active,
            },
            ModelInfo {
                id: "claude-3-5-haiku-20241022".to_string(),
                name: "Claude 3.5 Haiku".to_string(),
                description: Some("Fast and efficient model".to_string()),
                capabilities: ModelCapabilities {
                    text_generation: true,
                    tool_calling: true,
                    vision: true,
                    streaming: true,
                    caching: false,
                    json_mode: true,
                    reasoning: true,
                    code_generation: true,
                    multilingual: true,
                    custom: HashMap::new(),
                },
                limits: ModelLimits {
                    max_context_tokens: 200000,
                    max_output_tokens: 8192,
                    max_image_size_bytes: Some(5 * 1024 * 1024),
                    max_images_per_request: Some(20),
                    max_tool_calls: Some(20),
                    rate_limits: RateLimitInfo {
                        requests_per_minute: Some(200),
                        tokens_per_minute: Some(80000),
                        tokens_per_day: None,
                        concurrent_requests: Some(20),
                        current_usage: None,
                    },
                },
                pricing: ModelPricing {
                    input_cost_per_1k: 1.0,
                    output_cost_per_1k: 5.0,
                    cache_read_cost_per_1k: None,
                    cache_write_cost_per_1k: None,
                    currency: "USD".to_string(),
                },
                release_date: Some(chrono::DateTime::parse_from_rfc3339("2024-10-22T00:00:00Z").unwrap().with_timezone(&chrono::Utc)),
                status: ModelStatus::Active,
            },
        ])
    }
    
    async fn get_model(&self, model_id: &str) -> crate::Result<Arc<dyn Model>> {
        // Create the model
        let model = AnthropicModel::new(
            self.clone(),
            model_id.to_string(),
        );
        
        // Wrap it with provider
        let model_with_provider = AnthropicModelWithProvider::new(model, self.clone());
        
        // Create wrapper that implements both Model and LanguageModel
        let wrapper = AnthropicModelWrapper::new(model_with_provider);
        
        Ok(Arc::new(wrapper))
    }
    
    async fn health_check(&self) -> crate::Result<ProviderHealth> {
        // Check API availability
        let start = std::time::Instant::now();
        
        // Try to authenticate
        match self.auth.get_credentials().await {
            Ok(_) => {
                Ok(ProviderHealth {
                    available: true,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    error: None,
                    last_check: chrono::Utc::now(),
                    details: HashMap::new(),
                })
            }
            Err(e) => {
                Ok(ProviderHealth {
                    available: false,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    error: Some(e.to_string()),
                    last_check: chrono::Utc::now(),
                    details: HashMap::new(),
                })
            }
        }
    }
    
    fn get_config(&self) -> &ProviderConfig {
        &self.config
    }
    
    async fn update_config(&mut self, config: ProviderConfig) -> crate::Result<()> {
        self.config = config;
        Ok(())
    }
    
    async fn get_rate_limits(&self) -> crate::Result<RateLimitInfo> {
        // Return default rate limits for Anthropic
        Ok(RateLimitInfo {
            requests_per_minute: Some(100),
            tokens_per_minute: Some(40000),
            tokens_per_day: None,
            concurrent_requests: Some(10),
            current_usage: None,
        })
    }
    
    async fn get_usage(&self) -> crate::Result<UsageStats> {
        // Return empty usage stats for now
        Ok(UsageStats {
            total_requests: 0,
            total_tokens: 0,
            total_cost: 0.0,
            currency: "USD".to_string(),
            by_model: HashMap::new(),
            by_period: HashMap::new(),
        })
    }
}


/// Anthropic model implementation
pub struct AnthropicModel {
    id: String,
    provider: AnthropicProvider,
    client: Client,
    auth: Arc<dyn Auth>,
    rate_limiter: Arc<RateLimiter>,
    model_id: String,
}

impl AnthropicModel {
    /// Create new Anthropic model
    pub fn new(provider: AnthropicProvider, model_id: String) -> Self {
        Self {
            id: model_id.clone(),
            client: provider.client.clone(),
            auth: provider.auth.clone(),
            rate_limiter: provider.rate_limiter.clone(),
            model_id,
            provider,
        }
    }
    
    pub fn id(&self) -> &str {
        &self.id
    }
    
    pub fn name(&self) -> &str {
        &self.model_id
    }
    
    pub fn capabilities(&self) -> &ModelCapabilities {
        // Return a static reference to capabilities
        static CAPABILITIES: Lazy<ModelCapabilities> = Lazy::new(|| ModelCapabilities {
            text_generation: true,
            tool_calling: true,
            vision: true,
            streaming: true,
            caching: true,
            json_mode: true,
            reasoning: true,
            code_generation: true,
            multilingual: true,
            custom: HashMap::new(),
        });
        &*CAPABILITIES
    }
    
    pub fn config(&self) -> &ModelConfig {
        // Return a static reference to config
        static CONFIG: Lazy<ModelConfig> = Lazy::new(|| ModelConfig::default());
        &*CONFIG
    }
    
    pub fn metadata(&self) -> &ModelMetadata {
        // Return a static reference to metadata
        static METADATA: Lazy<ModelMetadata> = Lazy::new(|| ModelMetadata {
            family: "claude".to_string(),
            ..Default::default()
        });
        &*METADATA
    }
    
    pub async fn count_tokens(&self, messages: &[Message]) -> crate::Result<u32> {
        // Simple token estimation for now
        let mut total_tokens = 0u32;
        for message in messages {
            match &message.content {
                MessageContent::Text(text) => {
                    // Rough estimate: 1 token per 4 characters
                    total_tokens += (text.len() / 4) as u32;
                }
                MessageContent::Parts(parts) => {
                    for part in parts {
                        match part {
                            MessagePart::Text { text } => {
                                total_tokens += (text.len() / 4) as u32;
                            }
                            MessagePart::Image { .. } => {
                                // Images typically use ~1000 tokens
                                total_tokens += 1000;
                            }
                        }
                    }
                }
            }
        }
        Ok(total_tokens)
    }
    
    pub async fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> crate::Result<f64> {
        // Claude pricing (example rates)
        let input_cost_per_1k = match self.model_id.as_str() {
            "claude-3-opus-20240229" => 15.0,
            "claude-3-sonnet-20240229" => 3.0,
            "claude-3-haiku-20240307" => 0.25,
            _ => 3.0, // Default to Sonnet pricing
        };
        
        let output_cost_per_1k = match self.model_id.as_str() {
            "claude-3-opus-20240229" => 75.0,
            "claude-3-sonnet-20240229" => 15.0,
            "claude-3-haiku-20240307" => 1.25,
            _ => 15.0, // Default to Sonnet pricing
        };
        
        let input_cost = (input_tokens as f64 / 1000.0) * input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * output_cost_per_1k;
        
        Ok(input_cost + output_cost)
    }

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
    provider_id: String,
    capabilities: ModelCapabilities,
    config: ModelConfig,
    metadata: ModelMetadata,
}

impl AnthropicModelInfo {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id: id.clone(),
            name,
            provider_id: "anthropic".to_string(),
            capabilities: ModelCapabilities {
                text_generation: true,
                tool_calling: true,
                vision: true,
                streaming: true,
                caching: true,
                json_mode: true,
                reasoning: true,
                code_generation: true,
                multilingual: true,
                custom: HashMap::new(),
            },
            config: ModelConfig {
                model_id: id,
                ..Default::default()
            },
            metadata: ModelMetadata {
                family: "claude".to_string(),
                ..Default::default()
            },
        }
    }
}

#[async_trait]
impl Model for AnthropicModelInfo {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn provider_id(&self) -> &str { &self.provider_id }
    fn capabilities(&self) -> &ModelCapabilities { &self.capabilities }
    fn config(&self) -> &ModelConfig { &self.config }
    
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        Err(crate::Error::Other(anyhow::anyhow!("AnthropicModelInfo does not support generation directly")))
    }
    
    async fn stream(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<StreamChunk>> + Send>>> {
        Err(crate::Error::Other(anyhow::anyhow!("AnthropicModelInfo does not support streaming directly")))
    }
    
    async fn count_tokens(&self, _messages: &[Message]) -> crate::Result<u32> {
        Ok(0) // Placeholder
    }
    
    async fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> crate::Result<f64> {
        // Simple cost calculation based on Claude pricing
        let input_cost = (input_tokens as f64 / 1000.0) * 3.0;
        let output_cost = (output_tokens as f64 / 1000.0) * 15.0;
        Ok(input_cost + output_cost)
    }
    
    fn metadata(&self) -> &ModelMetadata { &self.metadata }
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
    inner: futures_util::stream::BoxStream<'static, std::result::Result<Bytes, reqwest::Error>>,
    buffer: String,
    current_tool_calls: Vec<ToolCall>,
    tool_call_buffer: HashMap<u32, (String, String)>, // index -> (name, partial_json)
    finished: bool,
}

impl AnthropicStream {
    fn new(stream: impl Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static) -> Self {
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

/// Anthropic model with associated provider
pub struct AnthropicModelWithProvider {
    model: AnthropicModel,
    provider: AnthropicProvider,
}

/// Wrapper that implements both Model and LanguageModel
pub struct AnthropicModelWrapper {
    inner: AnthropicModelWithProvider,
}

impl AnthropicModelWrapper {
    pub fn new(model_with_provider: AnthropicModelWithProvider) -> Self {
        Self { inner: model_with_provider }
    }
}

impl AnthropicModelWithProvider {
    /// Create a new Anthropic model with provider
    pub fn new(model: AnthropicModel, provider: AnthropicProvider) -> Self {
        Self { model, provider }
    }
    
    /// Get a reference to the model
    pub fn model(&self) -> &AnthropicModel {
        &self.model
    }
    
    /// Get a reference to the provider
    pub fn provider(&self) -> &AnthropicProvider {
        &self.provider
    }
}

#[async_trait]
impl Model for AnthropicModelWrapper {
    fn id(&self) -> &str { self.inner.model.id() }
    fn name(&self) -> &str { self.inner.model.name() }
    fn provider_id(&self) -> &str { "anthropic" }
    fn capabilities(&self) -> &ModelCapabilities { self.inner.model.capabilities() }
    fn config(&self) -> &ModelConfig { self.inner.model.config() }
    
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        self.inner.model.generate(messages, options).await
    }
    
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<Pin<Box<dyn Stream<Item = crate::Result<StreamChunk>> + Send>>> {
        // Convert GenerateOptions to StreamOptions
        let stream_options = StreamOptions {
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            tools: options.tools,
            stop_sequences: options.stop_sequences,
        };
        
        // Convert Box to Pin<Box>
        let stream = self.inner.model.stream(messages, stream_options).await?;
        Ok(Box::pin(stream))
    }
    
    async fn count_tokens(&self, messages: &[Message]) -> crate::Result<u32> {
        self.inner.model.count_tokens(messages).await
    }
    
    async fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> crate::Result<f64> {
        self.inner.model.estimate_cost(input_tokens, output_tokens).await
    }
    
    fn metadata(&self) -> &ModelMetadata {
        self.inner.model.metadata()
    }
}

#[async_trait]
impl LanguageModel for AnthropicModelWrapper {
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        self.inner.generate(messages, options).await
    }
    
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: StreamOptions,
    ) -> crate::Result<Box<dyn futures::Stream<Item = crate::Result<StreamChunk>> + Send + Unpin>> {
        self.inner.stream(messages, options).await
    }
    
    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }
    
    fn supports_vision(&self) -> bool {
        self.inner.supports_vision()
    }
    
    fn supports_caching(&self) -> bool {
        self.inner.supports_caching()
    }
}

#[async_trait]
impl LanguageModel for AnthropicModelWithProvider {
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        self.model.generate(messages, options).await
    }
    
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: StreamOptions,
    ) -> crate::Result<Box<dyn futures::Stream<Item = crate::Result<StreamChunk>> + Send + Unpin>> {
        self.model.stream(messages, options).await
    }
    
    fn supports_tools(&self) -> bool {
        true
    }
    
    fn supports_vision(&self) -> bool {
        true
    }
    
    fn supports_caching(&self) -> bool {
        true
    }
}

