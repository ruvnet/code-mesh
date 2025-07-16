//! Command implementations for the OpenCode CLI

pub mod ask;
pub mod config;
pub mod agent;
pub mod session;
pub mod memory;
pub mod provider;

use anyhow::Result;
use opencode_core::Engine;

/// Common trait for all commands
pub trait Command {
    /// Execute the command
    fn execute(&self, engine: &Engine) -> impl std::future::Future<Output = Result<()>> + Send;
}