//! Code Mesh WASM bindings

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;
use code_mesh_core::{Session, Message, MessageRole};

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
}

/// Code Mesh WASM API
#[wasm_bindgen]
pub struct CodeMesh {
    session: Session,
}

#[wasm_bindgen]
impl CodeMesh {
    /// Create a new Code Mesh instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            session: Session::new(),
        }
    }
    
    /// Get the current session ID
    #[wasm_bindgen(getter)]
    pub fn session_id(&self) -> String {
        self.session.id.clone()
    }
    
    /// Add a user message to the session
    pub async fn add_user_message(&mut self, content: String) -> Result<(), JsValue> {
        let message = Message::new(MessageRole::User, content);
        self.session.add_message(message);
        Ok(())
    }
    
    /// Generate a response from the AI
    pub async fn generate_response(&mut self, model: String) -> Result<String, JsValue> {
        // TODO: Implement actual generation
        Ok("WASM generation not yet implemented".to_string())
    }
    
    /// Get all messages in the session
    pub fn get_messages(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.session.messages)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    /// Clear the current session
    pub fn clear_session(&mut self) {
        self.session = Session::new();
    }
}

/// List available providers
#[wasm_bindgen]
pub async fn list_providers() -> Result<JsValue, JsValue> {
    // TODO: Implement provider listing
    let providers = vec!["anthropic", "openai", "mistral"];
    serde_wasm_bindgen::to_value(&providers)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// List available models for a provider
#[wasm_bindgen]
pub async fn list_models(provider: String) -> Result<JsValue, JsValue> {
    // TODO: Implement model listing
    let models = match provider.as_str() {
        "anthropic" => vec!["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
        "openai" => vec!["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"],
        _ => vec![],
    };
    serde_wasm_bindgen::to_value(&models)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}