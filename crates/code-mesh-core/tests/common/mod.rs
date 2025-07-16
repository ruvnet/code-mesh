//! Common test utilities and fixtures

use code_mesh_core::{
    auth::{AuthProvider, AuthStorage},
    llm::{ChatCompletion, LLMProvider, ModelInfo},
    session::Session,
    storage::Storage,
    CodeMeshResult,
};
use mockall::predicate::*;
use std::collections::HashMap;
use tempfile::TempDir;

pub mod fixtures;
pub mod mocks;

pub use fixtures::*;
pub use mocks::*;

/// Create a temporary directory for testing
pub fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Create test session with mock data
pub fn test_session() -> Session {
    Session::new("test-session".to_string(), "test-user".to_string())
}

/// Mock LLM response for testing
pub fn mock_chat_completion(content: &str) -> ChatCompletion {
    ChatCompletion {
        id: "test-completion".to_string(),
        content: content.to_string(),
        model: "test-model".to_string(),
        usage: Default::default(),
        created_at: chrono::Utc::now(),
    }
}

/// Setup test environment with all necessary mocks
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub auth_storage: MockAuthStorage,
    pub storage: MockStorage,
    pub llm_provider: MockLLMProvider,
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self {
            temp_dir: temp_dir(),
            auth_storage: MockAuthStorage::new(),
            storage: MockStorage::new(),
            llm_provider: MockLLMProvider::new(),
        }
    }

    pub fn setup_default_auth(&mut self) {
        self.auth_storage
            .expect_load_token()
            .returning(|| Ok(Some("test-token".to_string())));
            
        self.auth_storage
            .expect_save_token()
            .with(eq("test-token".to_string()))
            .returning(|_| Ok(()));
    }

    pub fn setup_default_storage(&mut self) {
        self.storage
            .expect_save()
            .returning(|_, _| Ok(()));
            
        self.storage
            .expect_load()
            .returning(|_| Ok(None));
    }

    pub fn setup_default_llm(&mut self) {
        self.llm_provider
            .expect_chat_completion()
            .returning(|_| {
                Box::pin(async {
                    Ok(mock_chat_completion("Test response"))
                })
            });
    }
}

/// Utility for creating test data
pub struct TestDataBuilder {
    data: HashMap<String, serde_json::Value>,
}

impl TestDataBuilder {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn with_field<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
        self.data.insert(
            key.to_string(),
            serde_json::to_value(value).expect("Failed to serialize test data"),
        );
        self
    }

    pub fn build(self) -> serde_json::Value {
        serde_json::Value::Object(self.data.into_iter().collect())
    }
}

/// Property testing utilities
pub mod property_testing {
    use proptest::prelude::*;

    prop_compose! {
        pub fn arb_session_id()(id in "[a-zA-Z0-9-]{1,50}") -> String {
            id
        }
    }

    prop_compose! {
        pub fn arb_user_id()(id in "[a-zA-Z0-9_]{1,30}") -> String {
            id
        }
    }

    prop_compose! {
        pub fn arb_file_content()(content in ".*{0,1000}") -> String {
            content
        }
    }

    prop_compose! {
        pub fn arb_json_content()(
            key in "[a-zA-Z]{1,20}",
            value in "[a-zA-Z0-9 ]{1,50}"
        ) -> serde_json::Value {
            serde_json::json!({key: value})
        }
    }
}

/// Async test utilities
pub mod async_utils {
    use std::future::Future;
    use tokio::time::{timeout, Duration};

    pub async fn with_timeout<F, T>(fut: F) -> T
    where
        F: Future<Output = T>,
    {
        timeout(Duration::from_secs(5), fut)
            .await
            .expect("Test timed out")
    }
}

/// Performance testing utilities
pub mod performance {
    use std::time::Instant;

    pub struct PerformanceTracker {
        start: Instant,
        checkpoints: Vec<(String, Instant)>,
    }

    impl PerformanceTracker {
        pub fn new() -> Self {
            Self {
                start: Instant::now(),
                checkpoints: Vec::new(),
            }
        }

        pub fn checkpoint(&mut self, name: &str) {
            self.checkpoints.push((name.to_string(), Instant::now()));
        }

        pub fn elapsed(&self) -> std::time::Duration {
            self.start.elapsed()
        }

        pub fn checkpoint_duration(&self, name: &str) -> Option<std::time::Duration> {
            self.checkpoints
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, time)| time.duration_since(self.start))
        }
    }
}