//! Anthropic authentication implementation

use async_trait::async_trait;
use serde::Deserialize;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use super::{Auth, AuthCredentials, AuthStorage};

const ANTHROPIC_OAUTH_URL: &str = "https://auth.anthropic.com/authorize";
const ANTHROPIC_TOKEN_URL: &str = "https://auth.anthropic.com/oauth/token";
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const REDIRECT_URI: &str = "http://localhost:60023/callback";

/// Anthropic authentication provider
pub struct AnthropicAuth {
    storage: Box<dyn AuthStorage>,
}

impl AnthropicAuth {
    pub fn new(storage: Box<dyn AuthStorage>) -> Self {
        Self { storage }
    }
    
    /// Start OAuth flow with PKCE
    pub async fn start_oauth_flow(&self) -> crate::Result<OAuthFlow> {
        // Generate PKCE challenge
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        
        // Build authorization URL
        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope=read:models&code_challenge={}&code_challenge_method=S256",
            ANTHROPIC_OAUTH_URL,
            CLIENT_ID,
            urlencoding::encode(REDIRECT_URI),
            challenge
        );
        
        Ok(OAuthFlow {
            auth_url,
            verifier,
            state: generate_state(),
        })
    }
    
    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str, verifier: &str) -> crate::Result<TokenResponse> {
        let client = reqwest::Client::new();
        
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", CLIENT_ID),
            ("code_verifier", verifier),
        ];
        
        let response = client
            .post(ANTHROPIC_TOKEN_URL)
            .form(&params)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(crate::Error::AuthenticationFailed(format!(
                "Token exchange failed: {}",
                error
            )));
        }
        
        let tokens: TokenResponse = response.json().await?;
        Ok(tokens)
    }
    
    /// Refresh access token using refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> crate::Result<TokenResponse> {
        let client = reqwest::Client::new();
        
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
        ];
        
        let response = client
            .post(ANTHROPIC_TOKEN_URL)
            .form(&params)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(crate::Error::AuthenticationFailed(format!(
                "Token refresh failed: {}",
                error
            )));
        }
        
        let tokens: TokenResponse = response.json().await?;
        Ok(tokens)
    }
}

#[async_trait]
impl Auth for AnthropicAuth {
    fn provider_id(&self) -> &str {
        "anthropic"
    }
    
    async fn get_credentials(&self) -> crate::Result<AuthCredentials> {
        if let Some(creds) = self.storage.get(self.provider_id()).await? {
            // Check if OAuth token needs refresh
            if creds.is_expired() {
                if let AuthCredentials::OAuth { refresh_token: Some(refresh), .. } = &creds {
                    let tokens = self.refresh_token(refresh).await?;
                    let expires_at = tokens.expires_at();
                    let new_creds = AuthCredentials::OAuth {
                        access_token: tokens.access_token,
                        refresh_token: tokens.refresh_token,
                        expires_at,
                    };
                    self.storage.set(self.provider_id(), new_creds.clone()).await?;
                    return Ok(new_creds);
                }
            }
            Ok(creds)
        } else {
            Err(crate::Error::AuthenticationFailed(
                "No credentials found. Please run 'code-mesh auth login'".to_string()
            ))
        }
    }
    
    async fn set_credentials(&self, credentials: AuthCredentials) -> crate::Result<()> {
        self.storage.set(self.provider_id(), credentials).await
    }
    
    async fn remove_credentials(&self) -> crate::Result<()> {
        self.storage.remove(self.provider_id()).await
    }
    
    async fn has_credentials(&self) -> bool {
        self.storage.get(self.provider_id()).await.ok().flatten().is_some()
    }
}

/// OAuth flow information
#[derive(Debug)]
pub struct OAuthFlow {
    pub auth_url: String,
    pub verifier: String,
    pub state: String,
}

/// Token response from OAuth
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
}

impl TokenResponse {
    pub fn expires_at(&self) -> Option<u64> {
        self.expires_in.map(|expires_in| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + expires_in
        })
    }
}

/// Generate code verifier for PKCE
fn generate_code_verifier() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Generate code challenge from verifier
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(&result)
}

/// Generate random state for OAuth
fn generate_state() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..16).map(|_| rand::thread_rng().gen()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}