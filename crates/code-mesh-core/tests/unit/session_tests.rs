//! Session management unit tests

use code_mesh_core::{session::*, llm::*};
use proptest::prelude::*;
use rstest::*;
use std::sync::Arc;

mod common;
use common::{mocks::*, fixtures::*, *};

#[test]
fn test_session_creation() {
    let session = Session::new("test-session-123".to_string(), "user-456".to_string());
    
    assert_eq!(session.id, "test-session-123");
    assert_eq!(session.user_id, "user-456");
    assert!(session.messages.is_empty());
    assert!(session.created_at <= chrono::Utc::now());
    assert!(session.updated_at <= chrono::Utc::now());
}

#[test]
fn test_session_add_message() {
    let mut session = Session::new("test".to_string(), "user".to_string());
    let initial_updated = session.updated_at;
    
    // Wait a moment to ensure time difference
    std::thread::sleep(std::time::Duration::from_millis(1));
    
    let message = ChatMessage {
        role: ChatRole::User,
        content: "Hello, world!".to_string(),
    };
    
    session.add_message(message.clone());
    
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content, "Hello, world!");
    assert_eq!(session.messages[0].role, ChatRole::User);
    assert!(session.updated_at > initial_updated);
}

#[test]
fn test_session_add_multiple_messages() {
    let mut session = Session::new("test".to_string(), "user".to_string());
    
    let messages = vec![
        ChatMessage {
            role: ChatRole::User,
            content: "First message".to_string(),
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "First response".to_string(),
        },
        ChatMessage {
            role: ChatRole::User,
            content: "Second message".to_string(),
        },
    ];
    
    for message in messages {
        session.add_message(message);
    }
    
    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[0].content, "First message");
    assert_eq!(session.messages[1].content, "First response");
    assert_eq!(session.messages[2].content, "Second message");
}

#[test]
fn test_session_clear_messages() {
    let mut session = SessionFixtures::session_with_history();
    
    assert!(!session.messages.is_empty());
    
    session.clear_messages();
    
    assert!(session.messages.is_empty());
}

#[test]
fn test_session_get_conversation_context() {
    let mut session = Session::new("test".to_string(), "user".to_string());
    
    // Add system message
    session.add_message(ChatMessage {
        role: ChatRole::System,
        content: "You are a helpful assistant.".to_string(),
    });
    
    // Add conversation
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "What is 2+2?".to_string(),
    });
    
    session.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: "2+2 equals 4.".to_string(),
    });
    
    let context = session.get_conversation_context();
    assert_eq!(context.len(), 3);
    assert_eq!(context[0].role, ChatRole::System);
    assert_eq!(context[1].role, ChatRole::User);
    assert_eq!(context[2].role, ChatRole::Assistant);
}

#[test]
fn test_session_message_count() {
    let mut session = Session::new("test".to_string(), "user".to_string());
    
    assert_eq!(session.message_count(), 0);
    
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "Hello".to_string(),
    });
    
    assert_eq!(session.message_count(), 1);
    
    session.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: "Hi there!".to_string(),
    });
    
    assert_eq!(session.message_count(), 2);
}

#[test]
fn test_session_last_message() {
    let mut session = Session::new("test".to_string(), "user".to_string());
    
    assert!(session.last_message().is_none());
    
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "First".to_string(),
    });
    
    assert_eq!(session.last_message().unwrap().content, "First");
    
    session.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: "Second".to_string(),
    });
    
    assert_eq!(session.last_message().unwrap().content, "Second");
}

#[test]
fn test_session_serialization() {
    let session = SessionFixtures::session_with_history();
    
    let serialized = serde_json::to_string(&session).unwrap();
    let deserialized: Session = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.id, session.id);
    assert_eq!(deserialized.user_id, session.user_id);
    assert_eq!(deserialized.messages.len(), session.messages.len());
    assert_eq!(deserialized.created_at, session.created_at);
    assert_eq!(deserialized.updated_at, session.updated_at);
}

#[test]
fn test_session_fixtures() {
    let basic = SessionFixtures::basic_session();
    assert_eq!(basic.id, "test-session-123");
    assert_eq!(basic.user_id, "user-456");
    assert!(basic.messages.is_empty());
    
    let with_history = SessionFixtures::session_with_history();
    assert_eq!(with_history.messages.len(), 2);
    assert_eq!(with_history.messages[0].role, ChatRole::User);
    assert_eq!(with_history.messages[1].role, ChatRole::Assistant);
    
    let expired = SessionFixtures::expired_session();
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
    assert!(expired.created_at < thirty_days_ago + chrono::Duration::hours(1));
}

// Property-based tests
proptest! {
    #[test]
    fn test_session_properties(
        session_id in "[a-zA-Z0-9-]{1,100}",
        user_id in "[a-zA-Z0-9_]{1,50}",
        message_count in 0usize..100
    ) {
        let mut session = Session::new(session_id.clone(), user_id.clone());
        
        prop_assert_eq!(session.id, session_id);
        prop_assert_eq!(session.user_id, user_id);
        prop_assert_eq!(session.messages.len(), 0);
        
        // Add messages
        for i in 0..message_count {
            session.add_message(ChatMessage {
                role: if i % 2 == 0 { ChatRole::User } else { ChatRole::Assistant },
                content: format!("Message {}", i),
            });
        }
        
        prop_assert_eq!(session.messages.len(), message_count);
        prop_assert_eq!(session.message_count(), message_count);
        
        if message_count > 0 {
            prop_assert!(session.last_message().is_some());
            prop_assert_eq!(session.last_message().unwrap().content, format!("Message {}", message_count - 1));
        } else {
            prop_assert!(session.last_message().is_none());
        }
    }

    #[test]
    fn test_session_serialization_roundtrip(
        session_id in "[a-zA-Z0-9-]{1,50}",
        user_id in "[a-zA-Z0-9_]{1,30}",
        messages in prop::collection::vec(
            (
                prop_oneof![
                    Just(ChatRole::User),
                    Just(ChatRole::Assistant),
                    Just(ChatRole::System)
                ],
                ".*{0,1000}"
            ),
            0..20
        )
    ) {
        let mut session = Session::new(session_id.clone(), user_id.clone());
        
        for (role, content) in messages.iter() {
            session.add_message(ChatMessage {
                role: *role,
                content: content.clone(),
            });
        }
        
        let serialized = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&serialized).unwrap();
        
        prop_assert_eq!(deserialized.id, session_id);
        prop_assert_eq!(deserialized.user_id, user_id);
        prop_assert_eq!(deserialized.messages.len(), messages.len());
        
        for (i, (role, content)) in messages.iter().enumerate() {
            prop_assert_eq!(deserialized.messages[i].role, *role);
            prop_assert_eq!(deserialized.messages[i].content, *content);
        }
    }
}

// Session manager tests
#[tokio::test]
async fn test_session_storage_mock() {
    let mut mock_storage = MockSessionStorage::new();
    let session = SessionFixtures::basic_session();
    
    mock_storage
        .expect_save_session()
        .with(mockall::predicate::eq(session.clone()))
        .times(1)
        .returning(|_| Ok(()));
    
    mock_storage
        .expect_load_session()
        .with(mockall::predicate::eq("test-session-123".to_string()))
        .times(1)
        .returning(|_| Ok(Some(SessionFixtures::basic_session())));
    
    // Test saving
    mock_storage.save_session(session).await.unwrap();
    
    // Test loading
    let loaded = mock_storage.load_session("test-session-123".to_string()).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().id, "test-session-123");
}

#[tokio::test]
async fn test_session_storage_list_sessions() {
    let mut mock_storage = MockSessionStorage::new();
    
    mock_storage
        .expect_list_sessions()
        .times(1)
        .returning(|| Ok(vec![
            "session-1".to_string(),
            "session-2".to_string(),
            "session-3".to_string(),
        ]));
    
    let sessions = mock_storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 3);
    assert!(sessions.contains(&"session-1".to_string()));
    assert!(sessions.contains(&"session-2".to_string()));
    assert!(sessions.contains(&"session-3".to_string()));
}

#[tokio::test]
async fn test_session_storage_delete_session() {
    let mut mock_storage = MockSessionStorage::new();
    
    mock_storage
        .expect_delete_session()
        .with(mockall::predicate::eq("test-session".to_string()))
        .times(1)
        .returning(|_| Ok(()));
    
    mock_storage.delete_session("test-session".to_string()).await.unwrap();
}

#[tokio::test]
async fn test_session_storage_nonexistent_session() {
    let mut mock_storage = MockSessionStorage::new();
    
    mock_storage
        .expect_load_session()
        .with(mockall::predicate::eq("nonexistent".to_string()))
        .times(1)
        .returning(|_| Ok(None));
    
    let result = mock_storage.load_session("nonexistent".to_string()).await.unwrap();
    assert!(result.is_none());
}

// In-memory session storage implementation for testing
#[derive(Clone)]
struct InMemorySessionStorage {
    sessions: Arc<parking_lot::RwLock<std::collections::HashMap<String, Session>>>,
}

impl InMemorySessionStorage {
    fn new() -> Self {
        Self {
            sessions: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionStorage for InMemorySessionStorage {
    async fn save_session(&self, session: Session) -> code_mesh_core::CodeMeshResult<()> {
        self.sessions.write().insert(session.id.clone(), session);
        Ok(())
    }

    async fn load_session(&self, session_id: String) -> code_mesh_core::CodeMeshResult<Option<Session>> {
        Ok(self.sessions.read().get(&session_id).cloned())
    }

    async fn list_sessions(&self) -> code_mesh_core::CodeMeshResult<Vec<String>> {
        Ok(self.sessions.read().keys().cloned().collect())
    }

    async fn delete_session(&self, session_id: String) -> code_mesh_core::CodeMeshResult<()> {
        self.sessions.write().remove(&session_id);
        Ok(())
    }
}

#[tokio::test]
async fn test_in_memory_session_storage() {
    let storage = InMemorySessionStorage::new();
    let session = SessionFixtures::session_with_history();
    
    // Test save and load
    storage.save_session(session.clone()).await.unwrap();
    let loaded = storage.load_session(session.id.clone()).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().id, session.id);
    
    // Test list sessions
    let sessions = storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert!(sessions.contains(&session.id));
    
    // Test delete
    storage.delete_session(session.id.clone()).await.unwrap();
    let deleted = storage.load_session(session.id).await.unwrap();
    assert!(deleted.is_none());
    
    let empty_list = storage.list_sessions().await.unwrap();
    assert!(empty_list.is_empty());
}

#[tokio::test]
async fn test_concurrent_session_operations() {
    let storage = Arc::new(InMemorySessionStorage::new());
    
    // Create multiple sessions concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let storage = Arc::clone(&storage);
            tokio::spawn(async move {
                let session = Session::new(format!("session-{}", i), format!("user-{}", i));
                storage.save_session(session).await
            })
        })
        .collect();
    
    // Wait for all saves to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
    
    // Verify all sessions were saved
    let sessions = storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 10);
    
    for i in 0..10 {
        assert!(sessions.contains(&format!("session-{}", i)));
    }
}

#[test]
fn test_session_conversation_flow() {
    let mut session = Session::new("conversation-test".to_string(), "user".to_string());
    
    // System message
    session.add_message(ChatMessage {
        role: ChatRole::System,
        content: "You are a helpful coding assistant.".to_string(),
    });
    
    // User asks question
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "How do I create a hash map in Rust?".to_string(),
    });
    
    // Assistant responds
    session.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: "You can create a HashMap in Rust using std::collections::HashMap...".to_string(),
    });
    
    // User follows up
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "Can you show me an example?".to_string(),
    });
    
    assert_eq!(session.message_count(), 4);
    
    let context = session.get_conversation_context();
    assert_eq!(context.len(), 4);
    
    // Verify conversation flow
    assert_eq!(context[0].role, ChatRole::System);
    assert_eq!(context[1].role, ChatRole::User);
    assert_eq!(context[2].role, ChatRole::Assistant);
    assert_eq!(context[3].role, ChatRole::User);
    
    // Last message should be the follow-up question
    assert_eq!(session.last_message().unwrap().content, "Can you show me an example?");
}

#[test]
fn test_session_memory_efficiency() {
    let mut session = Session::new("memory-test".to_string(), "user".to_string());
    
    // Add many messages to test memory usage
    for i in 0..1000 {
        session.add_message(ChatMessage {
            role: if i % 2 == 0 { ChatRole::User } else { ChatRole::Assistant },
            content: format!("Message number {}", i),
        });
    }
    
    assert_eq!(session.message_count(), 1000);
    
    // Clear messages and verify memory is released
    session.clear_messages();
    assert_eq!(session.message_count(), 0);
    assert!(session.messages.is_empty());
}

#[test]
fn test_session_edge_cases() {
    let mut session = Session::new("".to_string(), "".to_string()); // Empty IDs
    
    // Empty message content
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "".to_string(),
    });
    
    // Very long message content
    let long_content = "x".repeat(100_000);
    session.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: long_content.clone(),
    });
    
    assert_eq!(session.message_count(), 2);
    assert_eq!(session.messages[0].content, "");
    assert_eq!(session.messages[1].content, long_content);
    
    // Unicode content
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "Hello 世界! 🌍 Здравствуй мир!".to_string(),
    });
    
    assert_eq!(session.message_count(), 3);
    assert!(session.last_message().unwrap().content.contains("世界"));
}

#[test]
fn test_session_time_tracking() {
    let session = Session::new("time-test".to_string(), "user".to_string());
    let creation_time = session.created_at;
    let initial_update_time = session.updated_at;
    
    // Created and updated should be very close initially
    let time_diff = (initial_update_time - creation_time).num_milliseconds().abs();
    assert!(time_diff < 100); // Should be within 100ms
    
    // After adding a message, updated_at should change
    std::thread::sleep(std::time::Duration::from_millis(2));
    let mut session = session;
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "Test".to_string(),
    });
    
    assert!(session.updated_at > initial_update_time);
    assert_eq!(session.created_at, creation_time); // Created should not change
}