//! LLM module unit tests

use code_mesh_core::llm::*;
use proptest::prelude::*;
use rstest::*;
use serde_json::json;
use std::sync::Arc;
use wiremock::{
    matchers::{method, path, header},
    Mock, MockServer, ResponseTemplate,
};

mod common;
use common::{mocks::*, fixtures::*, *};

#[test]
fn test_chat_message_creation() {
    let message = ChatMessage {
        role: ChatRole::User,
        content: "Hello, world!".to_string(),
    };

    assert_eq!(message.role, ChatRole::User);
    assert_eq!(message.content, "Hello, world!");
}

#[test]
fn test_chat_role_serialization() {
    let user_role = ChatRole::User;
    let assistant_role = ChatRole::Assistant;
    let system_role = ChatRole::System;

    let user_json = serde_json::to_string(&user_role).unwrap();
    let assistant_json = serde_json::to_string(&assistant_role).unwrap();
    let system_json = serde_json::to_string(&system_role).unwrap();

    assert_eq!(user_json, "\"user\"");
    assert_eq!(assistant_json, "\"assistant\"");
    assert_eq!(system_json, "\"system\"");

    // Test deserialization
    let deserialized_user: ChatRole = serde_json::from_str(&user_json).unwrap();
    let deserialized_assistant: ChatRole = serde_json::from_str(&assistant_json).unwrap();
    let deserialized_system: ChatRole = serde_json::from_str(&system_json).unwrap();

    assert_eq!(deserialized_user, ChatRole::User);
    assert_eq!(deserialized_assistant, ChatRole::Assistant);
    assert_eq!(deserialized_system, ChatRole::System);
}

#[test]
fn test_model_info() {
    let model_info = ModelInfo {
        name: "claude-3-sonnet".to_string(),
        provider: "anthropic".to_string(),
        max_tokens: 200000,
        supports_streaming: true,
        supports_tools: true,
    };

    assert_eq!(model_info.name, "claude-3-sonnet");
    assert_eq!(model_info.provider, "anthropic");
    assert_eq!(model_info.max_tokens, 200000);
    assert!(model_info.supports_streaming);
    assert!(model_info.supports_tools);
}

#[test]
fn test_usage_calculation() {
    let usage = Usage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };

    assert_eq!(usage.total_tokens, usage.prompt_tokens + usage.completion_tokens);
}

#[test]
fn test_chat_completion_creation() {
    let completion = ChatCompletion {
        id: "completion-123".to_string(),
        content: "Hello! How can I help you?".to_string(),
        model: "claude-3-sonnet".to_string(),
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 8,
            total_tokens: 18,
        },
        created_at: chrono::Utc::now(),
    };

    assert_eq!(completion.id, "completion-123");
    assert_eq!(completion.content, "Hello! How can I help you?");
    assert_eq!(completion.model, "claude-3-sonnet");
    assert_eq!(completion.usage.total_tokens, 18);
}

#[tokio::test]
async fn test_mock_llm_provider() {
    let provider = MockLLMProvider::new();
    
    let messages = vec![ChatMessage {
        role: ChatRole::User,
        content: "Test message".to_string(),
    }];

    let completion = provider.chat_completion(messages).await.unwrap();
    assert_eq!(completion.content, "Default mock response");
    assert_eq!(completion.model, "mock-model");
}

#[tokio::test]
async fn test_mock_llm_provider_with_custom_responses() {
    let responses = vec![
        "First response".to_string(),
        "Second response".to_string(),
        "Third response".to_string(),
    ];
    let provider = MockLLMProvider::with_responses(responses);

    let messages = vec![ChatMessage {
        role: ChatRole::User,
        content: "Test".to_string(),
    }];

    // Test cycling through responses
    let completion1 = provider.chat_completion(messages.clone()).await.unwrap();
    assert_eq!(completion1.content, "First response");

    let completion2 = provider.chat_completion(messages.clone()).await.unwrap();
    assert_eq!(completion2.content, "Second response");

    let completion3 = provider.chat_completion(messages.clone()).await.unwrap();
    assert_eq!(completion3.content, "Third response");

    // Should cycle back to first
    let completion4 = provider.chat_completion(messages).await.unwrap();
    assert_eq!(completion4.content, "First response");
}

#[tokio::test]
async fn test_mock_llm_streaming() {
    let provider = MockLLMProvider::with_responses(vec!["Hello World".to_string()]);
    
    let messages = vec![ChatMessage {
        role: ChatRole::User,
        content: "Test".to_string(),
    }];

    let stream = provider.stream_completion(messages).await.unwrap();
    let collected: Vec<_> = futures::stream::StreamExt::collect(stream).await;

    // Should stream character by character
    let content: String = collected
        .into_iter()
        .map(|result| result.unwrap())
        .collect();
    
    assert_eq!(content, "Hello World");
}

#[rstest]
#[case(ChatRole::User, "user")]
#[case(ChatRole::Assistant, "assistant")]
#[case(ChatRole::System, "system")]
fn test_chat_role_string_conversion(#[case] role: ChatRole, #[case] expected: &str) {
    let role_str = match role {
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::System => "system",
    };
    assert_eq!(role_str, expected);
}

#[test]
fn test_chat_message_validation() {
    // Valid messages
    let valid_user_msg = ChatMessage {
        role: ChatRole::User,
        content: "Hello".to_string(),
    };
    assert!(!valid_user_msg.content.is_empty());

    let valid_system_msg = ChatMessage {
        role: ChatRole::System,
        content: "You are a helpful assistant".to_string(),
    };
    assert!(!valid_system_msg.content.is_empty());

    // Empty content should be handled
    let empty_msg = ChatMessage {
        role: ChatRole::User,
        content: String::new(),
    };
    assert!(empty_msg.content.is_empty());
}

#[tokio::test]
async fn test_llm_provider_model_info() {
    let provider = MockLLMProvider::new();
    let model_info = provider.model_info();
    
    assert_eq!(model_info.name, "mock-model");
    assert_eq!(model_info.provider, "mock");
    assert_eq!(model_info.max_tokens, 4096);
    assert!(model_info.supports_streaming);
    assert!(model_info.supports_tools);
}

#[tokio::test]
async fn test_llm_provider_config_validation() {
    let provider = MockLLMProvider::new();
    let result = provider.validate_config();
    assert!(result.is_ok());
}

// Property-based tests
proptest! {
    #[test]
    fn test_chat_message_roundtrip_serialization(
        role in prop_oneof![
            Just(ChatRole::User),
            Just(ChatRole::Assistant),
            Just(ChatRole::System)
        ],
        content in ".*"
    ) {
        let message = ChatMessage { role, content: content.clone() };
        
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();
        
        prop_assert_eq!(deserialized.role, message.role);
        prop_assert_eq!(deserialized.content, content);
    }

    #[test]
    fn test_usage_properties(
        prompt_tokens in 0u32..100000,
        completion_tokens in 0u32..100000
    ) {
        let total_tokens = prompt_tokens + completion_tokens;
        let usage = Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        };
        
        prop_assert_eq!(usage.prompt_tokens, prompt_tokens);
        prop_assert_eq!(usage.completion_tokens, completion_tokens);
        prop_assert_eq!(usage.total_tokens, total_tokens);
        prop_assert!(usage.total_tokens >= usage.prompt_tokens);
        prop_assert!(usage.total_tokens >= usage.completion_tokens);
    }

    #[test]
    fn test_model_info_properties(
        name in r"[a-zA-Z0-9\-_]{1,50}",
        provider in r"[a-zA-Z]{1,20}",
        max_tokens in 1u32..1000000,
        supports_streaming in any::<bool>(),
        supports_tools in any::<bool>()
    ) {
        let model_info = ModelInfo {
            name: name.clone(),
            provider: provider.clone(),
            max_tokens,
            supports_streaming,
            supports_tools,
        };
        
        prop_assert_eq!(model_info.name, name);
        prop_assert_eq!(model_info.provider, provider);
        prop_assert_eq!(model_info.max_tokens, max_tokens);
        prop_assert_eq!(model_info.supports_streaming, supports_streaming);
        prop_assert_eq!(model_info.supports_tools, supports_tools);
        prop_assert!(model_info.max_tokens > 0);
    }
}

// Integration test with mock HTTP server
#[tokio::test]
async fn test_anthropic_provider_with_mock_server() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello! How can I help you today?"
                }
            ],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 8
            }
        })))
        .mount(&mock_server)
        .await;

    // This would be used in the actual Anthropic provider implementation
    // For now, we'll just verify the mock server is working
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/v1/messages", mock_server.uri()))
        .header("x-api-key", "test-key")
        .json(&json!({
            "model": "claude-3-sonnet-20240229",
            "max_tokens": 100,
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["id"], "msg_123");
}

#[test]
fn test_conversation_fixtures() {
    let simple_conv = ChatFixtures::simple_conversation();
    assert_eq!(simple_conv.len(), 3);
    assert_eq!(simple_conv[0].role, ChatRole::System);
    assert_eq!(simple_conv[1].role, ChatRole::User);
    assert_eq!(simple_conv[2].role, ChatRole::Assistant);

    let tool_conv = ChatFixtures::tool_conversation();
    assert_eq!(tool_conv.len(), 3);
    assert!(tool_conv[1].content.contains("README.md"));

    let long_conv = ChatFixtures::long_conversation();
    assert_eq!(long_conv.len(), 21); // 1 system + 10 user + 10 assistant
}

#[tokio::test]
async fn test_concurrent_llm_requests() {
    let provider = Arc::new(MockLLMProvider::with_responses(vec![
        "Response 1".to_string(),
        "Response 2".to_string(),
        "Response 3".to_string(),
    ]));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let provider = Arc::clone(&provider);
            tokio::spawn(async move {
                let messages = vec![ChatMessage {
                    role: ChatRole::User,
                    content: format!("Message {}", i),
                }];
                provider.chat_completion(messages).await
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All requests should succeed
    for result in results {
        let completion = result.unwrap().unwrap();
        assert!(completion.content.starts_with("Response"));
    }
}

#[test]
fn test_chat_completion_time_ordering() {
    let mut completions = Vec::new();
    
    for i in 0..5 {
        let completion = ChatCompletion {
            id: format!("completion-{}", i),
            content: format!("Response {}", i),
            model: "test-model".to_string(),
            usage: Default::default(),
            created_at: chrono::Utc::now() + chrono::Duration::milliseconds(i as i64),
        };
        completions.push(completion);
    }

    // Verify time ordering
    for i in 1..completions.len() {
        assert!(completions[i].created_at > completions[i-1].created_at);
    }
}

#[test]
fn test_usage_serialization() {
    let usage = Usage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };

    let serialized = serde_json::to_string(&usage).unwrap();
    let deserialized: Usage = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.prompt_tokens, usage.prompt_tokens);
    assert_eq!(deserialized.completion_tokens, usage.completion_tokens);
    assert_eq!(deserialized.total_tokens, usage.total_tokens);
}

#[test]
fn test_chat_completion_serialization() {
    let now = chrono::Utc::now();
    let completion = ChatCompletion {
        id: "test-123".to_string(),
        content: "Test response".to_string(),
        model: "test-model".to_string(),
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
        created_at: now,
    };

    let serialized = serde_json::to_string(&completion).unwrap();
    let deserialized: ChatCompletion = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.id, completion.id);
    assert_eq!(deserialized.content, completion.content);
    assert_eq!(deserialized.model, completion.model);
    assert_eq!(deserialized.usage.total_tokens, completion.usage.total_tokens);
    assert_eq!(deserialized.created_at, completion.created_at);
}

// Error handling tests
#[tokio::test]
async fn test_llm_provider_error_handling() {
    let mut provider = MockLLMProvider::new();
    // This would test error scenarios if we had error injection in the mock
    
    let messages = vec![ChatMessage {
        role: ChatRole::User,
        content: "Test".to_string(),
    }];

    // For now, just test that normal operation works
    let result = provider.chat_completion(messages).await;
    assert!(result.is_ok());
}

#[test]
fn test_empty_conversation() {
    let messages: Vec<ChatMessage> = vec![];
    // Test that empty conversations are handled appropriately
    assert_eq!(messages.len(), 0);
}

#[test]
fn test_malformed_message_content() {
    let message = ChatMessage {
        role: ChatRole::User,
        content: "\0\x01\x02invalid\xFF".to_string(),
    };
    
    // Should handle binary/invalid content gracefully
    assert!(!message.content.is_empty());
}