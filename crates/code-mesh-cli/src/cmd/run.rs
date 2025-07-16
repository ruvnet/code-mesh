//! Run command implementation

use anyhow::Result;
use code_mesh_core::{
    Session, SessionManager, ProviderRegistry, MessageRole,
    llm::{GenerateOptions, Message, MessageContent},
};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

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
    let storage = code_mesh_core::storage::FileStorage::default()?;
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
        MessageRole::User,
        message.clone()
    ).await?;
    
    // Initialize provider registry
    pb.set_message("Loading providers...");
    let registry = ProviderRegistry::new();
    
    // TODO: Register actual providers
    // registry.register(Box::new(AnthropicProvider::new(...)));
    
    // Parse model selection
    let default_model = String::from("claude-3-sonnet-20240229");
    let model_str = model.as_ref().unwrap_or(&default_model);
    
    let (provider_id, model_id) = if let Some(slash_pos) = model_str.find('/') {
        let (provider, model) = model_str.split_at(slash_pos);
        (provider, &model[1..])
    } else {
        ("anthropic", model_str.as_str())
    };
    
    // Get provider and model
    let provider = registry.get(provider_id)
        .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_id))?;
    
    pb.set_message(format!("Connecting to {}...", provider.name()));
    let model = provider.get_model(model_id).await?;
    
    // Generate response
    pb.set_message("Generating response...");
    let messages = session.to_llm_messages();
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
        MessageRole::Assistant,
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