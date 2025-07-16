//! Authentication system for Code Mesh

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    path: std::path::PathBuf,
}

impl FileAuthStorage {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }
    
    pub fn default() -> crate::Result<Self> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| crate::Error::Other(anyhow::anyhow!("Could not find data directory")))?;
        let path = data_dir.join("code-mesh").join("auth.json");
        Ok(Self::new(path))
    }
}