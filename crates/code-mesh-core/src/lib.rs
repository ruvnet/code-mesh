//! Code Mesh Core - AI coding assistant core functionality
//! 
//! This crate provides the core abstractions and implementations for:
//! - LLM provider management
//! - Tool system
//! - Session management
//! - Authentication
//! - Storage and persistence

pub mod agent;
pub mod auth;
pub mod llm;
pub mod memory;
pub mod planner;
pub mod session;
pub mod storage;
pub mod tool;

// Re-export commonly used types
pub use llm::{Provider, Model, LanguageModel};
pub use tool::{Tool, ToolContext, ToolResult};
pub use auth::{Auth, AuthCredentials};
pub use storage::{Storage, StorageError};
pub use session::{Session, Message};

// Error types
pub mod error;
pub use error::{Error, Result};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");