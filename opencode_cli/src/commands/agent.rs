//! Agent command implementation

use anyhow::Result;
use opencode_core::Engine;
use crate::{AgentAction, cli::Output};

/// Execute the agent command
pub async fn execute(engine: Engine, action: &AgentAction) -> Result<()> {
    let mut output = Output::new();
    
    match action {
        AgentAction::List => {
            let agents = engine.agents().list_agents().await;
            output.table_header(&["Name", "State", "Messages", "Tokens"])?;
            
            for agent in agents {
                let stats = agent.get_stats().await;
                output.table_row(&[
                    &agent.name,
                    &format!("{:?}", agent.state),
                    &stats.messages_processed.to_string(),
                    &stats.tokens_consumed.to_string(),
                ])?;
            }
        }
        AgentAction::Create { name, provider, system_prompt } => {
            let prov = if let Some(provider_name) = provider {
                engine.get_provider(provider_name).await?
            } else {
                engine.get_default_provider().await?
            };
            
            let mut config = opencode_core::agent::AgentConfig::default();
            config.name = name.clone();
            if let Some(prompt) = system_prompt {
                config.system_prompt = Some(prompt.clone());
            }
            
            let agent = engine.agents().create_agent(name, prov, Some(config)).await?;
            output.success(&format!("Created agent: {}", agent.name))?;
        }
        AgentAction::Remove { name } => {
            if let Some(agent) = engine.agents().get_agent_by_name(name).await {
                engine.agents().remove_agent(agent.id).await;
                output.success(&format!("Removed agent: {}", name))?;
            } else {
                output.error(&format!("Agent not found: {}", name))?;
            }
        }
        AgentAction::Stats { name } => {
            if let Some(agent) = engine.agents().get_agent_by_name(name).await {
                let stats = agent.get_stats().await;
                output.json(&serde_json::to_value(stats)?)?;
            } else {
                output.error(&format!("Agent not found: {}", name))?;
            }
        }
        AgentAction::Reset { name } => {
            if let Some(agent) = engine.agents().get_agent_by_name(name).await {
                agent.reset().await;
                output.success(&format!("Reset agent: {}", name))?;
            } else {
                output.error(&format!("Agent not found: {}", name))?;
            }
        }
    }
    
    Ok(())
}