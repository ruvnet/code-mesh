//! Ask command implementation

use anyhow::Result;
use opencode_core::{Engine, agent::AgentConfig};
use crate::cli::Output;

/// Execute the ask command
pub async fn execute(
    engine: Engine,
    message: &str,
    agent_name: &str,
    system_prompt: Option<&str>,
    stream: bool,
) -> Result<()> {
    let mut output = Output::new();
    
    // Create or get agent
    let agent = if let Some(existing_agent) = engine.agents().get_agent_by_name(agent_name).await {
        existing_agent
    } else {
        let mut config = AgentConfig::default();
        config.name = agent_name.to_string();
        config.streaming = stream;
        
        if let Some(prompt) = system_prompt {
            config.system_prompt = Some(prompt.to_string());
        }
        
        let provider = engine.get_default_provider().await?;
        engine.agents().create_agent(agent_name, provider, Some(config)).await?
    };
    
    // Show thinking indicator
    let spinner = output.spinner("Thinking...");
    
    // Send message to agent
    let response = agent.send_message(message).await?;
    
    // Stop spinner
    spinner.finish_and_clear();
    
    // Display response
    output.println(&response.content)?;
    
    // Show usage information if available
    if let Some(usage) = response.usage {
        output.println("")?;
        output.info(&format!(
            "Tokens used: {} (prompt: {}, completion: {})",
            usage.total_tokens,
            usage.prompt_tokens,
            usage.completion_tokens
        ))?;
    }
    
    // Show finish reason if available
    if let Some(finish_reason) = response.finish_reason {
        output.info(&format!("Finish reason: {}", finish_reason))?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_core::Config;
    
    #[tokio::test]
    async fn test_ask_command() {
        // This would require a mock engine for proper testing
        // For now, we just test that the function signature is correct
    }
}