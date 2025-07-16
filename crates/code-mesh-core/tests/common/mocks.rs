//! Mock implementations for testing

use async_trait::async_trait;
use code_mesh_core::{
    auth::{AuthProvider, AuthStorage},
    llm::{ChatCompletion, ChatMessage, LLMProvider, ModelInfo, Usage},
    session::{Session, SessionStorage},
    storage::Storage,
    tool::{Tool, ToolResult},
    CodeMeshResult,
};
use mockall::mock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// Mock implementations using mockall
mock! {
    pub AuthStorage {}

    #[async_trait]
    impl AuthStorage for AuthStorage {
        async fn save_token(&self, token: String) -> CodeMeshResult<()>;
        async fn load_token(&self) -> CodeMeshResult<Option<String>>;
        async fn delete_token(&self) -> CodeMeshResult<()>;
    }
}

mock! {
    pub Storage {}

    #[async_trait]
    impl Storage for Storage {
        async fn save(&self, key: String, data: Value) -> CodeMeshResult<()>;
        async fn load(&self, key: String) -> CodeMeshResult<Option<Value>>;
        async fn delete(&self, key: String) -> CodeMeshResult<()>;
        async fn list_keys(&self, prefix: Option<String>) -> CodeMeshResult<Vec<String>>;
    }
}

mock! {
    pub LLMProvider {}

    #[async_trait]
    impl LLMProvider for LLMProvider {
        async fn chat_completion(&self, messages: Vec<ChatMessage>) -> CodeMeshResult<ChatCompletion>;
        async fn stream_completion(&self, messages: Vec<ChatMessage>) -> CodeMeshResult<Box<dyn futures::Stream<Item = CodeMeshResult<String>> + Send + Unpin>>;
        fn model_info(&self) -> ModelInfo;
        fn validate_config(&self) -> CodeMeshResult<()>;
    }
}

mock! {
    pub SessionStorage {}

    #[async_trait]
    impl SessionStorage for SessionStorage {
        async fn save_session(&self, session: Session) -> CodeMeshResult<()>;
        async fn load_session(&self, session_id: String) -> CodeMeshResult<Option<Session>>;
        async fn list_sessions(&self) -> CodeMeshResult<Vec<String>>;
        async fn delete_session(&self, session_id: String) -> CodeMeshResult<()>;
    }
}

mock! {
    pub Tool {}

    #[async_trait]
    impl Tool for Tool {
        fn name(&self) -> &str;
        fn description(&self) -> &str;
        fn parameters(&self) -> Value;
        async fn execute(&self, parameters: Value) -> CodeMeshResult<ToolResult>;
    }
}

// Manual mock implementations for more complex scenarios
#[derive(Clone)]
pub struct InMemoryStorage {
    data: Arc<parking_lot::RwLock<HashMap<String, Value>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, key: String, value: Value) {
        self.data.write().insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.data.read().get(key).cloned()
    }

    pub fn clear(&self) {
        self.data.write().clear();
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn save(&self, key: String, data: Value) -> CodeMeshResult<()> {
        self.data.write().insert(key, data);
        Ok(())
    }

    async fn load(&self, key: String) -> CodeMeshResult<Option<Value>> {
        Ok(self.data.read().get(&key).cloned())
    }

    async fn delete(&self, key: String) -> CodeMeshResult<()> {
        self.data.write().remove(&key);
        Ok(())
    }

    async fn list_keys(&self, prefix: Option<String>) -> CodeMeshResult<Vec<String>> {
        let keys: Vec<String> = self
            .data
            .read()
            .keys()
            .filter(|key| {
                prefix
                    .as_ref()
                    .map(|p| key.starts_with(p))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        Ok(keys)
    }
}

#[derive(Clone)]
pub struct MockLLMProvider {
    responses: Arc<parking_lot::RwLock<Vec<String>>>,
    current_index: Arc<parking_lot::RwLock<usize>>,
}

impl MockLLMProvider {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(parking_lot::RwLock::new(vec![
                "Default mock response".to_string()
            ])),
            current_index: Arc::new(parking_lot::RwLock::new(0)),
        }
    }

    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(parking_lot::RwLock::new(responses)),
            current_index: Arc::new(parking_lot::RwLock::new(0)),
        }
    }

    pub fn add_response(&self, response: String) {
        self.responses.write().push(response);
    }

    pub fn reset(&self) {
        *self.current_index.write() = 0;
    }
}

#[async_trait]
impl LLMProvider for MockLLMProvider {
    async fn chat_completion(&self, _messages: Vec<ChatMessage>) -> CodeMeshResult<ChatCompletion> {
        let responses = self.responses.read();
        let mut index = self.current_index.write();
        
        let response = responses
            .get(*index)
            .unwrap_or(&responses[0])
            .clone();
            
        *index = (*index + 1) % responses.len();

        Ok(ChatCompletion {
            id: format!("mock-completion-{}", *index),
            content: response,
            model: "mock-model".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 15,
                total_tokens: 25,
            },
            created_at: chrono::Utc::now(),
        })
    }

    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> CodeMeshResult<Box<dyn futures::Stream<Item = CodeMeshResult<String>> + Send + Unpin>> {
        let completion = self.chat_completion(messages).await?;
        let content = completion.content;
        
        let stream = futures::stream::iter(
            content
                .chars()
                .map(|c| Ok(c.to_string()))
                .collect::<Vec<_>>()
        );

        Ok(Box::new(Box::pin(stream)))
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: "mock-model".to_string(),
            provider: "mock".to_string(),
            max_tokens: 4096,
            supports_streaming: true,
            supports_tools: true,
        }
    }

    fn validate_config(&self) -> CodeMeshResult<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct CountingTool {
    pub call_count: Arc<parking_lot::RwLock<usize>>,
    pub name: String,
    pub response: ToolResult,
}

impl CountingTool {
    pub fn new(name: &str) -> Self {
        Self {
            call_count: Arc::new(parking_lot::RwLock::new(0)),
            name: name.to_string(),
            response: ToolResult {
                output: "Mock tool output".to_string(),
                metadata: Value::Object(serde_json::Map::new()),
            },
        }
    }

    pub fn with_response(mut self, response: ToolResult) -> Self {
        self.response = response;
        self
    }

    pub fn call_count(&self) -> usize {
        *self.call_count.read()
    }
}

#[async_trait]
impl Tool for CountingTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Mock tool for testing"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _parameters: Value) -> CodeMeshResult<ToolResult> {
        *self.call_count.write() += 1;
        Ok(self.response.clone())
    }
}

/// Test-specific error types for failure scenarios
#[derive(Debug, thiserror::Error)]
pub enum MockError {
    #[error("Simulated storage error")]
    Storage,
    #[error("Simulated network error")]
    Network,
    #[error("Simulated authentication error")]
    Auth,
    #[error("Simulated parsing error")]
    Parse,
}

impl From<MockError> for code_mesh_core::error::CodeMeshError {
    fn from(err: MockError) -> Self {
        match err {
            MockError::Storage => Self::Storage(err.to_string()),
            MockError::Network => Self::Network(err.to_string()),
            MockError::Auth => Self::Authentication(err.to_string()),
            MockError::Parse => Self::InvalidInput(err.to_string()),
        }
    }
}