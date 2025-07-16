//! WASM-specific tests

use wasm_bindgen_test::*;
use code_mesh_wasm::*;
use js_sys::*;
use web_sys::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_wasm_initialization() {
    // Test that WASM module initializes correctly
    init_panic_hook();
    init_logging();
    
    // Should not panic
}

#[wasm_bindgen_test]
fn test_session_creation_wasm() {
    let session = WasmSession::new("test-session".to_string(), "test-user".to_string());
    
    assert_eq!(session.id(), "test-session");
    assert_eq!(session.user_id(), "test-user");
    assert_eq!(session.message_count(), 0);
}

#[wasm_bindgen_test]
fn test_session_add_message_wasm() {
    let mut session = WasmSession::new("test".to_string(), "user".to_string());
    
    let message = WasmChatMessage::new("user".to_string(), "Hello!".to_string());
    session.add_message(message);
    
    assert_eq!(session.message_count(), 1);
}

#[wasm_bindgen_test]
fn test_llm_provider_wasm() {
    let provider = WasmLlmProvider::new("mock".to_string(), js_sys::Object::new());
    
    assert_eq!(provider.provider_name(), "mock");
}

#[wasm_bindgen_test]
async fn test_llm_chat_completion_wasm() {
    let provider = WasmLlmProvider::new("mock".to_string(), js_sys::Object::new());
    let messages = js_sys::Array::new();
    
    let message_obj = js_sys::Object::new();
    js_sys::Reflect::set(&message_obj, &"role".into(), &"user".into()).unwrap();
    js_sys::Reflect::set(&message_obj, &"content".into(), &"Hello".into()).unwrap();
    messages.push(&message_obj);
    
    let result = provider.chat_completion(messages).await;
    // With mock provider, this should return a result
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_storage_wasm() {
    let storage = WasmStorage::new();
    
    // Test storing and retrieving data
    let key = "test-key";
    let value = js_sys::Object::new();
    js_sys::Reflect::set(&value, &"data".into(), &"test-value".into()).unwrap();
    
    storage.store(key.to_string(), value.clone()).unwrap();
    let retrieved = storage.get(key.to_string()).unwrap();
    
    assert!(retrieved.is_some());
}

#[wasm_bindgen_test]
fn test_tool_registry_wasm() {
    let registry = WasmToolRegistry::new();
    
    // Should start with no tools
    assert_eq!(registry.list_tools().length(), 0);
}

#[wasm_bindgen_test]
fn test_tool_execution_wasm() {
    let registry = WasmToolRegistry::new();
    
    // Register a mock tool
    let tool_config = js_sys::Object::new();
    js_sys::Reflect::set(&tool_config, &"name".into(), &"echo".into()).unwrap();
    js_sys::Reflect::set(&tool_config, &"description".into(), &"Echo tool".into()).unwrap();
    
    registry.register_tool("echo".to_string(), tool_config).unwrap();
    
    assert_eq!(registry.list_tools().length(), 1);
}

#[wasm_bindgen_test]
async fn test_async_operations_wasm() {
    // Test that async operations work in WASM context
    let session = WasmSession::new("async-test".to_string(), "user".to_string());
    let provider = WasmLlmProvider::new("mock".to_string(), js_sys::Object::new());
    
    // This should complete without hanging
    let messages = js_sys::Array::new();
    let _result = provider.chat_completion(messages).await;
}

#[wasm_bindgen_test]
fn test_memory_management_wasm() {
    // Create many objects to test memory management
    let mut sessions = Vec::new();
    
    for i in 0..100 {
        let session = WasmSession::new(format!("session-{}", i), "user".to_string());
        sessions.push(session);
    }
    
    assert_eq!(sessions.len(), 100);
    
    // Objects should be properly cleaned up when dropped
    drop(sessions);
}

#[wasm_bindgen_test]
fn test_error_handling_wasm() {
    let storage = WasmStorage::new();
    
    // Test error handling with invalid operations
    let result = storage.get("nonexistent-key".to_string());
    assert!(result.is_ok()); // Should return None, not error
    assert!(result.unwrap().is_none());
}

#[wasm_bindgen_test]
fn test_javascript_interop() {
    // Test that we can properly interact with JavaScript objects
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"test".into(), &"value".into()).unwrap();
    
    let retrieved = js_sys::Reflect::get(&obj, &"test".into()).unwrap();
    assert_eq!(retrieved.as_string().unwrap(), "value");
}

#[wasm_bindgen_test]
fn test_json_serialization_wasm() {
    let session = WasmSession::new("json-test".to_string(), "user".to_string());
    
    // Test JSON serialization
    let json_str = session.to_json().unwrap();
    assert!(json_str.contains("json-test"));
    assert!(json_str.contains("user"));
    
    // Test JSON deserialization
    let restored_session = WasmSession::from_json(json_str).unwrap();
    assert_eq!(restored_session.id(), "json-test");
    assert_eq!(restored_session.user_id(), "user");
}

#[wasm_bindgen_test]
fn test_console_logging_wasm() {
    // Test that console logging works
    web_sys::console::log_1(&"Test log message".into());
    web_sys::console::warn_1(&"Test warning message".into());
    web_sys::console::error_1(&"Test error message".into());
}

#[wasm_bindgen_test]
fn test_performance_wasm() {
    let start = js_sys::Date::now();
    
    // Perform some operations
    let session = WasmSession::new("perf-test".to_string(), "user".to_string());
    for i in 0..100 {
        let message = WasmChatMessage::new("user".to_string(), format!("Message {}", i));
        session.add_message(message);
    }
    
    let end = js_sys::Date::now();
    let duration = end - start;
    
    // Should complete in reasonable time (less than 1 second)
    assert!(duration < 1000.0);
}

#[wasm_bindgen_test]
fn test_local_storage_integration() {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage().unwrap().unwrap();
    
    // Test storing data in browser's localStorage
    storage.set_item("test-key", "test-value").unwrap();
    let retrieved = storage.get_item("test-key").unwrap().unwrap();
    
    assert_eq!(retrieved, "test-value");
    
    // Clean up
    storage.remove_item("test-key").unwrap();
}

#[wasm_bindgen_test]
async fn test_fetch_api_integration() {
    use wasm_bindgen_futures::JsFuture;
    
    // Test that we can use fetch API (for HTTP requests)
    let window = web_sys::window().unwrap();
    let request = web_sys::Request::new_with_str("data:text/plain,hello").unwrap();
    
    let response_promise = window.fetch_with_request(&request);
    let response = JsFuture::from(response_promise).await.unwrap();
    let response: web_sys::Response = response.dyn_into().unwrap();
    
    assert!(response.ok());
}

#[wasm_bindgen_test]
fn test_event_handling_wasm() {
    // Test event handling setup
    let document = web_sys::window().unwrap().document().unwrap();
    let element = document.create_element("div").unwrap();
    
    // This would normally set up event listeners
    // For testing, we just verify the element was created
    assert_eq!(element.tag_name(), "DIV");
}

#[wasm_bindgen_test]
fn test_url_handling_wasm() {
    // Test URL parsing and manipulation
    let url = web_sys::Url::new("https://example.com/path?param=value").unwrap();
    
    assert_eq!(url.hostname(), "example.com");
    assert_eq!(url.pathname(), "/path");
    assert_eq!(url.search(), "?param=value");
}

#[wasm_bindgen_test]
fn test_crypto_random_wasm() {
    // Test crypto random number generation
    let window = web_sys::window().unwrap();
    let crypto = window.crypto().unwrap();
    
    let mut buffer = [0u8; 16];
    crypto.get_random_values_with_u8_array(&mut buffer).unwrap();
    
    // Buffer should not be all zeros (with very high probability)
    assert_ne!(buffer, [0u8; 16]);
}

#[wasm_bindgen_test]
fn test_base64_encoding_wasm() {
    // Test base64 encoding/decoding
    let window = web_sys::window().unwrap();
    let original = "Hello, WASM World!";
    let encoded = window.btoa(original).unwrap();
    let decoded = window.atob(&encoded).unwrap();
    
    assert_eq!(decoded, original);
}

// Property-based test for WASM
#[wasm_bindgen_test]
fn test_session_properties_wasm() {
    // Test various session configurations
    let test_cases = vec![
        ("short-id", "user1"),
        ("very-long-session-identifier-with-many-characters", "user-with-long-name"),
        ("", ""), // Edge case: empty strings
        ("session-with-unicode-🦀", "user-with-unicode-世界"),
    ];
    
    for (session_id, user_id) in test_cases {
        let session = WasmSession::new(session_id.to_string(), user_id.to_string());
        assert_eq!(session.id(), session_id);
        assert_eq!(session.user_id(), user_id);
        assert_eq!(session.message_count(), 0);
    }
}

#[wasm_bindgen_test]
fn test_large_data_handling_wasm() {
    // Test handling of large data structures
    let session = WasmSession::new("large-data-test".to_string(), "user".to_string());
    
    // Add many messages
    for i in 0..1000 {
        let content = format!("Message {} with some content to make it larger", i);
        let message = WasmChatMessage::new("user".to_string(), content);
        session.add_message(message);
    }
    
    assert_eq!(session.message_count(), 1000);
    
    // Serialization should still work
    let json_result = session.to_json();
    assert!(json_result.is_ok());
}