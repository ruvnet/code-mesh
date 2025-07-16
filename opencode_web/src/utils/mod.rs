//! Web interface utilities
//!
//! This module contains utility functions and helpers for the OpenCode web interface.

mod dom;
mod async_utils;
mod validation;
mod formatting;
mod storage;
mod crypto;
mod color;
mod keyboard;
mod clipboard;
mod url;
mod date;
mod error;
mod logger;
mod performance;
mod device;
mod network;

// Re-export utilities
pub use dom::*;
pub use async_utils::*;
pub use validation::*;
pub use formatting::*;
pub use storage::*;
pub use crypto::*;
pub use color::*;
pub use keyboard::*;
pub use clipboard::*;
pub use url::*;
pub use date::*;
pub use error::*;
pub use logger::*;
pub use performance::*;
pub use device::*;
pub use network::*;

// Common utilities
use wasm_bindgen::prelude::*;
use web_sys::*;
use js_sys::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Result type for utility functions
pub type UtilResult<T> = Result<T, UtilError>;

/// Common error types for utilities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UtilError {
    DomError(String),
    StorageError(String),
    ValidationError(String),
    NetworkError(String),
    ParseError(String),
    CryptoError(String),
    UnknownError(String),
}

impl std::fmt::Display for UtilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UtilError::DomError(msg) => write!(f, "DOM error: {}", msg),
            UtilError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            UtilError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            UtilError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            UtilError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            UtilError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
            UtilError::UnknownError(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for UtilError {}

/// General utility functions
pub mod general {
    use super::*;
    
    /// Generate a random UUID
    pub fn generate_uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Generate a random string of specified length
    pub fn generate_random_string(length: usize) -> String {
        use js_sys::Math;
        
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        
        (0..length)
            .map(|_| {
                let idx = (Math::random() * CHARS.len() as f64) as usize;
                CHARS[idx] as char
            })
            .collect()
    }
    
    /// Get current timestamp in milliseconds
    pub fn timestamp() -> u64 {
        Date::now() as u64
    }
    
    /// Get current timestamp in seconds
    pub fn timestamp_seconds() -> u64 {
        (Date::now() / 1000.0) as u64
    }
    
    /// Sleep for specified milliseconds
    pub async fn sleep(ms: u32) {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let window = web_sys::window().unwrap();
            let closure = Closure::once_into_js(move || {
                resolve.call0(&JsValue::NULL).unwrap();
            });
            window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                ms as i32,
            ).unwrap();
        });
        
        wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    }
    
    /// Debounce a function call
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
                if let Some(window) = web_sys::window() {
                    window.clear_timeout_with_handle(id);
                }
            }
            
            // Set new timeout
            if let Some(window) = web_sys::window() {
                let closure = Closure::once_into_js(callback);
                self.timeout_id = Some(
                    window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        delay as i32,
                    ).unwrap_or(0)
                );
            }
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
    
    /// Simple cache with TTL
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
    
    /// Event emitter for custom events
    pub struct EventEmitter {
        listeners: HashMap<String, Vec<Box<dyn Fn(&JsValue)>>>,
    }
    
    impl EventEmitter {
        pub fn new() -> Self {
            Self {
                listeners: HashMap::new(),
            }
        }
        
        pub fn on<F>(&mut self, event: &str, callback: F)
        where
            F: Fn(&JsValue) + 'static,
        {
            let listeners = self.listeners.entry(event.to_string()).or_insert_with(Vec::new);
            listeners.push(Box::new(callback));
        }
        
        pub fn emit(&self, event: &str, data: &JsValue) {
            if let Some(listeners) = self.listeners.get(event) {
                for listener in listeners {
                    listener(data);
                }
            }
        }
        
        pub fn off(&mut self, event: &str) {
            self.listeners.remove(event);
        }
        
        pub fn clear(&mut self) {
            self.listeners.clear();
        }
    }
    
    impl Default for EventEmitter {
        fn default() -> Self {
            Self::new()
        }
    }
    
    /// Simple state management
    pub struct State<T> {
        value: T,
        listeners: Vec<Box<dyn Fn(&T)>>,
    }
    
    impl<T> State<T> {
        pub fn new(initial: T) -> Self {
            Self {
                value: initial,
                listeners: Vec::new(),
            }
        }
        
        pub fn get(&self) -> &T {
            &self.value
        }
        
        pub fn set(&mut self, value: T) {
            self.value = value;
            self.notify();
        }
        
        pub fn update<F>(&mut self, updater: F)
        where
            F: FnOnce(&mut T),
        {
            updater(&mut self.value);
            self.notify();
        }
        
        pub fn subscribe<F>(&mut self, callback: F)
        where
            F: Fn(&T) + 'static,
        {
            self.listeners.push(Box::new(callback));
        }
        
        fn notify(&self) {
            for listener in &self.listeners {
                listener(&self.value);
            }
        }
    }
    
    /// Rate limiter
    pub struct RateLimiter {
        requests: Vec<u64>,
        max_requests: usize,
        window_ms: u64,
    }
    
    impl RateLimiter {
        pub fn new(max_requests: usize, window_ms: u64) -> Self {
            Self {
                requests: Vec::new(),
                max_requests,
                window_ms,
            }
        }
        
        pub fn try_request(&mut self) -> bool {
            let now = timestamp();
            
            // Remove old requests outside the window
            self.requests.retain(|&req_time| now - req_time < self.window_ms);
            
            if self.requests.len() < self.max_requests {
                self.requests.push(now);
                true
            } else {
                false
            }
        }
        
        pub fn reset(&mut self) {
            self.requests.clear();
        }
        
        pub fn remaining(&self) -> usize {
            self.max_requests.saturating_sub(self.requests.len())
        }
    }
    
    /// Simple queue implementation
    pub struct Queue<T> {
        items: Vec<T>,
        max_size: Option<usize>,
    }
    
    impl<T> Queue<T> {
        pub fn new() -> Self {
            Self {
                items: Vec::new(),
                max_size: None,
            }
        }
        
        pub fn with_capacity(max_size: usize) -> Self {
            Self {
                items: Vec::new(),
                max_size: Some(max_size),
            }
        }
        
        pub fn push(&mut self, item: T) {
            if let Some(max_size) = self.max_size {
                if self.items.len() >= max_size {
                    self.items.remove(0); // Remove oldest item
                }
            }
            self.items.push(item);
        }
        
        pub fn pop(&mut self) -> Option<T> {
            if self.items.is_empty() {
                None
            } else {
                Some(self.items.remove(0))
            }
        }
        
        pub fn peek(&self) -> Option<&T> {
            self.items.first()
        }
        
        pub fn len(&self) -> usize {
            self.items.len()
        }
        
        pub fn is_empty(&self) -> bool {
            self.items.is_empty()
        }
        
        pub fn clear(&mut self) {
            self.items.clear();
        }
    }
    
    impl<T> Default for Queue<T> {
        fn default() -> Self {
            Self::new()
        }
    }
}

// Re-export general utilities
pub use general::*;

// Environment detection
pub fn is_development() -> bool {
    cfg!(debug_assertions)
}

pub fn is_production() -> bool {
    !is_development()
}

// Browser detection
pub fn get_user_agent() -> String {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_else(|| "Unknown".to_string())
}

pub fn is_mobile_browser() -> bool {
    let user_agent = get_user_agent().to_lowercase();
    user_agent.contains("mobile") || user_agent.contains("android") || user_agent.contains("iphone")
}

pub fn is_safari() -> bool {
    let user_agent = get_user_agent().to_lowercase();
    user_agent.contains("safari") && !user_agent.contains("chrome")
}

pub fn is_chrome() -> bool {
    let user_agent = get_user_agent().to_lowercase();
    user_agent.contains("chrome")
}

pub fn is_firefox() -> bool {
    let user_agent = get_user_agent().to_lowercase();
    user_agent.contains("firefox")
}

// Feature detection
pub fn supports_local_storage() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .is_some()
}

pub fn supports_session_storage() -> bool {
    web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
        .is_some()
}

pub fn supports_websockets() -> bool {
    js_sys::eval("typeof WebSocket !== 'undefined'").unwrap().as_bool().unwrap_or(false)
}

pub fn supports_web_workers() -> bool {
    js_sys::eval("typeof Worker !== 'undefined'").unwrap().as_bool().unwrap_or(false)
}

pub fn supports_clipboard_api() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().clipboard())
        .is_some()
}

// Screen utilities
pub fn get_screen_size() -> (u32, u32) {
    let window = web_sys::window().unwrap();
    let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
    let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
    (width, height)
}

pub fn get_viewport_size() -> (u32, u32) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let element = document.document_element().unwrap();
    
    let width = element.client_width() as u32;
    let height = element.client_height() as u32;
    (width, height)
}

pub fn is_mobile_screen() -> bool {
    let (width, _) = get_screen_size();
    width <= 768
}

pub fn is_tablet_screen() -> bool {
    let (width, _) = get_screen_size();
    width > 768 && width <= 1024
}

pub fn is_desktop_screen() -> bool {
    let (width, _) = get_screen_size();
    width > 1024
}

// Console utilities
pub fn log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

pub fn warn(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

pub fn error(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
}

pub fn info(message: &str) {
    web_sys::console::info_1(&JsValue::from_str(message));
}

pub fn debug(message: &str) {
    if is_development() {
        web_sys::console::debug_1(&JsValue::from_str(message));
    }
}

// Performance utilities
pub fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

pub fn measure_performance<T, F>(name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = performance_now();
    let result = f();
    let end = performance_now();
    
    debug(&format!("{}: {:.2}ms", name, end - start));
    result
}

// Async utilities
pub async fn request_animation_frame() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window = web_sys::window().unwrap();
        let closure = Closure::once_into_js(move || {
            resolve.call0(&JsValue::NULL).unwrap();
        });
        window.request_animation_frame(closure.as_ref().unchecked_ref()).unwrap();
    });
    
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

pub async fn request_idle_callback() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window = web_sys::window().unwrap();
        let closure = Closure::once_into_js(move || {
            resolve.call0(&JsValue::NULL).unwrap();
        });
        
        // Check if requestIdleCallback is available
        if js_sys::eval("typeof requestIdleCallback !== 'undefined'").unwrap().as_bool().unwrap_or(false) {
            js_sys::eval(&format!(
                "requestIdleCallback({});",
                closure.as_ref().as_string().unwrap_or_default()
            )).unwrap();
        } else {
            // Fallback to setTimeout
            window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                0,
            ).unwrap();
        }
    });
    
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}