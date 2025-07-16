//! Code Mesh WASM bindings for browser and Node.js environments
//! 
//! This module provides comprehensive WebAssembly bindings for the Code Mesh 
//! system, including browser-compatible storage, HTTP clients, and performance
//! optimizations.

use wasm_bindgen::prelude::*;

mod performance;

pub use performance::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Array, Object, Promise, Uint8Array};
use web_sys::{console, window, Storage};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use std::collections::HashMap;

// Import core functionality
use code_mesh_core::{Session, Message, MessageRole};

// Global allocator for optimized memory usage
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Module declarations
mod browser;
mod storage;
mod http;
mod worker;
mod auth;
mod performance;
mod utils;

pub use browser::*;
pub use storage::*;
pub use http::*;
pub use worker::*;
pub use auth::*;
pub use performance::*;
pub use utils::*;

/// Initialize WASM module with optimal settings
#[wasm_bindgen(start)]
pub fn init() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Initialize logger
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    
    // Initialize performance monitoring
    performance::init_performance_monitor();
    
    console::log_1(&"Code Mesh WASM initialized successfully".into());
}

/// Configuration for Code Mesh WASM instance
#[derive(Serialize, Deserialize, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct CodeMeshConfig {
    /// Whether to use browser storage (IndexedDB)
    pub use_browser_storage: bool,
    /// Whether to enable offline capabilities
    pub enable_offline: bool,
    /// Whether to use web workers for background tasks
    pub use_web_workers: bool,
    /// Maximum memory usage in MB
    pub max_memory_mb: u32,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// API endpoint for cloud services
    pub api_endpoint: Option<String>,
    /// Authentication provider
    pub auth_provider: Option<String>,
}

impl Default for CodeMeshConfig {
    fn default() -> Self {
        Self {
            use_browser_storage: true,
            enable_offline: false,
            use_web_workers: true,
            max_memory_mb: 512,
            enable_performance_monitoring: true,
            api_endpoint: None,
            auth_provider: None,
        }
    }
}

#[wasm_bindgen]
impl CodeMeshConfig {
    /// Create a new configuration with default values
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create configuration optimized for browser usage
    #[wasm_bindgen]
    pub fn for_browser() -> Self {
        Self {
            use_browser_storage: true,
            enable_offline: true,
            use_web_workers: true,
            max_memory_mb: 256,
            enable_performance_monitoring: true,
            api_endpoint: Some("https://api.code-mesh.dev".to_string()),
            auth_provider: Some("browser".to_string()),
        }
    }
    
    /// Create configuration optimized for Node.js usage
    #[wasm_bindgen]
    pub fn for_node() -> Self {
        Self {
            use_browser_storage: false,
            enable_offline: false,
            use_web_workers: false,
            max_memory_mb: 1024,
            enable_performance_monitoring: false,
            api_endpoint: None,
            auth_provider: Some("node".to_string()),
        }
    }
}

/// Main Code Mesh WASM API
#[wasm_bindgen]
pub struct CodeMesh {
    session: Session,
    config: CodeMeshConfig,
    storage: Option<BrowserStorage>,
    http_client: HttpClient,
    performance_monitor: PerformanceMonitor,
    auth_manager: Option<AuthManager>,
}

#[wasm_bindgen]
impl CodeMesh {
    /// Create a new Code Mesh instance with default configuration
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::with_config(CodeMeshConfig::default())
    }
    
    /// Create a new Code Mesh instance with custom configuration
    #[wasm_bindgen]
    pub fn with_config(config: CodeMeshConfig) -> Self {
        let storage = if config.use_browser_storage {
            Some(BrowserStorage::new())
        } else {
            None
        };
        
        let http_client = HttpClient::new(config.api_endpoint.clone());
        let performance_monitor = PerformanceMonitor::new(config.enable_performance_monitoring);
        
        let auth_manager = config.auth_provider.as_ref().map(|provider| {
            AuthManager::new(provider.clone())
        });
        
        Self {
            session: Session::new(),
            config,
            storage,
            http_client,
            performance_monitor,
            auth_manager,
        }
    }
    
    /// Initialize the Code Mesh instance (async setup)
    #[wasm_bindgen]
    pub async fn initialize(&mut self) -> Result<(), JsValue> {
        self.performance_monitor.mark("initialize_start")?;
        
        // Initialize storage if enabled
        if let Some(storage) = &mut self.storage {
            storage.initialize().await?;
        }
        
        // Initialize authentication if configured
        if let Some(auth) = &mut self.auth_manager {
            auth.initialize().await?;
        }
        
        self.performance_monitor.mark("initialize_end")?;
        self.performance_monitor.measure("initialize", "initialize_start", "initialize_end")?;
        
        Ok(())
    }
    
    /// Get the current session ID
    #[wasm_bindgen(getter)]
    pub fn session_id(&self) -> String {
        self.session.id.clone()
    }
    
    /// Get configuration as JSON
    #[wasm_bindgen]
    pub fn get_config(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.config)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    /// Update configuration
    #[wasm_bindgen]
    pub fn update_config(&mut self, config: &JsValue) -> Result<(), JsValue> {
        let new_config: CodeMeshConfig = serde_wasm_bindgen::from_value(config.clone())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        self.config = new_config;
        Ok(())
    }
    
    /// Add a user message to the session
    #[wasm_bindgen]
    pub async fn add_user_message(&mut self, content: String) -> Result<(), JsValue> {
        self.performance_monitor.mark("add_message_start")?;
        
        let message = Message::new(MessageRole::User, content);
        self.session.add_message(message.clone());
        
        // Save to storage if enabled
        if let Some(storage) = &self.storage {
            storage.save_message(&self.session.id, &message).await?;
        }
        
        self.performance_monitor.mark("add_message_end")?;
        self.performance_monitor.measure("add_message", "add_message_start", "add_message_end")?;
        
        Ok(())
    }
    
    /// Generate a response from the AI
    #[wasm_bindgen]
    pub async fn generate_response(&mut self, model: String, api_key: Option<String>) -> Result<String, JsValue> {
        self.performance_monitor.mark("generate_start")?;
        
        // Use HTTP client for generation
        let response = self.http_client.generate_response(&self.session, &model, api_key).await?;
        
        // Add response to session
        let message = Message::new(MessageRole::Assistant, response.clone());
        self.session.add_message(message.clone());
        
        // Save to storage if enabled
        if let Some(storage) = &self.storage {
            storage.save_message(&self.session.id, &message).await?;
        }
        
        self.performance_monitor.mark("generate_end")?;
        self.performance_monitor.measure("generate", "generate_start", "generate_end")?;
        
        Ok(response)
    }
    
    /// Get all messages in the session
    #[wasm_bindgen]
    pub fn get_messages(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.session.messages)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    /// Load session from storage
    #[wasm_bindgen]
    pub async fn load_session(&mut self, session_id: String) -> Result<(), JsValue> {
        if let Some(storage) = &self.storage {
            if let Some(session) = storage.load_session(&session_id).await? {
                self.session = session;
            } else {
                return Err(JsValue::from_str("Session not found"));
            }
        } else {
            return Err(JsValue::from_str("Storage not available"));
        }
        Ok(())
    }
    
    /// Save current session to storage
    #[wasm_bindgen]
    pub async fn save_session(&self) -> Result<(), JsValue> {
        if let Some(storage) = &self.storage {
            storage.save_session(&self.session).await?;
        } else {
            return Err(JsValue::from_str("Storage not available"));
        }
        Ok(())
    }
    
    /// List all saved sessions
    #[wasm_bindgen]
    pub async fn list_sessions(&self) -> Result<JsValue, JsValue> {
        if let Some(storage) = &self.storage {
            let sessions = storage.list_sessions().await?;
            serde_wasm_bindgen::to_value(&sessions)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Storage not available"))
        }
    }
    
    /// Clear the current session
    #[wasm_bindgen]
    pub fn clear_session(&mut self) {
        self.session = Session::new();
    }
    
    /// Delete a session from storage
    #[wasm_bindgen]
    pub async fn delete_session(&self, session_id: String) -> Result<(), JsValue> {
        if let Some(storage) = &self.storage {
            storage.delete_session(&session_id).await?;
        } else {
            return Err(JsValue::from_str("Storage not available"));
        }
        Ok(())
    }
    
    /// Get performance metrics
    #[wasm_bindgen]
    pub fn get_performance_metrics(&self) -> Result<JsValue, JsValue> {
        self.performance_monitor.get_metrics()
    }
    
    /// Clear performance metrics
    #[wasm_bindgen]
    pub fn clear_performance_metrics(&mut self) {
        self.performance_monitor.clear_metrics();
    }
    
    /// Check if running in browser environment
    #[wasm_bindgen]
    pub fn is_browser(&self) -> bool {
        utils::is_browser()
    }
    
    /// Check if running in Node.js environment
    #[wasm_bindgen]
    pub fn is_node(&self) -> bool {
        utils::is_node()
    }
    
    /// Get clipboard content (browser only)
    #[wasm_bindgen]
    pub async fn get_clipboard(&self) -> Result<String, JsValue> {
        if !self.is_browser() {
            return Err(JsValue::from_str("Clipboard only available in browser"));
        }
        browser::get_clipboard().await
    }
    
    /// Set clipboard content (browser only)
    #[wasm_bindgen]
    pub async fn set_clipboard(&self, text: String) -> Result<(), JsValue> {
        if !self.is_browser() {
            return Err(JsValue::from_str("Clipboard only available in browser"));
        }
        browser::set_clipboard(text).await
    }
    
    /// Get memory usage statistics
    #[wasm_bindgen]
    pub fn get_memory_usage(&self) -> Result<JsValue, JsValue> {
        utils::get_memory_usage()
    }
}

/// List available providers
#[wasm_bindgen]
pub async fn list_providers() -> Result<JsValue, JsValue> {
    let providers = vec!["anthropic", "openai", "mistral", "cohere", "huggingface"];
    serde_wasm_bindgen::to_value(&providers)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// List available models for a provider
#[wasm_bindgen]
pub async fn list_models(provider: String) -> Result<JsValue, JsValue> {
    let models = match provider.as_str() {
        "anthropic" => vec![
            "claude-3-opus-20240229",
            "claude-3-sonnet-20240229", 
            "claude-3-haiku-20240307",
            "claude-3-5-sonnet-20241022"
        ],
        "openai" => vec![
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "gpt-4",
            "gpt-3.5-turbo"
        ],
        "mistral" => vec![
            "mistral-large",
            "mistral-medium",
            "mistral-small",
            "mistral-tiny"
        ],
        "cohere" => vec![
            "command-r-plus",
            "command-r",
            "command",
            "command-light"
        ],
        "huggingface" => vec![
            "meta-llama/Llama-2-70b-chat-hf",
            "microsoft/DialoGPT-large",
            "facebook/blenderbot-400M-distill"
        ],
        _ => vec![],
    };
    
    serde_wasm_bindgen::to_value(&models)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get platform information
#[wasm_bindgen]
pub fn get_platform_info() -> Result<JsValue, JsValue> {
    let info = utils::get_platform_info()?;
    serde_wasm_bindgen::to_value(&info)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Check WASM feature support
#[wasm_bindgen]
pub fn check_feature_support() -> Result<JsValue, JsValue> {
    let features = utils::check_wasm_features()?;
    serde_wasm_bindgen::to_value(&features)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Create a web worker for background processing
#[wasm_bindgen]
pub fn create_worker(script_url: String) -> Result<worker::CodeMeshWorker, JsValue> {
    worker::CodeMeshWorker::new(script_url)
}

/// Export for TypeScript definitions
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export interface CodeMeshMessage {
    role: "user" | "assistant" | "system";
    content: string;
    timestamp: string;
    id: string;
}

export interface CodeMeshSession {
    id: string;
    messages: CodeMeshMessage[];
    created_at: string;
    updated_at: string;
}

export interface PerformanceMetrics {
    marks: Record<string, number>;
    measures: Record<string, number>;
    memory_usage?: {
        used: number;
        total: number;
    };
}

export interface PlatformInfo {
    is_browser: boolean;
    is_node: boolean;
    user_agent?: string;
    platform?: string;
    language?: string;
}

export interface WasmFeatures {
    simd: boolean;
    threads: boolean;
    bulk_memory: boolean;
    reference_types: boolean;
}
"#;