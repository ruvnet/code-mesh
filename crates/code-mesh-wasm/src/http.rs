//! HTTP client implementation for browser and Node.js environments

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, Headers};
use js_sys::{Object, JSON};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use code_mesh_core::Session;

/// HTTP client for making API requests
pub struct HttpClient {
    base_url: Option<String>,
    default_headers: HashMap<String, String>,
    timeout_ms: u32,
}

#[derive(Serialize, Deserialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    choices: Vec<ApiChoice>,
    usage: Option<ApiUsage>,
}

#[derive(Serialize, Deserialize)]
struct ApiChoice {
    message: ApiMessage,
    finish_reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ApiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(base_url: Option<String>) -> Self {
        let mut default_headers = HashMap::new();
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());
        default_headers.insert("User-Agent".to_string(), "CodeMesh/1.0 (WASM)".to_string());
        
        Self {
            base_url,
            default_headers,
            timeout_ms: 30000, // 30 seconds
        }
    }
    
    /// Set timeout in milliseconds
    pub fn set_timeout(&mut self, timeout_ms: u32) {
        self.timeout_ms = timeout_ms;
    }
    
    /// Add a default header
    pub fn add_header(&mut self, key: String, value: String) {
        self.default_headers.insert(key, value);
    }
    
    /// Make a generic HTTP request
    pub async fn request(
        &self,
        method: &str,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<Response, JsValue> {
        let mut request_init = RequestInit::new();
        request_init.method(method);
        request_init.mode(RequestMode::Cors);
        
        // Set headers
        let headers_obj = Headers::new()?;
        
        // Add default headers
        for (key, value) in &self.default_headers {
            headers_obj.set(key, value)?;
        }
        
        // Add custom headers
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                headers_obj.set(&key, &value)?;
            }
        }
        
        request_init.headers(&headers_obj);
        
        // Set body if provided
        if let Some(body_str) = body {
            request_init.body(Some(&JsValue::from_str(&body_str)));
        }
        
        let request = Request::new_with_str_and_init(url, &request_init)?;
        
        // Make the request with timeout
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let fetch_promise = window.fetch_with_request(&request);
        
        // Create timeout promise
        let timeout_promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let timeout_id = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                self.timeout_ms as i32,
            ).unwrap();
            
            // Store timeout ID for potential cleanup
            let _ = timeout_id;
        });
        
        // Race between fetch and timeout
        let race_array = js_sys::Array::new();
        race_array.push(&fetch_promise);
        race_array.push(&timeout_promise);
        
        let result = JsFuture::from(js_sys::Promise::race(&race_array)).await?;
        
        // Check if result is a Response or timeout
        if result.is_instance_of::<Response>() {
            Ok(result.dyn_into::<Response>()?)
        } else {
            Err(JsValue::from_str("Request timeout"))
        }
    }
    
    /// Generate a response using AI API
    pub async fn generate_response(
        &self,
        session: &Session,
        model: &str,
        api_key: Option<String>,
    ) -> Result<String, JsValue> {
        // Determine provider and endpoint from model
        let (provider, endpoint) = self.get_provider_endpoint(model)?;
        
        // Prepare headers
        let mut headers = HashMap::new();
        if let Some(key) = api_key {
            match provider.as_str() {
                "anthropic" => {
                    headers.insert("x-api-key".to_string(), key);
                    headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
                }
                "openai" => {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", key));
                }
                "mistral" => {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", key));
                }
                _ => {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", key));
                }
            }
        }
        
        // Convert session messages to API format
        let api_messages: Vec<ApiMessage> = session
            .messages
            .iter()
            .map(|msg| ApiMessage {
                role: match msg.role {
                    code_mesh_core::MessageRole::User => "user".to_string(),
                    code_mesh_core::MessageRole::Assistant => "assistant".to_string(),
                    code_mesh_core::MessageRole::System => "system".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect();
        
        // Prepare request body based on provider
        let request_body = match provider.as_str() {
            "anthropic" => {
                let mut claude_request = Object::new();
                js_sys::Reflect::set(&claude_request, &"model".into(), &model.into())?;
                js_sys::Reflect::set(&claude_request, &"max_tokens".into(), &1024.into())?;
                
                let messages_array = js_sys::Array::new();
                for msg in &api_messages {
                    let msg_obj = Object::new();
                    js_sys::Reflect::set(&msg_obj, &"role".into(), &msg.role.clone().into())?;
                    js_sys::Reflect::set(&msg_obj, &"content".into(), &msg.content.clone().into())?;
                    messages_array.push(&msg_obj);
                }
                js_sys::Reflect::set(&claude_request, &"messages".into(), &messages_array)?;
                
                JSON::stringify(&claude_request)?
            }
            _ => {
                let api_request = ApiRequest {
                    model: model.to_string(),
                    messages: api_messages,
                    max_tokens: Some(1024),
                    temperature: Some(0.7),
                    stream: false,
                };
                
                serde_wasm_bindgen::to_value(&api_request)
                    .and_then(|v| JSON::stringify(&v))
                    .map_err(|e| JsValue::from_str(&e.to_string()))?
            }
        };
        
        // Make the request
        let response = self
            .request("POST", &endpoint, Some(headers), Some(request_body.as_string().unwrap()))
            .await?;
        
        if !response.ok() {
            let status = response.status();
            let error_text = JsFuture::from(response.text()?)
                .await?
                .as_string()
                .unwrap_or_default();
            return Err(JsValue::from_str(&format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }
        
        // Parse response
        let response_text = JsFuture::from(response.text()?)
            .await?
            .as_string()
            .unwrap_or_default();
        
        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        // Extract content based on provider
        let content = match provider.as_str() {
            "anthropic" => {
                response_json["content"][0]["text"]
                    .as_str()
                    .unwrap_or("No response content")
                    .to_string()
            }
            _ => {
                response_json["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("No response content")
                    .to_string()
            }
        };
        
        Ok(content)
    }
    
    /// Get provider and endpoint from model name
    fn get_provider_endpoint(&self, model: &str) -> Result<(String, String), JsValue> {
        let (provider, endpoint) = if model.starts_with("claude") {
            ("anthropic", "https://api.anthropic.com/v1/messages")
        } else if model.starts_with("gpt") {
            ("openai", "https://api.openai.com/v1/chat/completions")
        } else if model.starts_with("mistral") {
            ("mistral", "https://api.mistral.ai/v1/chat/completions")
        } else if model.starts_with("command") {
            ("cohere", "https://api.cohere.ai/v1/generate")
        } else {
            return Err(JsValue::from_str("Unknown model provider"));
        };
        
        // Use custom base URL if provided
        let final_endpoint = if let Some(base_url) = &self.base_url {
            format!("{}/v1/chat/completions", base_url)
        } else {
            endpoint.to_string()
        };
        
        Ok((provider.to_string(), final_endpoint))
    }
    
    /// Upload file to server
    pub async fn upload_file(
        &self,
        file_data: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<String, JsValue> {
        let endpoint = if let Some(base_url) = &self.base_url {
            format!("{}/v1/files/upload", base_url)
        } else {
            return Err(JsValue::from_str("No base URL configured for file upload"));
        };
        
        // Create FormData for file upload
        let form_data = web_sys::FormData::new()?;
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
            &js_sys::Array::of1(&js_sys::Uint8Array::from(file_data)),
            web_sys::BlobPropertyBag::new().type_(content_type),
        )?;
        
        form_data.append_with_blob_and_filename("file", &blob, filename)?;
        
        let mut request_init = RequestInit::new();
        request_init.method("POST");
        request_init.body(Some(&form_data));
        
        let request = Request::new_with_str_and_init(&endpoint, &request_init)?;
        
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let response = JsFuture::from(window.fetch_with_request(&request)).await?;
        let response: Response = response.dyn_into()?;
        
        if !response.ok() {
            return Err(JsValue::from_str(&format!(
                "File upload failed with status: {}",
                response.status()
            )));
        }
        
        let response_text = JsFuture::from(response.text()?)
            .await?
            .as_string()
            .unwrap_or_default();
        
        Ok(response_text)
    }
    
    /// Download file from server
    pub async fn download_file(&self, file_id: &str) -> Result<js_sys::Uint8Array, JsValue> {
        let endpoint = if let Some(base_url) = &self.base_url {
            format!("{}/v1/files/{}", base_url, file_id)
        } else {
            return Err(JsValue::from_str("No base URL configured for file download"));
        };
        
        let response = self.request("GET", &endpoint, None, None).await?;
        
        if !response.ok() {
            return Err(JsValue::from_str(&format!(
                "File download failed with status: {}",
                response.status()
            )));
        }
        
        let array_buffer = JsFuture::from(response.array_buffer()?).await?;
        Ok(js_sys::Uint8Array::new(&array_buffer))
    }
    
    /// Check server health
    pub async fn health_check(&self) -> Result<bool, JsValue> {
        let endpoint = if let Some(base_url) = &self.base_url {
            format!("{}/health", base_url)
        } else {
            return Ok(true); // No server to check
        };
        
        match self.request("GET", &endpoint, None, None).await {
            Ok(response) => Ok(response.ok()),
            Err(_) => Ok(false),
        }
    }
}

#[wasm_bindgen]
impl HttpClient {
    /// Create a new HTTP client (WASM constructor)
    #[wasm_bindgen(constructor)]
    pub fn new_wasm() -> HttpClient {
        Self::new(None)
    }
    
    /// Create with base URL (WASM method)
    #[wasm_bindgen]
    pub fn with_base_url(base_url: String) -> HttpClient {
        Self::new(Some(base_url))
    }
    
    /// Set timeout (WASM method)
    #[wasm_bindgen]
    pub fn set_timeout_wasm(&mut self, timeout_ms: u32) {
        self.set_timeout(timeout_ms);
    }
    
    /// Add header (WASM method)
    #[wasm_bindgen]
    pub fn add_header_wasm(&mut self, key: String, value: String) {
        self.add_header(key, value);
    }
    
    /// Make a simple GET request (WASM method)
    #[wasm_bindgen]
    pub async fn get(&self, url: String) -> Result<String, JsValue> {
        let response = self.request("GET", &url, None, None).await?;
        let text = JsFuture::from(response.text()?)
            .await?
            .as_string()
            .unwrap_or_default();
        Ok(text)
    }
    
    /// Make a simple POST request (WASM method)
    #[wasm_bindgen]
    pub async fn post(&self, url: String, body: String) -> Result<String, JsValue> {
        let response = self.request("POST", &url, None, Some(body)).await?;
        let text = JsFuture::from(response.text()?)
            .await?
            .as_string()
            .unwrap_or_default();
        Ok(text)
    }
}