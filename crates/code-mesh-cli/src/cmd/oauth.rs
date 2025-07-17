//! OAuth implementation for Code Mesh CLI

use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use url::Url;

/// PKCE challenge and verifier
#[derive(Debug)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

impl PkceChallenge {
    /// Generate a new PKCE challenge
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let verifier: String = (0..128)
            .map(|_| {
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
                chars[rng.gen_range(0..chars.len())] as char
            })
            .collect();
        
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
        
        Self { verifier, challenge }
    }
}

/// Anthropic OAuth implementation
pub struct AnthropicOAuth {
    client: Client,
    client_id: String,
}

impl AnthropicOAuth {
    /// Create a new Anthropic OAuth client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_string(),
        }
    }

    /// Generate authorization URL for Claude Pro/Max
    pub fn authorize_url_max(&self, pkce: &PkceChallenge) -> Result<String> {
        let mut url = Url::parse("https://claude.ai/oauth/authorize")?;
        url.query_pairs_mut()
            .append_pair("code", "true")
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", "https://console.anthropic.com/oauth/code/callback")
            .append_pair("scope", "org:create_api_key user:profile user:inference")
            .append_pair("code_challenge", &pkce.challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &pkce.verifier);
        
        Ok(url.to_string())
    }

    /// Generate authorization URL for Console (API Key creation)
    pub fn authorize_url_console(&self, pkce: &PkceChallenge) -> Result<String> {
        let mut url = Url::parse("https://console.anthropic.com/oauth/authorize")?;
        url.query_pairs_mut()
            .append_pair("code", "true")
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", "https://console.anthropic.com/oauth/code/callback")
            .append_pair("scope", "org:create_api_key user:profile user:inference")
            .append_pair("code_challenge", &pkce.challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &pkce.verifier);
        
        Ok(url.to_string())
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str, verifier: &str) -> Result<TokenResponse> {
        let code_parts: Vec<&str> = code.split('#').collect();
        let auth_code = code_parts[0];
        let state = code_parts.get(1).unwrap_or(&verifier);

        let mut params = HashMap::new();
        params.insert("code", auth_code);
        params.insert("state", state);
        params.insert("grant_type", "authorization_code");
        params.insert("client_id", &self.client_id);
        params.insert("redirect_uri", "https://console.anthropic.com/oauth/code/callback");
        params.insert("code_verifier", verifier);

        let response = self.client
            .post("https://console.anthropic.com/v1/oauth/token")
            .json(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Token exchange failed: {}", response.status()));
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    /// Create API key using OAuth access token
    pub async fn create_api_key(&self, access_token: &str) -> Result<ApiKeyResponse> {
        let response = self.client
            .post("https://api.anthropic.com/api/oauth/claude_cli/create_api_key")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json, text/plain, */*")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("API key creation failed: {}", response.status()));
        }

        let api_key_response: ApiKeyResponse = response.json().await?;
        Ok(api_key_response)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub raw_key: String,
    pub key_id: String,
}

/// GitHub OAuth implementation (placeholder)
pub struct GitHubOAuth {
    client: Client,
}

impl GitHubOAuth {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn start_device_flow(&self) -> Result<DeviceCodeResponse> {
        // This would implement GitHub's device flow
        // For now, return a placeholder
        Ok(DeviceCodeResponse {
            device_code: "placeholder".to_string(),
            user_code: "ABCD-EFGH".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            expires_in: 900,
            interval: 5,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u32,
    pub interval: u32,
}