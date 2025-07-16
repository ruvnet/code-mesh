//! Authentication module unit tests

use code_mesh_core::auth::*;
use proptest::prelude::*;
use rstest::*;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

mod common;
use common::{mocks::*, *};

#[tokio::test]
async fn test_auth_storage_file_operations() {
    let temp_dir = temp_dir();
    let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());

    // Test saving and loading token
    let token = "test-auth-token-123";
    auth_storage.save_token(token.to_string()).await.unwrap();
    
    let loaded_token = auth_storage.load_token().await.unwrap();
    assert_eq!(loaded_token, Some(token.to_string()));

    // Test deleting token
    auth_storage.delete_token().await.unwrap();
    let deleted_token = auth_storage.load_token().await.unwrap();
    assert_eq!(deleted_token, None);
}

#[tokio::test]
async fn test_auth_storage_nonexistent_token() {
    let temp_dir = temp_dir();
    let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());

    let token = auth_storage.load_token().await.unwrap();
    assert_eq!(token, None);
}

#[tokio::test]
async fn test_auth_storage_overwrite_token() {
    let temp_dir = temp_dir();
    let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());

    // Save first token
    auth_storage.save_token("token1".to_string()).await.unwrap();
    let token1 = auth_storage.load_token().await.unwrap();
    assert_eq!(token1, Some("token1".to_string()));

    // Overwrite with second token
    auth_storage.save_token("token2".to_string()).await.unwrap();
    let token2 = auth_storage.load_token().await.unwrap();
    assert_eq!(token2, Some("token2".to_string()));
}

#[tokio::test]
async fn test_auth_storage_concurrent_access() {
    let temp_dir = temp_dir();
    let auth_storage = Arc::new(AuthStorage::new(temp_dir.path().to_path_buf()));

    // Spawn multiple tasks that try to save tokens concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let storage = Arc::clone(&auth_storage);
            tokio::spawn(async move {
                storage.save_token(format!("token-{}", i)).await
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify that a token was saved (any of them is fine due to concurrency)
    let final_token = auth_storage.load_token().await.unwrap();
    assert!(final_token.is_some());
    assert!(final_token.unwrap().starts_with("token-"));
}

#[test]
fn test_auth_provider_anthropic_config() {
    let provider = AuthProvider::Anthropic {
        api_key: "test-key".to_string(),
        base_url: None,
    };

    match provider {
        AuthProvider::Anthropic { api_key, base_url } => {
            assert_eq!(api_key, "test-key");
            assert_eq!(base_url, None);
        }
        _ => panic!("Expected Anthropic provider"),
    }
}

#[test]
fn test_auth_provider_openai_config() {
    let provider = AuthProvider::OpenAI {
        api_key: "test-key".to_string(),
        base_url: Some("https://api.openai.com/v1".to_string()),
        organization: Some("org-123".to_string()),
    };

    match provider {
        AuthProvider::OpenAI { api_key, base_url, organization } => {
            assert_eq!(api_key, "test-key");
            assert_eq!(base_url, Some("https://api.openai.com/v1".to_string()));
            assert_eq!(organization, Some("org-123".to_string()));
        }
        _ => panic!("Expected OpenAI provider"),
    }
}

#[rstest]
#[case("sk-ant-1234567890", true)]
#[case("", false)]
#[case("invalid", false)]
#[case("sk-ant-", false)]
fn test_validate_anthropic_key(#[case] key: &str, #[case] expected_valid: bool) {
    let provider = AuthProvider::Anthropic {
        api_key: key.to_string(),
        base_url: None,
    };

    let result = provider.validate();
    assert_eq!(result.is_ok(), expected_valid);
}

#[rstest]
#[case("sk-1234567890", true)]
#[case("", false)]
#[case("invalid", false)]
fn test_validate_openai_key(#[case] key: &str, #[case] expected_valid: bool) {
    let provider = AuthProvider::OpenAI {
        api_key: key.to_string(),
        base_url: None,
        organization: None,
    };

    let result = provider.validate();
    assert_eq!(result.is_ok(), expected_valid);
}

// Property-based tests
proptest! {
    #[test]
    fn test_auth_storage_property_based(
        token in r"[a-zA-Z0-9\-_]{10,100}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_dir = temp_dir();
            let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());

            // Save and load should be consistent
            auth_storage.save_token(token.clone()).await.unwrap();
            let loaded = auth_storage.load_token().await.unwrap();
            prop_assert_eq!(loaded, Some(token));
        });
    }

    #[test]
    fn test_auth_provider_serialization(
        api_key in r"[a-zA-Z0-9\-_]{10,50}",
        base_url in option::of(r"https?://[a-zA-Z0-9\.\-/]+")
    ) {
        let provider = AuthProvider::Anthropic {
            api_key: api_key.clone(),
            base_url: base_url.clone(),
        };

        // Serialize and deserialize should be consistent
        let serialized = serde_json::to_string(&provider).unwrap();
        let deserialized: AuthProvider = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            AuthProvider::Anthropic { api_key: deserialized_key, base_url: deserialized_url } => {
                prop_assert_eq!(deserialized_key, api_key);
                prop_assert_eq!(deserialized_url, base_url);
            }
            _ => prop_assert!(false, "Deserialized to wrong variant"),
        }
    }
}

#[tokio::test]
async fn test_auth_storage_permissions() {
    let temp_dir = temp_dir();
    let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());

    // Save a token
    auth_storage.save_token("secret-token".to_string()).await.unwrap();

    // Check that the token file has restricted permissions (owner only)
    let token_file = temp_dir.path().join("auth_token");
    let metadata = fs::metadata(token_file).await.unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        // Should be 0o600 (read/write for owner only)
        assert_eq!(mode & 0o777, 0o600);
    }
}

#[tokio::test]
async fn test_auth_storage_invalid_directory() {
    let invalid_path = std::path::PathBuf::from("/nonexistent/path");
    let auth_storage = AuthStorage::new(invalid_path);

    let result = auth_storage.save_token("test".to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_auth_storage_corrupted_file() {
    let temp_dir = temp_dir();
    let token_file = temp_dir.path().join("auth_token");
    
    // Create a corrupted token file
    fs::write(&token_file, b"\xFF\xFE\xFF\xFE").await.unwrap();

    let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());
    let result = auth_storage.load_token().await;
    
    // Should handle corrupted file gracefully
    assert!(result.is_err() || result.unwrap().is_none());
}

#[test]
fn test_auth_provider_display() {
    let anthropic = AuthProvider::Anthropic {
        api_key: "sk-ant-test".to_string(),
        base_url: None,
    };

    let openai = AuthProvider::OpenAI {
        api_key: "sk-test".to_string(),
        base_url: None,
        organization: None,
    };

    // Test that display doesn't leak sensitive information
    let anthropic_display = format!("{}", anthropic);
    let openai_display = format!("{}", openai);

    assert!(!anthropic_display.contains("sk-ant-test"));
    assert!(!openai_display.contains("sk-test"));
    assert!(anthropic_display.contains("Anthropic"));
    assert!(openai_display.contains("OpenAI"));
}

#[test]
fn test_auth_provider_debug() {
    let provider = AuthProvider::Anthropic {
        api_key: "sk-ant-test".to_string(),
        base_url: None,
    };

    let debug_output = format!("{:?}", provider);
    
    // Debug output should mask sensitive data
    assert!(!debug_output.contains("sk-ant-test"));
    assert!(debug_output.contains("[REDACTED]") || debug_output.contains("***"));
}

// Integration tests with mock storage
#[tokio::test]
async fn test_auth_with_mock_storage() {
    let mut mock_storage = MockAuthStorage::new();
    
    mock_storage
        .expect_save_token()
        .with(eq("test-token".to_string()))
        .times(1)
        .returning(|_| Ok(()));

    mock_storage
        .expect_load_token()
        .times(1)
        .returning(|| Ok(Some("test-token".to_string())));

    // Use the mock in our code
    mock_storage.save_token("test-token".to_string()).await.unwrap();
    let token = mock_storage.load_token().await.unwrap();
    
    assert_eq!(token, Some("test-token".to_string()));
}

// Benchmarking tests using criterion would go here
#[cfg(feature = "bench")]
mod benchmarks {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_auth_storage_save_load(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("auth_storage_save_load", |b| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = temp_dir();
                let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());
                
                auth_storage.save_token(black_box("benchmark-token".to_string())).await.unwrap();
                let _token = auth_storage.load_token().await.unwrap();
            });
        });
    }

    criterion_group!(benches, bench_auth_storage_save_load);
    criterion_main!(benches);
}