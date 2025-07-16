//! Provider command implementation

use anyhow::Result;
use opencode_core::Engine;
use crate::{ProviderAction, cli::Output};

/// Execute the provider command
pub async fn execute(engine: Engine, action: &ProviderAction) -> Result<()> {
    let mut output = Output::new();
    
    match action {
        ProviderAction::List => {
            let providers = engine.list_providers().await;
            output.table_header(&["Name", "Type", "Status"])?;
            
            for provider_name in providers {
                if let Ok(provider) = engine.get_provider(&provider_name).await {
                    let status = if provider.is_available().await {
                        "Available"
                    } else {
                        "Unavailable"
                    };
                    
                    output.table_row(&[
                        &provider_name,
                        &provider.provider_type().to_string(),
                        status,
                    ])?;
                }
            }
        }
        ProviderAction::Test { name } => {
            match engine.get_provider(name).await {
                Ok(provider) => {
                    output.info(&format!("Testing provider: {}", name))?;
                    
                    let spinner = output.spinner("Validating configuration...");
                    let result = provider.validate_config().await;
                    spinner.finish_and_clear();
                    
                    match result {
                        Ok(()) => {
                            output.success(&format!("Provider {} is working correctly", name))?;
                        }
                        Err(e) => {
                            output.error(&format!("Provider {} test failed: {}", name, e))?;
                        }
                    }
                }
                Err(e) => {
                    output.error(&format!("Provider {} not found: {}", name, e))?;
                }
            }
        }
        ProviderAction::Models { name } => {
            match engine.get_provider(name).await {
                Ok(provider) => {
                    output.info(&format!("Getting models for provider: {}", name))?;
                    
                    let spinner = output.spinner("Fetching models...");
                    let models = provider.get_models().await;
                    spinner.finish_and_clear();
                    
                    match models {
                        Ok(model_list) => {
                            output.table_header(&["Model"])?;
                            for model in model_list {
                                output.table_row(&[&model])?;
                            }
                        }
                        Err(e) => {
                            output.error(&format!("Failed to fetch models: {}", e))?;
                        }
                    }
                }
                Err(e) => {
                    output.error(&format!("Provider {} not found: {}", name, e))?;
                }
            }
        }
        ProviderAction::Add { name, provider_type, api_key, base_url, model } => {
            use opencode_core::config::ProviderConfig;
            use std::collections::HashMap;
            
            let config = ProviderConfig {
                provider_type: provider_type.clone(),
                api_key: api_key.clone(),
                base_url: base_url.clone(),
                model: model.clone(),
                settings: HashMap::new(),
            };
            
            let provider_type_enum = opencode_core::providers::ProviderType::from(provider_type.as_str());
            
            match opencode_core::providers::ProviderFactory::create_provider(provider_type_enum, &config) {
                Ok(provider) => {
                    engine.add_provider(name.clone(), provider).await;
                    output.success(&format!("Added provider: {}", name))?;
                }
                Err(e) => {
                    output.error(&format!("Failed to add provider: {}", e))?;
                }
            }
        }
        ProviderAction::Remove { name } => {
            if engine.remove_provider(name).await {
                output.success(&format!("Removed provider: {}", name))?;
            } else {
                output.error(&format!("Provider not found: {}", name))?;
            }
        }
    }
    
    Ok(())
}