//! Local provider implementation for self-hosted models

use super::{LLMProvider, LLMRequest, LLMResponse, ProviderError, ProviderType, Usage};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Local/self-hosted model provider
/// Compatible with OpenAI API format (used by many local serving solutions)
pub struct LocalProvider {
    client: Client,
    base_url: String,
    config: HashMap<String, serde_json::Value>,
}

/// Local API response format (OpenAI-compatible)
#[derive(Debug, Deserialize)]
struct LocalResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<LocalChoice>,
    usage: Option<LocalUsage>,
}

/// Local choice object
#[derive(Debug, Deserialize)]
struct LocalChoice {
    index: u32,
    message: LocalMessage,
    finish_reason: Option<String>,
}

/// Local message object
#[derive(Debug, Deserialize)]
struct LocalMessage {
    role: String,
    content: String,
}

/// Local usage object
#[derive(Debug, Deserialize)]
struct LocalUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Local models response
#[derive(Debug, Deserialize)]
struct LocalModelsResponse {
    object: String,
    data: Vec<LocalModel>,
}

/// Local model object
#[derive(Debug, Deserialize)]
struct LocalModel {
    id: String,
    object: String,
    created: Option<u64>,
    owned_by: Option<String>,
}

impl LocalProvider {
    /// Create a new local provider
    pub fn new(base_url: String) -> Result<Self, ProviderError> {
        let client = Client::new();
        
        Ok(LocalProvider {
            client,
            base_url,
            config: HashMap::new(),
        })
    }
    
    /// Convert internal request to OpenAI-compatible format
    fn convert_request(&self, request: &LLMRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|m| {
            let role = match m.role {
                super::MessageRole::System => "system",
                super::MessageRole::User => "user", 
                super::MessageRole::Assistant => "assistant",
                super::MessageRole::Tool => "tool",
            };
            
            serde_json::json!({
                "role": role,
                "content": m.content
            })
        }).collect();
        
        serde_json::json!({
            "model": request.model.as_ref().unwrap_or(&self.default_model()),
            "messages": messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "frequency_penalty": request.frequency_penalty,
            "presence_penalty": request.presence_penalty,
            "stop": request.stop,
            "stream": request.stream
        })
    }
    
    /// Convert local response to internal format
    fn convert_response(&self, response: LocalResponse) -> Result<LLMResponse, ProviderError> {
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
}

#[async_trait]
impl LLMProvider for LocalProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Local
    }
    
    fn name(&self) -> String {
        "Local".to_string()
    }
    
    async fn is_available(&self) -> bool {
        // Try to fetch models to check if the server is available
        self.get_models().await.is_ok()
    }
    
    async fn get_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/v1/models", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(ProviderError::Unavailable(
                format!("Local server returned status: {}", response.status())
            ));
        }
        
        let models_response: LocalModelsResponse = response.json().await?;
        Ok(models_response.data.into_iter().map(|m| m.id).collect())
    }
    
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, ProviderError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let local_request = self.convert_request(&request);
        
        let response = self.client
            .post(&url)
            .json(&local_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::Generic(error_text));
        }
        
        let local_response: LocalResponse = response.json().await?;
        self.convert_response(local_response)
    }
    
    async fn complete_stream(&self, request: LLMRequest) -> Result<Box<dyn Stream<Item = Result<LLMResponse, ProviderError>> + Send + Unpin>, ProviderError> {
        // For simplicity, we'll implement streaming later
        // For now, just return a single response wrapped in a stream
        let response = self.complete(request).await?;
        let stream = futures::stream::once(async move { Ok(response) });
        Ok(Box::new(Box::pin(stream)))
    }
    
    fn default_model(&self) -> String {
        // Common default for local models
        "local-model".to_string()
    }
    
    fn get_config(&self) -> HashMap<String, serde_json::Value> {
        self.config.clone()
    }
    
    fn update_config(&mut self, config: HashMap<String, serde_json::Value>) -> Result<(), ProviderError> {
        self.config = config;
        Ok(())
    }
    
    async fn validate_config(&self) -> Result<(), ProviderError> {
        // Try to fetch models to validate the server is accessible
        self.get_models().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{Message, MessageRole, LLMRequest};
    
    #[test]
    fn test_local_provider_creation() {
        let provider = LocalProvider::new("http://localhost:8080".to_string()).unwrap();
        assert_eq!(provider.provider_type(), ProviderType::Local);
        assert_eq!(provider.name(), "Local");
        assert_eq!(provider.default_model(), "local-model");
    }
    
    #[test]
    fn test_request_conversion() {
        let provider = LocalProvider::new("http://localhost:8080".to_string()).unwrap();
        
        let request = LLMRequest {
            messages: vec![
                Message::system("You are a helpful assistant"),
                Message::user("Hello"),
            ],
            model: Some("test-model".to_string()),
            temperature: Some(0.8),
            max_tokens: Some(100),
            ..Default::default()
        };
        
        let converted = provider.convert_request(&request);
        
        assert_eq!(converted["model"], "test-model");
        assert_eq!(converted["temperature"], 0.8);
        assert_eq!(converted["max_tokens"], 100);
        assert_eq!(converted["messages"].as_array().unwrap().len(), 2);
    }
    
    #[test]
    fn test_config_operations() {
        let mut provider = LocalProvider::new("http://localhost:8080".to_string()).unwrap();
        
        let mut config = HashMap::new();
        config.insert("timeout".to_string(), serde_json::json!(30));
        
        provider.update_config(config.clone()).unwrap();
        assert_eq!(provider.get_config(), config);
    }
}