//! Session management implementation

use super::{Session, Message, MessageRole};
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session manager for handling multiple sessions
pub struct SessionManager {
    storage: Box<dyn Storage>,
    active_sessions: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self {
            storage,
            active_sessions: HashMap::new(),
        }
    }
    
    /// Create a new session
    pub async fn create_session(&mut self) -> crate::Result<Session> {
        let session = Session::new();
        self.active_sessions.insert(session.id.clone(), session.clone());
        
        // Persist to storage
        self.save_session(&session).await?;
        
        Ok(session)
    }
    
    /// Get a session by ID
    pub async fn get_session(&mut self, id: &str) -> crate::Result<Option<Session>> {
        // Check active sessions first
        if let Some(session) = self.active_sessions.get(id) {
            return Ok(Some(session.clone()));
        }
        
        // Try loading from storage
        if let Some(data) = self.storage.get(&format!("session:{}", id)).await? {
            let session: Session = serde_json::from_slice(&data)?;
            self.active_sessions.insert(id.to_string(), session.clone());
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }
    
    /// List all sessions
    pub async fn list_sessions(&self) -> crate::Result<Vec<SessionInfo>> {
        let keys = self.storage.list(Some("session:")).await?;
        let mut sessions = Vec::new();
        
        for key in keys {
            if let Some(data) = self.storage.get(&key).await? {
                if let Ok(session) = serde_json::from_slice::<Session>(&data) {
                    sessions.push(SessionInfo {
                        id: session.id.clone(),
                        created_at: session.created_at,
                        updated_at: session.updated_at,
                        title: session.metadata.title.clone(),
                        message_count: session.messages.len(),
                    });
                }
            }
        }
        
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }
    
    /// Add a message to a session
    pub async fn add_message(
        &mut self,
        session_id: &str,
        role: MessageRole,
        content: String,
    ) -> crate::Result<Message> {
        let message = Message::new(role, content);
        
        // Update session
        {
            let session = self.get_session_mut(session_id)?;
            session.add_message(message.clone());
        }
        
        // Persist changes
        if let Some(session) = self.active_sessions.get(session_id) {
            self.save_session(session).await?;
        }
        
        Ok(message)
    }
    
    /// Continue the last session
    pub async fn continue_last_session(&mut self) -> crate::Result<Option<Session>> {
        let sessions = self.list_sessions().await?;
        if let Some(info) = sessions.first() {
            self.get_session(&info.id).await
        } else {
            Ok(None)
        }
    }
    
    /// Save session to storage
    async fn save_session(&self, session: &Session) -> crate::Result<()> {
        let key = format!("session:{}", session.id);
        let data = serde_json::to_vec(session)?;
        self.storage.set(&key, &data).await?;
        Ok(())
    }
    
    /// Get mutable session reference
    fn get_session_mut(&mut self, id: &str) -> crate::Result<&mut Session> {
        self.active_sessions.get_mut(id)
            .ok_or_else(|| crate::Error::Session(format!("Session {} not found", id)))
    }
}

/// Session summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub title: Option<String>,
    pub message_count: usize,
}