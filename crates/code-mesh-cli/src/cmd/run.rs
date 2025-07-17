//! Run command implementation

use anyhow::Result;
use code_mesh_core::{
    session::{Session, SessionManager, MessageRole as SessionMessageRole}, 
    llm::{ProviderRegistry, GenerateOptions, Message, MessageContent, MessageRole},
    auth::AuthStorage,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use dialoguer::{Select, Input, Password, Confirm};
use console::style;
use super::oauth::{AnthropicOAuth, PkceChallenge};

pub async fn execute(
    message: String,
    continue_session: bool,
    session_id: Option<String>,
    model: Option<String>,
    mode: Option<String>,
) -> Result<()> {
    // Initialize progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Initializing Code Mesh...");
    
    // Initialize storage and session manager
    let storage = code_mesh_core::storage::FileStorage::default()
        .map_err(|e| anyhow::anyhow!("Failed to initialize storage: {}", e))?;
    let mut session_manager = SessionManager::new(Box::new(storage));
    
    // Get or create session
    let mut session = if let Some(id) = session_id {
        pb.set_message("Loading session...");
        session_manager.get_session(&id).await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?
    } else if continue_session {
        pb.set_message("Continuing last session...");
        session_manager.continue_last_session().await?
            .ok_or_else(|| anyhow::anyhow!("No previous session found"))?
    } else {
        pb.set_message("Creating new session...");
        session_manager.create_session().await?
    };
    
    // Add user message
    let user_msg = session_manager.add_message(
        &session.id,
        SessionMessageRole::User,
        message.clone()
    ).await?;
    
    // Initialize provider registry
    pb.set_message("Loading providers...");
    let auth_storage = code_mesh_core::auth::FileAuthStorage::default_with_result()
        .map_err(|e| anyhow::anyhow!("Failed to initialize auth storage: {}", e))?;
    let auth_storage_arc = std::sync::Arc::new(auth_storage);
    let mut registry = ProviderRegistry::new(auth_storage_arc.clone());
    
    // Discover providers from storage (this will create providers with stored credentials)
    pb.set_message("Discovering providers from storage...");
    registry.discover_from_storage().await
        .map_err(|e| anyhow::anyhow!("Failed to discover providers: {}", e))?;
    
    // Also discover from environment variables
    registry.discover_from_env().await
        .map_err(|e| anyhow::anyhow!("Failed to discover providers from env: {}", e))?;
    
    // Initialize all discovered providers
    pb.set_message("Initializing providers...");
    registry.initialize_all().await
        .map_err(|e| anyhow::anyhow!("Failed to initialize providers: {}", e))?;
    
    // Parse model selection
    let default_model = String::from("claude-3-sonnet-20240229");
    let model_str = model.as_ref().unwrap_or(&default_model);
    
    let (provider_id, model_id) = if let Some(slash_pos) = model_str.find('/') {
        let (provider, model) = model_str.split_at(slash_pos);
        (provider, &model[1..])
    } else {
        ("anthropic", model_str.as_str())
    };
    
    // Get the provider from registry
    let provider = match registry.get(provider_id).await {
        Some(provider) => provider,
        None => {
            // No provider available, prompt for authentication
            pb.finish_and_clear();
            prompt_for_authentication(provider_id).await?;
            
            // Retry after authentication by re-discovering providers
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap());
            pb.set_message("Reconnecting...");
            
            // Re-discover providers from storage
            registry.discover_from_storage().await
                .map_err(|e| anyhow::anyhow!("Failed to re-discover providers: {}", e))?;
            
            registry.get(provider_id).await
                .ok_or_else(|| anyhow::anyhow!("Authentication failed for provider: {}", provider_id))?
        }
    };
    
    pb.set_message(format!("Connecting to {}...", provider.name()));
    let model = provider.get_model(model_id).await?;
    
    // Generate response
    pb.set_message("Generating response...");
    let messages = session.messages.iter()
        .map(|msg| Message {
            role: match msg.role {
                SessionMessageRole::System => MessageRole::System,
                SessionMessageRole::User => MessageRole::User,
                SessionMessageRole::Assistant => MessageRole::Assistant,
                SessionMessageRole::Tool => MessageRole::Tool,
            },
            content: MessageContent::Text(msg.content.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();
    let options = GenerateOptions {
        temperature: Some(0.7),
        max_tokens: Some(4096),
        ..Default::default()
    };
    
    let result = model.generate(messages, options).await?;
    
    pb.finish_and_clear();
    
    // Add assistant message
    let assistant_msg = session_manager.add_message(
        &session.id,
        SessionMessageRole::Assistant,
        result.content.clone()
    ).await?;
    
    // Print response
    println!("\n{}\n", result.content);
    
    // Print session info
    eprintln!("\n📍 Session: {}", &session.id[session.id.len()-8..]);
    eprintln!("💡 Model: {}/{}", provider_id, model_id);
    eprintln!("🔢 Tokens: {} in, {} out", 
        result.usage.prompt_tokens, 
        result.usage.completion_tokens
    );
    
    Ok(())
}

async fn prompt_for_authentication(provider_id: &str) -> Result<()> {
    println!();
    println!("{}", style("🔐 Authentication Required").bold().cyan());
    println!("{}", style(format!("No valid authentication found for provider: {}", provider_id)).yellow());
    println!();
    
    // Show authentication options
    let auth_options = vec![
        "API Key",
        "Claude OAuth",
        "GitHub OAuth",
        "Configure manually",
        "Exit"
    ];
    
    let selection = Select::new()
        .with_prompt("How would you like to authenticate?")
        .items(&auth_options)
        .default(0)
        .interact()?;
    
    match selection {
        0 => {
            // API Key authentication
            println!();
            println!("{}", style("API Key Authentication").bold().green());
            println!("Please enter your API key for {}:", provider_id);
            
            let api_key = Password::new()
                .with_prompt("API Key")
                .with_confirmation("Confirm API Key", "Keys don't match")
                .interact()?;
            
            if api_key.is_empty() {
                return Err(anyhow::anyhow!("API key cannot be empty"));
            }
            
            // Save API key (implement actual storage)
            save_api_key(provider_id, &api_key).await?;
            
            println!("{}", style("✅ API key saved successfully!").green());
        }
        1 => {
            // Claude OAuth
            println!();
            println!("{}", style("Claude OAuth Authentication").bold().green());
            
            let method = Select::new()
                .with_prompt("Choose OAuth method")
                .items(&["Claude Pro/Max", "Console (API Key Creation)"])
                .default(0)
                .interact()?;
            
            match method {
                0 => handle_anthropic_oauth("max").await?,
                1 => handle_anthropic_oauth("console").await?,
                _ => unreachable!(),
            }
        }
        2 => {
            // GitHub OAuth
            println!();
            println!("{}", style("GitHub OAuth Authentication").bold().green());
            println!("Opening GitHub OAuth in your browser...");
            
            // Implement OAuth flow
            handle_oauth_flow("github").await?;
        }
        3 => {
            // Manual configuration
            println!();
            println!("{}", style("Manual Configuration").bold().green());
            println!("Please configure authentication manually in your config file.");
            println!("Config location: ~/.code-mesh/config.toml");
            
            let open_config = Confirm::new()
                .with_prompt("Would you like to open the config file?")
                .interact()?;
            
            if open_config {
                open_config_file().await?;
            }
        }
        4 => {
            // Exit
            println!("Authentication cancelled.");
            std::process::exit(0);
        }
        _ => unreachable!(),
    }
    
    Ok(())
}

async fn save_api_key(provider_id: &str, api_key: &str) -> Result<()> {
    use code_mesh_core::auth::AuthCredentials;
    
    let auth_storage = code_mesh_core::auth::FileAuthStorage::default_with_result()?;
    let credentials = AuthCredentials::api_key(api_key.to_string());
    
    auth_storage.set(provider_id, credentials).await?;
    
    println!("API key saved for provider: {}", provider_id);
    println!("Key length: {}", api_key.len());
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
        save_api_key("anthropic", &api_key_response.raw_key).await?;
        
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

async fn handle_oauth_flow(provider: &str) -> Result<()> {
    // TODO: Implement OAuth flow
    match provider {
        "claude" => {
            println!("Opening Claude OAuth...");
            // Would open: https://console.anthropic.com/oauth/authorize?...
        }
        "github" => {
            println!("Opening GitHub OAuth...");
            // Would open: https://github.com/login/oauth/authorize?...
        }
        _ => return Err(anyhow::anyhow!("Unsupported OAuth provider: {}", provider)),
    }
    
    println!("Please complete the authentication in your browser.");
    println!("Press Enter when you've completed the authentication...");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    println!("{}", style("✅ OAuth authentication completed!").green());
    Ok(())
}

async fn open_config_file() -> Result<()> {
    let config_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".code-mesh")
        .join("config.toml");
    
    // Create config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Create default config if it doesn't exist
    if !config_path.exists() {
        let default_config = r#"# Code Mesh Configuration

[providers.anthropic]
# api_key = "your-api-key-here"
# base_url = "https://api.anthropic.com"

[providers.openai]
# api_key = "your-api-key-here"
# base_url = "https://api.openai.com"

[providers.github]
# token = "your-github-token-here"
# base_url = "https://api.github.com"

[general]
default_provider = "anthropic"
default_model = "claude-3-sonnet-20240229"
"#;
        std::fs::write(&config_path, default_config)?;
    }
    
    // Try to open the config file
    match open::that(&config_path) {
        Ok(_) => println!("Config file opened: {}", config_path.display()),
        Err(e) => {
            println!("Could not open config file automatically: {}", e);
            println!("Please manually edit: {}", config_path.display());
        }
    }
    
    Ok(())
}