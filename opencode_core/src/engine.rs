//! Main OpenCode engine that coordinates all components
//!
//! This module provides the main Engine struct that orchestrates
//! agents, sessions, memory, and file system operations.

use crate::agent::{AgentOrchestrator, AgentConfig, AgentHandle};
use crate::config::Config;
use crate::filesystem::FileSystemManager;
use crate::memory::MemoryManager;
use crate::providers::{LLMProvider, ProviderFactory, ProviderType};
use crate::session::SessionManager;
use crate::OpenCodeResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main OpenCode engine
pub struct Engine {
    /// Configuration
    config: Config,
    
    /// Agent orchestrator
    agent_orchestrator: Arc<AgentOrchestrator>,
    
    /// Session manager
    session_manager: Arc<SessionManager>,
    
    /// Memory manager
    memory_manager: Arc<RwLock<MemoryManager>>,
    
    /// File system manager
    filesystem_manager: Arc<FileSystemManager>,
    
    /// Available providers
    providers: Arc<RwLock<HashMap<String, Arc<dyn LLMProvider>>>>,
    
    /// Engine statistics
    stats: Arc<RwLock<EngineStats>>,
}

/// Engine statistics
#[derive(Debug, Clone, Default)]
pub struct EngineStats {
    /// Total agents created
    pub agents_created: u64,
    
    /// Total sessions created
    pub sessions_created: u64,
    
    /// Total messages processed
    pub messages_processed: u64,
    
    /// Total tokens consumed
    pub tokens_consumed: u64,
    
    /// Engine uptime
    pub uptime: std::time::Duration,
    
    /// Start time
    pub start_time: std::time::Instant,
}

/// Engine configuration builder
pub struct EngineBuilder {
    config: Option<Config>,
    memory_manager: Option<Arc<RwLock<MemoryManager>>>,
    filesystem_manager: Option<Arc<FileSystemManager>>,
    session_manager: Option<Arc<SessionManager>>,
}

impl Engine {
    /// Create a new engine with default configuration
    pub async fn new() -> OpenCodeResult<Self> {
        let config = Config::load().await?;
        Self::with_config(config).await
    }
    
    /// Create a new engine with custom configuration
    pub async fn with_config(config: Config) -> OpenCodeResult<Self> {
        EngineBuilder::new()
            .with_config(config)
            .build()
            .await
    }
    
    /// Get engine configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
    
    /// Get agent orchestrator
    pub fn agents(&self) -> Arc<AgentOrchestrator> {
        self.agent_orchestrator.clone()
    }
    
    /// Get session manager
    pub fn sessions(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }
    
    /// Get memory manager
    pub fn memory(&self) -> Arc<RwLock<MemoryManager>> {
        self.memory_manager.clone()
    }
    
    /// Get file system manager
    pub fn filesystem(&self) -> Arc<FileSystemManager> {
        self.filesystem_manager.clone()
    }
    
    /// Create a new agent with default provider
    pub async fn create_agent(&self, name: &str) -> OpenCodeResult<AgentHandle> {
        let provider = self.get_default_provider().await?;
        let config = AgentConfig {
            name: name.to_string(),
            ..self.config.agents.clone().into()
        };
        
        let handle = self.agent_orchestrator.create_agent(name, provider, Some(config)).await?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.agents_created += 1;
        }
        
        Ok(handle)
    }
    
    /// Create a new agent with specific provider
    pub async fn create_agent_with_provider(&self, name: &str, provider_name: &str) -> OpenCodeResult<AgentHandle> {
        let provider = self.get_provider(provider_name).await?;
        let config = AgentConfig {
            name: name.to_string(),
            ..self.config.agents.clone().into()
        };
        
        let handle = self.agent_orchestrator.create_agent(name, provider, Some(config)).await?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.agents_created += 1;
        }
        
        Ok(handle)
    }
    
    /// Get a provider by name
    pub async fn get_provider(&self, name: &str) -> OpenCodeResult<Arc<dyn LLMProvider>> {
        let providers = self.providers.read().await;
        providers.get(name)
            .cloned()
            .ok_or_else(|| crate::OpenCodeError::Generic(format!("Provider '{}' not found", name)))
    }
    
    /// Get the default provider
    pub async fn get_default_provider(&self) -> OpenCodeResult<Arc<dyn LLMProvider>> {
        if let Some(default_name) = &self.config.default_provider {
            self.get_provider(default_name).await
        } else {
            let providers = self.providers.read().await;
            providers.values().next()
                .cloned()
                .ok_or_else(|| crate::OpenCodeError::Generic("No providers available".to_string()))
        }
    }
    
    /// List available providers
    pub async fn list_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }
    
    /// Add a provider
    pub async fn add_provider(&self, name: String, provider: Arc<dyn LLMProvider>) {
        let mut providers = self.providers.write().await;
        providers.insert(name, provider);
    }
    
    /// Remove a provider
    pub async fn remove_provider(&self, name: &str) -> bool {
        let mut providers = self.providers.write().await;
        providers.remove(name).is_some()
    }
    
    /// Get engine statistics
    pub async fn stats(&self) -> EngineStats {
        let mut stats = self.stats.read().await.clone();
        stats.uptime = stats.start_time.elapsed();
        stats
    }
    
    /// Update message statistics
    pub async fn update_message_stats(&self, message_count: u64, token_count: u64) {
        let mut stats = self.stats.write().await;
        stats.messages_processed += message_count;
        stats.tokens_consumed += token_count;
    }
    
    /// Shutdown the engine
    pub async fn shutdown(&self) -> OpenCodeResult<()> {
        // Save all sessions
        self.session_manager.save_all().await?;
        
        // Perform cleanup
        let memory = self.memory_manager.read().await;
        memory.cleanup().await?;
        
        log::info!("Engine shutdown completed");
        Ok(())
    }
    
    /// Initialize providers from configuration
    async fn initialize_providers(config: &Config) -> OpenCodeResult<HashMap<String, Arc<dyn LLMProvider>>> {
        let mut providers = HashMap::new();
        
        for (name, provider_config) in &config.providers {
            let provider_type = ProviderType::from(provider_config.provider_type.as_str());
            
            match ProviderFactory::create_provider(provider_type, provider_config) {
                Ok(provider) => {
                    // Validate provider configuration
                    if let Err(e) = provider.validate_config().await {
                        log::warn!("Failed to validate provider '{}': {}", name, e);
                        continue;
                    }
                    
                    providers.insert(name.clone(), provider);
                    log::info!("Initialized provider: {}", name);
                }
                Err(e) => {
                    log::error!("Failed to create provider '{}': {}", name, e);
                }
            }
        }
        
        if providers.is_empty() {
            return Err(crate::OpenCodeError::Generic("No providers could be initialized".to_string()));
        }
        
        Ok(providers)
    }
}

// Convert AgentDefaultConfig to AgentConfig
impl From<crate::config::AgentDefaultConfig> for AgentConfig {
    fn from(default: crate::config::AgentDefaultConfig) -> Self {
        AgentConfig {
            name: "default".to_string(),
            system_prompt: default.system_prompt,
            model: None,
            temperature: default.temperature,
            max_tokens: default.max_tokens,
            timeout: default.timeout,
            streaming: false,
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl EngineBuilder {
    /// Create a new engine builder
    pub fn new() -> Self {
        EngineBuilder {
            config: None,
            memory_manager: None,
            filesystem_manager: None,
            session_manager: None,
        }
    }
    
    /// Set configuration
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }
    
    /// Set memory manager
    pub fn with_memory_manager(mut self, memory_manager: Arc<RwLock<MemoryManager>>) -> Self {
        self.memory_manager = Some(memory_manager);
        self
    }
    
    /// Set filesystem manager
    pub fn with_filesystem_manager(mut self, filesystem_manager: Arc<FileSystemManager>) -> Self {
        self.filesystem_manager = Some(filesystem_manager);
        self
    }
    
    /// Set session manager
    pub fn with_session_manager(mut self, session_manager: Arc<SessionManager>) -> Self {
        self.session_manager = Some(session_manager);
        self
    }
    
    /// Build the engine
    pub async fn build(self) -> OpenCodeResult<Engine> {
        let config = self.config.unwrap_or_else(|| Config::default());
        
        // Initialize components
        let memory_manager = self.memory_manager.unwrap_or_else(|| {
            let memory_config = crate::memory::MemoryConfig {
                max_entries: config.memory.max_entries,
                default_expiration: Some(config.memory.expiration_seconds),
                persistent: config.memory.persistent,
                storage_dir: config.memory.storage_dir.clone(),
                cleanup_interval: 300,
            };
            Arc::new(RwLock::new(MemoryManager::with_config(memory_config)))
        });
        
        let filesystem_manager = self.filesystem_manager.unwrap_or_else(|| {
            Arc::new(FileSystemManager::new())
        });
        
        let session_manager = self.session_manager.unwrap_or_else(|| {
            Arc::new(SessionManager::new())
        });
        
        // Initialize providers
        let providers = Engine::initialize_providers(&config).await?;
        
        // Create agent orchestrator
        let agent_orchestrator = Arc::new(AgentOrchestrator::new(
            session_manager.clone(),
            memory_manager.clone(),
            config.agents.clone().into(),
        ));
        
        // Initialize statistics
        let stats = Arc::new(RwLock::new(EngineStats {
            start_time: std::time::Instant::now(),
            ..Default::default()
        }));
        
        Ok(Engine {
            config,
            agent_orchestrator,
            session_manager,
            memory_manager,
            filesystem_manager,
            providers: Arc::new(RwLock::new(providers)),
            stats,
        })
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ProviderConfig};
    use crate::providers::ProviderType;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_engine_creation() {
        let engine = Engine::new().await;
        // Engine creation might fail if no providers are configured
        // This is expected behavior
    }
    
    #[tokio::test]
    async fn test_engine_builder() {
        let config = Config::default();
        let builder = EngineBuilder::new().with_config(config);
        
        // Build might fail without valid providers
        let _result = builder.build().await;
    }
    
    #[tokio::test]
    async fn test_engine_with_mock_config() {
        let mut config = Config::default();
        
        // Add a mock provider configuration
        let mut providers = HashMap::new();
        providers.insert("mock".to_string(), ProviderConfig {
            provider_type: "custom".to_string(),
            api_key: Some("test_key".to_string()),
            base_url: None,
            model: None,
            settings: HashMap::new(),
        });
        config.providers = providers;
        
        let result = Engine::with_config(config).await;
        // This should fail because custom provider is not implemented
        assert!(result.is_err());
    }
    
    #[test]
    fn test_engine_stats_default() {
        let stats = EngineStats::default();
        assert_eq!(stats.agents_created, 0);
        assert_eq!(stats.sessions_created, 0);
        assert_eq!(stats.messages_processed, 0);
        assert_eq!(stats.tokens_consumed, 0);
    }
    
    #[test]
    fn test_agent_config_conversion() {
        let default_config = crate::config::AgentDefaultConfig {
            system_prompt: Some("Test prompt".to_string()),
            temperature: 0.8,
            max_tokens: Some(1000),
            timeout: 60,
        };
        
        let agent_config: AgentConfig = default_config.into();
        assert_eq!(agent_config.name, "default");
        assert_eq!(agent_config.system_prompt, Some("Test prompt".to_string()));
        assert_eq!(agent_config.temperature, 0.8);
        assert_eq!(agent_config.max_tokens, Some(1000));
        assert_eq!(agent_config.timeout, 60);
    }
}