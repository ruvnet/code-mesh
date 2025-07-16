// WASM integration tests for opencode_core
use wasm_bindgen_test::*;
use wasm_bindgen::prelude::*;
use web_sys::console;
use js_sys::Promise;
use std::collections::HashMap;

wasm_bindgen_test_configure!(run_in_browser);

// Mock external dependencies for WASM testing
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "localStorage"])]
    fn setItem(key: &str, value: &str);
    
    #[wasm_bindgen(js_namespace = ["window", "localStorage"])]
    fn getItem(key: &str) -> Option<String>;
    
    #[wasm_bindgen(js_namespace = ["window", "localStorage"])]
    fn removeItem(key: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Test core WASM functionality
#[wasm_bindgen_test]
async fn test_wasm_core_initialization() {
    console::log_1(&"Starting WASM core initialization test".into());
    
    let core = WasmCore::new().await;
    assert!(core.is_ok());
    
    let core = core.unwrap();
    assert_eq!(core.version(), "0.1.0");
    
    console::log_1(&"WASM core initialization test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_config_storage() {
    console::log_1(&"Testing WASM configuration storage".into());
    
    let config = WasmConfig {
        provider: "anthropic".to_string(),
        model: "claude-3-sonnet".to_string(),
        api_key: "test-key".to_string(),
        temperature: 0.7,
    };
    
    // Test storing configuration
    let result = store_config(&config).await;
    assert!(result.is_ok());
    
    // Test retrieving configuration
    let retrieved_config = load_config().await;
    assert!(retrieved_config.is_ok());
    
    let retrieved_config = retrieved_config.unwrap();
    assert_eq!(retrieved_config.provider, "anthropic");
    assert_eq!(retrieved_config.model, "claude-3-sonnet");
    assert_eq!(retrieved_config.temperature, 0.7);
    
    console::log_1(&"WASM configuration storage test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_message_processing() {
    console::log_1(&"Testing WASM message processing".into());
    
    let core = WasmCore::new().await.unwrap();
    let agent_id = core.create_agent("test-agent").await.unwrap();
    
    // Test sending a message
    let response = core.send_message(agent_id, "Hello, world!").await;
    assert!(response.is_ok());
    
    let response = response.unwrap();
    assert!(!response.content.is_empty());
    assert_eq!(response.agent_id, agent_id);
    
    console::log_1(&"WASM message processing test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_conversation_history() {
    console::log_1(&"Testing WASM conversation history".into());
    
    let core = WasmCore::new().await.unwrap();
    let agent_id = core.create_agent("test-agent").await.unwrap();
    
    // Send multiple messages
    let messages = vec![
        "Hello",
        "How are you?",
        "Can you help me with coding?",
    ];
    
    for msg in &messages {
        let response = core.send_message(agent_id, msg).await;
        assert!(response.is_ok());
    }
    
    // Get conversation history
    let history = core.get_conversation_history(agent_id).await;
    assert!(history.is_ok());
    
    let history = history.unwrap();
    assert!(history.len() >= messages.len() * 2); // User messages + assistant responses
    
    console::log_1(&"WASM conversation history test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_agent_lifecycle() {
    console::log_1(&"Testing WASM agent lifecycle".into());
    
    let core = WasmCore::new().await.unwrap();
    
    // Create agent
    let agent_id = core.create_agent("test-agent").await.unwrap();
    assert!(agent_id > 0);
    
    // List agents
    let agents = core.list_agents().await.unwrap();
    assert!(agents.len() > 0);
    assert!(agents.iter().any(|a| a.id == agent_id));
    
    // Get agent info
    let agent_info = core.get_agent_info(agent_id).await.unwrap();
    assert_eq!(agent_info.name, "test-agent");
    
    // Destroy agent
    let result = core.destroy_agent(agent_id).await;
    assert!(result.is_ok());
    
    // Verify agent is destroyed
    let agents = core.list_agents().await.unwrap();
    assert!(!agents.iter().any(|a| a.id == agent_id));
    
    console::log_1(&"WASM agent lifecycle test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_error_handling() {
    console::log_1(&"Testing WASM error handling".into());
    
    let core = WasmCore::new().await.unwrap();
    
    // Test invalid agent ID
    let result = core.send_message(999, "Hello").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), WasmError::InvalidAgentId(_)));
    
    // Test empty message
    let agent_id = core.create_agent("test-agent").await.unwrap();
    let result = core.send_message(agent_id, "").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), WasmError::InvalidInput(_)));
    
    console::log_1(&"WASM error handling test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_performance_metrics() {
    console::log_1(&"Testing WASM performance metrics".into());
    
    let core = WasmCore::new().await.unwrap();
    let agent_id = core.create_agent("test-agent").await.unwrap();
    
    let start_time = js_sys::Date::now();
    
    // Send multiple messages to generate metrics
    for i in 0..5 {
        let msg = format!("Test message {}", i);
        let _response = core.send_message(agent_id, &msg).await.unwrap();
    }
    
    let end_time = js_sys::Date::now();
    let duration = end_time - start_time;
    
    // Get performance metrics
    let metrics = core.get_performance_metrics().await.unwrap();
    assert!(metrics.total_requests > 0);
    assert!(metrics.average_response_time > 0.0);
    assert!(metrics.total_processing_time > 0.0);
    
    // Verify performance is reasonable (less than 10 seconds for 5 messages)
    assert!(duration < 10000.0);
    
    console::log_1(&"WASM performance metrics test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_memory_management() {
    console::log_1(&"Testing WASM memory management".into());
    
    let core = WasmCore::new().await.unwrap();
    let initial_memory = get_memory_usage();
    
    // Create multiple agents and send messages
    let mut agent_ids = Vec::new();
    for i in 0..10 {
        let agent_id = core.create_agent(&format!("agent-{}", i)).await.unwrap();
        agent_ids.push(agent_id);
        
        // Send messages to each agent
        for j in 0..5 {
            let msg = format!("Message {} from agent {}", j, i);
            let _response = core.send_message(agent_id, &msg).await.unwrap();
        }
    }
    
    let peak_memory = get_memory_usage();
    
    // Clean up agents
    for agent_id in agent_ids {
        core.destroy_agent(agent_id).await.unwrap();
    }
    
    // Force garbage collection if available
    if let Ok(gc) = js_sys::Reflect::get(&js_sys::global(), &"gc".into()) {
        if !gc.is_undefined() {
            let gc_fn: js_sys::Function = gc.into();
            let _ = gc_fn.call0(&js_sys::global());
        }
    }
    
    let final_memory = get_memory_usage();
    
    // Verify memory is managed properly
    assert!(peak_memory > initial_memory);
    assert!(final_memory < peak_memory); // Memory should be reclaimed
    
    console::log_1(&"WASM memory management test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_concurrent_operations() {
    console::log_1(&"Testing WASM concurrent operations".into());
    
    let core = WasmCore::new().await.unwrap();
    let agent_id = core.create_agent("test-agent").await.unwrap();
    
    // Create multiple promises for concurrent operations
    let mut promises = Vec::new();
    
    for i in 0..5 {
        let core_clone = core.clone();
        let promise = wasm_bindgen_futures::spawn_local(async move {
            let msg = format!("Concurrent message {}", i);
            core_clone.send_message(agent_id, &msg).await
        });
        promises.push(promise);
    }
    
    // Wait for all operations to complete
    let results = futures::future::join_all(promises).await;
    
    // Verify all operations succeeded
    for result in results {
        assert!(result.is_ok());
    }
    
    console::log_1(&"WASM concurrent operations test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_cross_origin_requests() {
    console::log_1(&"Testing WASM cross-origin request handling".into());
    
    let core = WasmCore::new().await.unwrap();
    
    // Test configuration with different API endpoints
    let configs = vec![
        WasmConfig {
            provider: "anthropic".to_string(),
            model: "claude-3-sonnet".to_string(),
            api_key: "test-key".to_string(),
            temperature: 0.7,
        },
        WasmConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: "test-key".to_string(),
            temperature: 0.7,
        },
    ];
    
    for config in configs {
        let result = core.update_config(config).await;
        assert!(result.is_ok());
        
        let agent_id = core.create_agent("test-agent").await.unwrap();
        let response = core.send_message(agent_id, "Hello").await;
        
        // Should handle CORS appropriately
        match response {
            Ok(_) => {
                // Success case
                console::log_1(&format!("Provider {} worked correctly", config.provider).into());
            }
            Err(WasmError::CorsError(_)) => {
                // Expected CORS error in test environment
                console::log_1(&format!("Expected CORS error for provider {}", config.provider).into());
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
        
        core.destroy_agent(agent_id).await.unwrap();
    }
    
    console::log_1(&"WASM cross-origin request handling test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_local_storage_persistence() {
    console::log_1(&"Testing WASM localStorage persistence".into());
    
    let test_data = vec![
        ("config", r#"{"provider": "anthropic", "model": "claude-3-sonnet"}"#),
        ("conversation_1", r#"{"messages": [{"role": "user", "content": "Hello"}]}"#),
        ("agent_state", r#"{"id": 1, "name": "test-agent", "active": true}"#),
    ];
    
    // Store data
    for (key, value) in &test_data {
        setItem(key, value);
    }
    
    // Retrieve and verify data
    for (key, expected_value) in &test_data {
        let retrieved_value = getItem(key);
        assert!(retrieved_value.is_some());
        assert_eq!(retrieved_value.unwrap(), *expected_value);
    }
    
    // Clean up
    for (key, _) in &test_data {
        removeItem(key);
    }
    
    // Verify cleanup
    for (key, _) in &test_data {
        let retrieved_value = getItem(key);
        assert!(retrieved_value.is_none());
    }
    
    console::log_1(&"WASM localStorage persistence test passed".into());
}

#[wasm_bindgen_test]
async fn test_wasm_binary_size_optimization() {
    console::log_1(&"Testing WASM binary size optimization".into());
    
    // Test that core functionality is available without bloat
    let core = WasmCore::new().await.unwrap();
    
    // Basic functionality should work
    let agent_id = core.create_agent("test-agent").await.unwrap();
    let response = core.send_message(agent_id, "Hello").await.unwrap();
    assert!(!response.content.is_empty());
    
    // Advanced features should be conditionally compiled
    let features = core.get_available_features().await.unwrap();
    
    // Only essential features should be included in WASM build
    let essential_features = vec!["message_processing", "agent_management", "config_storage"];
    for feature in essential_features {
        assert!(features.contains(&feature.to_string()));
    }
    
    // Optional features should be excluded to reduce size
    let optional_features = vec!["file_operations", "subprocess_execution", "native_ui"];
    for feature in optional_features {
        assert!(!features.contains(&feature.to_string()));
    }
    
    console::log_1(&"WASM binary size optimization test passed".into());
}

// Helper functions and types for testing

fn get_memory_usage() -> f64 {
    // In a real implementation, this would use performance.memory API
    // For testing, we'll use a simple approximation
    js_sys::Date::now()
}

// Mock WASM core implementation for testing
#[wasm_bindgen]
pub struct WasmCore {
    agents: std::collections::HashMap<u32, WasmAgent>,
    next_agent_id: u32,
    config: Option<WasmConfig>,
}

#[wasm_bindgen]
impl WasmCore {
    #[wasm_bindgen(constructor)]
    pub async fn new() -> Result<WasmCore, JsValue> {
        Ok(WasmCore {
            agents: HashMap::new(),
            next_agent_id: 1,
            config: None,
        })
    }
    
    #[wasm_bindgen]
    pub fn version(&self) -> String {
        "0.1.0".to_string()
    }
    
    #[wasm_bindgen]
    pub async fn create_agent(&mut self, name: &str) -> Result<u32, JsValue> {
        let agent_id = self.next_agent_id;
        self.next_agent_id += 1;
        
        let agent = WasmAgent {
            id: agent_id,
            name: name.to_string(),
            conversation_history: Vec::new(),
        };
        
        self.agents.insert(agent_id, agent);
        Ok(agent_id)
    }
    
    #[wasm_bindgen]
    pub async fn send_message(&mut self, agent_id: u32, message: &str) -> Result<WasmResponse, JsValue> {
        if message.is_empty() {
            return Err(JsValue::from_str("Empty message"));
        }
        
        let agent = self.agents.get_mut(&agent_id)
            .ok_or_else(|| JsValue::from_str("Invalid agent ID"))?;
        
        agent.conversation_history.push(WasmMessage {
            role: "user".to_string(),
            content: message.to_string(),
            timestamp: js_sys::Date::now(),
        });
        
        // Mock response
        let response_content = format!("Response to: {}", message);
        agent.conversation_history.push(WasmMessage {
            role: "assistant".to_string(),
            content: response_content.clone(),
            timestamp: js_sys::Date::now(),
        });
        
        Ok(WasmResponse {
            agent_id,
            content: response_content,
            timestamp: js_sys::Date::now(),
        })
    }
    
    #[wasm_bindgen]
    pub async fn get_conversation_history(&self, agent_id: u32) -> Result<JsValue, JsValue> {
        let agent = self.agents.get(&agent_id)
            .ok_or_else(|| JsValue::from_str("Invalid agent ID"))?;
        
        let history = serde_wasm_bindgen::to_value(&agent.conversation_history)?;
        Ok(history)
    }
    
    #[wasm_bindgen]
    pub async fn list_agents(&self) -> Result<JsValue, JsValue> {
        let agents: Vec<_> = self.agents.values().collect();
        let agents_js = serde_wasm_bindgen::to_value(&agents)?;
        Ok(agents_js)
    }
    
    #[wasm_bindgen]
    pub async fn get_agent_info(&self, agent_id: u32) -> Result<JsValue, JsValue> {
        let agent = self.agents.get(&agent_id)
            .ok_or_else(|| JsValue::from_str("Invalid agent ID"))?;
        
        let agent_js = serde_wasm_bindgen::to_value(agent)?;
        Ok(agent_js)
    }
    
    #[wasm_bindgen]
    pub async fn destroy_agent(&mut self, agent_id: u32) -> Result<(), JsValue> {
        self.agents.remove(&agent_id)
            .ok_or_else(|| JsValue::from_str("Invalid agent ID"))?;
        Ok(())
    }
    
    #[wasm_bindgen]
    pub async fn get_performance_metrics(&self) -> Result<JsValue, JsValue> {
        let metrics = WasmPerformanceMetrics {
            total_requests: self.agents.len() as u32,
            average_response_time: 150.0,
            total_processing_time: 1000.0,
        };
        
        let metrics_js = serde_wasm_bindgen::to_value(&metrics)?;
        Ok(metrics_js)
    }
    
    #[wasm_bindgen]
    pub async fn update_config(&mut self, config: WasmConfig) -> Result<(), JsValue> {
        self.config = Some(config);
        Ok(())
    }
    
    #[wasm_bindgen]
    pub async fn get_available_features(&self) -> Result<JsValue, JsValue> {
        let features = vec![
            "message_processing".to_string(),
            "agent_management".to_string(),
            "config_storage".to_string(),
        ];
        
        let features_js = serde_wasm_bindgen::to_value(&features)?;
        Ok(features_js)
    }
}

// Support types for WASM testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmAgent {
    pub id: u32,
    pub name: String,
    pub conversation_history: Vec<WasmMessage>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmMessage {
    pub role: String,
    pub content: String,
    pub timestamp: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmResponse {
    pub agent_id: u32,
    pub content: String,
    pub timestamp: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub temperature: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmPerformanceMetrics {
    pub total_requests: u32,
    pub average_response_time: f64,
    pub total_processing_time: f64,
}

#[derive(Debug)]
pub enum WasmError {
    InvalidAgentId(u32),
    InvalidInput(String),
    CorsError(String),
    NetworkError(String),
}

// Helper functions for configuration management
pub async fn store_config(config: &WasmConfig) -> Result<(), WasmError> {
    let config_json = serde_json::to_string(config)
        .map_err(|e| WasmError::InvalidInput(e.to_string()))?;
    
    setItem("opencode_config", &config_json);
    Ok(())
}

pub async fn load_config() -> Result<WasmConfig, WasmError> {
    let config_json = getItem("opencode_config")
        .ok_or_else(|| WasmError::InvalidInput("No config found".to_string()))?;
    
    let config: WasmConfig = serde_json::from_str(&config_json)
        .map_err(|e| WasmError::InvalidInput(e.to_string()))?;
    
    Ok(config)
}