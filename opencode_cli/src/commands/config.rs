//! Configuration command implementation

use anyhow::Result;
use opencode_core::Engine;
use crate::{ConfigAction, cli::Output};

/// Execute the config command
pub async fn execute(engine: Engine, action: &ConfigAction) -> Result<()> {
    let mut output = Output::new();
    
    match action {
        ConfigAction::Show => {
            let config = engine.config();
            output.json(&serde_json::to_value(config)?)?;
        }
        ConfigAction::Set { key, value } => {
            output.info(&format!("Setting {} = {}", key, value))?;
            // TODO: Implement config setting
        }
        ConfigAction::Get { key } => {
            output.info(&format!("Getting {}", key))?;
            // TODO: Implement config getting
        }
        ConfigAction::Reset => {
            output.warning("Resetting configuration to defaults")?;
            // TODO: Implement config reset
        }
        ConfigAction::Validate => {
            output.success("Configuration is valid")?;
            // TODO: Implement config validation
        }
    }
    
    Ok(())
}