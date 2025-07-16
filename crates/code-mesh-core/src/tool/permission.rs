//! Permission system for tool execution
//! Provides user confirmation prompts and access control for destructive operations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Permission request for destructive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub metadata: Value,
    pub risk_level: RiskLevel,
}

/// Risk levels for different operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,      // Read operations, safe writes
    Medium,   // File modifications, process execution
    High,     // System modifications, dangerous commands
    Critical, // Irreversible operations
}

/// Result of a permission check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionResult {
    Granted,
    Denied(String),
    RequiresConfirmation(PermissionRequest),
}

/// Permission provider trait
#[async_trait]
pub trait PermissionProvider: Send + Sync {
    /// Check if an operation is permitted
    async fn check_permission(&self, request: &PermissionRequest) -> PermissionResult;
    
    /// Grant permission for a specific request
    async fn grant_permission(&self, request_id: &str) -> Result<(), String>;
    
    /// Deny permission for a specific request
    async fn deny_permission(&self, request_id: &str, reason: String) -> Result<(), String>;
}

/// Interactive permission provider that prompts users
pub struct InteractivePermissionProvider {
    pending_requests: Arc<RwLock<HashMap<String, PermissionRequest>>>,
    auto_approve_low_risk: bool,
}

impl InteractivePermissionProvider {
    pub fn new(auto_approve_low_risk: bool) -> Self {
        Self {
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            auto_approve_low_risk,
        }
    }
}

#[async_trait]
impl PermissionProvider for InteractivePermissionProvider {
    async fn check_permission(&self, request: &PermissionRequest) -> PermissionResult {
        // Auto-approve low-risk operations if configured
        if self.auto_approve_low_risk && request.risk_level == RiskLevel::Low {
            return PermissionResult::Granted;
        }
        
        // For higher risk operations, require explicit confirmation
        let mut pending = self.pending_requests.write().await;
        pending.insert(request.id.clone(), request.clone());
        
        PermissionResult::RequiresConfirmation(request.clone())
    }
    
    async fn grant_permission(&self, request_id: &str) -> Result<(), String> {
        let mut pending = self.pending_requests.write().await;
        pending.remove(request_id);
        Ok(())
    }
    
    async fn deny_permission(&self, request_id: &str, reason: String) -> Result<(), String> {
        let mut pending = self.pending_requests.write().await;
        pending.remove(request_id);
        Err(reason)
    }
}

/// Auto-approve permission provider for testing
pub struct AutoApprovePermissionProvider;

#[async_trait]
impl PermissionProvider for AutoApprovePermissionProvider {
    async fn check_permission(&self, _request: &PermissionRequest) -> PermissionResult {
        PermissionResult::Granted
    }
    
    async fn grant_permission(&self, _request_id: &str) -> Result<(), String> {
        Ok(())
    }
    
    async fn deny_permission(&self, _request_id: &str, reason: String) -> Result<(), String> {
        Err(reason)
    }
}

/// Global permission manager
pub struct PermissionManager {
    provider: Box<dyn PermissionProvider>,
}

impl PermissionManager {
    pub fn new(provider: Box<dyn PermissionProvider>) -> Self {
        Self { provider }
    }
    
    /// Ask for permission to perform an operation
    pub async fn ask(&self, request: PermissionRequest) -> Result<(), crate::error::Error> {
        match self.provider.check_permission(&request).await {
            PermissionResult::Granted => Ok(()),
            PermissionResult::Denied(reason) => {
                Err(crate::error::Error::Other(anyhow::anyhow!(
                    "Permission denied: {}",
                    reason
                )))
            }
            PermissionResult::RequiresConfirmation(_) => {
                // In a real implementation, this would trigger a UI prompt
                // For now, we'll auto-deny operations requiring confirmation
                Err(crate::error::Error::Other(anyhow::anyhow!(
                    "Operation requires user confirmation: {}",
                    request.title
                )))
            }
        }
    }
}

/// Helper function to create permission requests
pub fn create_permission_request(
    id: impl Into<String>,
    session_id: impl Into<String>,
    title: impl Into<String>,
    risk_level: RiskLevel,
    metadata: Value,
) -> PermissionRequest {
    PermissionRequest {
        id: id.into(),
        session_id: session_id.into(),
        title: title.into(),
        description: None,
        metadata,
        risk_level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_auto_approve_low_risk() {
        let provider = InteractivePermissionProvider::new(true);
        let request = create_permission_request(
            "test",
            "session1",
            "Read file",
            RiskLevel::Low,
            json!({}),
        );
        
        let result = provider.check_permission(&request).await;
        assert!(matches!(result, PermissionResult::Granted));
    }
    
    #[tokio::test]
    async fn test_requires_confirmation_high_risk() {
        let provider = InteractivePermissionProvider::new(true);
        let request = create_permission_request(
            "test",
            "session1",
            "Delete system files",
            RiskLevel::Critical,
            json!({}),
        );
        
        let result = provider.check_permission(&request).await;
        assert!(matches!(result, PermissionResult::RequiresConfirmation(_)));
    }
    
    #[tokio::test]
    async fn test_auto_approve_provider() {
        let provider = AutoApprovePermissionProvider;
        let request = create_permission_request(
            "test",
            "session1",
            "Any operation",
            RiskLevel::Critical,
            json!({}),
        );
        
        let result = provider.check_permission(&request).await;
        assert!(matches!(result, PermissionResult::Granted));
    }
}