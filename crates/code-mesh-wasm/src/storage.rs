//! Browser storage implementation using IndexedDB

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, IdbDatabase, IdbObjectStore, IdbRequest, IdbTransaction};
use js_sys::{Object, Array};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use code_mesh_core::{Session, Message};

/// Browser storage implementation using IndexedDB
#[wasm_bindgen]
pub struct BrowserStorage {
    db_name: String,
    version: u32,
    db: Option<IdbDatabase>,
}

#[derive(Serialize, Deserialize)]
struct StoredSession {
    id: String,
    messages: Vec<Message>,
    created_at: String,
    updated_at: String,
    metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct SessionMetadata {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    message_count: usize,
}

impl BrowserStorage {
    /// Create a new browser storage instance
    pub fn new() -> Self {
        Self {
            db_name: "code_mesh_storage".to_string(),
            version: 1,
            db: None,
        }
    }
    
    /// Create a new browser storage instance with custom database name
    pub fn with_db_name(db_name: String) -> Self {
        Self {
            db_name,
            version: 1,
            db: None,
        }
    }
    
    /// Initialize the IndexedDB database
    pub async fn initialize(&mut self) -> Result<(), JsValue> {
        let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let indexed_db = window
            .indexed_db()?
            .ok_or_else(|| JsValue::from_str("IndexedDB not supported"))?;
        
        // Open database
        let open_request = indexed_db.open_with_u32(&self.db_name, self.version)?;
        
        // Set up database schema on upgrade
        let onupgradeneeded = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let request: IdbRequest = target.dyn_into().unwrap();
            let db: IdbDatabase = request.result().unwrap().dyn_into().unwrap();
            
            // Create sessions object store
            if !db.object_store_names().contains("sessions") {
                let store = db.create_object_store("sessions").unwrap();
                store.create_index("updated_at", &JsValue::from_str("updated_at")).unwrap();
            }
            
            // Create messages object store
            if !db.object_store_names().contains("messages") {
                let store = db.create_object_store("messages").unwrap();
                store.create_index("session_id", &JsValue::from_str("session_id")).unwrap();
                store.create_index("timestamp", &JsValue::from_str("timestamp")).unwrap();
            }
            
            // Create preferences object store
            if !db.object_store_names().contains("preferences") {
                db.create_object_store("preferences").unwrap();
            }
        }) as Box<dyn FnMut(_)>);
        
        open_request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
        onupgradeneeded.forget();
        
        // Wait for database to open
        let db_result = JsFuture::from(open_request).await?;
        let db: IdbDatabase = db_result.dyn_into()?;
        
        self.db = Some(db);
        Ok(())
    }
    
    /// Save a session to storage
    pub async fn save_session(&self, session: &Session) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str_and_mode(
            "sessions",
            web_sys::IdbTransactionMode::Readwrite,
        )?;
        
        let store = transaction.object_store("sessions")?;
        
        let stored_session = StoredSession {
            id: session.id.clone(),
            messages: session.messages.clone(),
            created_at: session.created_at.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        };
        
        let serialized = serde_wasm_bindgen::to_value(&stored_session)?;
        let request = store.put_with_key(&serialized, &JsValue::from_str(&session.id))?;
        
        JsFuture::from(request).await?;
        Ok(())
    }
    
    /// Load a session from storage
    pub async fn load_session(&self, session_id: &str) -> Result<Option<Session>, JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str("sessions")?;
        let store = transaction.object_store("sessions")?;
        let request = store.get(&JsValue::from_str(session_id))?;
        
        let result = JsFuture::from(request).await?;
        
        if result.is_undefined() {
            return Ok(None);
        }
        
        let stored_session: StoredSession = serde_wasm_bindgen::from_value(result)?;
        
        let mut session = Session::new();
        session.id = stored_session.id;
        session.messages = stored_session.messages;
        session.created_at = stored_session.created_at;
        
        Ok(Some(session))
    }
    
    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionMetadata>, JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str("sessions")?;
        let store = transaction.object_store("sessions")?;
        let request = store.get_all()?;
        
        let result = JsFuture::from(request).await?;
        let sessions_array: Array = result.dyn_into()?;
        
        let mut sessions = Vec::new();
        for i in 0..sessions_array.length() {
            let session_value = sessions_array.get(i);
            let stored_session: StoredSession = serde_wasm_bindgen::from_value(session_value)?;
            
            let title = if stored_session.messages.is_empty() {
                "Empty Session".to_string()
            } else {
                stored_session.messages[0].content.chars().take(50).collect::<String>()
            };
            
            sessions.push(SessionMetadata {
                id: stored_session.id,
                title,
                created_at: stored_session.created_at,
                updated_at: stored_session.updated_at,
                message_count: stored_session.messages.len(),
            });
        }
        
        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(sessions)
    }
    
    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str_and_mode(
            "sessions",
            web_sys::IdbTransactionMode::Readwrite,
        )?;
        
        let store = transaction.object_store("sessions")?;
        let request = store.delete(&JsValue::from_str(session_id))?;
        
        JsFuture::from(request).await?;
        
        // Also delete associated messages
        self.delete_session_messages(session_id).await?;
        
        Ok(())
    }
    
    /// Save a message to storage
    pub async fn save_message(&self, session_id: &str, message: &Message) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str_and_mode(
            "messages",
            web_sys::IdbTransactionMode::Readwrite,
        )?;
        
        let store = transaction.object_store("messages")?;
        
        let mut message_data = Object::new();
        js_sys::Reflect::set(&message_data, &"id".into(), &message.id.clone().into())?;
        js_sys::Reflect::set(&message_data, &"session_id".into(), &session_id.into())?;
        js_sys::Reflect::set(&message_data, &"role".into(), &format!("{:?}", message.role).into())?;
        js_sys::Reflect::set(&message_data, &"content".into(), &message.content.clone().into())?;
        js_sys::Reflect::set(&message_data, &"timestamp".into(), &message.timestamp.clone().into())?;
        
        let key = format!("{}_{}", session_id, message.id);
        let request = store.put_with_key(&message_data, &JsValue::from_str(&key))?;
        
        JsFuture::from(request).await?;
        Ok(())
    }
    
    /// Delete all messages for a session
    async fn delete_session_messages(&self, session_id: &str) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str_and_mode(
            "messages",
            web_sys::IdbTransactionMode::Readwrite,
        )?;
        
        let store = transaction.object_store("messages")?;
        let index = store.index("session_id")?;
        let request = index.get_all_with_key(&JsValue::from_str(session_id))?;
        
        let result = JsFuture::from(request).await?;
        let messages_array: Array = result.dyn_into()?;
        
        for i in 0..messages_array.length() {
            let message = messages_array.get(i);
            let message_id = js_sys::Reflect::get(&message, &"id".into())?;
            let key = format!("{}_{}", session_id, message_id.as_string().unwrap_or_default());
            let delete_request = store.delete(&JsValue::from_str(&key))?;
            JsFuture::from(delete_request).await?;
        }
        
        Ok(())
    }
    
    /// Save preferences
    pub async fn save_preference(&self, key: &str, value: &str) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str_and_mode(
            "preferences",
            web_sys::IdbTransactionMode::Readwrite,
        )?;
        
        let store = transaction.object_store("preferences")?;
        let request = store.put_with_key(&JsValue::from_str(value), &JsValue::from_str(key))?;
        
        JsFuture::from(request).await?;
        Ok(())
    }
    
    /// Load preferences
    pub async fn load_preference(&self, key: &str) -> Result<Option<String>, JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str("preferences")?;
        let store = transaction.object_store("preferences")?;
        let request = store.get(&JsValue::from_str(key))?;
        
        let result = JsFuture::from(request).await?;
        
        if result.is_undefined() {
            Ok(None)
        } else {
            Ok(result.as_string())
        }
    }
    
    /// Clear all storage
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str_sequence_and_mode(
            &Array::of3(&"sessions".into(), &"messages".into(), &"preferences".into()),
            web_sys::IdbTransactionMode::Readwrite,
        )?;
        
        let sessions_store = transaction.object_store("sessions")?;
        let messages_store = transaction.object_store("messages")?;
        let preferences_store = transaction.object_store("preferences")?;
        
        let clear_sessions = JsFuture::from(sessions_store.clear()?);
        let clear_messages = JsFuture::from(messages_store.clear()?);
        let clear_preferences = JsFuture::from(preferences_store.clear()?);
        
        clear_sessions.await?;
        clear_messages.await?;
        clear_preferences.await?;
        
        Ok(())
    }
    
    /// Get storage usage statistics
    pub async fn get_storage_stats(&self) -> Result<JsValue, JsValue> {
        let db = self.db.as_ref().ok_or_else(|| JsValue::from_str("Database not initialized"))?;
        
        let transaction = db.transaction_with_str_sequence(&Array::of3(
            &"sessions".into(),
            &"messages".into(),
            &"preferences".into(),
        ))?;
        
        let sessions_store = transaction.object_store("sessions")?;
        let messages_store = transaction.object_store("messages")?;
        let preferences_store = transaction.object_store("preferences")?;
        
        let sessions_count = JsFuture::from(sessions_store.count()?).await?;
        let messages_count = JsFuture::from(messages_store.count()?).await?;
        let preferences_count = JsFuture::from(preferences_store.count()?).await?;
        
        let stats = Object::new();
        js_sys::Reflect::set(&stats, &"sessions_count".into(), &sessions_count)?;
        js_sys::Reflect::set(&stats, &"messages_count".into(), &messages_count)?;
        js_sys::Reflect::set(&stats, &"preferences_count".into(), &preferences_count)?;
        
        Ok(stats.into())
    }
}

#[wasm_bindgen]
impl BrowserStorage {
    /// Create a new browser storage instance (WASM constructor)
    #[wasm_bindgen(constructor)]
    pub fn new_wasm() -> BrowserStorage {
        Self::new()
    }
    
    /// Create with custom database name (WASM method)
    #[wasm_bindgen]
    pub fn with_custom_db(db_name: String) -> BrowserStorage {
        Self::with_db_name(db_name)
    }
}