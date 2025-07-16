use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    FinishReason, GenerateOptions, GenerateResult, LanguageModel, Message, MessageContent,
    MessagePart, MessageRole, StreamChunk, StreamOptions, ToolCall, ToolDefinition, Usage,
};
use crate::auth::{Auth, AuthCredentials};

/// GitHub Copilot provider implementation
pub struct GitHubCopilotProvider {
    auth: Box<dyn Auth>,
    client: Client,
    models: HashMap<String, GitHubCopilotModel>,
}

#[derive(Debug, Clone)]
pub struct GitHubCopilotModel {
    pub id: String,
    pub name: String,
    pub max_tokens: u32,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_caching: bool,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
struct CopilotRequest {
    model: String,
    messages: Vec<CopilotMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<CopilotTool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CopilotMessage {
    role: String,
    content: CopilotContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<CopilotToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum CopilotContent {
    Text(String),
    Parts(Vec<CopilotContentPart>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum CopilotContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: CopilotImageUrl },
}

#[derive(Debug, Serialize, Deserialize)]
struct CopilotImageUrl {
    url: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CopilotTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: CopilotFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct CopilotFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct CopilotToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: CopilotFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct CopilotFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct CopilotResponse {
    choices: Vec<CopilotChoice>,
    usage: CopilotUsage,
}

#[derive(Debug, Deserialize)]
struct CopilotChoice {
    message: CopilotMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CopilotUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl GitHubCopilotProvider {
    const CLIENT_ID: &'static str = "Iv1.b507a08c87ecfe98";
    const DEVICE_CODE_URL: &'static str = "https://github.com/login/device/code";
    const ACCESS_TOKEN_URL: &'static str = "https://github.com/login/oauth/access_token";
    const COPILOT_TOKEN_URL: &'static str = "https://api.github.com/copilot_internal/v2/token";
    const API_BASE: &'static str = "https://api.githubcopilot.com";
    
    pub fn new(auth: Box<dyn Auth>) -> Self {
        let client = Client::new();
        let models = Self::default_models();
        
        Self {
            auth,
            client,
            models,
        }
    }
    
    fn default_models() -> HashMap<String, GitHubCopilotModel> {
        let mut models = HashMap::new();
        
        models.insert(
            "gpt-4o".to_string(),
            GitHubCopilotModel {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                max_tokens: 4096,
                supports_tools: true,
                supports_vision: true,
                supports_caching: false,
            },
        );
        
        models.insert(
            "gpt-4o-mini".to_string(),
            GitHubCopilotModel {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o Mini".to_string(),
                max_tokens: 4096,
                supports_tools: true,
                supports_vision: true,
                supports_caching: false,
            },
        );
        
        models.insert(
            "o1-preview".to_string(),
            GitHubCopilotModel {
                id: "o1-preview".to_string(),
                name: "OpenAI o1 Preview".to_string(),
                max_tokens: 32768,
                supports_tools: false,
                supports_vision: false,
                supports_caching: false,
            },
        );
        
        models
    }
    
    /// Start device code flow for authentication
    pub async fn start_device_flow() -> crate::Result<DeviceCodeResponse> {
        let client = Client::new();
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
    
    /// Poll for access token
    pub async fn poll_for_token(device_code: &str) -> crate::Result<Option<String>> {
        let client = Client::new();
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
            return Ok(None);
        }
        
        let token_response: AccessTokenResponse = response
            .json()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Failed to parse token response: {}", e)))?;
            
        if let Some(access_token) = token_response.access_token {
            Ok(Some(access_token))
        } else if token_response.error.as_deref() == Some("authorization_pending") {
            Ok(None)
        } else {
            Err(crate::Error::Other(anyhow::anyhow!(
                "Token exchange failed: {:?}",
                token_response.error
            )))
        }
    }
    
    /// Get Copilot API token using GitHub OAuth token
    pub async fn get_copilot_token(github_token: &str) -> crate::Result<AuthCredentials> {
        let client = Client::new();
        
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
    
    async fn get_auth_headers(&self) -> crate::Result<HashMap<String, String>> {
        let credentials = self.auth.get_credentials().await?;
        
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "GitHubCopilotChat/0.26.7".to_string());
        headers.insert("Editor-Version".to_string(), "vscode/1.99.3".to_string());
        headers.insert("Editor-Plugin-Version".to_string(), "copilot-chat/0.26.7".to_string());
        headers.insert("Openai-Intent".to_string(), "conversation-edits".to_string());
        
        match credentials {
            AuthCredentials::OAuth { access_token, refresh_token, expires_at } => {
                // Check if token is expired and refresh if needed
                if let Some(exp) = expires_at {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    
                    if now >= exp {
                        if let Some(github_token) = refresh_token {
                            let new_creds = Self::get_copilot_token(&github_token).await?;
                            self.auth.set_credentials(new_creds.clone()).await?;
                            
                            if let AuthCredentials::OAuth { access_token, .. } = new_creds {
                                headers.insert("Authorization".to_string(), format!("Bearer {}", access_token));
                            }
                        } else {
                            return Err(crate::Error::Other(anyhow::anyhow!("Token expired and no refresh token available")));
                        }
                    } else {
                        headers.insert("Authorization".to_string(), format!("Bearer {}", access_token));
                    }
                } else {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", access_token));
                }
            }
            _ => {
                return Err(crate::Error::Other(anyhow::anyhow!(
                    "Invalid credentials for GitHub Copilot"
                )));
            }
        }
        
        Ok(headers)
    }
    
    fn convert_messages(&self, messages: Vec<Message>) -> Vec<CopilotMessage> {
        messages
            .into_iter()
            .map(|msg| self.convert_message(msg))
            .collect()
    }
    
    fn convert_message(&self, message: Message) -> CopilotMessage {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
        .to_string();
        
        let content = match message.content {
            MessageContent::Text(text) => CopilotContent::Text(text),
            MessageContent::Parts(parts) => {
                let copilot_parts: Vec<CopilotContentPart> = parts
                    .into_iter()
                    .filter_map(|part| match part {
                        MessagePart::Text { text } => Some(CopilotContentPart::Text { text }),
                        MessagePart::Image { image } => {
                            if let Some(url) = image.url {
                                Some(CopilotContentPart::ImageUrl {
                                    image_url: CopilotImageUrl {
                                        url,
                                        detail: Some("auto".to_string()),
                                    },
                                })
                            } else if let Some(base64) = image.base64 {
                                Some(CopilotContentPart::ImageUrl {
                                    image_url: CopilotImageUrl {
                                        url: format!("data:{};base64,{}", image.mime_type, base64),
                                        detail: Some("auto".to_string()),
                                    },
                                })
                            } else {
                                None
                            }
                        }
                    })
                    .collect();
                CopilotContent::Parts(copilot_parts)
            }
        };
        
        let tool_calls = message.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|call| CopilotToolCall {
                    id: call.id,
                    tool_type: "function".to_string(),
                    function: CopilotFunctionCall {
                        name: call.name,
                        arguments: call.arguments.to_string(),
                    },
                })
                .collect()
        });
        
        CopilotMessage {
            role,
            content,
            name: message.name,
            tool_calls,
            tool_call_id: message.tool_call_id,
        }
    }
    
    fn convert_tools(&self, tools: Vec<ToolDefinition>) -> Vec<CopilotTool> {
        tools
            .into_iter()
            .map(|tool| CopilotTool {
                tool_type: "function".to_string(),
                function: CopilotFunction {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                },
            })
            .collect()
    }
    
    fn parse_finish_reason(&self, reason: Option<String>) -> FinishReason {
        match reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        }
    }
}

pub struct GitHubCopilotModelWithProvider {
    model: GitHubCopilotModel,
    provider: GitHubCopilotProvider,
}

impl GitHubCopilotModelWithProvider {
    pub fn new(model: GitHubCopilotModel, provider: GitHubCopilotProvider) -> Self {
        Self { model, provider }
    }
}

#[async_trait]
impl LanguageModel for GitHubCopilotModelWithProvider {
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> crate::Result<GenerateResult> {
        let headers = self.provider.get_auth_headers().await?;
        let copilot_messages = self.provider.convert_messages(messages);
        let tools = self.provider.convert_tools(options.tools);
        
        let request = CopilotRequest {
            model: self.model.id.clone(),
            messages: copilot_messages,
            max_tokens: options.max_tokens.unwrap_or(self.model.max_tokens),
            temperature: options.temperature,
            tools,
            stop: options.stop_sequences,
            stream: Some(false),
        };
        
        let mut req_builder = self
            .provider
            .client
            .post(&format!("{}/v1/chat/completions", GitHubCopilotProvider::API_BASE))
            .header("Content-Type", "application/json");
            
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }
        
        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(crate::Error::Other(anyhow::anyhow!(
                "API request failed with status {}: {}",
                status,
                body
            )));
        }
        
        let copilot_response: CopilotResponse = response
            .json()
            .await
            .map_err(|e| crate::Error::Other(anyhow::anyhow!("Failed to parse response: {}", e)))?;
            
        let choice = copilot_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| crate::Error::Other(anyhow::anyhow!("No choices in response")))?;
            
        let content = match choice.message.content {
            CopilotContent::Text(text) => text,
            CopilotContent::Parts(parts) => {
                parts
                    .into_iter()
                    .filter_map(|part| match part {
                        CopilotContentPart::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }
        };
        
        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|call| ToolCall {
                id: call.id,
                name: call.function.name,
                arguments: serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            })
            .collect();
            
        Ok(GenerateResult {
            content,
            tool_calls,
            usage: Usage {
                prompt_tokens: copilot_response.usage.prompt_tokens,
                completion_tokens: copilot_response.usage.completion_tokens,
                total_tokens: copilot_response.usage.total_tokens,
            },
            finish_reason: self.provider.parse_finish_reason(choice.finish_reason),
        })
    }
    
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: StreamOptions,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<StreamChunk>> + Send + Unpin>> {
        // Similar to generate but with stream: true
        // Implementation would handle SSE stream parsing
        Err(crate::Error::Other(anyhow::anyhow!(
            "Streaming not yet implemented for GitHub Copilot"
        )))
    }
    
    fn supports_tools(&self) -> bool {
        self.model.supports_tools
    }
    
    fn supports_vision(&self) -> bool {
        self.model.supports_vision
    }
    
    fn supports_caching(&self) -> bool {
        self.model.supports_caching
    }
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
    fn test_default_models() {
        let models = GitHubCopilotProvider::default_models();
        assert!(!models.is_empty());
        assert!(models.contains_key("gpt-4o"));
        assert!(models.contains_key("gpt-4o-mini"));
        assert!(models.contains_key("o1-preview"));
    }
    
    #[test]
    fn test_model_capabilities() {
        let models = GitHubCopilotProvider::default_models();
        let gpt4o = models.get("gpt-4o").unwrap();
        assert!(gpt4o.supports_tools);
        assert!(gpt4o.supports_vision);
        
        let o1 = models.get("o1-preview").unwrap();
        assert!(!o1.supports_tools);
        assert!(!o1.supports_vision);
    }
}