//! Auth command implementation

use anyhow::Result;
use dialoguer::{Select, Input, Password, Confirm};
use console::style;
use std::collections::HashMap;
use super::oauth::{AnthropicOAuth, PkceChallenge};
use open;
use code_mesh_core::auth::AuthStorage;

pub async fn login() -> Result<()> {
    println!();
    println!("{}", style("🔐 Code Mesh Authentication").bold().cyan());
    println!();
    
    // Show provider options
    let providers = vec![
        "Anthropic (Claude)",
        "OpenAI",
        "GitHub Copilot",
        "Google (Gemini)",
        "Azure OpenAI",
        "Local/Self-hosted"
    ];
    
    let selection = Select::new()
        .with_prompt("Select authentication provider")
        .items(&providers)
        .default(0)
        .interact()?;
    
    let provider_id = match selection {
        0 => "anthropic",
        1 => "openai", 
        2 => "github",
        3 => "google",
        4 => "azure",
        5 => "local",
        _ => unreachable!(),
    };
    
    authenticate_provider(provider_id).await?;
    
    Ok(())
}

pub async fn logout(provider: &str) -> Result<()> {
    println!();
    println!("{}", style(format!("🚪 Logging out from {}", provider)).bold().yellow());
    
    let auth_storage = code_mesh_core::auth::FileAuthStorage::default_with_result()?;
    
    // Check if provider is authenticated
    match auth_storage.get(provider).await? {
        Some(_) => {
            let confirm = Confirm::new()
                .with_prompt(&format!("Are you sure you want to logout from {}?", provider))
                .interact()?;
            
            if confirm {
                auth_storage.remove(provider).await?;
                println!("{}", style(format!("✅ Successfully logged out from {}", provider)).green());
            } else {
                println!("Logout cancelled.");
            }
        }
        None => {
            println!("{}", style(format!("❌ Not authenticated with {}", provider)).red());
        }
    }
    
    Ok(())
}

pub async fn list() -> Result<()> {
    println!();
    println!("{}", style("🔑 Authentication Status").bold().cyan());
    println!();
    
    let auth_storage = code_mesh_core::auth::FileAuthStorage::default_with_result()?;
    let stored_providers = auth_storage.list().await?;
    
    let all_providers = vec![
        ("anthropic", "Claude"),
        ("openai", "OpenAI"),
        ("github", "GitHub Copilot"),
        ("google", "Google Gemini"),
        ("azure", "Azure OpenAI"),
        ("local", "Local/Self-hosted"),
    ];
    
    for (id, name) in all_providers {
        let status = if stored_providers.contains(&id.to_string()) {
            // Check if credentials are expired
            match auth_storage.get(id).await? {
                Some(credentials) => {
                    if credentials.is_expired() {
                        "🔄 Expired (needs refresh)"
                    } else {
                        match credentials {
                            code_mesh_core::auth::AuthCredentials::ApiKey { .. } => "✅ Authenticated (API Key)",
                            code_mesh_core::auth::AuthCredentials::OAuth { .. } => "✅ Authenticated (OAuth)",
                            code_mesh_core::auth::AuthCredentials::Custom { .. } => "✅ Authenticated (Custom)",
                        }
                    }
                }
                None => "❌ Not authenticated",
            }
        } else {
            "❌ Not authenticated"
        };
        
        println!("{:<20} {:<15} {}", id, name, status);
    }
    
    println!();
    
    // Show storage location
    let storage_path = dirs::home_dir()
        .map(|home| home.join(".code-mesh").join("auth.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("~/.code-mesh/auth.json"));
    
    println!("{}", style(format!("📁 Storage: {}", storage_path.display())).dim());
    println!("{}", style("💡 Use 'code-mesh auth login' to authenticate with a provider").dim());
    
    Ok(())
}

async fn authenticate_provider(provider_id: &str) -> Result<()> {
    println!();
    println!("{}", style(format!("Setting up authentication for {}", provider_id)).bold().green());
    
    match provider_id {
        "anthropic" => {
            println!();
            println!("Choose authentication method for Anthropic Claude:");
            
            let auth_method = Select::new()
                .with_prompt("Authentication method")
                .items(&[
                    "Claude Pro/Max OAuth",
                    "Console OAuth (API Key Creation)",
                    "Manual API Key"
                ])
                .default(0)
                .interact()?;
            
            match auth_method {
                0 => {
                    // Claude Pro/Max OAuth
                    println!();
                    println!("{}", style("Claude Pro/Max OAuth").bold().green());
                    handle_anthropic_oauth("max").await?;
                }
                1 => {
                    // Console OAuth with API key creation
                    println!();
                    println!("{}", style("Console OAuth (API Key Creation)").bold().green());
                    handle_anthropic_oauth("console").await?;
                }
                2 => {
                    // Manual API key
                    println!();
                    println!("To authenticate with Anthropic Claude:");
                    println!("1. Visit: https://console.anthropic.com/");
                    println!("2. Create an API key");
                    println!("3. Enter the API key below");
                    println!();
                    
                    let api_key = Password::new()
                        .with_prompt("Claude API Key")
                        .with_confirmation("Confirm API Key", "Keys don't match")
                        .interact()?;
                    
                    save_credentials(provider_id, &[("api_key", &api_key)]).await?;
                }
                _ => unreachable!(),
            }
        }
        "openai" => {
            println!();
            println!("To authenticate with OpenAI:");
            println!("1. Visit: https://platform.openai.com/api-keys");
            println!("2. Create an API key");
            println!("3. Enter the API key below");
            println!();
            
            let api_key = Password::new()
                .with_prompt("OpenAI API Key")
                .with_confirmation("Confirm API Key", "Keys don't match")
                .interact()?;
            
            save_credentials(provider_id, &[("api_key", &api_key)]).await?;
        }
        "github" => {
            println!();
            println!("To authenticate with GitHub Copilot:");
            println!("1. Visit: https://github.com/settings/tokens");
            println!("2. Create a personal access token with 'copilot' scope");
            println!("3. Enter the token below");
            println!();
            
            let token = Password::new()
                .with_prompt("GitHub Token")
                .with_confirmation("Confirm Token", "Tokens don't match")
                .interact()?;
            
            save_credentials(provider_id, &[("token", &token)]).await?;
        }
        "google" => {
            println!();
            println!("To authenticate with Google Gemini:");
            println!("1. Visit: https://makersuite.google.com/app/apikey");
            println!("2. Create an API key");
            println!("3. Enter the API key below");
            println!();
            
            let api_key = Password::new()
                .with_prompt("Gemini API Key")
                .with_confirmation("Confirm API Key", "Keys don't match")
                .interact()?;
            
            save_credentials(provider_id, &[("api_key", &api_key)]).await?;
        }
        "azure" => {
            println!();
            println!("To authenticate with Azure OpenAI:");
            println!("1. Get your endpoint URL from Azure portal");
            println!("2. Get your API key from Azure portal");
            println!("3. Enter the details below");
            println!();
            
            let endpoint: String = Input::new()
                .with_prompt("Azure OpenAI Endpoint")
                .interact_text()?;
            
            let api_key = Password::new()
                .with_prompt("Azure API Key")
                .with_confirmation("Confirm API Key", "Keys don't match")
                .interact()?;
            
            save_credentials(provider_id, &[("endpoint", &endpoint), ("api_key", &api_key)]).await?;
        }
        "local" => {
            println!();
            println!("To configure local/self-hosted provider:");
            println!("1. Enter the base URL for your local service");
            println!("2. Enter any required API key (if needed)");
            println!();
            
            let base_url: String = Input::new()
                .with_prompt("Base URL")
                .default("http://localhost:8080".to_string())
                .interact_text()?;
            
            let needs_auth = Confirm::new()
                .with_prompt("Does your local service require authentication?")
                .interact()?;
            
            if needs_auth {
                let api_key = Password::new()
                    .with_prompt("API Key")
                    .interact()?;
                
                save_credentials(provider_id, &[("base_url", &base_url), ("api_key", &api_key)]).await?;
            } else {
                save_credentials(provider_id, &[("base_url", &base_url)]).await?;
            }
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported provider: {}", provider_id));
        }
    }
    
    println!();
    println!("{}", style(format!("✅ Authentication configured for {}", provider_id)).green());
    println!("{}", style("💡 You can now use this provider with 'code-mesh run'").dim());
    
    Ok(())
}

async fn save_credentials(provider_id: &str, credentials: &[(&str, &str)]) -> Result<()> {
    use code_mesh_core::auth::AuthCredentials;
    
    println!();
    println!("{}", style("Saving credentials...").dim());
    
    for (key, value) in credentials {
        println!("  {}: {} characters", key, value.len());
    }
    
    let auth_storage = code_mesh_core::auth::FileAuthStorage::default_with_result()?;
    
    // For now, assume the first credential is the API key
    if let Some((key, value)) = credentials.first() {
        let auth_credentials = if key == &"api_key" {
            AuthCredentials::api_key(value.to_string())
        } else {
            // Create a custom credential
            let mut data = std::collections::HashMap::new();
            for (k, v) in credentials {
                data.insert(k.to_string(), serde_json::Value::String(v.to_string()));
            }
            AuthCredentials::Custom { data }
        };
        
        auth_storage.set(provider_id, auth_credentials).await?;
    }
    
    println!("{}", style("✅ Credentials saved securely").green());
    
    Ok(())
}

async fn handle_anthropic_oauth(mode: &str) -> Result<()> {
    println!();
    println!("{}", style(format!("Setting up Claude OAuth ({})", mode)).bold().green());
    
    // Generate PKCE challenge
    let pkce = PkceChallenge::new();
    let oauth = AnthropicOAuth::new();
    
    // Generate authorization URL
    let auth_url = match mode {
        "max" => oauth.authorize_url_max(&pkce)?,
        "console" => oauth.authorize_url_console(&pkce)?,
        _ => return Err(anyhow::anyhow!("Invalid OAuth mode: {}", mode)),
    };
    
    println!("Opening browser to complete OAuth flow...");
    println!("URL: {}", auth_url);
    
    // Try to open browser
    match open::that(&auth_url) {
        Ok(_) => println!("{}", style("✅ Browser opened successfully").green()),
        Err(e) => {
            println!("{}", style("⚠️  Could not open browser automatically").yellow());
            println!("Please manually open the URL above in your browser.");
            println!("Error: {}", e);
        }
    }
    
    println!();
    println!("After authorizing in your browser, you'll be redirected to a page with an authorization code.");
    println!("Please copy the entire code (including any # suffix) and paste it here:");
    
    let auth_code = Input::<String>::new()
        .with_prompt("Authorization code")
        .interact_text()?;
    
    if auth_code.trim().is_empty() {
        return Err(anyhow::anyhow!("Authorization code cannot be empty"));
    }
    
    println!();
    println!("{}", style("Exchanging code for tokens...").dim());
    
    // Exchange code for tokens
    let token_response = oauth.exchange_code(&auth_code, &pkce.verifier).await?;
    
    if mode == "console" {
        println!("{}", style("Creating API key...").dim());
        
        // Create API key using access token
        let api_key_response = oauth.create_api_key(&token_response.access_token).await?;
        
        // Save API key
        save_credentials("anthropic", &[("api_key", &api_key_response.raw_key)]).await?;
        
        println!("{}", style("✅ API key created and saved successfully!").green());
        println!("Key ID: {}", api_key_response.key_id);
    } else {
        // For max mode, save OAuth tokens
        save_oauth_tokens("anthropic", &token_response).await?;
        
        println!("{}", style("✅ OAuth tokens saved successfully!").green());
    }
    
    Ok(())
}

async fn save_oauth_tokens(provider_id: &str, tokens: &super::oauth::TokenResponse) -> Result<()> {
    use code_mesh_core::auth::AuthCredentials;
    
    let auth_storage = code_mesh_core::auth::FileAuthStorage::default_with_result()?;
    
    // Calculate expiration time
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + tokens.expires_in;
    
    let credentials = AuthCredentials::oauth(
        tokens.access_token.clone(),
        Some(tokens.refresh_token.clone()),
        Some(expires_at),
    );
    
    auth_storage.set(provider_id, credentials).await?;
    
    println!("OAuth tokens saved for provider: {}", provider_id);
    println!("Access token length: {}", tokens.access_token.len());
    println!("Refresh token length: {}", tokens.refresh_token.len());
    println!("Expires in: {} seconds", tokens.expires_in);
    Ok(())
}