//! Session management for Code Mesh

use crate::llm::Message as LlmMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub mod manager;
pub use manager::{SessionManager, SessionInfo};

/// Session represents a conversation with the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    pub metadata: SessionMetadata,
}

impl Session {
    /// Create a new session
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: format!("session_{}", Uuid::new_v4()),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            metadata: SessionMetadata::default(),
        }
    }
    
    /// Add a message to the session
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }
    
    /// Get messages as LLM format
    pub fn to_llm_messages(&self) -> Vec<LlmMessage> {
        self.messages.iter().map(|m| m.to_llm_message()).collect()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}

impl Message {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: format!("msg_{}", Uuid::new_v4()),
            role,
            content,
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }
    
    pub fn to_llm_message(&self) -> LlmMessage {
        LlmMessage {
            role: match self.role {
                MessageRole::System => crate::llm::MessageRole::System,
                MessageRole::User => crate::llm::MessageRole::User,
                MessageRole::Assistant => crate::llm::MessageRole::Assistant,
                MessageRole::Tool => crate::llm::MessageRole::Tool,
            },
            content: crate::llm::MessageContent::Text(self.content.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub title: Option<String>,
    pub mode: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub tokens_used: Option<TokenUsage>,
    pub tool_calls: Vec<ToolCall>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
}