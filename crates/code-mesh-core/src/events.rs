//! Event system for Code Mesh Core
//!
//! This module provides a comprehensive event system for communication
//! between different components of the Code Mesh ecosystem.

use crate::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "native")]
use tokio::sync::{broadcast, RwLock};

#[cfg(feature = "wasm")]
use parking_lot::RwLock;

/// Event trait that all events must implement
pub trait Event: Send + Sync + Clone + std::fmt::Debug + 'static {
    /// Event type identifier
    fn event_type(&self) -> &'static str;
    
    /// Event priority
    fn priority(&self) -> EventPriority {
        EventPriority::Normal
    }
    
    /// Whether this event should be persisted
    fn persistent(&self) -> bool {
        false
    }
}

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Event handler trait
#[async_trait]
pub trait EventHandler<E: Event>: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: &E) -> Result<()>;
    
    /// Handler priority (higher values are called first)
    fn priority(&self) -> i32 {
        0
    }
    
    /// Whether this handler should receive events before others
    fn early(&self) -> bool {
        false
    }
}

/// Boxed event handler that can handle any event
type BoxedHandler = Box<dyn EventHandlerDyn + Send + Sync>;

/// Dynamic event handler trait for type erasure
#[async_trait]
trait EventHandlerDyn {
    async fn handle_dyn(&self, event: &(dyn Any + Send + Sync)) -> Result<()>;
    fn priority(&self) -> i32;
    fn early(&self) -> bool;
}

/// Wrapper to implement EventHandlerDyn for concrete handlers
struct EventHandlerWrapper<E: Event, H: EventHandler<E>> {
    handler: H,
    _phantom: std::marker::PhantomData<E>,
}

#[async_trait]
impl<E: Event, H: EventHandler<E>> EventHandlerDyn for EventHandlerWrapper<E, H> {
    async fn handle_dyn(&self, event: &(dyn Any + Send + Sync)) -> Result<()> {
        if let Some(typed_event) = event.downcast_ref::<E>() {
            self.handler.handle(typed_event).await
        } else {
            Err(Error::Other(anyhow::anyhow!("Event type mismatch")))
        }
    }

    fn priority(&self) -> i32 {
        self.handler.priority()
    }

    fn early(&self) -> bool {
        self.handler.early()
    }
}

/// Event bus for managing event distribution
pub struct EventBus {
    #[cfg(feature = "native")]
    handlers: RwLock<HashMap<TypeId, Vec<BoxedHandler>>>,
    
    #[cfg(feature = "native")]
    broadcast_senders: RwLock<HashMap<TypeId, broadcast::Sender<Arc<dyn Any + Send + Sync>>>>,
    
    #[cfg(feature = "wasm")]
    handlers: RwLock<HashMap<TypeId, Vec<BoxedHandler>>>,
    
    /// Maximum number of queued events per type
    max_queue_size: usize,
    
    /// Whether to enable event tracing
    tracing_enabled: bool,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            #[cfg(feature = "native")]
            broadcast_senders: RwLock::new(HashMap::new()),
            max_queue_size: 1000,
            tracing_enabled: true,
        }
    }

    /// Create a new event bus with custom configuration
    pub fn with_config(max_queue_size: usize, tracing_enabled: bool) -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            #[cfg(feature = "native")]
            broadcast_senders: RwLock::new(HashMap::new()),
            max_queue_size,
            tracing_enabled,
        }
    }

    /// Subscribe to events of a specific type
    pub async fn subscribe<E: Event, H: EventHandler<E> + 'static>(&self, handler: H) -> Result<()> {
        let type_id = TypeId::of::<E>();
        let boxed_handler = Box::new(EventHandlerWrapper {
            handler,
            _phantom: std::marker::PhantomData::<E>,
        });

        #[cfg(feature = "native")]
        {
            let mut handlers = self.handlers.write().await;
            let handlers_list = handlers.entry(type_id).or_insert_with(Vec::new);
            handlers_list.push(boxed_handler);
            
            // Sort by priority and early flag
            handlers_list.sort_by(|a, b| {
                match (a.early(), b.early()) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.priority().cmp(&a.priority()),
                }
            });
        }

        #[cfg(feature = "wasm")]
        {
            let mut handlers = self.handlers.write();
            let handlers_list = handlers.entry(type_id).or_insert_with(Vec::new);
            handlers_list.push(boxed_handler);
            
            // Sort by priority and early flag
            handlers_list.sort_by(|a, b| {
                match (a.early(), b.early()) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.priority().cmp(&a.priority()),
                }
            });
        }

        if self.tracing_enabled {
            tracing::debug!("Subscribed to event type: {}", std::any::type_name::<E>());
        }

        Ok(())
    }

    /// Publish an event to all subscribers
    pub async fn publish<E: Event>(&self, event: E) -> Result<()> {
        let type_id = TypeId::of::<E>();
        
        if self.tracing_enabled {
            tracing::debug!(
                "Publishing event: {} with priority: {:?}",
                event.event_type(),
                event.priority()
            );
        }

        // Handle direct subscriptions
        #[cfg(feature = "native")]
        {
            let handlers = self.handlers.read().await;
            if let Some(handlers_list) = handlers.get(&type_id) {
                for handler in handlers_list {
                    if let Err(e) = handler.handle_dyn(&event as &(dyn Any + Send + Sync)).await {
                        tracing::error!("Error handling event: {}", e);
                        // Continue processing other handlers
                    }
                }
            }

            // Handle broadcast subscriptions
            let senders = self.broadcast_senders.read().await;
            if let Some(sender) = senders.get(&type_id) {
                let arc_event: Arc<dyn Any + Send + Sync> = Arc::new(event.clone());
                if sender.send(arc_event).is_err() {
                    // No receivers, which is fine
                }
            }
        }

        #[cfg(feature = "wasm")]
        {
            let handlers = self.handlers.read();
            if let Some(handlers_list) = handlers.get(&type_id) {
                for handler in handlers_list {
                    if let Err(e) = handler.handle_dyn(&event as &(dyn Any + Send + Sync)).await {
                        tracing::error!("Error handling event: {}", e);
                        // Continue processing other handlers
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a broadcast channel for streaming events
    #[cfg(feature = "native")]
    pub async fn create_stream<E: Event>(&self) -> broadcast::Receiver<Arc<dyn Any + Send + Sync>> {
        let type_id = TypeId::of::<E>();
        
        let mut senders = self.broadcast_senders.write().await;
        let sender = senders.entry(type_id).or_insert_with(|| {
            let (sender, _) = broadcast::channel(self.max_queue_size);
            sender
        });
        
        sender.subscribe()
    }

    /// Unsubscribe from all events (clears all handlers)
    pub async fn clear(&self) {
        #[cfg(feature = "native")]
        {
            self.handlers.write().await.clear();
            self.broadcast_senders.write().await.clear();
        }

        #[cfg(feature = "wasm")]
        {
            self.handlers.write().clear();
        }
    }

    /// Get the number of handlers for a specific event type
    pub async fn handler_count<E: Event>(&self) -> usize {
        let type_id = TypeId::of::<E>();
        
        #[cfg(feature = "native")]
        {
            self.handlers.read().await
                .get(&type_id)
                .map(|h| h.len())
                .unwrap_or(0)
        }

        #[cfg(feature = "wasm")]
        {
            self.handlers.read()
                .get(&type_id)
                .map(|h| h.len())
                .unwrap_or(0)
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Common events used throughout Code Mesh
pub mod events {
    use super::*;
    use chrono::{DateTime, Utc};

    /// Session-related events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SessionCreated {
        pub session_id: String,
        pub timestamp: DateTime<Utc>,
        pub metadata: HashMap<String, serde_json::Value>,
    }

    impl Event for SessionCreated {
        fn event_type(&self) -> &'static str {
            "session.created"
        }

        fn persistent(&self) -> bool {
            true
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SessionEnded {
        pub session_id: String,
        pub timestamp: DateTime<Utc>,
        pub reason: String,
    }

    impl Event for SessionEnded {
        fn event_type(&self) -> &'static str {
            "session.ended"
        }

        fn persistent(&self) -> bool {
            true
        }
    }

    /// Message-related events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MessageSent {
        pub session_id: String,
        pub message_id: String,
        pub role: String,
        pub content: String,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for MessageSent {
        fn event_type(&self) -> &'static str {
            "message.sent"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::Normal
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MessageReceived {
        pub session_id: String,
        pub message_id: String,
        pub role: String,
        pub content: String,
        pub timestamp: DateTime<Utc>,
        pub tokens_used: Option<u32>,
    }

    impl Event for MessageReceived {
        fn event_type(&self) -> &'static str {
            "message.received"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::Normal
        }
    }

    /// Tool-related events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolExecuted {
        pub session_id: String,
        pub tool_id: String,
        pub tool_name: String,
        pub arguments: serde_json::Value,
        pub result: serde_json::Value,
        pub duration_ms: u64,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for ToolExecuted {
        fn event_type(&self) -> &'static str {
            "tool.executed"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::Normal
        }

        fn persistent(&self) -> bool {
            true
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolFailed {
        pub session_id: String,
        pub tool_id: String,
        pub tool_name: String,
        pub arguments: serde_json::Value,
        pub error: String,
        pub duration_ms: u64,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for ToolFailed {
        fn event_type(&self) -> &'static str {
            "tool.failed"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::High
        }

        fn persistent(&self) -> bool {
            true
        }
    }

    /// Provider-related events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ProviderConnected {
        pub provider_id: String,
        pub provider_name: String,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for ProviderConnected {
        fn event_type(&self) -> &'static str {
            "provider.connected"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::High
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ProviderDisconnected {
        pub provider_id: String,
        pub provider_name: String,
        pub reason: String,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for ProviderDisconnected {
        fn event_type(&self) -> &'static str {
            "provider.disconnected"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::High
        }
    }

    /// Storage-related events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DataStored {
        pub key: String,
        pub size_bytes: u64,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for DataStored {
        fn event_type(&self) -> &'static str {
            "storage.stored"
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DataRetrieved {
        pub key: String,
        pub size_bytes: u64,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for DataRetrieved {
        fn event_type(&self) -> &'static str {
            "storage.retrieved"
        }
    }

    /// Error-related events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ErrorOccurred {
        pub error_id: String,
        pub component: String,
        pub error_message: String,
        pub error_code: Option<String>,
        pub context: HashMap<String, serde_json::Value>,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for ErrorOccurred {
        fn event_type(&self) -> &'static str {
            "error.occurred"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::Critical
        }

        fn persistent(&self) -> bool {
            true
        }
    }

    /// System-related events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SystemStarted {
        pub version: String,
        pub features: Vec<String>,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for SystemStarted {
        fn event_type(&self) -> &'static str {
            "system.started"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::High
        }

        fn persistent(&self) -> bool {
            true
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SystemShutdown {
        pub reason: String,
        pub timestamp: DateTime<Utc>,
    }

    impl Event for SystemShutdown {
        fn event_type(&self) -> &'static str {
            "system.shutdown"
        }

        fn priority(&self) -> EventPriority {
            EventPriority::Critical
        }

        fn persistent(&self) -> bool {
            true
        }
    }
}

/// Convenience macro for creating simple event handlers
#[macro_export]
macro_rules! event_handler {
    ($event_type:ty, $handler_fn:expr) => {
        struct SimpleEventHandler {
            handler: fn(&$event_type) -> Result<()>,
        }

        #[async_trait]
        impl EventHandler<$event_type> for SimpleEventHandler {
            async fn handle(&self, event: &$event_type) -> Result<()> {
                (self.handler)(event)
            }
        }

        SimpleEventHandler {
            handler: $handler_fn,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[derive(Debug, Clone)]
    struct TestEvent {
        message: String,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> &'static str {
            "test.event"
        }
    }

    struct TestHandler {
        counter: Arc<AtomicU32>,
    }

    #[async_trait]
    impl EventHandler<TestEvent> for TestHandler {
        async fn handle(&self, _event: &TestEvent) -> Result<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));
        
        let handler = TestHandler {
            counter: counter.clone(),
        };

        bus.subscribe(handler).await.unwrap();

        let event = TestEvent {
            message: "Hello, World!".to_string(),
        };

        bus.publish(event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}