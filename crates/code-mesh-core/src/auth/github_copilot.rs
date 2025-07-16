use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{Auth, AuthCredentials, AuthStorage};

/// GitHub Copilot authentication implementation
pub struct GitHubCopilotAuth {
    storage: Box<dyn AuthStorage>,
}

#[derive(Debug, Serialize)]
struct DeviceCodeRequest {
    client_id: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u32,
    interval: u32,
}

#[derive(Debug, Serialize)]
struct AccessTokenRequest {
    client_id: String,
    device_code: String,
    grant_type: String,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CopilotTokenResponse {
    token: String,
    expires_at: u64,
    refresh_in: u64,
    endpoints: CopilotEndpoints,
}

#[derive(Debug, Deserialize)]
struct CopilotEndpoints {
    api: String,
}

impl GitHubCopilotAuth {
    const CLIENT_ID: &'static str = "Iv1.b507a08c87ecfe98";
    const PROVIDER_ID: &'static str = "github-copilot";
    const DEVICE_CODE_URL: &'static str = "https://github.com/login/device/code";
    const ACCESS_TOKEN_URL: &'static str = "https://github.com/login/oauth/access_token";
    const COPILOT_TOKEN_URL: &'static str = "https://api.github.com/copilot_internal/v2/token";
    
    pub fn new(storage: Box<dyn AuthStorage>) -> Self {
        Self { storage }
    }
    
    /// Start device code flow for GitHub authentication
    pub async fn start_device_flow() -> crate::Result<DeviceCodeResponse> {
        let client = reqwest::Client::new();
        let request = DeviceCodeRequest {
            client_id: Self::CLIENT_ID.to_string(),
            scope: "read:user".to_string(),
        };
        
        let response = client
            .post(Self::DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "GitHubCopilotChat/0.26.7")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Device code request failed: {}", e)))?;
            
        if !response.status().is_success() {
            return Err(crate::Error::Other(anyhow::anyhow!(
                "Device code request failed with status: {}",
                response.status()
            )));
        }
        
        let device_response: DeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Failed to parse device code response: {}", e)))?;
            
        Ok(device_response)
    }
    
    /// Poll for GitHub access token during device flow
    pub async fn poll_for_token(device_code: &str) -> crate::Result<GitHubCopilotAuthResult> {
        let client = reqwest::Client::new();
        let request = AccessTokenRequest {
            client_id: Self::CLIENT_ID.to_string(),
            device_code: device_code.to_string(),
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
        };
        
        let response = client
            .post(Self::ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "GitHubCopilotChat/0.26.7")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Token poll request failed: {}", e)))?;
            
        if !response.status().is_success() {
            return Ok(GitHubCopilotAuthResult::Failed);
        }
        
        let token_response: AccessTokenResponse = response
            .json()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Failed to parse token response: {}", e)))?;
            
        if let Some(access_token) = token_response.access_token {
            Ok(GitHubCopilotAuthResult::Complete(access_token))
        } else if token_response.error.as_deref() == Some("authorization_pending") {
            Ok(GitHubCopilotAuthResult::Pending)
        } else {
            Ok(GitHubCopilotAuthResult::Failed)
        }
    }
    
    /// Exchange GitHub OAuth token for Copilot API token
    pub async fn get_copilot_token(github_token: &str) -> crate::Result<AuthCredentials> {
        let client = reqwest::Client::new();
        
        let response = client
            .get(Self::COPILOT_TOKEN_URL)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", github_token))
            .header("User-Agent", "GitHubCopilotChat/0.26.7")
            .header("Editor-Version", "vscode/1.99.3")
            .header("Editor-Plugin-Version", "copilot-chat/0.26.7")
            .send()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Copilot token request failed: {}", e)))?;
            
        if !response.status().is_success() {
            return Err(crate::Error::Other(anyhow::anyhow!(
                "Copilot token request failed with status: {}",
                response.status()
            )));
        }
        
        let token_response: CopilotTokenResponse = response
            .json()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Failed to parse copilot token response: {}", e)))?;
            
        Ok(AuthCredentials::OAuth {
            access_token: token_response.token,
            refresh_token: Some(github_token.to_string()), // Store GitHub token for refresh
            expires_at: Some(token_response.expires_at),
        })
    }
    
    /// Get valid Copilot access token, refreshing if necessary
    pub async fn get_access_token(&self) -> crate::Result<Option<String>> {
        let credentials = match self.storage.get(Self::PROVIDER_ID).await? {
            Some(creds) => creds,
            None => return Ok(None),
        };
        
        match credentials {
            AuthCredentials::OAuth { access_token, refresh_token, expires_at } => {
                // Check if token is expired
                if let Some(exp) = expires_at {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    
                    if now >= exp {
                        // Token expired, try to refresh using GitHub token
                        if let Some(github_token) = refresh_token {
                            match Self::get_copilot_token(&github_token).await {
                                Ok(new_creds) => {
                                    self.storage.set(Self::PROVIDER_ID, new_creds.clone()).await?;
                                    if let AuthCredentials::OAuth { access_token, .. } = new_creds {
                                        return Ok(Some(access_token));
                                    }
                                }
                                Err(_) => {
                                    // Refresh failed, remove credentials
                                    self.storage.remove(Self::PROVIDER_ID).await?;
                                    return Ok(None);
                                }
                            }
                        } else {
                            return Ok(None);
                        }
                    }
                }
                
                Ok(Some(access_token))
            }
            _ => Ok(None),
        }
    }
}

#[async_trait]
impl Auth for GitHubCopilotAuth {
    fn provider_id(&self) -> &str {
        Self::PROVIDER_ID
    }
    
    async fn get_credentials(&self) -> crate::Result<AuthCredentials> {
        match self.storage.get(Self::PROVIDER_ID).await? {
            Some(creds) => {
                // Ensure credentials are fresh
                if creds.is_expired() {
                    match &creds {
                        AuthCredentials::OAuth { refresh_token: Some(github_token), .. } => {
                            let new_creds = Self::get_copilot_token(github_token).await?;
                            self.storage.set(Self::PROVIDER_ID, new_creds.clone()).await?;
                            Ok(new_creds)
                        }
                        _ => Err(crate::Error::Other(anyhow::anyhow!("Credentials expired and cannot be refreshed"))),
                    }
                } else {
                    Ok(creds)
                }
            }
            None => Err(crate::Error::Other(anyhow::anyhow!("No credentials found for GitHub Copilot"))),
        }
    }
    
    async fn set_credentials(&self, credentials: AuthCredentials) -> crate::Result<()> {
        self.storage.set(Self::PROVIDER_ID, credentials).await
    }
    
    async fn remove_credentials(&self) -> crate::Result<()> {
        self.storage.remove(Self::PROVIDER_ID).await
    }
    
    async fn has_credentials(&self) -> bool {
        self.storage.get(Self::PROVIDER_ID).await
            .map(|opt| opt.is_some())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub enum GitHubCopilotAuthResult {
    Pending,
    Complete(String), // GitHub OAuth token
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum GitHubCopilotError {
    #[error("Device code flow failed")]
    DeviceCodeFailed,
    
    #[error("Token exchange failed")]
    TokenExchangeFailed,
    
    #[error("Authentication expired")]
    AuthenticationExpired,
    
    #[error("Copilot token request failed")]
    CopilotTokenFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_constants() {
        assert_eq!(GitHubCopilotAuth::CLIENT_ID, "Iv1.b507a08c87ecfe98");
        assert_eq!(GitHubCopilotAuth::PROVIDER_ID, "github-copilot");
        assert!(GitHubCopilotAuth::DEVICE_CODE_URL.contains("github.com"));
    }
    
    #[test]
    fn test_auth_result() {
        match GitHubCopilotAuthResult::Pending {
            GitHubCopilotAuthResult::Pending => (),
            _ => panic!("Expected Pending"),
        }
        
        match GitHubCopilotAuthResult::Complete("token".to_string()) {
            GitHubCopilotAuthResult::Complete(token) => assert_eq!(token, "token"),
            _ => panic!("Expected Complete"),
        }
    }
}