//! Test the Anthropic provider implementation

use std::collections::HashMap;
use tokio_test;

use crate::{
    auth::{Auth, AuthCredentials},
    llm::{
        anthropic::AnthropicProvider,
        Message, MessageRole, MessageContent, GenerateOptions, StreamOptions,
        Provider, LanguageModel,
    },
};

/// Mock authentication for testing
struct MockAuth {
    credentials: AuthCredentials,
}

#[async_trait::async_trait]
impl Auth for MockAuth {
    fn provider_id(&self) -> &str {
        "anthropic"
    }
    
    async fn get_credentials(&self) -> crate::Result<AuthCredentials> {
        Ok(self.credentials.clone())
    }
    
    async fn set_credentials(&self, credentials: AuthCredentials) -> crate::Result<()> {
        // In real implementation, would store credentials
        Ok(())
    }
    
    async fn remove_credentials(&self) -> crate::Result<()> {
        Ok(())
    }
    
    async fn has_credentials(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_anthropic_provider_creation() {
    let auth = Box::new(MockAuth {
        credentials: AuthCredentials::ApiKey {
            key: "sk-ant-test123".to_string(),
        }
    });
    
    let provider = AnthropicProvider::new(auth);
    assert_eq!(provider.id(), "anthropic");
    assert_eq!(provider.name(), "Anthropic");
    assert_eq!(provider.api_endpoint(), Some("https://api.anthropic.com/v1"));
}

#[tokio::test]
async fn test_anthropic_models() {
    let auth = Box::new(MockAuth {
        credentials: AuthCredentials::ApiKey {
            key: "sk-ant-test123".to_string(),
        }
    });
    
    let provider = AnthropicProvider::new(auth);
    let models = provider.models();
    
    assert!(models.contains_key("claude-3-5-sonnet-20241022"));
    assert!(models.contains_key("claude-3-5-haiku-20241022"));
    assert!(models.contains_key("claude-3-opus-20240229"));
    
    let sonnet = &models["claude-3-5-sonnet-20241022"];
    assert_eq!(sonnet.name(), "Claude 3.5 Sonnet");
    assert!(sonnet.supports_tool_calls());
    assert!(sonnet.supports_vision());
    assert!(sonnet.supports_caching());
}

#[tokio::test]
async fn test_message_conversion() {
    let messages = vec![
        Message {
            role: MessageRole::System,
            content: MessageContent::Text("You are a helpful assistant.".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        Message {
            role: MessageRole::User,
            content: MessageContent::Text("Hello!".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ];
    
    // Test that system messages are extracted properly
    let (system_prompt, anthropic_messages) = crate::llm::anthropic::convert_messages_with_system(messages).unwrap();
    
    assert_eq!(system_prompt, Some("You are a helpful assistant.".to_string()));
    assert_eq!(anthropic_messages.len(), 1);
    assert_eq!(anthropic_messages[0]["role"], "user");
    assert_eq!(anthropic_messages[0]["content"], "Hello!");
}

#[tokio::test]
async fn test_rate_limiter() {
    use std::time::Instant;
    use crate::llm::anthropic::RateLimiter;
    
    let limiter = RateLimiter::new();
    
    let start = Instant::now();
    limiter.acquire().await; // First request should be immediate
    let first_duration = start.elapsed();
    
    limiter.acquire().await; // Second request should be delayed
    let second_duration = start.elapsed();
    
    // Second request should take at least the minimum interval
    assert!(second_duration >= first_duration + limiter.min_interval);
}

#[tokio::test]
async fn test_credential_validation() {
    let auth = Box::new(MockAuth {
        credentials: AuthCredentials::ApiKey {
            key: "sk-ant-test123".to_string(),
        }
    });
    
    let provider = AnthropicProvider::new(auth);
    let result = provider.validate_and_refresh_credentials().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "sk-ant-test123");
}

#[tokio::test]
async fn test_invalid_api_key() {
    let auth = Box::new(MockAuth {
        credentials: AuthCredentials::ApiKey {
            key: "invalid-key".to_string(),
        }
    });
    
    let provider = AnthropicProvider::new(auth);
    let result = provider.validate_and_refresh_credentials().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_streaming_setup() {
    let auth = Box::new(MockAuth {
        credentials: AuthCredentials::ApiKey {
            key: "sk-ant-test123".to_string(),
        }
    });
    
    let provider = AnthropicProvider::new(auth);
    let model = provider.get_model("claude-3-5-sonnet-20241022").await.unwrap();
    
    // Test that streaming is supported
    assert!(model.supports_tools());
    
    // Note: We can't test actual streaming without a real API key
    // This test just verifies the setup is correct
}

/// Test the complete Anthropic provider functionality
#[tokio::test] 
async fn test_anthropic_provider_completeness() {
    let auth = Box::new(MockAuth {
        credentials: AuthCredentials::ApiKey {
            key: "sk-ant-test123".to_string(),
        }
    });
    
    let provider = AnthropicProvider::new(auth);
    
    // Test provider metadata
    assert_eq!(provider.id(), "anthropic");
    assert_eq!(provider.name(), "Anthropic");
    assert!(provider.api_endpoint().is_some());
    assert!(!provider.env_vars().is_empty());
    assert!(provider.npm_package().is_some());
    
    // Test models
    let models = provider.models();
    assert!(!models.is_empty());
    
    for (id, model) in models.iter() {
        assert_eq!(model.id(), id);
        assert!(!model.name().is_empty());
        assert!(!model.release_date().is_empty());
        assert!(model.context_limit() > 0);
        assert!(model.output_limit() > 0);
    }
    
    // Test getting a specific model
    let model_result = provider.get_model("claude-3-5-sonnet-20241022").await;
    assert!(model_result.is_ok());
    
    let model = model_result.unwrap();
    assert!(model.supports_tools());
    assert!(model.supports_vision());
}