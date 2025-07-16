//! Models command implementation

use crate::cmd::{CliError, Result, UI, Config};
use std::collections::HashMap;

/// Execute the models command
pub async fn execute(provider_filter: Option<String>) -> Result<()> {
    let mut ui = UI::new();
    
    ui.info("Available AI models")?;
    ui.println("")?;

    // Get available models
    let models = get_available_models().await?;
    
    // Filter by provider if specified
    let filtered_models: HashMap<String, Vec<ModelInfo>> = if let Some(ref provider) = provider_filter {
        models
            .into_iter()
            .filter(|(p, _)| p == provider)
            .collect()
    } else {
        models
    };

    if filtered_models.is_empty() {
        ui.warning("No models found")?;
        if let Some(provider) = provider_filter {
            ui.info(&format!("Provider '{}' not found or has no models", provider))?;
        }
        return Ok(());
    }

    // Display models grouped by provider
    for (provider_name, provider_models) in filtered_models {
        ui.println(&console::Style::new().bold().apply_to(&provider_name).to_string())?;
        ui.println(&"─".repeat(provider_name.len()))?;

        if provider_models.is_empty() {
            ui.dim("  No models available")?;
        } else {
            let mut table = crate::cmd::ui::Table::new(vec![
                "Model".to_string(),
                "Type".to_string(),
                "Context".to_string(),
                "Status".to_string(),
            ]);

            for model in provider_models {
                let context = if model.context_length > 1000000 {
                    format!("{}M", model.context_length / 1000000)
                } else if model.context_length > 1000 {
                    format!("{}K", model.context_length / 1000)
                } else {
                    model.context_length.to_string()
                };

                let status = if model.available {
                    "✓ Available".to_string()
                } else {
                    "✗ Unavailable".to_string()
                };

                table.add_row(vec![
                    model.name,
                    model.model_type,
                    context,
                    status,
                ]);
            }

            table.print(&mut ui)?;
        }
        ui.println("")?;
    }

    // Show authentication status
    show_auth_status(&mut ui).await?;

    Ok(())
}

/// Model information
#[derive(Debug, Clone)]
struct ModelInfo {
    id: String,
    name: String,
    model_type: String,
    context_length: u32,
    available: bool,
    description: Option<String>,
    supports_streaming: bool,
    supports_function_calling: bool,
}

/// Get available models from all providers
async fn get_available_models() -> Result<HashMap<String, Vec<ModelInfo>>> {
    let mut models = HashMap::new();

    // Add Anthropic models
    models.insert("Anthropic".to_string(), get_anthropic_models().await?);
    
    // Add OpenAI models
    models.insert("OpenAI".to_string(), get_openai_models().await?);
    
    // Add Google models
    models.insert("Google".to_string(), get_google_models().await?);
    
    // Add GitHub Copilot models
    models.insert("GitHub".to_string(), get_github_models().await?);

    Ok(models)
}

/// Get Anthropic models
async fn get_anthropic_models() -> Result<Vec<ModelInfo>> {
    let models = vec![
        ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            model_type: "Chat".to_string(),
            context_length: 200000,
            available: is_provider_available("anthropic").await,
            description: Some("Anthropic's most intelligent model".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
        ModelInfo {
            id: "claude-3-opus-20240229".to_string(),
            name: "Claude 3 Opus".to_string(),
            model_type: "Chat".to_string(),
            context_length: 200000,
            available: is_provider_available("anthropic").await,
            description: Some("Anthropic's most powerful model".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
        ModelInfo {
            id: "claude-3-sonnet-20240229".to_string(),
            name: "Claude 3 Sonnet".to_string(),
            model_type: "Chat".to_string(),
            context_length: 200000,
            available: is_provider_available("anthropic").await,
            description: Some("Balanced intelligence and speed".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
        ModelInfo {
            id: "claude-3-haiku-20240307".to_string(),
            name: "Claude 3 Haiku".to_string(),
            model_type: "Chat".to_string(),
            context_length: 200000,
            available: is_provider_available("anthropic").await,
            description: Some("Fast and efficient responses".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
    ];

    Ok(models)
}

/// Get OpenAI models
async fn get_openai_models() -> Result<Vec<ModelInfo>> {
    let models = vec![
        ModelInfo {
            id: "gpt-4-turbo-preview".to_string(),
            name: "GPT-4 Turbo".to_string(),
            model_type: "Chat".to_string(),
            context_length: 128000,
            available: is_provider_available("openai").await,
            description: Some("OpenAI's most capable model".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
        ModelInfo {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            model_type: "Chat".to_string(),
            context_length: 8192,
            available: is_provider_available("openai").await,
            description: Some("High-quality reasoning and coding".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
        ModelInfo {
            id: "gpt-3.5-turbo".to_string(),
            name: "GPT-3.5 Turbo".to_string(),
            model_type: "Chat".to_string(),
            context_length: 4096,
            available: is_provider_available("openai").await,
            description: Some("Fast and efficient for most tasks".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
    ];

    Ok(models)
}

/// Get Google models
async fn get_google_models() -> Result<Vec<ModelInfo>> {
    let models = vec![
        ModelInfo {
            id: "gemini-pro".to_string(),
            name: "Gemini Pro".to_string(),
            model_type: "Chat".to_string(),
            context_length: 32768,
            available: is_provider_available("google").await,
            description: Some("Google's most capable model".to_string()),
            supports_streaming: true,
            supports_function_calling: true,
        },
        ModelInfo {
            id: "gemini-pro-vision".to_string(),
            name: "Gemini Pro Vision".to_string(),
            model_type: "Multimodal".to_string(),
            context_length: 32768,
            available: is_provider_available("google").await,
            description: Some("Supports text and image input".to_string()),
            supports_streaming: true,
            supports_function_calling: false,
        },
    ];

    Ok(models)
}

/// Get GitHub Copilot models
async fn get_github_models() -> Result<Vec<ModelInfo>> {
    let models = vec![
        ModelInfo {
            id: "gpt-4".to_string(),
            name: "GitHub Copilot Chat".to_string(),
            model_type: "Chat".to_string(),
            context_length: 8192,
            available: is_provider_available("github").await,
            description: Some("GitHub's AI pair programmer".to_string()),
            supports_streaming: true,
            supports_function_calling: false,
        },
    ];

    Ok(models)
}

/// Check if a provider is available (has authentication)
async fn is_provider_available(provider: &str) -> bool {
    // Check for environment variables first
    match provider {
        "anthropic" => std::env::var("ANTHROPIC_API_KEY").is_ok(),
        "openai" => std::env::var("OPENAI_API_KEY").is_ok(),
        "google" => std::env::var("GOOGLE_API_KEY").is_ok(),
        "github" => std::env::var("GITHUB_TOKEN").is_ok(),
        _ => false,
    }
    // TODO: Also check stored credentials using AuthManager
}

/// Show authentication status
async fn show_auth_status(ui: &mut UI) -> Result<()> {
    ui.info("Authentication Status")?;
    ui.println("")?;

    let providers = vec![
        ("Anthropic", "ANTHROPIC_API_KEY"),
        ("OpenAI", "OPENAI_API_KEY"),
        ("Google", "GOOGLE_API_KEY"),
        ("GitHub", "GITHUB_TOKEN"),
    ];

    let mut table = crate::cmd::ui::Table::new(vec![
        "Provider".to_string(),
        "Status".to_string(),
        "Source".to_string(),
    ]);

    for (provider_name, env_var) in providers {
        let (status, source) = if std::env::var(env_var).is_ok() {
            ("✓ Authenticated".to_string(), "Environment".to_string())
        } else {
            // TODO: Check stored credentials
            ("✗ Not authenticated".to_string(), "None".to_string())
        };

        table.add_row(vec![provider_name.to_string(), status, source]);
    }

    table.print(&mut *ui)?;

    ui.println("")?;
    ui.dim("Run 'code-mesh auth login' to set up authentication")?;

    Ok(())
}