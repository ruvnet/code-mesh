//! Web interface services
//!
//! This module contains service layers for the OpenCode web interface.

mod agent_service;
mod memory_service;
mod provider_service;
mod chat_service;
mod config_service;
mod notification_service;
mod theme_service;
mod storage_service;
mod websocket_service;
mod api_service;
mod auth_service;
mod metrics_service;

// Re-export services
pub use agent_service::AgentService;
pub use memory_service::MemoryService;
pub use provider_service::ProviderService;
pub use chat_service::ChatService;
pub use config_service::ConfigService;
pub use notification_service::NotificationService;
pub use theme_service::ThemeService;
pub use storage_service::StorageService;
pub use websocket_service::WebSocketService;
pub use api_service::ApiService;
pub use auth_service::AuthService;
pub use metrics_service::MetricsService;

// Service utilities
pub mod utils {
    use wasm_bindgen::prelude::*;
    use web_sys::{Storage, Window};
    use serde::{Serialize, Deserialize};
    
    /// Service error types
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum ServiceError {
        NetworkError(String),
        StorageError(String),
        ParseError(String),
        AuthError(String),
        ValidationError(String),
        UnknownError(String),
    }
    
    impl std::fmt::Display for ServiceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ServiceError::NetworkError(msg) => write!(f, "Network error: {}", msg),
                ServiceError::StorageError(msg) => write!(f, "Storage error: {}", msg),
                ServiceError::ParseError(msg) => write!(f, "Parse error: {}", msg),
                ServiceError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
                ServiceError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
                ServiceError::UnknownError(msg) => write!(f, "Unknown error: {}", msg),
            }
        }
    }
    
    impl std::error::Error for ServiceError {}
    
    /// Service result type
    pub type ServiceResult<T> = Result<T, ServiceError>;
    
    /// Get window object
    pub fn window() -> Result<Window, ServiceError> {
        web_sys::window().ok_or_else(|| ServiceError::UnknownError("No window object".to_string()))
    }
    
    /// Get local storage
    pub fn local_storage() -> Result<Storage, ServiceError> {
        window()?
            .local_storage()
            .map_err(|e| ServiceError::StorageError(format!("Failed to get local storage: {:?}", e)))?
            .ok_or_else(|| ServiceError::StorageError("Local storage not available".to_string()))
    }
    
    /// Get session storage
    pub fn session_storage() -> Result<Storage, ServiceError> {
        window()?
            .session_storage()
            .map_err(|e| ServiceError::StorageError(format!("Failed to get session storage: {:?}", e)))?
            .ok_or_else(|| ServiceError::StorageError("Session storage not available".to_string()))
    }
    
    /// Store data in local storage
    pub fn store_local<T: Serialize>(key: &str, value: &T) -> ServiceResult<()> {
        let storage = local_storage()?;
        let serialized = serde_json::to_string(value)
            .map_err(|e| ServiceError::ParseError(format!("Failed to serialize: {}", e)))?;
        
        storage.set_item(key, &serialized)
            .map_err(|e| ServiceError::StorageError(format!("Failed to store item: {:?}", e)))?;
        
        Ok(())
    }
    
    /// Load data from local storage
    pub fn load_local<T: for<'de> Deserialize<'de>>(key: &str) -> ServiceResult<Option<T>> {
        let storage = local_storage()?;
        
        match storage.get_item(key) {
            Ok(Some(data)) => {
                let deserialized = serde_json::from_str(&data)
                    .map_err(|e| ServiceError::ParseError(format!("Failed to deserialize: {}", e)))?;
                Ok(Some(deserialized))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ServiceError::StorageError(format!("Failed to load item: {:?}", e))),
        }
    }
    
    /// Remove data from local storage
    pub fn remove_local(key: &str) -> ServiceResult<()> {
        let storage = local_storage()?;
        storage.remove_item(key)
            .map_err(|e| ServiceError::StorageError(format!("Failed to remove item: {:?}", e)))?;
        Ok(())
    }
    
    /// Clear all local storage
    pub fn clear_local() -> ServiceResult<()> {
        let storage = local_storage()?;
        storage.clear()
            .map_err(|e| ServiceError::StorageError(format!("Failed to clear storage: {:?}", e)))?;
        Ok(())
    }
    
    /// Store data in session storage
    pub fn store_session<T: Serialize>(key: &str, value: &T) -> ServiceResult<()> {
        let storage = session_storage()?;
        let serialized = serde_json::to_string(value)
            .map_err(|e| ServiceError::ParseError(format!("Failed to serialize: {}", e)))?;
        
        storage.set_item(key, &serialized)
            .map_err(|e| ServiceError::StorageError(format!("Failed to store item: {:?}", e)))?;
        
        Ok(())
    }
    
    /// Load data from session storage
    pub fn load_session<T: for<'de> Deserialize<'de>>(key: &str) -> ServiceResult<Option<T>> {
        let storage = session_storage()?;
        
        match storage.get_item(key) {
            Ok(Some(data)) => {
                let deserialized = serde_json::from_str(&data)
                    .map_err(|e| ServiceError::ParseError(format!("Failed to deserialize: {}", e)))?;
                Ok(Some(deserialized))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ServiceError::StorageError(format!("Failed to load item: {:?}", e))),
        }
    }
    
    /// Generate unique request ID
    pub fn generate_request_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Get current timestamp
    pub fn timestamp() -> u64 {
        js_sys::Date::now() as u64
    }
    
    /// Format timestamp for display
    pub fn format_timestamp(timestamp: u64) -> String {
        let date = js_sys::Date::new_with_milliseconds(timestamp as f64);
        date.to_iso_string().into()
    }
    
    /// Validate email format
    pub fn is_valid_email(email: &str) -> bool {
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        email_regex.is_match(email)
    }
    
    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }
    
    /// Sanitize HTML content
    pub fn sanitize_html(html: &str) -> String {
        // Basic HTML sanitization - remove script tags and dangerous attributes
        let mut sanitized = html.to_string();
        
        // Remove script tags
        sanitized = sanitized.replace("<script", "&lt;script");
        sanitized = sanitized.replace("</script>", "&lt;/script&gt;");
        
        // Remove dangerous attributes
        sanitized = sanitized.replace("onclick=", "data-onclick=");
        sanitized = sanitized.replace("onload=", "data-onload=");
        sanitized = sanitized.replace("onerror=", "data-onerror=");
        sanitized = sanitized.replace("onmouseover=", "data-onmouseover=");
        
        sanitized
    }
    
    /// Escape HTML content
    pub fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
    
    /// Truncate text with ellipsis
    pub fn truncate_text(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            text.to_string()
        } else {
            format!("{}...", &text[..max_length.saturating_sub(3)])
        }
    }
    
    /// Format file size
    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        
        if bytes == 0 {
            return "0 B".to_string();
        }
        
        let unit_index = (bytes as f64).log2().floor() as usize / 10;
        let unit_index = unit_index.min(UNITS.len() - 1);
        
        let size = bytes as f64 / (1024_f64).powi(unit_index as i32);
        
        format!("{:.1} {}", size, UNITS[unit_index])
    }
    
    /// Debounce function calls
    pub struct Debouncer {
        timeout_id: Option<i32>,
    }
    
    impl Debouncer {
        pub fn new() -> Self {
            Self { timeout_id: None }
        }
        
        pub fn debounce<F>(&mut self, delay: u32, callback: F)
        where
            F: FnOnce() + 'static,
        {
            // Clear existing timeout
            if let Some(id) = self.timeout_id {
                window().unwrap().clear_timeout_with_handle(id);
            }
            
            // Set new timeout
            let closure = Closure::once_into_js(callback);
            self.timeout_id = Some(
                window().unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        delay as i32,
                    )
                    .unwrap()
            );
        }
    }
    
    impl Default for Debouncer {
        fn default() -> Self {
            Self::new()
        }
    }
    
    /// Throttle function calls
    pub struct Throttler {
        last_call: u64,
        delay: u64,
    }
    
    impl Throttler {
        pub fn new(delay: u64) -> Self {
            Self {
                last_call: 0,
                delay,
            }
        }
        
        pub fn throttle<F>(&mut self, callback: F) -> bool
        where
            F: FnOnce(),
        {
            let now = timestamp();
            
            if now - self.last_call >= self.delay {
                self.last_call = now;
                callback();
                true
            } else {
                false
            }
        }
    }
    
    /// Cache with TTL
    pub struct Cache<T> {
        data: Option<T>,
        timestamp: u64,
        ttl: u64,
    }
    
    impl<T> Cache<T> {
        pub fn new(ttl: u64) -> Self {
            Self {
                data: None,
                timestamp: 0,
                ttl,
            }
        }
        
        pub fn get(&self) -> Option<&T> {
            let now = timestamp();
            if now - self.timestamp < self.ttl {
                self.data.as_ref()
            } else {
                None
            }
        }
        
        pub fn set(&mut self, data: T) {
            self.data = Some(data);
            self.timestamp = timestamp();
        }
        
        pub fn invalidate(&mut self) {
            self.data = None;
            self.timestamp = 0;
        }
        
        pub fn is_expired(&self) -> bool {
            let now = timestamp();
            now - self.timestamp >= self.ttl
        }
    }
    
    impl<T> Default for Cache<T> {
        fn default() -> Self {
            Self::new(300000) // 5 minutes default TTL
        }
    }
}

// Service trait
pub trait Service {
    type Error;
    
    fn init() -> impl std::future::Future<Output = Result<Self, Self::Error>> + Send
    where
        Self: Sized;
    
    fn cleanup(&mut self) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}

// Service registry
pub struct ServiceRegistry {
    services: std::collections::HashMap<String, Box<dyn std::any::Any>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
        }
    }
    
    pub fn register<T: 'static>(&mut self, name: String, service: T) {
        self.services.insert(name, Box::new(service));
    }
    
    pub fn get<T: 'static>(&self, name: &str) -> Option<&T> {
        self.services.get(name)?.downcast_ref::<T>()
    }
    
    pub fn remove(&mut self, name: &str) -> Option<Box<dyn std::any::Any>> {
        self.services.remove(name)
    }
    
    pub fn clear(&mut self) {
        self.services.clear();
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Global service registry
static mut SERVICE_REGISTRY: Option<ServiceRegistry> = None;

/// Initialize global service registry
pub fn init_service_registry() {
    unsafe {
        SERVICE_REGISTRY = Some(ServiceRegistry::new());
    }
}

/// Get global service registry
pub fn get_service_registry() -> Option<&'static mut ServiceRegistry> {
    unsafe { SERVICE_REGISTRY.as_mut() }
}

/// Register a service globally
pub fn register_service<T: 'static>(name: String, service: T) {
    if let Some(registry) = get_service_registry() {
        registry.register(name, service);
    }
}

/// Get a service from global registry
pub fn get_service<T: 'static>(name: &str) -> Option<&T> {
    get_service_registry()?.get(name)
}