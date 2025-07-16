//! Session management for OpenCode
//!
//! This module handles session lifecycle, conversation persistence,
//! and state management across different environments.

use crate::providers::Message;
use crate::OpenCodeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session name
    pub name: String,
    
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    
    /// Maximum history length
    pub max_history_length: usize,
    
    /// Enable persistence
    pub persistent: bool,
    
    /// Storage directory
    pub storage_dir: Option<PathBuf>,
    
    /// Session metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    /// Session is active
    Active,
    
    /// Session is paused
    Paused,
    
    /// Session is archived
    Archived,
    
    /// Session has been terminated
    Terminated,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub id: Uuid,
    
    /// Session name
    pub name: String,
    
    /// Current state
    pub state: SessionState,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
    
    /// Session statistics
    pub stats: SessionStats,
    
    /// Session metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total messages in session
    pub message_count: usize,
    
    /// Total tokens used
    pub token_count: u64,
    
    /// Number of agents in session
    pub agent_count: usize,
    
    /// Session duration in seconds
    pub duration: u64,
}

/// Session data for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Session information
    pub info: SessionInfo,
    
    /// Message history
    pub history: Vec<Message>,
    
    /// Session variables
    pub variables: HashMap<String, serde_json::Value>,
    
    /// Context data
    pub context: HashMap<String, serde_json::Value>,
}

/// Session errors
#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    
    #[error("Session is in invalid state: {0:?}")]
    InvalidState(SessionState),
    
    #[error("Session storage error: {0}")]
    Storage(String),
    
    #[error("Session limit exceeded")]
    LimitExceeded,
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Main session implementation
pub struct Session {
    /// Session information
    info: SessionInfo,
    
    /// Session configuration
    config: SessionConfig,
    
    /// Message history
    history: Vec<Message>,
    
    /// Session variables
    variables: HashMap<String, serde_json::Value>,
    
    /// Context data
    context: HashMap<String, serde_json::Value>,
    
    /// Auto-save timer
    #[cfg(feature = "native-runtime")]
    auto_save_task: Option<tokio::task::JoinHandle<()>>,
}

/// Session manager for handling multiple sessions
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<Uuid, Arc<Mutex<Session>>>>>,
    
    /// Default session configuration
    default_config: SessionConfig,
    
    /// Storage backend
    storage: Arc<dyn SessionStorage>,
}

/// Session storage trait
#[async_trait::async_trait]
pub trait SessionStorage: Send + Sync {
    /// Save session data
    async fn save_session(&self, session: &SessionData) -> Result<(), SessionError>;
    
    /// Load session data
    async fn load_session(&self, id: Uuid) -> Result<Option<SessionData>, SessionError>;
    
    /// Delete session data
    async fn delete_session(&self, id: Uuid) -> Result<(), SessionError>;
    
    /// List all sessions
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, SessionError>;
    
    /// Check if session exists
    async fn exists(&self, id: Uuid) -> Result<bool, SessionError>;
}

/// File-based session storage
#[cfg(feature = "native-runtime")]
pub struct FileSessionStorage {
    storage_dir: PathBuf,
}

/// In-memory session storage (for WASM)
pub struct MemorySessionStorage {
    sessions: Arc<RwLock<HashMap<Uuid, SessionData>>>,
}

/// Browser localStorage session storage
#[cfg(feature = "wasm-runtime")]
pub struct BrowserSessionStorage;

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            name: "default".to_string(),
            auto_save_interval: 300, // 5 minutes
            max_history_length: 1000,
            persistent: true,
            storage_dir: None,
            metadata: HashMap::new(),
        }
    }
}

impl Default for SessionStats {
    fn default() -> Self {
        SessionStats {
            message_count: 0,
            token_count: 0,
            agent_count: 0,
            duration: 0,
        }
    }
}

impl Session {
    /// Create a new session
    pub fn new(name: String) -> Self {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        
        let info = SessionInfo {
            id,
            name: name.clone(),
            state: SessionState::Active,
            created_at: now,
            last_activity: now,
            stats: SessionStats::default(),
            metadata: HashMap::new(),
        };
        
        let config = SessionConfig {
            name,
            ..Default::default()
        };
        
        Session {
            info,
            config,
            history: Vec::new(),
            variables: HashMap::new(),
            context: HashMap::new(),
            #[cfg(feature = "native-runtime")]
            auto_save_task: None,
        }
    }
    
    /// Create a session with custom configuration
    pub fn with_config(config: SessionConfig) -> Self {
        let mut session = Self::new(config.name.clone());
        session.config = config;
        session
    }
    
    /// Get session ID
    pub fn id(&self) -> Uuid {
        self.info.id
    }
    
    /// Get session name
    pub fn name(&self) -> &str {
        &self.info.name
    }
    
    /// Get session state
    pub fn state(&self) -> &SessionState {
        &self.info.state
    }
    
    /// Get session info
    pub fn info(&self) -> &SessionInfo {
        &self.info
    }
    
    /// Get session configuration
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
    
    /// Add a message to the session
    pub fn add_message(&mut self, message: Message) {
        self.history.push(message);
        self.info.last_activity = chrono::Utc::now();
        self.info.stats.message_count = self.history.len();
        
        // Trim history if it exceeds maximum length
        if self.history.len() > self.config.max_history_length {
            self.history.remove(0);
        }
    }
    
    /// Get message history
    pub fn history(&self) -> &[Message] {
        &self.history
    }
    
    /// Clear message history
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.info.stats.message_count = 0;
        self.info.last_activity = chrono::Utc::now();
    }
    
    /// Set a session variable
    pub fn set_variable(&mut self, key: String, value: serde_json::Value) {
        self.variables.insert(key, value);
        self.info.last_activity = chrono::Utc::now();
    }
    
    /// Get a session variable
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }
    
    /// Remove a session variable
    pub fn remove_variable(&mut self, key: &str) -> Option<serde_json::Value> {
        let result = self.variables.remove(key);
        if result.is_some() {
            self.info.last_activity = chrono::Utc::now();
        }
        result
    }
    
    /// Set context data
    pub fn set_context(&mut self, key: String, value: serde_json::Value) {
        self.context.insert(key, value);
        self.info.last_activity = chrono::Utc::now();
    }
    
    /// Get context data
    pub fn get_context(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }
    
    /// Pause the session
    pub fn pause(&mut self) {
        self.info.state = SessionState::Paused;
        self.info.last_activity = chrono::Utc::now();
    }
    
    /// Resume the session
    pub fn resume(&mut self) {
        if self.info.state == SessionState::Paused {
            self.info.state = SessionState::Active;
            self.info.last_activity = chrono::Utc::now();
        }
    }
    
    /// Archive the session
    pub fn archive(&mut self) {
        self.info.state = SessionState::Archived;
        self.info.last_activity = chrono::Utc::now();
    }
    
    /// Terminate the session
    pub fn terminate(&mut self) {
        self.info.state = SessionState::Terminated;
        self.info.last_activity = chrono::Utc::now();
        
        #[cfg(feature = "native-runtime")]
        {
            if let Some(task) = self.auto_save_task.take() {
                task.abort();
            }
        }
    }
    
    /// Get session duration
    pub fn duration(&self) -> chrono::Duration {
        self.info.last_activity - self.info.created_at
    }
    
    /// Convert session to data for persistence
    pub fn to_data(&self) -> SessionData {
        SessionData {
            info: self.info.clone(),
            history: self.history.clone(),
            variables: self.variables.clone(),
            context: self.context.clone(),
        }
    }
    
    /// Create session from data
    pub fn from_data(data: SessionData, config: SessionConfig) -> Self {
        Session {
            info: data.info,
            config,
            history: data.history,
            variables: data.variables,
            context: data.context,
            #[cfg(feature = "native-runtime")]
            auto_save_task: None,
        }
    }
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        let storage = create_default_storage();
        
        SessionManager {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_config: SessionConfig::default(),
            storage,
        }
    }
    
    /// Create session manager with custom storage
    pub fn with_storage(storage: Arc<dyn SessionStorage>) -> Self {
        SessionManager {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_config: SessionConfig::default(),
            storage,
        }
    }
    
    /// Create a new session
    pub async fn create_session(&self) -> OpenCodeResult<Arc<Mutex<Session>>> {
        self.create_session_with_config(self.default_config.clone()).await
    }
    
    /// Create a session with custom configuration
    pub async fn create_session_with_config(&self, config: SessionConfig) -> OpenCodeResult<Arc<Mutex<Session>>> {
        let session = Session::with_config(config);
        let id = session.id();
        let session_arc = Arc::new(Mutex::new(session));
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(id, session_arc.clone());
        
        Ok(session_arc)
    }
    
    /// Get a session by ID
    pub async fn get_session(&self, id: Uuid) -> Option<Arc<Mutex<Session>>> {
        let sessions = self.sessions.read().await;
        sessions.get(&id).cloned()
    }
    
    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut session_infos = Vec::new();
        
        for session_arc in sessions.values() {
            let session = session_arc.lock().await;
            session_infos.push(session.info().clone());
        }
        
        session_infos
    }
    
    /// Save a session to storage
    pub async fn save_session(&self, id: Uuid) -> Result<(), SessionError> {
        let sessions = self.sessions.read().await;
        if let Some(session_arc) = sessions.get(&id) {
            let session = session_arc.lock().await;
            let data = session.to_data();
            self.storage.save_session(&data).await?;
        }
        Ok(())
    }
    
    /// Load a session from storage
    pub async fn load_session(&self, id: Uuid) -> Result<Option<Arc<Mutex<Session>>>, SessionError> {
        if let Some(data) = self.storage.load_session(id).await? {
            let session = Session::from_data(data, self.default_config.clone());
            let session_arc = Arc::new(Mutex::new(session));
            
            let mut sessions = self.sessions.write().await;
            sessions.insert(id, session_arc.clone());
            
            Ok(Some(session_arc))
        } else {
            Ok(None)
        }
    }
    
    /// Delete a session
    pub async fn delete_session(&self, id: Uuid) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&id);
        self.storage.delete_session(id).await?;
        Ok(())
    }
    
    /// Save all sessions
    pub async fn save_all(&self) -> Result<(), SessionError> {
        let sessions = self.sessions.read().await;
        for session_arc in sessions.values() {
            let session = session_arc.lock().await;
            let data = session.to_data();
            self.storage.save_session(&data).await?;
        }
        Ok(())
    }
}

// Storage implementations
impl MemorySessionStorage {
    pub fn new() -> Self {
        MemorySessionStorage {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn save_session(&self, session: &SessionData) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.info.id, session.clone());
        Ok(())
    }
    
    async fn load_session(&self, id: Uuid) -> Result<Option<SessionData>, SessionError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(&id).cloned())
    }
    
    async fn delete_session(&self, id: Uuid) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&id);
        Ok(())
    }
    
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, SessionError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.values().map(|s| s.info.clone()).collect())
    }
    
    async fn exists(&self, id: Uuid) -> Result<bool, SessionError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.contains_key(&id))
    }
}

#[cfg(feature = "native-runtime")]
impl FileSessionStorage {
    pub fn new(storage_dir: PathBuf) -> Self {
        FileSessionStorage { storage_dir }
    }
}

#[cfg(feature = "native-runtime")]
#[async_trait::async_trait]
impl SessionStorage for FileSessionStorage {
    async fn save_session(&self, session: &SessionData) -> Result<(), SessionError> {
        tokio::fs::create_dir_all(&self.storage_dir).await?;
        
        let file_path = self.storage_dir.join(format!("{}.json", session.info.id));
        let content = serde_json::to_string_pretty(session)?;
        tokio::fs::write(file_path, content).await?;
        
        Ok(())
    }
    
    async fn load_session(&self, id: Uuid) -> Result<Option<SessionData>, SessionError> {
        let file_path = self.storage_dir.join(format!("{}.json", id));
        
        if !file_path.exists() {
            return Ok(None);
        }
        
        let content = tokio::fs::read_to_string(file_path).await?;
        let session: SessionData = serde_json::from_str(&content)?;
        
        Ok(Some(session))
    }
    
    async fn delete_session(&self, id: Uuid) -> Result<(), SessionError> {
        let file_path = self.storage_dir.join(format!("{}.json", id));
        
        if file_path.exists() {
            tokio::fs::remove_file(file_path).await?;
        }
        
        Ok(())
    }
    
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, SessionError> {
        let mut sessions = Vec::new();
        let mut dir = tokio::fs::read_dir(&self.storage_dir).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(session_data) = serde_json::from_str::<SessionData>(&content) {
                        sessions.push(session_data.info);
                    }
                }
            }
        }
        
        Ok(sessions)
    }
    
    async fn exists(&self, id: Uuid) -> Result<bool, SessionError> {
        let file_path = self.storage_dir.join(format!("{}.json", id));
        Ok(file_path.exists())
    }
}

#[cfg(feature = "wasm-runtime")]
#[async_trait::async_trait]
impl SessionStorage for BrowserSessionStorage {
    async fn save_session(&self, session: &SessionData) -> Result<(), SessionError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| SessionError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| SessionError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| SessionError::Storage("localStorage not available".to_string()))?;
        
        let key = format!("opencode_session_{}", session.info.id);
        let value = serde_json::to_string(session)?;
        
        storage.set_item(&key, &value)
            .map_err(|_| SessionError::Storage("Failed to save session".to_string()))?;
        
        Ok(())
    }
    
    async fn load_session(&self, id: Uuid) -> Result<Option<SessionData>, SessionError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| SessionError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| SessionError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| SessionError::Storage("localStorage not available".to_string()))?;
        
        let key = format!("opencode_session_{}", id);
        
        match storage.get_item(&key) {
            Ok(Some(value)) => {
                let session: SessionData = serde_json::from_str(&value)?;
                Ok(Some(session))
            }
            Ok(None) => Ok(None),
            Err(_) => Err(SessionError::Storage("Failed to load session".to_string())),
        }
    }
    
    async fn delete_session(&self, id: Uuid) -> Result<(), SessionError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| SessionError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| SessionError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| SessionError::Storage("localStorage not available".to_string()))?;
        
        let key = format!("opencode_session_{}", id);
        storage.remove_item(&key)
            .map_err(|_| SessionError::Storage("Failed to delete session".to_string()))?;
        
        Ok(())
    }
    
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, SessionError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| SessionError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| SessionError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| SessionError::Storage("localStorage not available".to_string()))?;
        
        let mut sessions = Vec::new();
        let length = storage.length()
            .map_err(|_| SessionError::Storage("Cannot get storage length".to_string()))?;
        
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with("opencode_session_") {
                    if let Ok(Some(value)) = storage.get_item(&key) {
                        if let Ok(session_data) = serde_json::from_str::<SessionData>(&value) {
                            sessions.push(session_data.info);
                        }
                    }
                }
            }
        }
        
        Ok(sessions)
    }
    
    async fn exists(&self, id: Uuid) -> Result<bool, SessionError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| SessionError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| SessionError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| SessionError::Storage("localStorage not available".to_string()))?;
        
        let key = format!("opencode_session_{}", id);
        
        match storage.get_item(&key) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err(SessionError::Storage("Failed to check session existence".to_string())),
        }
    }
}

// Create default storage based on feature flags
fn create_default_storage() -> Arc<dyn SessionStorage> {
    #[cfg(feature = "native-runtime")]
    {
        if let Some(config_dir) = dirs::config_dir() {
            let storage_dir = config_dir.join("opencode").join("sessions");
            Arc::new(FileSessionStorage::new(storage_dir))
        } else {
            Arc::new(MemorySessionStorage::new())
        }
    }
    
    #[cfg(feature = "wasm-runtime")]
    {
        Arc::new(BrowserSessionStorage)
    }
    
    #[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
    {
        Arc::new(MemorySessionStorage::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_creation() {
        let session = Session::new("test_session".to_string());
        assert_eq!(session.name(), "test_session");
        assert_eq!(session.state(), &SessionState::Active);
        assert_eq!(session.history().len(), 0);
    }
    
    #[test]
    fn test_session_variables() {
        let mut session = Session::new("test".to_string());
        
        session.set_variable("key1".to_string(), serde_json::json!("value1"));
        assert_eq!(session.get_variable("key1"), Some(&serde_json::json!("value1")));
        
        let removed = session.remove_variable("key1");
        assert_eq!(removed, Some(serde_json::json!("value1")));
        assert_eq!(session.get_variable("key1"), None);
    }
    
    #[test]
    fn test_session_state_transitions() {
        let mut session = Session::new("test".to_string());
        
        assert_eq!(session.state(), &SessionState::Active);
        
        session.pause();
        assert_eq!(session.state(), &SessionState::Paused);
        
        session.resume();
        assert_eq!(session.state(), &SessionState::Active);
        
        session.archive();
        assert_eq!(session.state(), &SessionState::Archived);
        
        session.terminate();
        assert_eq!(session.state(), &SessionState::Terminated);
    }
    
    #[tokio::test]
    async fn test_memory_session_storage() {
        let storage = MemorySessionStorage::new();
        
        let session = Session::new("test".to_string());
        let data = session.to_data();
        let id = data.info.id;
        
        // Save session
        storage.save_session(&data).await.unwrap();
        
        // Load session
        let loaded = storage.load_session(id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().info.id, id);
        
        // Check existence
        assert!(storage.exists(id).await.unwrap());
        
        // Delete session
        storage.delete_session(id).await.unwrap();
        assert!(!storage.exists(id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_session_manager() {
        let storage = Arc::new(MemorySessionStorage::new());
        let manager = SessionManager::with_storage(storage);
        
        let session = manager.create_session().await.unwrap();
        let id = session.lock().await.id();
        
        // Check session exists
        let retrieved = manager.get_session(id).await;
        assert!(retrieved.is_some());
        
        // List sessions
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);
        
        // Save session
        manager.save_session(id).await.unwrap();
        
        // Delete session
        manager.delete_session(id).await.unwrap();
        let retrieved = manager.get_session(id).await;
        assert!(retrieved.is_none());
    }
}