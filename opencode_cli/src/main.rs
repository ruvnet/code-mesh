//! OpenCode CLI - Terminal interface for the OpenCode AI coding assistant
//!
//! This is the command-line interface for OpenCode, providing both
//! single-command execution and interactive TUI modes.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use opencode_core::{initialize, Config, Engine};
use std::path::PathBuf;
use tokio::signal;

mod cli;
mod commands;
mod tui;
mod utils;

use cli::*;
use commands::*;

/// OpenCode CLI application
#[derive(Parser)]
#[command(
    name = "opencode",
    version,
    about = "AI coding assistant built for the terminal",
    long_about = "OpenCode is an AI-powered coding assistant that helps you write, review, and understand code directly in your terminal."
)]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Quiet mode (suppress non-error output)
    #[arg(short, long, global = true)]
    quiet: bool,
    
    /// Provider to use (overrides config)
    #[arg(short, long, value_name = "PROVIDER")]
    provider: Option<String>,
    
    /// Model to use (overrides config)
    #[arg(short, long, value_name = "MODEL")]
    model: Option<String>,
    
    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Start interactive TUI mode
    #[command(name = "tui")]
    Tui {
        /// Agent name to create
        #[arg(short, long, default_value = "assistant")]
        agent: String,
        
        /// System prompt for the agent
        #[arg(short, long)]
        system_prompt: Option<String>,
    },
    
    /// Send a single message to an agent
    #[command(name = "ask")]
    Ask {
        /// The message to send
        message: String,
        
        /// Agent name to use
        #[arg(short, long, default_value = "assistant")]
        agent: String,
        
        /// System prompt for the agent
        #[arg(short, long)]
        system_prompt: Option<String>,
        
        /// Stream the response
        #[arg(long)]
        stream: bool,
    },
    
    /// Configuration management
    #[command(name = "config")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    /// Agent management
    #[command(name = "agent")]
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    
    /// Session management
    #[command(name = "session")]
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    
    /// Memory management
    #[command(name = "memory")]
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    
    /// Provider management
    #[command(name = "provider")]
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
}

/// Configuration actions
#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        
        /// Configuration value
        value: String,
    },
    
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
    
    /// Reset configuration to defaults
    Reset,
    
    /// Validate configuration
    Validate,
}

/// Agent actions
#[derive(Subcommand)]
pub enum AgentAction {
    /// List active agents
    List,
    
    /// Create a new agent
    Create {
        /// Agent name
        name: String,
        
        /// Provider to use
        #[arg(short, long)]
        provider: Option<String>,
        
        /// System prompt
        #[arg(short, long)]
        system_prompt: Option<String>,
    },
    
    /// Remove an agent
    Remove {
        /// Agent name
        name: String,
    },
    
    /// Show agent statistics
    Stats {
        /// Agent name
        name: String,
    },
    
    /// Reset agent conversation
    Reset {
        /// Agent name
        name: String,
    },
}

/// Session actions
#[derive(Subcommand)]
pub enum SessionAction {
    /// List sessions
    List,
    
    /// Create a new session
    Create {
        /// Session name
        name: String,
    },
    
    /// Load a session
    Load {
        /// Session ID
        id: String,
    },
    
    /// Save current session
    Save {
        /// Session ID
        id: String,
    },
    
    /// Delete a session
    Delete {
        /// Session ID
        id: String,
    },
}

/// Memory actions
#[derive(Subcommand)]
pub enum MemoryAction {
    /// List memory entries
    List,
    
    /// Store a value in memory
    Store {
        /// Key
        key: String,
        
        /// Value (JSON)
        value: String,
    },
    
    /// Retrieve a value from memory
    Get {
        /// Key
        key: String,
    },
    
    /// Delete a value from memory
    Delete {
        /// Key
        key: String,
    },
    
    /// Search memory entries
    Search {
        /// Search query
        query: String,
    },
    
    /// Clear all memory
    Clear,
    
    /// Show memory statistics
    Stats,
}

/// Provider actions
#[derive(Subcommand)]
pub enum ProviderAction {
    /// List available providers
    List,
    
    /// Test a provider
    Test {
        /// Provider name
        name: String,
    },
    
    /// Show provider models
    Models {
        /// Provider name
        name: String,
    },
    
    /// Add a provider
    Add {
        /// Provider name
        name: String,
        
        /// Provider type
        #[arg(short, long)]
        provider_type: String,
        
        /// API key
        #[arg(short, long)]
        api_key: Option<String>,
        
        /// Base URL
        #[arg(short, long)]
        base_url: Option<String>,
        
        /// Model name
        #[arg(short, long)]
        model: Option<String>,
    },
    
    /// Remove a provider
    Remove {
        /// Provider name
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    setup_logging(cli.verbose, cli.quiet)?;
    
    // Initialize OpenCode core
    initialize().await
        .context("Failed to initialize OpenCode core")?;
    
    // Load configuration
    let config = load_config(&cli).await
        .context("Failed to load configuration")?;
    
    // Create engine
    let engine = Engine::with_config(config).await
        .context("Failed to create OpenCode engine")?;
    
    // Handle shutdown signal
    let shutdown_signal = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    };
    
    // Execute command
    let result = match &cli.command {
        Some(Commands::Tui { agent, system_prompt }) => {
            tui::run_tui(engine, agent, system_prompt.as_deref(), shutdown_signal).await
        }
        Some(Commands::Ask { message, agent, system_prompt, stream }) => {
            commands::ask::execute(engine, message, agent, system_prompt.as_deref(), *stream).await
        }
        Some(Commands::Config { action }) => {
            commands::config::execute(engine, action).await
        }
        Some(Commands::Agent { action }) => {
            commands::agent::execute(engine, action).await
        }
        Some(Commands::Session { action }) => {
            commands::session::execute(engine, action).await
        }
        Some(Commands::Memory { action }) => {
            commands::memory::execute(engine, action).await
        }
        Some(Commands::Provider { action }) => {
            commands::provider::execute(engine, action).await
        }
        None => {
            // No command provided, start TUI by default
            tui::run_tui(engine, "assistant", None, shutdown_signal).await
        }
    };
    
    // Cleanup
    if let Err(e) = engine.shutdown().await {
        log::error!("Error during shutdown: {}", e);
    }
    
    result
}

/// Setup logging based on verbosity flags
fn setup_logging(verbose: bool, quiet: bool) -> Result<()> {
    let level = if quiet {
        log::LevelFilter::Error
    } else if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    
    env_logger::Builder::from_default_env()
        .filter_level(level)
        .init();
    
    Ok(())
}

/// Load configuration from file or defaults
async fn load_config(cli: &Cli) -> Result<Config> {
    let mut config = if let Some(config_path) = &cli.config {
        Config::load_from_file(config_path).await
            .context("Failed to load configuration file")?
    } else {
        Config::load().await
            .context("Failed to load default configuration")?
    };
    
    // Apply CLI overrides
    if let Some(provider) = &cli.provider {
        config.default_provider = Some(provider.clone());
    }
    
    if let Some(model) = &cli.model {
        // Apply model to all providers
        for provider_config in config.providers.values_mut() {
            provider_config.model = Some(model.clone());
        }
    }
    
    // Validate configuration
    config.validate()
        .context("Configuration validation failed")?;
    
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    
    #[test]
    fn test_cli_parsing() {
        // Test basic command parsing
        let cli = Cli::try_parse_from(["opencode", "--help"]);
        assert!(cli.is_err()); // Help should exit
        
        let cli = Cli::try_parse_from(["opencode", "ask", "Hello, world!"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Ask { .. })));
        
        let cli = Cli::try_parse_from(["opencode", "config", "show"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Config { .. })));
    }
    
    #[test]
    fn test_cli_flags() {
        let cli = Cli::try_parse_from(["opencode", "-v", "--provider", "openai"]).unwrap();
        assert!(cli.verbose);
        assert_eq!(cli.provider, Some("openai".to_string()));
        
        let cli = Cli::try_parse_from(["opencode", "-q", "--model", "gpt-4"]).unwrap();
        assert!(cli.quiet);
        assert_eq!(cli.model, Some("gpt-4".to_string()));
    }
    
    #[test]
    fn test_subcommand_parsing() {
        let cli = Cli::try_parse_from([
            "opencode", "ask", "Hello", "--agent", "test", "--stream"
        ]).unwrap();
        
        if let Some(Commands::Ask { message, agent, stream, .. }) = cli.command {
            assert_eq!(message, "Hello");
            assert_eq!(agent, "test");
            assert!(stream);
        } else {
            panic!("Expected Ask command");
        }
    }
}