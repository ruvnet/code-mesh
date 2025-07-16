//! Authentication manager for Code Mesh

use super::{Auth, AuthCredentials, AuthStorage, FileAuthStorage};
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication manager for managing provider credentials
pub struct AuthManager {
    storage: Box<dyn AuthStorage>,
    providers: Arc<RwLock<HashMap<String, Box<dyn Auth>>>>,
}

impl AuthManager {
    /// Create a new authentication manager
    pub async fn new() -> Result<Self> {
        let storage = FileAuthStorage::default_with_result()?;
        Ok(Self {
            storage: Box::new(storage),
            providers: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Create with custom storage
    pub fn with_storage(storage: Box<dyn AuthStorage>) -> Self {
        Self {
            storage,
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a provider authentication
    pub async fn register_provider(&self, auth: Box<dyn Auth>) {
        let mut providers = self.providers.write().await;
        providers.insert(auth.provider_id().to_string(), auth);
    }
    
    /// Get credentials for a provider
    pub async fn get_credentials(&self, provider_id: &str) -> Result<Option<AuthCredentials>> {
        // First check if we have a provider registered
        let providers = self.providers.read().await;
        if let Some(provider) = providers.get(provider_id) {
            if provider.has_credentials().await {
                return Ok(Some(provider.get_credentials().await?));
            }
        }
        
        // Otherwise check storage
        self.storage.get(provider_id).await
    }
    
    /// Set credentials for a provider
    pub async fn set_credentials(&self, provider_id: &str, credentials: AuthCredentials) -> Result<()> {
        // If we have a provider registered, use it
        let providers = self.providers.read().await;
        if let Some(provider) = providers.get(provider_id) {
            provider.set_credentials(credentials.clone()).await?;
        }
        
        // Always persist to storage
        self.storage.set(provider_id, credentials).await
    }
    
    /// Remove credentials for a provider
    pub async fn remove_credentials(&self, provider_id: &str) -> Result<()> {
        // If we have a provider registered, use it
        let providers = self.providers.read().await;
        if let Some(provider) = providers.get(provider_id) {
            provider.remove_credentials().await?;
        }
        
        // Always remove from storage
        self.storage.remove(provider_id).await
    }
    
    /// List all providers with stored credentials
    pub async fn list_credentials(&self) -> Result<Vec<String>> {
        self.storage.list().await
    }
    
    /// Check if a provider has credentials
    pub async fn has_credentials(&self, provider_id: &str) -> bool {
        // Check registered provider first
        let providers = self.providers.read().await;
        if let Some(provider) = providers.get(provider_id) {
            if provider.has_credentials().await {
                return true;
            }
        }
        
        // Check storage
        self.storage.get(provider_id).await.unwrap_or(None).is_some()
    }
}