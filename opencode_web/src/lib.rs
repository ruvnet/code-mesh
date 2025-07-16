//! OpenCode Web Interface
//!
//! This module provides a web-based interface for OpenCode using Yew and WebAssembly.
//! It offers a rich, interactive experience for using OpenCode in web browsers.

use wasm_bindgen::prelude::*;
use yew::prelude::*;

mod app;
mod components;
mod pages;
mod services;
mod utils;

use app::App;

/// Main entry point for the web application
#[wasm_bindgen(start)]
pub fn run_app() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Initialize logging
    console_log::init_with_level(log::Level::Debug).expect("Failed to initialize logger");
    
    log::info!("Starting OpenCode Web Interface");
    
    // Mount the app
    yew::Renderer::<App>::new().render();
}

/// Export the main function for JavaScript
#[wasm_bindgen]
pub fn main() {
    run_app();
}

/// Get version information
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Initialize the OpenCode web interface
#[wasm_bindgen]
pub async fn init_opencode() -> Result<(), JsValue> {
    opencode_core::initialize().await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    log::info!("OpenCode core initialized successfully");
    Ok(())
}

/// Get runtime information
#[wasm_bindgen]
pub fn get_runtime_info() -> JsValue {
    let info = opencode_core::runtime_info();
    serde_wasm_bindgen::to_value(&info).unwrap_or(JsValue::NULL)
}

// Re-export commonly used types
pub use opencode_core::wasm_bindings::*;

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }
    
    #[wasm_bindgen_test]
    fn test_get_runtime_info() {
        let info = get_runtime_info();
        assert!(!info.is_null());
        assert!(!info.is_undefined());
    }
    
    #[wasm_bindgen_test]
    async fn test_init_opencode() {
        let result = init_opencode().await;
        assert!(result.is_ok());
    }
}