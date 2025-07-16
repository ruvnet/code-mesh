//! # Code Mesh Core
//! 
//! **Code Mesh Core** is the foundational library for the Code Mesh AI coding assistant.
//! It provides a comprehensive set of abstractions and implementations for building
//! AI-powered development tools.
//! 
//! ## Features
//! 
//! - **🤖 Multi-LLM Support**: Unified interface for Anthropic Claude, OpenAI GPT, Google Gemini, and more
//! - **🛠️ Extensible Tool System**: Built-in tools for file operations, code execution, web search, and custom extensions
//! - **💾 Session Management**: Persistent conversation history with intelligent context management
//! - **🔐 Secure Authentication**: OAuth and API key support with encrypted credential storage
//! - **🌐 Cross-Platform**: Native performance with WebAssembly compatibility
//! - **🧠 Agent Orchestration**: Multi-agent coordination for complex coding workflows
//! 
//! ## Quick Start
//! 
//! ```rust
//! use code_mesh_core::{Session, LanguageModel, ProviderRegistry, ToolRegistry};
//! use tokio;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize provider registry
//!     let mut providers = ProviderRegistry::new();
//!     providers.register_anthropic("your-api-key")?;
//!     
//!     // Create a new session
//!     let mut session = Session::new();
//!     session.add_user_message("Help me implement a binary search function");
//!     
//!     // Get a language model
//!     let model = providers.get_model("anthropic/claude-3-opus")?;
//!     
//!     // Generate response
//!     let response = model.complete(&session.build_prompt()).await?;
//!     session.add_assistant_message(response);
//!     
//!     println!("Assistant: {}", session.last_message().content);
//!     Ok(())
//! }
//! ```
//! 
//! ## Architecture Overview
//! 
//! Code Mesh Core is built around several key abstractions:
//! 
//! ### Language Models ([`llm`] module)
//! 
//! The [`LanguageModel`] trait provides a unified interface for interacting with different
//! AI providers. Implementations are available for major providers:
//! 
//! - [`AnthropicProvider`] - Claude models via Anthropic API
//! - [`OpenAIProvider`] - GPT models via OpenAI API
//! - [`MistralProvider`] - Mistral models via Mistral AI API
//! 
//! ### Tools ([`tool`] module)
//! 
//! The [`Tool`] trait enables AI agents to interact with external systems:
//! 
//! - [`FileTools`] - Read, write, and search files
//! - [`BashTool`] - Execute shell commands safely
//! - [`WebTool`] - Search the web and fetch documentation
//! - [`GitTool`] - Git operations and repository management
//! 
//! ### Sessions ([`session`] module)
//! 
//! Sessions manage conversation state and context:
//! 
//! - [`Session`] - Core conversation management
//! - [`SessionManager`] - Persistence and retrieval
//! - [`Message`] - Individual conversation messages
//! 
//! ### Authentication ([`auth`] module)
//! 
//! Secure credential management for AI providers:
//! 
//! - [`Auth`] - Authentication interface
//! - [`CredentialStore`] - Encrypted credential storage
//! - [`OAuthFlow`] - OAuth authentication flows
//! 
//! ## Examples
//! 
//! ### Multi-Provider Setup
//! 
//! ```rust
//! use code_mesh_core::{ProviderRegistry, Provider};
//! 
//! let mut registry = ProviderRegistry::new();
//! 
//! // Add multiple providers
//! registry.register_anthropic("anthropic-key")?;
//! registry.register_openai("openai-key")?;
//! registry.register_mistral("mistral-key")?;
//! 
//! // Use different models for different tasks
//! let planning_model = registry.get_model("anthropic/claude-3-opus")?;
//! let coding_model = registry.get_model("openai/gpt-4")?;
//! let testing_model = registry.get_model("mistral/mistral-large")?;
//! ```
//! 
//! ### Tool Integration
//! 
//! ```rust
//! use code_mesh_core::{ToolRegistry, FileTools, BashTool};
//! 
//! let mut tools = ToolRegistry::new();
//! tools.register(Box::new(FileTools::new()));
//! tools.register(Box::new(BashTool::new()));
//! 
//! // Tools can be used by AI agents
//! let context = ToolContext::new();
//! let result = tools.execute("read_file", &["src/main.rs"], &context).await?;
//! ```
//! 
//! ### Session Persistence
//! 
//! ```rust
//! use code_mesh_core::{SessionManager, Storage};
//! 
//! let storage = Storage::new("./sessions")?;
//! let mut manager = SessionManager::new(storage);
//! 
//! // Save session
//! let session_id = manager.save_session(&session).await?;
//! 
//! // Load session later
//! let restored_session = manager.load_session(&session_id).await?;
//! ```
//! 
//! ## Feature Flags
//! 
//! Code Mesh Core supports conditional compilation based on target platform:
//! 
//! - `native` (default): Full native functionality including file system access
//! - `wasm`: WebAssembly-compatible subset with browser APIs
//! - `openai`: OpenAI provider support
//! - `anthropic`: Anthropic provider support  
//! - `mistral`: Mistral provider support
//! 
//! ## Error Handling
//! 
//! All public APIs use the [`Result`] type with [`Error`] for consistent error handling.
//! Errors are categorized by type and provide detailed context for debugging.
//! 
//! ## Performance Considerations
//! 
//! - **Async/Await**: All I/O operations are asynchronous for better performance
//! - **Connection Pooling**: HTTP clients use connection pooling for efficiency
//! - **Caching**: Intelligent caching of model responses and file contents
//! - **Memory Management**: Bounded memory usage with configurable limits

// Core modules
pub mod agent;
pub mod auth;
pub mod llm;
pub mod memory;
pub mod planner;
pub mod session;
pub mod storage;
pub mod tool;

// Utility modules
pub mod config;
pub mod error;
pub mod events;
pub mod features;
pub mod permission;
pub mod sync;
pub mod utils;

// Generated prompts
pub mod prompts;

// Re-export commonly used types
pub use llm::{
    Provider, Model, LanguageModel, ProviderRegistry,
    Message, MessageRole, MessageContent, GenerateOptions,
    GenerateResult, StreamChunk, Usage, FinishReason
};
pub use tool::{
    Tool, ToolContext, ToolResult, ToolRegistry, ToolError,
    ToolDefinition
};
pub use auth::{Auth, AuthCredentials, AuthStorage};
pub use storage::{Storage, StorageError};
pub use session::{
    Session, Message as SessionMessage, SessionManager, 
    MessageRole as SessionMessageRole, SessionMetadata
};
pub use config::{Config, ConfigManager, ProviderConfig, ToolConfig};
pub use permission::{PermissionManager, PermissionContext, PermissionLevel};

// Error types
pub use error::{Error, Result};

// Event system
pub use events::{Event, EventBus, EventHandler};

// Synchronization primitives
pub use sync::{AsyncMutex, AsyncRwLock, Debouncer};

// Version and build information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");


/// Runtime compatibility layer
#[cfg(feature = "native")]
pub mod runtime {
    pub use tokio::*;
    pub type Runtime = tokio::runtime::Runtime;
    pub type Handle = tokio::runtime::Handle;
}

#[cfg(feature = "wasm")]
pub mod runtime {
    pub use wasm_bindgen_futures::*;
    // WASM doesn't have a runtime concept like tokio
    pub type Runtime = ();
    pub type Handle = ();
}