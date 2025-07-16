//! WASM bindings for OpenCode
//!
//! This module provides JavaScript-callable functions for using OpenCode
//! in WebAssembly environments.

#[cfg(feature = "wasm-runtime")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm-runtime")]
use crate::engine::Engine;
#[cfg(feature = "wasm-runtime")]
use crate::config::Config;
#[cfg(feature = "wasm-runtime")]
use crate::agent::AgentHandle;
#[cfg(feature = "wasm-runtime")]
use std::sync::Arc;
#[cfg(feature = "wasm-runtime")]
use tokio::sync::Mutex;

/// JavaScript-accessible OpenCode engine
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub struct OpenCodeEngine {
    engine: Arc<Mutex<Engine>>,
}

/// JavaScript-accessible agent handle
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub struct OpenCodeAgent {
    handle: AgentHandle,
}

/// JavaScript-accessible agent response
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub struct OpenCodeResponse {
    content: String,
    usage: Option<String>, // JSON string
    finish_reason: Option<String>,
    metadata: String, // JSON string
}

/// JavaScript-accessible runtime information
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub struct RuntimeInfo {
    is_wasm: bool,
    is_native: bool,
    has_filesystem: bool,
    has_command_execution: bool,
    has_file_watching: bool,
    version: String,
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
impl OpenCodeEngine {
    /// Create a new OpenCode engine
    #[wasm_bindgen(constructor)]
    pub fn new() -> OpenCodeEngine {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();
        
        // This is a placeholder - actual initialization would be async
        OpenCodeEngine {
            engine: Arc::new(Mutex::new(
                // We'll need to create a mock engine for synchronous construction
                // Real implementation would require async initialization
                Engine::new().unwrap() // This will fail without proper async setup
            )),
        }
    }
    
    /// Initialize the engine (async)
    #[wasm_bindgen]
    pub async fn init() -> Result<OpenCodeEngine, JsValue> {
        // Initialize logging
        crate::initialize().await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        // Create engine with default configuration
        let engine = Engine::new().await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(OpenCodeEngine {
            engine: Arc::new(Mutex::new(engine)),
        })
    }
    
    /// Initialize the engine with custom configuration
    #[wasm_bindgen]
    pub async fn init_with_config(config_json: &str) -> Result<OpenCodeEngine, JsValue> {
        // Initialize logging
        crate::initialize().await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        // Parse configuration
        let config: Config = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Config parse error: {}", e)))?;
        
        // Create engine with custom configuration
        let engine = Engine::with_config(config).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(OpenCodeEngine {
            engine: Arc::new(Mutex::new(engine)),
        })
    }
    
    /// Create a new agent
    #[wasm_bindgen]
    pub async fn create_agent(&self, name: &str) -> Result<OpenCodeAgent, JsValue> {
        let engine = self.engine.lock().await;
        let handle = engine.create_agent(name).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(OpenCodeAgent { handle })
    }
    
    /// Create a new agent with specific provider
    #[wasm_bindgen]
    pub async fn create_agent_with_provider(&self, name: &str, provider: &str) -> Result<OpenCodeAgent, JsValue> {
        let engine = self.engine.lock().await;
        let handle = engine.create_agent_with_provider(name, provider).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(OpenCodeAgent { handle })
    }
    
    /// List available providers
    #[wasm_bindgen]
    pub async fn list_providers(&self) -> Vec<String> {
        let engine = self.engine.lock().await;
        engine.list_providers().await
    }
    
    /// Get engine statistics as JSON
    #[wasm_bindgen]
    pub async fn get_stats(&self) -> String {
        let engine = self.engine.lock().await;
        let stats = engine.stats().await;
        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Store value in memory
    #[wasm_bindgen]
    pub async fn store_memory(&self, key: &str, value: &str) -> Result<(), JsValue> {
        let engine = self.engine.lock().await;
        let memory = engine.memory();
        let memory_guard = memory.read().await;
        
        let json_value: serde_json::Value = serde_json::from_str(value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        memory_guard.store(key, json_value).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(())
    }
    
    /// Retrieve value from memory
    #[wasm_bindgen]
    pub async fn retrieve_memory(&self, key: &str) -> Option<String> {
        let engine = self.engine.lock().await;
        let memory = engine.memory();
        let memory_guard = memory.read().await;
        
        if let Ok(Some(value)) = memory_guard.retrieve(key).await {
            serde_json::to_string(&value).ok()
        } else {
            None
        }
    }
    
    /// List memory keys
    #[wasm_bindgen]
    pub async fn list_memory(&self) -> Vec<String> {
        let engine = self.engine.lock().await;
        let memory = engine.memory();
        let memory_guard = memory.read().await;
        
        memory_guard.list().await.unwrap_or_else(|_| Vec::new())
    }
    
    /// Search memory
    #[wasm_bindgen]
    pub async fn search_memory(&self, query: &str) -> Vec<String> {
        let engine = self.engine.lock().await;
        let memory = engine.memory();
        let memory_guard = memory.read().await;
        
        memory_guard.search(query).await.unwrap_or_else(|_| Vec::new())
    }
    
    /// Shutdown the engine
    #[wasm_bindgen]
    pub async fn shutdown(&self) -> Result<(), JsValue> {
        let engine = self.engine.lock().await;
        engine.shutdown().await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(())
    }
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
impl OpenCodeAgent {
    /// Send a message to the agent
    #[wasm_bindgen]
    pub async fn send_message(&self, message: &str) -> Result<OpenCodeResponse, JsValue> {
        let response = self.handle.send_message(message).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        let usage_json = response.usage.map(|u| 
            serde_json::to_string(&u).unwrap_or_else(|_| "{}".to_string())
        );
        
        let metadata_json = serde_json::to_string(&response.metadata)
            .unwrap_or_else(|_| "{}".to_string());
        
        Ok(OpenCodeResponse {
            content: response.content,
            usage: usage_json,
            finish_reason: response.finish_reason,
            metadata: metadata_json,
        })
    }
    
    /// Get agent statistics as JSON
    #[wasm_bindgen]
    pub async fn get_stats(&self) -> String {
        let stats = self.handle.get_stats().await;
        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Get conversation history as JSON
    #[wasm_bindgen]
    pub async fn get_history(&self) -> String {
        let history = self.handle.get_history().await;
        serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string())
    }
    
    /// Clear conversation history
    #[wasm_bindgen]
    pub async fn clear_history(&self) {
        self.handle.clear_history().await;
    }
    
    /// Reset the agent
    #[wasm_bindgen]
    pub async fn reset(&self) {
        self.handle.reset().await;
    }
    
    /// Get agent ID
    #[wasm_bindgen]
    pub fn get_id(&self) -> String {
        self.handle.id.to_string()
    }
    
    /// Get agent name
    #[wasm_bindgen]
    pub fn get_name(&self) -> String {
        self.handle.name.clone()
    }
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
impl OpenCodeResponse {
    /// Get response content
    #[wasm_bindgen(getter)]
    pub fn content(&self) -> String {
        self.content.clone()
    }
    
    /// Get usage information as JSON
    #[wasm_bindgen(getter)]
    pub fn usage(&self) -> Option<String> {
        self.usage.clone()
    }
    
    /// Get finish reason
    #[wasm_bindgen(getter)]
    pub fn finish_reason(&self) -> Option<String> {
        self.finish_reason.clone()
    }
    
    /// Get metadata as JSON
    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> String {
        self.metadata.clone()
    }
}

/// Global functions for WASM
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub async fn initialize_opencode() -> Result<(), JsValue> {
    crate::initialize().await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(())
}

/// Get runtime information
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub fn get_runtime_info() -> RuntimeInfo {
    let info = crate::runtime_info();
    RuntimeInfo {
        is_wasm: info.is_wasm,
        is_native: info.is_native,
        has_filesystem: info.has_filesystem,
        has_command_execution: info.has_command_execution,
        has_file_watching: info.has_file_watching,
        version: info.version,
    }
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
impl RuntimeInfo {
    /// Check if running in WASM
    #[wasm_bindgen(getter)]
    pub fn is_wasm(&self) -> bool {
        self.is_wasm
    }
    
    /// Check if running natively
    #[wasm_bindgen(getter)]
    pub fn is_native(&self) -> bool {
        self.is_native
    }
    
    /// Check if filesystem is available
    #[wasm_bindgen(getter)]
    pub fn has_filesystem(&self) -> bool {
        self.has_filesystem
    }
    
    /// Check if command execution is available
    #[wasm_bindgen(getter)]
    pub fn has_command_execution(&self) -> bool {
        self.has_command_execution
    }
    
    /// Check if file watching is available
    #[wasm_bindgen(getter)]
    pub fn has_file_watching(&self) -> bool {
        self.has_file_watching
    }
    
    /// Get version
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> String {
        self.version.clone()
    }
}

/// Utility functions for JavaScript
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub fn log_to_console(message: &str) {
    crate::wasm::utils::log(message);
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub fn error_to_console(message: &str) {
    crate::wasm::utils::error(message);
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub fn warn_to_console(message: &str) {
    crate::wasm::utils::warn(message);
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub fn get_timestamp() -> f64 {
    crate::wasm::utils::now()
}

#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
pub async fn sleep_ms(ms: u32) {
    crate::wasm::utils::sleep(ms).await;
}

// Export commonly used types for JavaScript
#[cfg(feature = "wasm-runtime")]
#[wasm_bindgen]
extern "C" {
    /// Console log function
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    /// Console error function
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
    
    /// Console warn function
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
}

// Re-export for use in other modules
#[cfg(feature = "wasm-runtime")]
pub use wasm_bindgen::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn test_wasm_bindings_compilation() {
        // This test ensures the WASM bindings compile
        // Actual functionality would need to be tested in a browser environment
    }
    
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn test_runtime_info() {
        let info = get_runtime_info();
        assert!(info.is_wasm());
        assert!(!info.is_native());
        assert!(!info.version().is_empty());
    }
}