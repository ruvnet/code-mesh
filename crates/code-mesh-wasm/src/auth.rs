//! Authentication and security utilities for WASM

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Storage, CryptoKey, SubtleCrypto};
use js_sys::{Object, Promise, Uint8Array, Array};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Authentication manager for handling various auth providers
#[wasm_bindgen]
pub struct AuthManager {
    provider: String,
    token_storage_key: String,
    storage: Option<Storage>,
    current_token: Option<String>,
    crypto: Option<SubtleCrypto>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub token_type: String,
    pub scope: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthConfig {
    pub provider: String,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub auto_refresh: bool,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(provider: String) -> Self {
        let storage = window()
            .and_then(|w| w.local_storage().ok())
            .flatten();
        
        let crypto = window()
            .and_then(|w| w.crypto().ok())
            .and_then(|c| c.subtle().ok());
        
        Self {
            provider: provider.clone(),
            token_storage_key: format!("code_mesh_auth_{}", provider),
            storage,
            current_token: None,
            crypto,
        }
    }
    
    /// Initialize authentication
    pub async fn initialize(&mut self) -> Result<(), JsValue> {
        // Try to load existing token from storage
        if let Some(storage) = &self.storage {
            if let Ok(Some(token_str)) = storage.get_item(&self.token_storage_key) {
                // Decrypt and parse token
                if let Ok(decrypted) = self.decrypt_token(&token_str).await {
                    if let Ok(token) = serde_json::from_str::<AuthToken>(&decrypted) {
                        // Check if token is still valid
                        if self.is_token_valid(&token) {
                            self.current_token = Some(token.access_token);
                        } else {
                            // Try to refresh if possible
                            if let Some(refresh_token) = token.refresh_token {
                                if let Ok(new_token) = self.refresh_token(&refresh_token).await {
                                    self.store_token(&new_token).await?;
                                    self.current_token = Some(new_token.access_token);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if token is valid (not expired)
    fn is_token_valid(&self, token: &AuthToken) -> bool {
        if let Some(expires_at) = &token.expires_at {
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                return chrono::Utc::now() < expires.with_timezone(&chrono::Utc);
            }
        }
        true // Assume valid if no expiry info
    }
    
    /// Store token securely
    async fn store_token(&self, token: &AuthToken) -> Result<(), JsValue> {
        if let Some(storage) = &self.storage {
            let token_json = serde_json::to_string(token)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            
            let encrypted = self.encrypt_token(&token_json).await?;
            storage.set_item(&self.token_storage_key, &encrypted)?;
        }
        Ok(())
    }
    
    /// Encrypt token for storage
    async fn encrypt_token(&self, token: &str) -> Result<String, JsValue> {
        if let Some(crypto) = &self.crypto {
            // Generate a key for encryption
            let key_params = Object::new();
            js_sys::Reflect::set(&key_params, &"name".into(), &"AES-GCM".into())?;
            js_sys::Reflect::set(&key_params, &"length".into(), &256.into())?;
            
            let key_promise = crypto.generate_key_with_object(&key_params, true, &Array::of2(&"encrypt".into(), &"decrypt".into()))?;
            let key = JsFuture::from(key_promise).await?;
            let crypto_key: CryptoKey = key.dyn_into()?;
            
            // Generate IV
            let iv = crypto.get_random_values_with_u8_array(&mut [0u8; 12])?;
            
            // Encrypt
            let algorithm = Object::new();
            js_sys::Reflect::set(&algorithm, &"name".into(), &"AES-GCM".into())?;
            js_sys::Reflect::set(&algorithm, &"iv".into(), &iv)?;
            
            let data = Uint8Array::from(token.as_bytes());
            let encrypt_promise = crypto.encrypt_with_object_and_buffer_source(&algorithm, &crypto_key, &data)?;
            let encrypted_buffer = JsFuture::from(encrypt_promise).await?;
            
            // Convert to base64
            let encrypted_array = Uint8Array::new(&encrypted_buffer);
            let mut encrypted_bytes = vec![0u8; encrypted_array.length() as usize];
            encrypted_array.copy_to(&mut encrypted_bytes);
            
            Ok(base64::encode(&encrypted_bytes))
        } else {
            // Fallback to base64 encoding (not secure!)
            Ok(base64::encode(token.as_bytes()))
        }
    }
    
    /// Decrypt token from storage
    async fn decrypt_token(&self, encrypted_token: &str) -> Result<String, JsValue> {
        if self.crypto.is_some() {
            // Implement decryption logic
            // For now, fall back to base64 decode
            base64::decode(encrypted_token)
                .map_err(|e| JsValue::from_str(&e.to_string()))
                .and_then(|bytes| {
                    String::from_utf8(bytes)
                        .map_err(|e| JsValue::from_str(&e.to_string()))
                })
        } else {
            // Fallback to base64 decode
            base64::decode(encrypted_token)
                .map_err(|e| JsValue::from_str(&e.to_string()))
                .and_then(|bytes| {
                    String::from_utf8(bytes)
                        .map_err(|e| JsValue::from_str(&e.to_string()))
                })
        }
    }
    
    /// Authenticate with OAuth2 flow
    pub async fn authenticate_oauth2(&mut self, config: AuthConfig) -> Result<AuthToken, JsValue> {
        let redirect_uri = config.redirect_uri.unwrap_or_else(|| {
            window()
                .and_then(|w| w.location().href().ok())
                .unwrap_or_default()
        });
        
        let auth_url = self.build_auth_url(&config, &redirect_uri)?;
        
        // Open popup for authentication
        let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let popup = window.open_with_url_and_target_and_features(
            &auth_url,
            "_blank",
            "width=500,height=600,scrollbars=yes,resizable=yes",
        )?;
        
        // Wait for callback
        let token = self.wait_for_oauth_callback(popup).await?;
        
        // Store token
        self.store_token(&token).await?;
        self.current_token = Some(token.access_token.clone());
        
        Ok(token)
    }
    
    /// Build OAuth2 authorization URL
    fn build_auth_url(&self, config: &AuthConfig, redirect_uri: &str) -> Result<String, JsValue> {
        let client_id = config.client_id.as_ref()
            .ok_or_else(|| JsValue::from_str("Client ID required for OAuth2"))?;
        
        let mut url = match self.provider.as_str() {
            "github" => "https://github.com/login/oauth/authorize".to_string(),
            "google" => "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            "microsoft" => "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
            _ => return Err(JsValue::from_str("Unsupported OAuth2 provider")),
        };
        
        url.push_str(&format!("?client_id={}", client_id));
        url.push_str(&format!("&redirect_uri={}", redirect_uri));
        url.push_str("&response_type=code");
        
        if let Some(scope) = &config.scope {
            url.push_str(&format!("&scope={}", scope));
        }
        
        // Add state for security
        let state = self.generate_state();
        url.push_str(&format!("&state={}", state));
        
        Ok(url)
    }
    
    /// Generate random state for OAuth2
    fn generate_state(&self) -> String {
        let mut bytes = [0u8; 32];
        if let Some(crypto) = window().and_then(|w| w.crypto().ok()) {
            let _ = crypto.get_random_values_with_u8_array(&mut bytes);
        }
        base64::encode(&bytes)
    }
    
    /// Wait for OAuth2 callback
    async fn wait_for_oauth_callback(&self, popup: Option<web_sys::Window>) -> Result<AuthToken, JsValue> {
        // This is a simplified implementation
        // In practice, you'd need to handle the callback properly
        
        // For now, return a mock token
        Ok(AuthToken {
            access_token: "mock_token".to_string(),
            refresh_token: Some("mock_refresh".to_string()),
            expires_at: Some(
                (chrono::Utc::now() + chrono::Duration::hours(1))
                    .to_rfc3339()
            ),
            token_type: "Bearer".to_string(),
            scope: None,
        })
    }
    
    /// Authenticate with API key
    pub fn authenticate_api_key(&mut self, api_key: String) -> Result<(), JsValue> {
        // Validate API key format based on provider
        match self.provider.as_str() {
            "anthropic" => {
                if !api_key.starts_with("sk-ant-") {
                    return Err(JsValue::from_str("Invalid Anthropic API key format"));
                }
            }
            "openai" => {
                if !api_key.starts_with("sk-") {
                    return Err(JsValue::from_str("Invalid OpenAI API key format"));
                }
            }
            _ => {} // No specific validation
        }
        
        self.current_token = Some(api_key);
        Ok(())
    }
    
    /// Refresh access token
    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, JsValue> {
        // Implement token refresh logic based on provider
        // For now, return a mock token
        Ok(AuthToken {
            access_token: "refreshed_token".to_string(),
            refresh_token: Some(refresh_token.to_string()),
            expires_at: Some(
                (chrono::Utc::now() + chrono::Duration::hours(1))
                    .to_rfc3339()
            ),
            token_type: "Bearer".to_string(),
            scope: None,
        })
    }
    
    /// Get current access token
    pub fn get_access_token(&self) -> Option<String> {
        self.current_token.clone()
    }
    
    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        self.current_token.is_some()
    }
    
    /// Sign out
    pub async fn sign_out(&mut self) -> Result<(), JsValue> {
        self.current_token = None;
        
        if let Some(storage) = &self.storage {
            storage.remove_item(&self.token_storage_key)?;
        }
        
        Ok(())
    }
    
    /// Get user info (if supported by provider)
    pub async fn get_user_info(&self) -> Result<JsValue, JsValue> {
        let token = self.current_token.as_ref()
            .ok_or_else(|| JsValue::from_str("Not authenticated"))?;
        
        let endpoint = match self.provider.as_str() {
            "github" => "https://api.github.com/user",
            "google" => "https://www.googleapis.com/oauth2/v2/userinfo",
            _ => return Err(JsValue::from_str("User info not supported for this provider")),
        };
        
        // Make API request
        let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let fetch_promise = window.fetch_with_str(endpoint);
        let response = JsFuture::from(fetch_promise).await?;
        let response: web_sys::Response = response.dyn_into()?;
        
        if !response.ok() {
            return Err(JsValue::from_str("Failed to get user info"));
        }
        
        let json_promise = response.json()?;
        JsFuture::from(json_promise).await
    }
}

#[wasm_bindgen]
impl AuthManager {
    /// Create new auth manager (WASM constructor)
    #[wasm_bindgen(constructor)]
    pub fn new_wasm(provider: String) -> AuthManager {
        Self::new(provider)
    }
    
    /// Initialize (WASM method)
    #[wasm_bindgen]
    pub async fn initialize_wasm(&mut self) -> Result<(), JsValue> {
        self.initialize().await
    }
    
    /// Authenticate with API key (WASM method)
    #[wasm_bindgen]
    pub fn authenticate_api_key_wasm(&mut self, api_key: String) -> Result<(), JsValue> {
        self.authenticate_api_key(api_key)
    }
    
    /// Get access token (WASM method)
    #[wasm_bindgen]
    pub fn get_access_token_wasm(&self) -> Option<String> {
        self.get_access_token()
    }
    
    /// Check authentication status (WASM method)
    #[wasm_bindgen]
    pub fn is_authenticated_wasm(&self) -> bool {
        self.is_authenticated()
    }
    
    /// Sign out (WASM method)
    #[wasm_bindgen]
    pub async fn sign_out_wasm(&mut self) -> Result<(), JsValue> {
        self.sign_out().await
    }
    
    /// Get user info (WASM method)
    #[wasm_bindgen]
    pub async fn get_user_info_wasm(&self) -> Result<JsValue, JsValue> {
        self.get_user_info().await
    }
}

/// Generate secure random bytes
#[wasm_bindgen]
pub fn generate_random_bytes(length: usize) -> Result<js_sys::Uint8Array, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let crypto = window.crypto()?;
    
    let mut bytes = vec![0u8; length];
    crypto.get_random_values_with_u8_array(&mut bytes)?;
    
    Ok(js_sys::Uint8Array::from(&bytes[..]))
}

/// Hash a string using SHA-256
#[wasm_bindgen]
pub async fn hash_sha256(data: String) -> Result<String, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let crypto = window.crypto()?;
    let subtle = crypto.subtle()?;
    
    let data_bytes = Uint8Array::from(data.as_bytes());
    let hash_promise = subtle.digest_with_str_and_buffer_source("SHA-256", &data_bytes)?;
    let hash_buffer = JsFuture::from(hash_promise).await?;
    
    let hash_array = Uint8Array::new(&hash_buffer);
    let mut hash_bytes = vec![0u8; hash_array.length() as usize];
    hash_array.copy_to(&mut hash_bytes);
    
    // Convert to hex string
    Ok(hash_bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Verify if running in secure context
#[wasm_bindgen]
pub fn is_secure_context() -> bool {
    window()
        .and_then(|w| w.is_secure_context())
        .unwrap_or(false)
}