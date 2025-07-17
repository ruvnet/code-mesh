//! Authentication system for Code Mesh

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod anthropic;
pub mod storage;
pub mod github_copilot;
pub mod manager;

pub use anthropic::AnthropicAuth;
pub use github_copilot::{GitHubCopilotAuth, GitHubCopilotAuthResult};
pub use manager::AuthManager;

/// Authentication trait for provider credentials
#[async_trait]
pub trait Auth: Send + Sync {
    /// Get the provider ID this auth is for
    fn provider_id(&self) -> &str;
    
    /// Get current valid credentials
    async fn get_credentials(&self) -> crate::Result<AuthCredentials>;
    
    /// Store new credentials
    async fn set_credentials(&self, credentials: AuthCredentials) -> crate::Result<()>;
    
    /// Remove stored credentials
    async fn remove_credentials(&self) -> crate::Result<()>;
    
    /// Check if credentials are available
    async fn has_credentials(&self) -> bool;
}

/// Authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthCredentials {
    /// API key authentication
    ApiKey {
        key: String,
    },
    
    /// OAuth authentication
    OAuth {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<u64>,
    },
    
    /// Custom authentication
    Custom {
        data: HashMap<String, serde_json::Value>,
    },
}

impl AuthCredentials {
    /// Check if credentials are expired
    pub fn is_expired(&self) -> bool {
        match self {
            AuthCredentials::OAuth { expires_at: Some(exp), .. } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now >= *exp
            }
            _ => false,
        }
    }
    
    /// Create API key credentials
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey { key: key.into() }
    }
    
    /// Create OAuth credentials
    pub fn oauth(
        access_token: impl Into<String>,
        refresh_token: Option<impl Into<String>>,
        expires_at: Option<u64>,
    ) -> Self {
        Self::OAuth {
            access_token: access_token.into(),
            refresh_token: refresh_token.map(|t| t.into()),
            expires_at,
        }
    }
}

/// Authentication storage trait
#[async_trait]
pub trait AuthStorage: Send + Sync {
    /// Get credentials for a provider
    async fn get(&self, provider_id: &str) -> crate::Result<Option<AuthCredentials>>;
    
    /// Set credentials for a provider
    async fn set(&self, provider_id: &str, credentials: AuthCredentials) -> crate::Result<()>;
    
    /// Remove credentials for a provider
    async fn remove(&self, provider_id: &str) -> crate::Result<()>;
    
    /// List all stored provider IDs
    async fn list(&self) -> crate::Result<Vec<String>>;
}

/// File-based authentication storage
pub struct FileAuthStorage {
    path: PathBuf,
}

impl FileAuthStorage {
    /// Create a new file-based auth storage
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    
    /// Create default file-based auth storage with error handling
    pub fn default_with_result() -> crate::Result<Self> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| crate::Error::Storage(crate::storage::StorageError::Other("Could not determine home directory".to_string())))?;
        let path = home_dir.join(".code-mesh").join("auth.json");
        Ok(Self::new(path))
    }
}

impl Default for FileAuthStorage {
    fn default() -> Self {
        // Use a default path in the user's home directory
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let path = home_dir.join(".code-mesh").join("auth.json");
        Self::new(path)
    }
}

/// Shared authentication storage wrapper
/// This allows us to use Arc<dyn AuthStorage> with code that expects Box<dyn AuthStorage>
pub struct SharedAuthStorage {
    inner: std::sync::Arc<dyn AuthStorage>,
}

impl SharedAuthStorage {
    pub fn new(storage: std::sync::Arc<dyn AuthStorage>) -> Self {
        Self { inner: storage }
    }
}

#[async_trait]
impl AuthStorage for SharedAuthStorage {
    async fn get(&self, provider_id: &str) -> crate::Result<Option<AuthCredentials>> {
        self.inner.get(provider_id).await
    }
    
    async fn set(&self, provider_id: &str, credentials: AuthCredentials) -> crate::Result<()> {
        self.inner.set(provider_id, credentials).await
    }
    
    async fn remove(&self, provider_id: &str) -> crate::Result<()> {
        self.inner.remove(provider_id).await
    }
    
    async fn list(&self) -> crate::Result<Vec<String>> {
        self.inner.list().await
    }
}

