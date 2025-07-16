//! Error handling for the CLI

use code_mesh_core::Error as CoreError;
use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

/// CLI-specific error types
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("User cancelled operation")]
    Cancelled,

    #[error("Operation timed out")]
    Timeout,

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        CliError::Unknown(err.to_string())
    }
}

impl CliError {
    /// Create a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            CliError::Core(e) => format!("Internal error: {}", e),
            CliError::Config(msg) => format!("Configuration issue: {}", msg),
            CliError::Auth(msg) => format!("Authentication failed: {}", msg),
            CliError::Session(msg) => format!("Session error: {}", msg),
            CliError::Network(msg) => format!("Network error: {}", msg),
            CliError::FileSystem(msg) => format!("File system error: {}", msg),
            CliError::InvalidInput(msg) => format!("Invalid input: {}", msg),
            CliError::Cancelled => "Operation was cancelled by user".to_string(),
            CliError::Timeout => "Operation timed out".to_string(),
            CliError::CommandFailed(msg) => format!("Command failed: {}", msg),
            CliError::Server(msg) => format!("Server error: {}", msg),
            CliError::ModelNotFound(model) => format!("Model '{}' not found", model),
            CliError::ProviderNotConfigured(provider) => {
                format!("Provider '{}' is not configured. Run 'code-mesh auth login' to set up authentication.", provider)
            }
            CliError::Io(e) => format!("File operation failed: {}", e),
            CliError::Json(e) => format!("Data parsing error: {}", e),
            CliError::Http(msg) => format!("HTTP request failed: {}", msg),
            CliError::Unknown(msg) => format!("Unexpected error: {}", msg),
        }
    }

    /// Get suggestion for fixing the error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            CliError::Auth(_) => Some("Try running 'code-mesh auth login' to authenticate".to_string()),
            CliError::Config(_) => Some("Check your configuration file or run 'code-mesh init' to create a new one".to_string()),
            CliError::ModelNotFound(_) => Some("Run 'code-mesh models' to see available models".to_string()),
            CliError::ProviderNotConfigured(_) => Some("Run 'code-mesh auth login' to configure the provider".to_string()),
            CliError::Network(_) => Some("Check your internet connection and try again".to_string()),
            CliError::FileSystem(_) => Some("Check file permissions and available disk space".to_string()),
            CliError::Server(_) => Some("The server may be down. Try again later".to_string()),
            _ => None,
        }
    }

    /// Check if this error should be retried
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CliError::Network(_) | CliError::Http(_) | CliError::Timeout | CliError::Server(_)
        )
    }

    /// Get error code for programmatic handling
    pub fn code(&self) -> &'static str {
        match self {
            CliError::Core(_) => "CORE_ERROR",
            CliError::Config(_) => "CONFIG_ERROR",
            CliError::Auth(_) => "AUTH_ERROR",
            CliError::Session(_) => "SESSION_ERROR",
            CliError::Network(_) => "NETWORK_ERROR",
            CliError::FileSystem(_) => "FILESYSTEM_ERROR",
            CliError::InvalidInput(_) => "INVALID_INPUT",
            CliError::Cancelled => "CANCELLED",
            CliError::Timeout => "TIMEOUT",
            CliError::CommandFailed(_) => "COMMAND_FAILED",
            CliError::Server(_) => "SERVER_ERROR",
            CliError::ModelNotFound(_) => "MODEL_NOT_FOUND",
            CliError::ProviderNotConfigured(_) => "PROVIDER_NOT_CONFIGURED",
            CliError::Io(_) => "IO_ERROR",
            CliError::Json(_) => "JSON_ERROR",
            CliError::Http(_) => "HTTP_ERROR",
            CliError::Unknown(_) => "UNKNOWN_ERROR",
        }
    }
}

/// Helper trait for converting errors to CLI errors
pub trait IntoCliError<T> {
    fn into_cli_error(self) -> Result<T>;
}

impl<T, E> IntoCliError<T> for std::result::Result<T, E>
where
    E: Into<CliError>,
{
    fn into_cli_error(self) -> Result<T> {
        self.map_err(|e| e.into())
    }
}

/// Error handler for the CLI
pub struct ErrorHandler;

impl ErrorHandler {
    pub fn handle_error(error: &CliError, ui: &mut crate::cmd::ui::UI) -> i32 {
        // Print user-friendly error message
        let _ = ui.error(&error.user_message());

        // Print suggestion if available
        if let Some(suggestion) = error.suggestion() {
            let _ = ui.info(&format!("Suggestion: {}", suggestion));
        }

        // Return appropriate exit code
        match error {
            CliError::Cancelled => 130, // Standard exit code for user cancellation
            CliError::InvalidInput(_) => 2, // Standard exit code for invalid arguments
            CliError::Auth(_) => 1,
            CliError::Config(_) => 1,
            CliError::Network(_) => 1,
            CliError::Server(_) => 1,
            _ => 1, // General error
        }
    }
}