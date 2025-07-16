//! # OpenCode Core
//!
//! Core engine for the OpenCode AI coding assistant. This crate provides the fundamental
//! functionality for managing AI agents, LLM provider interactions, session management,
//! and file system operations. It's designed to work in both native and WebAssembly
//! environments through conditional compilation.
//!
//! ## Features
//!
//! - **Multi-provider LLM support**: OpenAI, Anthropic, and local models
//! - **Agent orchestration**: Multiple AI agents working on the same project
//! - **Session management**: Persistent conversation history and context
//! - **File system abstraction**: Works in both native and browser environments
//! - **Async-first design**: Built on top of async/await patterns
//! - **Cross-platform**: Supports native CLI and WebAssembly targets
//!
//! ## Architecture
//!
//! The core is organized into several key modules:
//!
//! - [`agent`]: AI agent management and orchestration
//! - [`providers`]: LLM provider implementations
//! - [`session`]: Session and conversation management
//! - [`filesystem`]: File system abstraction layer
//! - [`config`]: Configuration management
//! - [`memory`]: Memory and context storage
//!
//! ## Usage
//!
//! ```rust
//! use opencode_core::{Engine, Config, providers::OpenAIProvider};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::load().await?;
//!     let provider = OpenAIProvider::new(config.openai_api_key)?;
//!     let engine = Engine::new(provider, config);
//!     
//!     let agent = engine.create_agent("coding_assistant").await?;
//!     let response = agent.send_message("Help me write a Rust function").await?;
//!     
//!     println!("Response: {}", response.content);
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use log::{debug, info, warn};
use std::collections::HashMap;
use uuid::Uuid;

// Re-export commonly used types
pub use crate::agent::{Agent, AgentConfig, AgentHandle, AgentState};
pub use crate::config::{Config, ProviderConfig};
pub use crate::engine::Engine;
pub use crate::providers::{LLMProvider, ProviderType};
pub use crate::session::{Session, SessionManager};

// Core modules
pub mod agent;
pub mod config;
pub mod engine;
pub mod filesystem;
pub mod memory;
pub mod providers;
pub mod session;

// Conditional modules based on target
#[cfg(feature = "native-runtime")]
pub mod native;

#[cfg(feature = "wasm-runtime")]
pub mod wasm;

// WASM bindings
#[cfg(feature = "wasm-runtime")]
pub mod wasm_bindings;

/// Core error types for the OpenCode system
#[derive(thiserror::Error, Debug)]
pub enum OpenCodeError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("Provider error: {0}")]
    Provider(#[from] providers::ProviderError),
    
    #[error("Agent error: {0}")]
    Agent(#[from] agent::AgentError),
    
    #[error("Session error: {0}")]
    Session(#[from] session::SessionError),
    
    #[error("File system error: {0}")]
    FileSystem(#[from] filesystem::FileSystemError),
    
    #[error("Memory error: {0}")]
    Memory(#[from] memory::MemoryError),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Global result type for the OpenCode system
pub type OpenCodeResult<T> = Result<T, OpenCodeError>;

/// Initialize the OpenCode core system
///
/// This function sets up logging, initializes the runtime environment,
/// and performs any necessary platform-specific setup.
pub async fn initialize() -> OpenCodeResult<()> {
    // Initialize logging
    #[cfg(feature = "native-runtime")]
    {
        env_logger::init();
        info!("OpenCode Core initialized in native mode");
    }
    
    #[cfg(feature = "wasm-runtime")]
    {
        console_log::init_with_level(log::Level::Info).ok();
        console_error_panic_hook::set_once();
        info!("OpenCode Core initialized in WASM mode");
    }
    
    debug!("Core initialization complete");
    Ok(())
}

/// Get the current runtime environment
pub fn runtime_info() -> RuntimeInfo {
    RuntimeInfo {
        is_wasm: cfg!(target_arch = "wasm32"),
        is_native: cfg!(not(target_arch = "wasm32")),
        has_filesystem: cfg!(feature = "native-runtime"),
        has_command_execution: cfg!(feature = "command-execution"),
        has_file_watching: cfg!(feature = "file-watching"),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Information about the current runtime environment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeInfo {
    pub is_wasm: bool,
    pub is_native: bool,
    pub has_filesystem: bool,
    pub has_command_execution: bool,
    pub has_file_watching: bool,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_initialize_native() {
        let result = initialize().await;
        assert!(result.is_ok());
    }
    
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_initialize_wasm() {
        let result = initialize().await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_runtime_info() {
        let info = runtime_info();
        assert_eq!(info.is_wasm, cfg!(target_arch = "wasm32"));
        assert_eq!(info.is_native, cfg!(not(target_arch = "wasm32")));
        assert!(!info.version.is_empty());
    }
}