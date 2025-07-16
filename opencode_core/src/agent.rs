//! Agent management and orchestration system
//!
//! This module provides the core agent functionality for OpenCode,
//! including agent creation, message handling, and multi-agent coordination.

use crate::providers::{LLMProvider, LLMRequest, LLMResponse, Message, MessageRole};
use crate::session::{Session, SessionManager};
use crate::memory::MemoryManager;
use crate::OpenCodeResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,
    
    /// System prompt for the agent
    pub system_prompt: Option<String>,
    
    /// Model to use for this agent
    pub model: Option<String>,
    
    /// Temperature setting
    pub temperature: f32,
    
    /// Maximum tokens per response
    pub max_tokens: Option<u32>,
    
    /// Request timeout in seconds
    pub timeout: u64,
    
    /// Enable streaming responses
    pub streaming: bool,
    
    /// Agent-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Agent state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentState {
    /// Agent is idle and ready to receive messages
    Idle,
    
    /// Agent is processing a request
    Processing,
    
    /// Agent is waiting for user input or confirmation
    Waiting,
    
    /// Agent has encountered an error
    Error(String),
    
    /// Agent has been terminated
    Terminated,
}

/// Agent handle for external interaction
#[derive(Debug, Clone)]
pub struct AgentHandle {
    pub id: Uuid,
    pub name: String,
    pub state: AgentState,
    agent_ref: Arc<Mutex<Agent>>,
}

/// Agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Response content
    pub content: String,
    
    /// Token usage information
    pub usage: Option<crate::providers::Usage>,
    
    /// Finish reason
    pub finish_reason: Option<String>,
    
    /// Response metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Agent errors
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),
    
    #[error("Agent is in invalid state: {0:?}")]
    InvalidState(AgentState),
    
    #[error("Provider error: {0}")]
    Provider(#[from] crate::providers::ProviderError),
    
    #[error("Session error: {0}")]
    Session(#[from] crate::session::SessionError),
    
    #[error("Memory error: {0}")]
    Memory(#[from] crate::memory::MemoryError),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Main agent implementation
pub struct Agent {
    /// Unique agent identifier
    id: Uuid,
    
    /// Agent configuration
    config: AgentConfig,
    
    /// Current agent state
    state: AgentState,
    
    /// LLM provider
    provider: Arc<dyn LLMProvider>,
    
    /// Session manager
    session: Arc<Mutex<Session>>,
    
    /// Memory manager
    memory: Arc<RwLock<MemoryManager>>,
    
    /// Message history
    history: Vec<Message>,
    
    /// Agent statistics
    stats: AgentStats,
}

/// Agent statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    /// Total messages processed
    pub messages_processed: u64,
    
    /// Total tokens consumed
    pub tokens_consumed: u64,
    
    /// Total requests made
    pub requests_made: u64,
    
    /// Average response time in milliseconds
    pub avg_response_time: f64,
    
    /// Error count
    pub error_count: u64,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Agent orchestrator for managing multiple agents
pub struct AgentOrchestrator {
    /// Active agents
    agents: Arc<RwLock<HashMap<Uuid, AgentHandle>>>,
    
    /// Session manager
    session_manager: Arc<SessionManager>,
    
    /// Memory manager
    memory_manager: Arc<RwLock<MemoryManager>>,
    
    /// Default agent configuration
    default_config: AgentConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            name: "assistant".to_string(),
            system_prompt: Some("You are a helpful AI coding assistant.".to_string()),
            model: None,
            temperature: 0.7,
            max_tokens: None,
            timeout: 30,
            streaming: false,
            metadata: HashMap::new(),
        }
    }
}

impl Default for AgentStats {
    fn default() -> Self {
        let now = chrono::Utc::now();
        AgentStats {
            messages_processed: 0,
            tokens_consumed: 0,
            requests_made: 0,
            avg_response_time: 0.0,
            error_count: 0,
            created_at: now,
            last_activity: now,
        }
    }
}

impl Agent {
    /// Create a new agent
    pub fn new(
        config: AgentConfig,
        provider: Arc<dyn LLMProvider>,
        session: Arc<Mutex<Session>>,
        memory: Arc<RwLock<MemoryManager>>,
    ) -> Self {
        let id = Uuid::new_v4();
        let mut history = Vec::new();
        
        // Add system prompt to history if provided
        if let Some(system_prompt) = &config.system_prompt {
            history.push(Message::system(system_prompt));
        }
        
        Agent {
            id,
            config,
            state: AgentState::Idle,
            provider,
            session,
            memory,
            history,
            stats: AgentStats::default(),
        }
    }
    
    /// Get agent ID
    pub fn id(&self) -> Uuid {
        self.id
    }
    
    /// Get agent name
    pub fn name(&self) -> &str {
        &self.config.name
    }
    
    /// Get agent state
    pub fn state(&self) -> &AgentState {
        &self.state
    }
    
    /// Get agent configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    /// Get agent statistics
    pub fn stats(&self) -> &AgentStats {
        &self.stats
    }
    
    /// Send a message to the agent
    pub async fn send_message(&mut self, content: &str) -> Result<AgentResponse, AgentError> {
        if self.state != AgentState::Idle {
            return Err(AgentError::InvalidState(self.state.clone()));
        }
        
        self.state = AgentState::Processing;
        self.stats.last_activity = chrono::Utc::now();
        
        let start_time = std::time::Instant::now();
        
        // Add user message to history
        let user_message = Message::user(content);
        self.history.push(user_message);
        
        // Create LLM request
        let request = LLMRequest {
            messages: self.history.clone(),
            model: self.config.model.clone(),
            temperature: Some(self.config.temperature),
            max_tokens: self.config.max_tokens,
            stream: self.config.streaming,
            ..Default::default()
        };
        
        // Send request to provider
        let result = self.provider.complete(request).await;
        
        match result {
            Ok(response) => {
                // Add assistant response to history
                let assistant_message = Message::assistant(&response.content);
                self.history.push(assistant_message);
                
                // Update statistics
                self.stats.messages_processed += 1;
                self.stats.requests_made += 1;
                if let Some(usage) = &response.usage {
                    self.stats.tokens_consumed += usage.total_tokens as u64;
                }
                
                let elapsed = start_time.elapsed().as_millis() as f64;
                self.stats.avg_response_time = if self.stats.requests_made == 1 {
                    elapsed
                } else {
                    (self.stats.avg_response_time * (self.stats.requests_made - 1) as f64 + elapsed) / self.stats.requests_made as f64
                };
                
                self.state = AgentState::Idle;
                
                // Store in memory
                self.store_interaction(content, &response.content).await?;
                
                Ok(AgentResponse {
                    content: response.content,
                    usage: response.usage,
                    finish_reason: response.finish_reason,
                    metadata: response.metadata,
                })
            }
            Err(error) => {
                self.stats.error_count += 1;
                self.state = AgentState::Error(error.to_string());
                Err(AgentError::Provider(error))
            }
        }
    }
    
    /// Get conversation history
    pub fn history(&self) -> &[Message] {
        &self.history
    }
    
    /// Clear conversation history
    pub fn clear_history(&mut self) {
        self.history.clear();
        
        // Re-add system prompt if it exists
        if let Some(system_prompt) = &self.config.system_prompt {
            self.history.push(Message::system(system_prompt));
        }
    }
    
    /// Reset agent state
    pub fn reset(&mut self) {
        self.state = AgentState::Idle;
        self.clear_history();
    }
    
    /// Store interaction in memory
    async fn store_interaction(&self, user_message: &str, assistant_response: &str) -> Result<(), AgentError> {
        let memory = self.memory.read().await;
        
        let interaction = serde_json::json!({
            "agent_id": self.id,
            "agent_name": self.config.name,
            "user_message": user_message,
            "assistant_response": assistant_response,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        memory.store(&format!("interaction_{}", Uuid::new_v4()), interaction).await?;
        Ok(())
    }
}

impl AgentHandle {
    /// Send a message to the agent
    pub async fn send_message(&self, content: &str) -> Result<AgentResponse, AgentError> {
        let mut agent = self.agent_ref.lock().await;
        agent.send_message(content).await
    }
    
    /// Get agent statistics
    pub async fn get_stats(&self) -> AgentStats {
        let agent = self.agent_ref.lock().await;
        agent.stats().clone()
    }
    
    /// Get conversation history
    pub async fn get_history(&self) -> Vec<Message> {
        let agent = self.agent_ref.lock().await;
        agent.history().to_vec()
    }
    
    /// Clear conversation history
    pub async fn clear_history(&self) {
        let mut agent = self.agent_ref.lock().await;
        agent.clear_history();
    }
    
    /// Reset agent
    pub async fn reset(&self) {
        let mut agent = self.agent_ref.lock().await;
        agent.reset();
    }
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator
    pub fn new(
        session_manager: Arc<SessionManager>,
        memory_manager: Arc<RwLock<MemoryManager>>,
        default_config: AgentConfig,
    ) -> Self {
        AgentOrchestrator {
            agents: Arc::new(RwLock::new(HashMap::new())),
            session_manager,
            memory_manager,
            default_config,
        }
    }
    
    /// Create a new agent
    pub async fn create_agent(
        &self,
        name: &str,
        provider: Arc<dyn LLMProvider>,
        config: Option<AgentConfig>,
    ) -> OpenCodeResult<AgentHandle> {
        let mut agent_config = config.unwrap_or_else(|| self.default_config.clone());
        agent_config.name = name.to_string();
        
        let session = self.session_manager.create_session().await?;
        let agent = Agent::new(agent_config, provider, session, self.memory_manager.clone());
        
        let handle = AgentHandle {
            id: agent.id(),
            name: agent.name().to_string(),
            state: agent.state().clone(),
            agent_ref: Arc::new(Mutex::new(agent)),
        };
        
        let mut agents = self.agents.write().await;
        agents.insert(handle.id, handle.clone());
        
        Ok(handle)
    }
    
    /// Get an agent by ID
    pub async fn get_agent(&self, id: Uuid) -> Option<AgentHandle> {
        let agents = self.agents.read().await;
        agents.get(&id).cloned()
    }
    
    /// Get an agent by name
    pub async fn get_agent_by_name(&self, name: &str) -> Option<AgentHandle> {
        let agents = self.agents.read().await;
        agents.values().find(|agent| agent.name == name).cloned()
    }
    
    /// List all agents
    pub async fn list_agents(&self) -> Vec<AgentHandle> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }
    
    /// Remove an agent
    pub async fn remove_agent(&self, id: Uuid) -> bool {
        let mut agents = self.agents.write().await;
        agents.remove(&id).is_some()
    }
    
    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{ProviderType, LLMProvider};
    use crate::session::SessionManager;
    use crate::memory::MemoryManager;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    // Mock provider for testing
    struct MockProvider;
    
    #[async_trait]
    impl LLMProvider for MockProvider {
        fn provider_type(&self) -> ProviderType {
            ProviderType::Custom("mock".to_string())
        }
        
        fn name(&self) -> String {
            "Mock".to_string()
        }
        
        async fn is_available(&self) -> bool {
            true
        }
        
        async fn get_models(&self) -> Result<Vec<String>, crate::providers::ProviderError> {
            Ok(vec!["mock-model".to_string()])
        }
        
        async fn complete(&self, _request: LLMRequest) -> Result<LLMResponse, crate::providers::ProviderError> {
            Ok(LLMResponse {
                content: "Mock response".to_string(),
                model: "mock-model".to_string(),
                usage: None,
                finish_reason: Some("stop".to_string()),
                metadata: HashMap::new(),
            })
        }
        
        async fn complete_stream(&self, _request: LLMRequest) -> Result<Box<dyn futures::Stream<Item = Result<LLMResponse, crate::providers::ProviderError>> + Send + Unpin>, crate::providers::ProviderError> {
            todo!()
        }
        
        fn default_model(&self) -> String {
            "mock-model".to_string()
        }
        
        fn get_config(&self) -> HashMap<String, serde_json::Value> {
            HashMap::new()
        }
        
        fn update_config(&mut self, _config: HashMap<String, serde_json::Value>) -> Result<(), crate::providers::ProviderError> {
            Ok(())
        }
        
        async fn validate_config(&self) -> Result<(), crate::providers::ProviderError> {
            Ok(())
        }
    }
    
    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.name, "assistant");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.timeout, 30);
        assert!(!config.streaming);
    }
    
    #[test]
    fn test_agent_stats_default() {
        let stats = AgentStats::default();
        assert_eq!(stats.messages_processed, 0);
        assert_eq!(stats.tokens_consumed, 0);
        assert_eq!(stats.requests_made, 0);
        assert_eq!(stats.avg_response_time, 0.0);
        assert_eq!(stats.error_count, 0);
    }
    
    #[tokio::test]
    async fn test_agent_creation() {
        let config = AgentConfig::default();
        let provider = Arc::new(MockProvider);
        let session = Arc::new(Mutex::new(crate::session::Session::new("test".to_string())));
        let memory = Arc::new(RwLock::new(MemoryManager::new()));
        
        let agent = Agent::new(config, provider, session, memory);
        
        assert_eq!(agent.name(), "assistant");
        assert_eq!(agent.state(), &AgentState::Idle);
        assert_eq!(agent.stats().messages_processed, 0);
    }
    
    #[tokio::test]
    async fn test_agent_orchestrator() {
        let session_manager = Arc::new(SessionManager::new());
        let memory_manager = Arc::new(RwLock::new(MemoryManager::new()));
        let default_config = AgentConfig::default();
        
        let orchestrator = AgentOrchestrator::new(session_manager, memory_manager, default_config);
        
        let provider = Arc::new(MockProvider);
        let handle = orchestrator.create_agent("test_agent", provider, None).await.unwrap();
        
        assert_eq!(handle.name, "test_agent");
        assert_eq!(orchestrator.agent_count().await, 1);
        
        let retrieved = orchestrator.get_agent_by_name("test_agent").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, handle.id);
    }
}