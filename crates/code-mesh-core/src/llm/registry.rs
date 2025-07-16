use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow;

use super::{
    ProviderRegistry, ModelConfig, ProviderConfig, ProviderSource, 
    AnthropicProvider, OpenAIProvider, GitHubCopilotProvider,
    AnthropicModelWithProvider, OpenAIModelWithProvider, GitHubCopilotModelWithProvider,
    LanguageModel,
};
use crate::auth::{AuthStorage, AnthropicAuth, GitHubCopilotAuth};

/// Central registry for managing LLM providers and models
pub struct LLMRegistry {
    provider_registry: ProviderRegistry,
    model_cache: Arc<RwLock<HashMap<String, Arc<dyn LanguageModel>>>>,
}

impl LLMRegistry {
    /// Create new LLM registry with authentication storage
    pub fn new(storage: Arc<dyn AuthStorage>) -> Self {
        Self {
            provider_registry: ProviderRegistry::new(storage),
            model_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Initialize the registry with default configurations
    pub async fn initialize(&mut self) -> crate::Result<()> {
        // Load default model configurations
        self.load_default_configs().await?;
        
        // Discover providers from environment and storage
        self.provider_registry.discover_from_env().await?;
        self.provider_registry.discover_from_storage().await?;
        
        // Initialize all discovered providers
        self.provider_registry.initialize_all().await?;
        
        Ok(())
    }
    
    /// Load configurations from models.dev API
    pub async fn load_models_dev_configs(&mut self) -> crate::Result<()> {
        self.provider_registry.load_models_dev().await
    }
    
    /// Load configurations from file
    pub async fn load_config_file(&mut self, path: &str) -> crate::Result<()> {
        self.provider_registry.load_configs(path).await
    }
    
    /// Get a model by provider and model ID
    pub async fn get_model(&self, provider_id: &str, model_id: &str) -> crate::Result<Arc<dyn LanguageModel>> {
        let cache_key = format!("{}:{}", provider_id, model_id);
        
        // Check cache first
        {
            let cache = self.model_cache.read().await;
            if let Some(model) = cache.get(&cache_key) {
                return Ok(model.clone());
            }
        }
        
        // Get provider and create model
        let provider = self.provider_registry.get(provider_id).await
            .ok_or_else(|| crate::Error::Other(anyhow::anyhow!("Provider not found: {}", provider_id)))?;
            
        let model = provider.get_model(model_id).await?;
        
        // We can't cast Arc<dyn Model> to Arc<dyn LanguageModel> directly
        // For now, return an error indicating this design issue
        return Err(crate::Error::Other(anyhow::anyhow!(
            "Model trait and LanguageModel trait are incompatible - cannot cast between them"
        )));
    }
    
    /// Get model from string (provider/model or just model)
    pub async fn get_model_from_string(&self, model_str: &str) -> crate::Result<Arc<dyn LanguageModel>> {
        let (provider_id, model_id) = ProviderRegistry::parse_model(model_str);
        self.get_model(&provider_id, &model_id).await
    }
    
    /// Get default model for a provider
    pub async fn get_default_model(&self, provider_id: &str) -> crate::Result<Arc<dyn LanguageModel>> {
        let model = self.provider_registry.get_default_model(provider_id).await?;
        // We can't cast Arc<dyn Model> to Arc<dyn LanguageModel> directly
        // For now, return an error indicating this design issue
        return Err(crate::Error::Other(anyhow::anyhow!(
            "Model trait and LanguageModel trait are incompatible - cannot cast between them"
        )));
    }
    
    /// Get the best available model across all providers
    pub async fn get_best_model(&self) -> crate::Result<Arc<dyn LanguageModel>> {
        let available_providers = self.provider_registry.available().await;
        
        if available_providers.is_empty() {
            return Err(crate::Error::Other(anyhow::anyhow!("No providers available")));
        }
        
        // Priority order for providers
        let provider_priority = ["anthropic", "openai", "github-copilot"];
        
        for provider_id in provider_priority {
            if available_providers.contains(&provider_id.to_string()) {
                if let Ok(model) = self.get_default_model(provider_id).await {
                    return Ok(model);
                }
            }
        }
        
        // Fall back to first available provider
        self.get_default_model(&available_providers[0]).await
    }
    
    /// List all available providers
    pub async fn list_providers(&self) -> Vec<String> {
        self.provider_registry.list().await
    }
    
    /// List available providers (those that can authenticate)
    pub async fn list_available_providers(&self) -> Vec<String> {
        self.provider_registry.available().await
    }
    
    /// List models for a provider
    pub async fn list_models(&self, provider_id: &str) -> crate::Result<Vec<ModelConfig>> {
        let provider = self.provider_registry.get(provider_id).await
            .ok_or_else(|| crate::Error::Other(anyhow::anyhow!("Provider not found: {}", provider_id)))?;
            
        let model_infos = provider.list_models().await?;
        
        // Convert ModelInfo to ModelConfig
        Ok(model_infos.into_iter().map(|info| ModelConfig {
            model_id: info.id,
            ..Default::default()
        }).collect())
    }
    
    /// Clear model cache
    pub async fn clear_cache(&self) {
        let mut cache = self.model_cache.write().await;
        cache.clear();
    }
    
    /// Get cache statistics
    pub async fn cache_stats(&self) -> HashMap<String, usize> {
        let cache = self.model_cache.read().await;
        let mut stats = HashMap::new();
        stats.insert("cached_models".to_string(), cache.len());
        stats
    }
    
    /// Load default provider configurations
    async fn load_default_configs(&mut self) -> crate::Result<()> {
        // This would load built-in configurations for known providers
        // For now, providers have their default models built-in
        Ok(())
    }
    
    /// Register a custom provider
    pub async fn register_provider(&mut self, provider: Arc<dyn super::Provider>) {
        self.provider_registry.register(provider).await;
    }
}

/// Helper function to create an LLM registry with file-based auth storage
pub async fn create_default_registry() -> crate::Result<LLMRegistry> {
    let storage = Arc::new(crate::auth::FileAuthStorage::default_with_result()?) as Arc<dyn AuthStorage>;
    let mut registry = LLMRegistry::new(storage);
    registry.initialize().await?;
    Ok(registry)
}

/// Helper function to create registry with models.dev configurations
pub async fn create_registry_with_models_dev() -> crate::Result<LLMRegistry> {
    let mut registry = create_default_registry().await?;
    registry.load_models_dev_configs().await?;
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::storage::FileAuthStorage;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_registry_creation() {
        let temp_dir = tempdir().unwrap();
        let auth_path = temp_dir.path().join("auth.json");
        let storage = Arc::new(FileAuthStorage::new(auth_path));
        
        let registry = LLMRegistry::new(storage);
        let providers = registry.list_providers().await;
        
        // Initially empty
        assert_eq!(providers, Vec::<String>::new());
    }
    
    #[tokio::test]
    async fn test_cache_operations() {
        let temp_dir = tempdir().unwrap();
        let auth_path = temp_dir.path().join("auth.json");
        let storage = Arc::new(FileAuthStorage::new(auth_path));
        
        let registry = LLMRegistry::new(storage);
        
        // Check empty cache
        let stats = registry.cache_stats().await;
        assert_eq!(stats.get("cached_models"), Some(&0));
        
        // Clear empty cache
        registry.clear_cache().await;
        let stats = registry.cache_stats().await;
        assert_eq!(stats.get("cached_models"), Some(&0));
    }
}