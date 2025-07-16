//! Code Mesh CLI - AI-powered coding assistant

use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing::info;
use tracing_subscriber;

mod cmd;
// mod tui; // TODO: Implement TUI

#[derive(Parser)]
#[command(name = "code-mesh")]
#[command(about = "AI-powered coding assistant", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Increase verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Run Code Mesh with a message
    Run {
        /// Message to send
        message: Vec<String>,
        
        /// Continue the last session
        #[arg(short, long)]
        continue_session: bool,
        
        /// Session ID to continue
        #[arg(short, long)]
        session: Option<String>,
        
        /// Model to use (provider/model format)
        #[arg(short, long)]
        model: Option<String>,
        
        /// Mode to use (chat, plan, etc.)
        #[arg(long)]
        mode: Option<String>,
    },
    
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    
    /// Initialize a new project
    Init {
        /// Project path
        #[arg(default_value = ".")]
        path: String,
    },
    
    /// Show status and health information
    Status {
        /// Show detailed status
        #[arg(short, long)]
        detailed: bool,
    },
    
    /// Start API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
        
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    
    /// List available models
    Models {
        /// Filter by provider
        #[arg(short, long)]
        provider: Option<String>,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Log in to a provider
    Login,
    
    /// Log out from a provider
    Logout {
        /// Provider to log out from
        provider: String,
    },
    
    /// List authenticated providers
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    
    tracing_subscriber::fmt()
        .with_max_level(match log_level {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        })
        .init();
    
    // Execute command
    match cli.command {
        Commands::Run { 
            message, 
            continue_session, 
            session, 
            model, 
            mode 
        } => {
            info!("Running Code Mesh");
            let result = cmd::run::execute(
                message.join(" "),
                continue_session,
                session,
                model,
                mode,
            ).await;

            if let Err(e) = result {
                let mut ui = cmd::UI::new();
                let cli_error = cmd::CliError::Unknown(e.to_string());
                let exit_code = cmd::error::ErrorHandler::handle_error(&cli_error, &mut ui);
                std::process::exit(exit_code);
            }
        }
        
        Commands::Auth { command } => {
            match command {
                AuthCommands::Login => {
                    info!("Starting authentication");
                    cmd::auth::login().await?;
                }
                AuthCommands::Logout { provider } => {
                    info!("Logging out from {}", provider);
                    cmd::auth::logout(&provider).await?;
                }
                AuthCommands::List => {
                    cmd::auth::list().await?;
                }
            }
        }
        
        Commands::Init { path } => {
            info!("Initializing project at {}", path);
            cmd::init::execute(&path).await?;
        }
        
        Commands::Status { detailed } => {
            cmd::status::execute(detailed).await?;
        }
        
        Commands::Serve { port, host } => {
            info!("Starting server on {}:{}", host, port);
            cmd::serve::execute(&host, port).await?;
        }
        
        Commands::Models { provider } => {
            cmd::models::execute(provider).await?;
        }
    }
    
    Ok(())
}

// Command implementations are in cmd/ module