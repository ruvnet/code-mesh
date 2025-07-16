//! Authentication storage implementation

use async_trait::async_trait;
use super::{AuthStorage, AuthCredentials};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;

#[derive(Serialize, Deserialize)]
struct AuthData {
    credentials: HashMap<String, AuthCredentials>,
}

#[async_trait]
impl AuthStorage for super::FileAuthStorage {
    async fn get(&self, provider_id: &str) -> crate::Result<Option<AuthCredentials>> {
        if !self.path.exists() {
            return Ok(None);
        }
        
        let data = fs::read_to_string(&self.path).await?;
        let auth_data: AuthData = serde_json::from_str(&data)?;
        
        Ok(auth_data.credentials.get(provider_id).cloned())
    }
    
    async fn set(&self, provider_id: &str, credentials: AuthCredentials) -> crate::Result<()> {
        // Create parent directory if needed
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Load existing data or create new
        let mut auth_data = if self.path.exists() {
            let data = fs::read_to_string(&self.path).await?;
            serde_json::from_str(&data)?
        } else {
            AuthData {
                credentials: HashMap::new(),
            }
        };
        
        // Update credentials
        auth_data.credentials.insert(provider_id.to_string(), credentials);
        
        // Write back with restrictive permissions
        let json = serde_json::to_string_pretty(&auth_data)?;
        fs::write(&self.path, json).await?;
        
        // Set file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.path).await?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.path, perms).await?;
        }
        
        Ok(())
    }
    
    async fn remove(&self, provider_id: &str) -> crate::Result<()> {
        if !self.path.exists() {
            return Ok(());
        }
        
        let data = fs::read_to_string(&self.path).await?;
        let mut auth_data: AuthData = serde_json::from_str(&data)?;
        
        auth_data.credentials.remove(provider_id);
        
        let json = serde_json::to_string_pretty(&auth_data)?;
        fs::write(&self.path, json).await?;
        
        Ok(())
    }
    
    async fn list(&self) -> crate::Result<Vec<String>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        
        let data = fs::read_to_string(&self.path).await?;
        let auth_data: AuthData = serde_json::from_str(&data)?;
        
        Ok(auth_data.credentials.keys().cloned().collect())
    }
}