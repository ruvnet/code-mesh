//! Session command implementation

use anyhow::Result;
use opencode_core::Engine;
use crate::{SessionAction, cli::Output};

/// Execute the session command
pub async fn execute(engine: Engine, action: &SessionAction) -> Result<()> {
    let mut output = Output::new();
    
    match action {
        SessionAction::List => {
            let sessions = engine.sessions().list_sessions().await;
            output.table_header(&["ID", "Name", "State", "Created", "Messages"])?;
            
            for session in sessions {
                output.table_row(&[
                    &session.id.to_string(),
                    &session.name,
                    &format!("{:?}", session.state),
                    &session.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    &session.stats.message_count.to_string(),
                ])?;
            }
        }
        SessionAction::Create { name } => {
            let config = opencode_core::session::SessionConfig {
                name: name.clone(),
                ..Default::default()
            };
            
            let session = engine.sessions().create_session_with_config(config).await?;
            let id = session.lock().await.id();
            output.success(&format!("Created session: {} (ID: {})", name, id))?;
        }
        SessionAction::Load { id } => {
            if let Ok(uuid) = id.parse::<uuid::Uuid>() {
                if let Ok(Some(session)) = engine.sessions().load_session(uuid).await {
                    let session_guard = session.lock().await;
                    output.success(&format!("Loaded session: {}", session_guard.name()))?;
                } else {
                    output.error(&format!("Session not found: {}", id))?;
                }
            } else {
                output.error(&format!("Invalid session ID: {}", id))?;
            }
        }
        SessionAction::Save { id } => {
            if let Ok(uuid) = id.parse::<uuid::Uuid>() {
                if let Err(e) = engine.sessions().save_session(uuid).await {
                    output.error(&format!("Failed to save session: {}", e))?;
                } else {
                    output.success(&format!("Saved session: {}", id))?;
                }
            } else {
                output.error(&format!("Invalid session ID: {}", id))?;
            }
        }
        SessionAction::Delete { id } => {
            if let Ok(uuid) = id.parse::<uuid::Uuid>() {
                if let Err(e) = engine.sessions().delete_session(uuid).await {
                    output.error(&format!("Failed to delete session: {}", e))?;
                } else {
                    output.success(&format!("Deleted session: {}", id))?;
                }
            } else {
                output.error(&format!("Invalid session ID: {}", id))?;
            }
        }
    }
    
    Ok(())
}