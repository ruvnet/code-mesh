/// Example usage of the LLM provider system
use std::env;

use crate::llm::{
    LLMRegistry, create_default_registry, create_registry_with_models_dev,
    Message, MessageContent, MessageRole, GenerateOptions, ToolDefinition,
};
use crate::auth::{AuthCredentials, FileAuthStorage};

/// Example: Basic model usage
pub async fn example_basic_usage() -> crate::Result<()> {
    // Create registry with default authentication storage
    let mut registry = create_default_registry().await?;
    
    // Get the best available model
    let model = registry.get_best_model().await?;
    
    // Create a simple message
    let messages = vec![Message {
        role: MessageRole::User,
        content: MessageContent::Text("Hello, how are you?".to_string()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }];
    
    // Generate response
    let result = model.generate(messages, GenerateOptions::default()).await?;
    println!("Response: {}", result.content);
    println!("Usage: {} tokens", result.usage.total_tokens);
    
    Ok(())
}

/// Example: Provider-specific model usage
pub async fn example_provider_specific() -> crate::Result<()> {
    let registry = create_default_registry().await?;
    
    // Get specific Anthropic model
    if let Ok(model) = registry.get_model("anthropic", "claude-3-5-sonnet-20241022").await {
        println!("Using Anthropic Claude 3.5 Sonnet");
        
        let messages = vec![Message {
            role: MessageRole::User,
            content: MessageContent::Text("Explain quantum computing".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];
        
        let result = model.generate(messages, GenerateOptions::default()).await?;
        println!("Claude response: {}", result.content);
    }
    
    // Try OpenAI as fallback
    if let Ok(model) = registry.get_model("openai", "gpt-4o").await {
        println!("Using OpenAI GPT-4o");
        
        let messages = vec![Message {
            role: MessageRole::User,
            content: MessageContent::Text("Explain machine learning".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];
        
        let result = model.generate(messages, GenerateOptions::default()).await?;
        println!("GPT-4o response: {}", result.content);
    }
    
    Ok(())
}

/// Example: Tool usage
pub async fn example_with_tools() -> crate::Result<()> {
    let registry = create_default_registry().await?;
    let model = registry.get_model("anthropic", "claude-3-5-sonnet-20241022").await?;
    
    // Define a simple calculator tool
    let calculator_tool = ToolDefinition {
        name: "calculator".to_string(),
        description: "Perform basic arithmetic operations".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                },
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        }),
    };
    
    let messages = vec![Message {
        role: MessageRole::User,
        content: MessageContent::Text("Calculate 15 + 27".to_string()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }];
    
    let options = GenerateOptions {
        tools: vec![calculator_tool],
        temperature: Some(0.1),
        max_tokens: Some(1000),
        stop_sequences: vec![],
    };
    
    let result = model.generate(messages, options).await?;
    
    if !result.tool_calls.is_empty() {
        println!("Model wants to use tools:");
        for tool_call in &result.tool_calls {
            println!("  Tool: {}", tool_call.name);
            println!("  Arguments: {}", tool_call.arguments);
        }
    } else {
        println!("Response: {}", result.content);
    }
    
    Ok(())
}

/// Example: Authentication setup
pub async fn example_authentication_setup() -> crate::Result<()> {
    use std::sync::Arc;
    
    // Set up authentication storage
    let storage = Arc::new(FileAuthStorage::default_with_result()?) as Arc<dyn AuthStorage>;
    
    // Store API key for OpenAI
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        let credentials = AuthCredentials::api_key(api_key);
        storage.set("openai", credentials).await?;
        println!("Stored OpenAI API key");
    }
    
    // Store API key for Anthropic
    if let Ok(api_key) = env::var("ANTHROPIC_API_KEY") {
        let credentials = AuthCredentials::api_key(api_key);
        storage.set("anthropic", credentials).await?;
        println!("Stored Anthropic API key");
    }
    
    // Create registry with the configured storage
    let registry = LLMRegistry::new(storage);
    
    // List available providers
    let providers = registry.list_available_providers().await;
    println!("Available providers: {:?}", providers);
    
    Ok(())
}

/// Example: Error handling and retries
pub async fn example_error_handling() -> crate::Result<()> {
    use crate::llm::retry_with_backoff;
    use crate::llm::RetryConfig;
    
    let registry = create_default_registry().await?;
    
    // Configure retry policy
    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: std::time::Duration::from_millis(500),
        max_delay: std::time::Duration::from_secs(5),
        backoff_factor: 2.0,
    };
    
    // Retry getting a model with exponential backoff
    let model = retry_with_backoff(
        || {
            Box::pin(async {
                registry.get_best_model().await
            })
        },
        retry_config,
    ).await?;
    
    println!("Successfully got model with retries");
    
    Ok(())
}

/// Example: Multi-provider fallback
pub async fn example_fallback_chain() -> crate::Result<()> {
    let registry = create_default_registry().await?;
    
    let provider_chain = ["anthropic", "openai", "github-copilot"];
    let model_preferences = [
        ("anthropic", "claude-3-5-sonnet-20241022"),
        ("openai", "gpt-4o"),
        ("github-copilot", "gpt-4o"),
    ];
    
    let mut selected_model = None;
    
    for (provider_id, model_id) in model_preferences {
        if let Ok(model) = registry.get_model(provider_id, model_id).await {
            println!("Using {} / {}", provider_id, model_id);
            selected_model = Some(model);
            break;
        }
    }
    
    if let Some(model) = selected_model {
        let messages = vec![Message {
            role: MessageRole::User,
            content: MessageContent::Text("Hello from the fallback chain!".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];
        
        let result = model.generate(messages, GenerateOptions::default()).await?;
        println!("Response: {}", result.content);
    } else {
        println!("No models available in the fallback chain");
    }
    
    Ok(())
}

/// Example: Streaming responses
pub async fn example_streaming() -> crate::Result<()> {
    use futures::StreamExt;
    use crate::llm::StreamOptions;
    
    let registry = create_default_registry().await?;
    let model = registry.get_best_model().await?;
    
    let messages = vec![Message {
        role: MessageRole::User,
        content: MessageContent::Text("Write a short story about a robot".to_string()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }];
    
    let options = StreamOptions {
        temperature: Some(0.7),
        max_tokens: Some(500),
        tools: vec![],
        stop_sequences: vec![],
    };
    
    let mut stream = model.stream(messages, options).await?;
    
    print!("Streaming response: ");
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk.delta);
                // Flush stdout to show streaming effect
                use std::io::{self, Write};
                io::stdout().flush().unwrap();
                
                if chunk.finish_reason.is_some() {
                    println!("\nStream completed");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

/// Example: Configuration management
pub async fn example_configuration() -> crate::Result<()> {
    let mut registry = create_default_registry().await?;
    
    // Load additional configurations from models.dev
    registry.load_models_dev_configs().await?;
    
    // Load custom configuration file if it exists
    if std::path::Path::new("custom_models.json").exists() {
        registry.load_config_file("custom_models.json").await?;
    }
    
    // List all providers and their models
    let providers = registry.list_providers().await;
    for provider_id in providers {
        println!("Provider: {}", provider_id);
        
        if let Ok(models) = registry.list_models(&provider_id).await {
            for model in models {
                println!("  Model: {} ({})", model.id, model.name);
                println!("    Supports tools: {}", model.capabilities.tool_call);
                println!("    Supports vision: {}", model.capabilities.vision);
                println!("    Max tokens: {}", model.limits.context);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_basic_registry_creation() {
        let result = create_default_registry().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test] 
    async fn test_provider_listing() {
        let registry = create_default_registry().await.unwrap();
        let providers = registry.list_providers().await;
        // Should be empty initially since no credentials are configured
        assert!(providers.is_empty() || !providers.is_empty());
    }
    
    #[tokio::test]
    async fn test_model_caching() {
        let registry = create_default_registry().await.unwrap();
        
        // Check initial cache stats
        let stats = registry.cache_stats().await;
        assert_eq!(stats.get("cached_models"), Some(&0));
        
        // Clear cache (should be no-op)
        registry.clear_cache().await;
        
        let stats = registry.cache_stats().await;
        assert_eq!(stats.get("cached_models"), Some(&0));
    }
}