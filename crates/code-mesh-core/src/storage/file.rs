//! File-based storage implementation

use async_trait::async_trait;
use super::{Storage, StorageError};
use std::path::PathBuf;
use tokio::fs;

#[async_trait]
impl Storage for super::FileStorage {
    async fn set(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        let path = self.key_to_path(key);
        
        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Write data
        fs::write(path, value).await?;
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let path = self.key_to_path(key);
        
        match fs::read(path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.key_to_path(key);
        
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, StorageError> {
        let mut keys = Vec::new();
        let mut entries = fs::read_dir(&self.base_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(prefix) = prefix {
                    if name.starts_with(prefix) {
                        keys.push(name.to_string());
                    }
                } else {
                    keys.push(name.to_string());
                }
            }
        }
        
        Ok(keys)
    }
    
    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let path = self.key_to_path(key);
        Ok(path.exists())
    }
}