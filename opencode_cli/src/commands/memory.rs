//! Memory command implementation

use anyhow::Result;
use opencode_core::Engine;
use crate::{MemoryAction, cli::Output};

/// Execute the memory command
pub async fn execute(engine: Engine, action: &MemoryAction) -> Result<()> {
    let mut output = Output::new();
    let memory = engine.memory();
    
    match action {
        MemoryAction::List => {
            let memory_guard = memory.read().await;
            let keys = memory_guard.list().await?;
            
            output.table_header(&["Key", "Preview"])?;
            for key in keys {
                if let Ok(Some(value)) = memory_guard.retrieve(&key).await {
                    let preview = if value.is_string() {
                        crate::utils::truncate_string(&value.as_str().unwrap_or(""), 50)
                    } else {
                        crate::utils::truncate_string(&value.to_string(), 50)
                    };
                    output.table_row(&[&key, &preview])?;
                }
            }
        }
        MemoryAction::Store { key, value } => {
            let memory_guard = memory.read().await;
            let json_value: serde_json::Value = serde_json::from_str(value)?;
            memory_guard.store(key, json_value).await?;
            output.success(&format!("Stored value for key: {}", key))?;
        }
        MemoryAction::Get { key } => {
            let memory_guard = memory.read().await;
            if let Some(value) = memory_guard.retrieve(key).await? {
                output.json(&value)?;
            } else {
                output.error(&format!("Key not found: {}", key))?;
            }
        }
        MemoryAction::Delete { key } => {
            let memory_guard = memory.read().await;
            memory_guard.delete(key).await?;
            output.success(&format!("Deleted key: {}", key))?;
        }
        MemoryAction::Search { query } => {
            let memory_guard = memory.read().await;
            let results = memory_guard.search(query).await?;
            
            output.table_header(&["Key"])?;
            for key in results {
                output.table_row(&[&key])?;
            }
        }
        MemoryAction::Clear => {
            let memory_guard = memory.read().await;
            memory_guard.clear().await?;
            output.success("Cleared all memory")?;
        }
        MemoryAction::Stats => {
            let memory_guard = memory.read().await;
            let stats = memory_guard.stats().await?;
            output.json(&serde_json::to_value(stats)?)?;
        }
    }
    
    Ok(())
}