//! WASM-specific functionality for OpenCode
//!
//! This module provides WASM-compatible implementations and utilities
//! for running OpenCode in browser environments.

#[cfg(feature = "wasm-runtime")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm-runtime")]
use web_sys::console;

/// WASM-specific utilities
#[cfg(feature = "wasm-runtime")]
pub mod utils {
    use super::*;
    
    /// Log a message to the browser console
    pub fn log(message: &str) {
        console::log_1(&message.into());
    }
    
    /// Log an error to the browser console
    pub fn error(message: &str) {
        console::error_1(&message.into());
    }
    
    /// Log a warning to the browser console
    pub fn warn(message: &str) {
        console::warn_1(&message.into());
    }
    
    /// Get current timestamp in milliseconds
    pub fn now() -> f64 {
        js_sys::Date::now()
    }
    
    /// Sleep for a specified number of milliseconds
    pub async fn sleep(ms: u32) {
        let mut callback = |resolve: js_sys::Function, _reject: js_sys::Function| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
                .unwrap();
        };
        
        let promise = js_sys::Promise::new(&mut callback);
        wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    }
    
    /// Get user agent string
    pub fn get_user_agent() -> String {
        web_sys::window()
            .and_then(|window| window.navigator().user_agent().ok())
            .unwrap_or_else(|| "unknown".to_string())
    }
    
    /// Check if running in secure context (HTTPS)
    pub fn is_secure_context() -> bool {
        web_sys::window()
            .map(|window| window.is_secure_context())
            .unwrap_or(false)
    }
    
    /// Get current URL
    pub fn get_current_url() -> String {
        web_sys::window()
            .and_then(|window| window.location().href().ok())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// Browser storage utilities
#[cfg(feature = "wasm-runtime")]
pub mod storage {
    use super::*;
    use std::collections::HashMap;
    
    /// Local storage interface
    pub struct LocalStorage;
    
    impl LocalStorage {
        /// Get item from local storage
        pub fn get(key: &str) -> Option<String> {
            web_sys::window()?
                .local_storage().ok()??
                .get_item(key).ok()?
        }
        
        /// Set item in local storage
        pub fn set(key: &str, value: &str) -> Result<(), JsValue> {
            web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?
                .local_storage()
                .map_err(|_| JsValue::from_str("Cannot access localStorage"))?
                .ok_or_else(|| JsValue::from_str("localStorage not available"))?
                .set_item(key, value)
                .map_err(|_| JsValue::from_str("Failed to set item"))
        }
        
        /// Remove item from local storage
        pub fn remove(key: &str) -> Result<(), JsValue> {
            web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?
                .local_storage()
                .map_err(|_| JsValue::from_str("Cannot access localStorage"))?
                .ok_or_else(|| JsValue::from_str("localStorage not available"))?
                .remove_item(key)
                .map_err(|_| JsValue::from_str("Failed to remove item"))
        }
        
        /// Clear all items from local storage
        pub fn clear() -> Result<(), JsValue> {
            web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?
                .local_storage()
                .map_err(|_| JsValue::from_str("Cannot access localStorage"))?
                .ok_or_else(|| JsValue::from_str("localStorage not available"))?
                .clear()
                .map_err(|_| JsValue::from_str("Failed to clear storage"))
        }
        
        /// Get all keys in local storage
        pub fn keys() -> Result<Vec<String>, JsValue> {
            let storage = web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?
                .local_storage()
                .map_err(|_| JsValue::from_str("Cannot access localStorage"))?
                .ok_or_else(|| JsValue::from_str("localStorage not available"))?;
            
            let length = storage.length()
                .map_err(|_| JsValue::from_str("Cannot get storage length"))?;
            
            let mut keys = Vec::new();
            for i in 0..length {
                if let Ok(Some(key)) = storage.key(i) {
                    keys.push(key);
                }
            }
            
            Ok(keys)
        }
    }
    
    /// Session storage interface
    pub struct SessionStorage;
    
    impl SessionStorage {
        /// Get item from session storage
        pub fn get(key: &str) -> Option<String> {
            web_sys::window()?
                .session_storage().ok()??
                .get_item(key).ok()?
        }
        
        /// Set item in session storage
        pub fn set(key: &str, value: &str) -> Result<(), JsValue> {
            web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?
                .session_storage()
                .map_err(|_| JsValue::from_str("Cannot access sessionStorage"))?
                .ok_or_else(|| JsValue::from_str("sessionStorage not available"))?
                .set_item(key, value)
                .map_err(|_| JsValue::from_str("Failed to set item"))
        }
        
        /// Remove item from session storage
        pub fn remove(key: &str) -> Result<(), JsValue> {
            web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?
                .session_storage()
                .map_err(|_| JsValue::from_str("Cannot access sessionStorage"))?
                .ok_or_else(|| JsValue::from_str("sessionStorage not available"))?
                .remove_item(key)
                .map_err(|_| JsValue::from_str("Failed to remove item"))
        }
        
        /// Clear all items from session storage
        pub fn clear() -> Result<(), JsValue> {
            web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?
                .session_storage()
                .map_err(|_| JsValue::from_str("Cannot access sessionStorage"))?
                .ok_or_else(|| JsValue::from_str("sessionStorage not available"))?
                .clear()
                .map_err(|_| JsValue::from_str("Failed to clear storage"))
        }
    }
}

/// DOM utilities
#[cfg(feature = "wasm-runtime")]
pub mod dom {
    use super::*;
    use web_sys::{Document, Element, HtmlElement, Window};
    
    /// Get the window object
    pub fn window() -> Option<Window> {
        web_sys::window()
    }
    
    /// Get the document object
    pub fn document() -> Option<Document> {
        window()?.document()
    }
    
    /// Get element by ID
    pub fn get_element_by_id(id: &str) -> Option<Element> {
        document()?.get_element_by_id(id)
    }
    
    /// Create a new element
    pub fn create_element(tag: &str) -> Result<Element, JsValue> {
        document()
            .ok_or_else(|| JsValue::from_str("No document object"))?
            .create_element(tag)
    }
    
    /// Set element text content
    pub fn set_text_content(element: &Element, text: &str) {
        element.set_text_content(Some(text));
    }
    
    /// Get element text content
    pub fn get_text_content(element: &Element) -> Option<String> {
        element.text_content()
    }
    
    /// Add class to element
    pub fn add_class(element: &Element, class: &str) -> Result<(), JsValue> {
        element.class_list().add_1(class)
    }
    
    /// Remove class from element
    pub fn remove_class(element: &Element, class: &str) -> Result<(), JsValue> {
        element.class_list().remove_1(class)
    }
    
    /// Set element attribute
    pub fn set_attribute(element: &Element, name: &str, value: &str) -> Result<(), JsValue> {
        element.set_attribute(name, value)
    }
    
    /// Get element attribute
    pub fn get_attribute(element: &Element, name: &str) -> Option<String> {
        element.get_attribute(name)
    }
    
    /// Append child to element
    pub fn append_child(parent: &Element, child: &Element) -> Result<(), JsValue> {
        parent.append_child(child)?;
        Ok(())
    }
    
    /// Remove child from element
    pub fn remove_child(parent: &Element, child: &Element) -> Result<(), JsValue> {
        parent.remove_child(child)?;
        Ok(())
    }
    
    /// Show an alert dialog
    pub fn alert(message: &str) {
        if let Some(window) = window() {
            let _ = window.alert_with_message(message);
        }
    }
    
    /// Show a confirm dialog
    pub fn confirm(message: &str) -> bool {
        window()
            .and_then(|w| w.confirm_with_message(message).ok())
            .unwrap_or(false)
    }
    
    /// Show a prompt dialog
    pub fn prompt(message: &str, default_value: Option<&str>) -> Option<String> {
        window()?
            .prompt_with_message_and_default(message, default_value.unwrap_or(""))
            .ok()?
    }
}

/// Fetch utilities for HTTP requests
#[cfg(feature = "wasm-runtime")]
pub mod fetch {
    use super::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    
    /// HTTP request builder
    pub struct RequestBuilder {
        url: String,
        method: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<String>,
        mode: RequestMode,
    }
    
    impl RequestBuilder {
        /// Create a new request builder
        pub fn new(url: &str) -> Self {
            RequestBuilder {
                url: url.to_string(),
                method: "GET".to_string(),
                headers: std::collections::HashMap::new(),
                body: None,
                mode: RequestMode::Cors,
            }
        }
        
        /// Set HTTP method
        pub fn method(mut self, method: &str) -> Self {
            self.method = method.to_string();
            self
        }
        
        /// Add header
        pub fn header(mut self, key: &str, value: &str) -> Self {
            self.headers.insert(key.to_string(), value.to_string());
            self
        }
        
        /// Set request body
        pub fn body(mut self, body: &str) -> Self {
            self.body = Some(body.to_string());
            self
        }
        
        /// Set request mode
        pub fn mode(mut self, mode: RequestMode) -> Self {
            self.mode = mode;
            self
        }
        
        /// Send the request
        pub async fn send(self) -> Result<Response, JsValue> {
            let mut opts = RequestInit::new();
            opts.method(&self.method);
            opts.mode(self.mode);
            
            if let Some(body) = self.body {
                opts.body(Some(&JsValue::from_str(&body)));
            }
            
            let request = Request::new_with_str_and_init(&self.url, &opts)?;
            
            // Set headers
            for (key, value) in self.headers {
                request.headers().set(&key, &value)?;
            }
            
            let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
            let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
            
            Ok(resp_value.dyn_into::<Response>()?)
        }
    }
    
    /// Simple GET request
    pub async fn get(url: &str) -> Result<Response, JsValue> {
        RequestBuilder::new(url).send().await
    }
    
    /// Simple POST request
    pub async fn post(url: &str, body: &str) -> Result<Response, JsValue> {
        RequestBuilder::new(url)
            .method("POST")
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
    }
    
    /// Get response text
    pub async fn response_text(response: Response) -> Result<String, JsValue> {
        let text_promise = response.text()?;
        let text_value = JsFuture::from(text_promise).await?;
        Ok(text_value.as_string().unwrap_or_default())
    }
    
    /// Get response JSON
    pub async fn response_json(response: Response) -> Result<JsValue, JsValue> {
        let json_promise = response.json()?;
        JsFuture::from(json_promise).await
    }
}

/// WebSocket utilities
#[cfg(feature = "wasm-runtime")]
pub mod websocket {
    use super::*;
    use js_sys::Function;
    use wasm_bindgen::closure::Closure;
    use web_sys::{MessageEvent, WebSocket, CloseEvent, ErrorEvent};
    
    /// WebSocket wrapper
    pub struct WebSocketWrapper {
        ws: WebSocket,
        _onmessage: Closure<dyn FnMut(MessageEvent)>,
        _onopen: Closure<dyn FnMut(JsValue)>,
        _onerror: Closure<dyn FnMut(ErrorEvent)>,
        _onclose: Closure<dyn FnMut(CloseEvent)>,
    }
    
    impl WebSocketWrapper {
        /// Create a new WebSocket connection
        pub fn new(
            url: &str,
            on_message: Box<dyn FnMut(String)>,
            on_open: Box<dyn FnMut()>,
            on_error: Box<dyn FnMut(String)>,
            on_close: Box<dyn FnMut(u16, String)>,
        ) -> Result<Self, JsValue> {
            let ws = WebSocket::new(url)?;
            
            let mut on_message = on_message;
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    on_message(txt.into());
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            
            let mut on_open = on_open;
            let onopen = Closure::wrap(Box::new(move |_| {
                on_open();
            }) as Box<dyn FnMut(JsValue)>);
            
            let mut on_error = on_error;
            let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
                on_error(e.message());
            }) as Box<dyn FnMut(ErrorEvent)>);
            
            let mut on_close = on_close;
            let onclose = Closure::wrap(Box::new(move |e: CloseEvent| {
                on_close(e.code(), e.reason());
            }) as Box<dyn FnMut(CloseEvent)>);
            
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            
            Ok(WebSocketWrapper {
                ws,
                _onmessage: onmessage,
                _onopen: onopen,
                _onerror: onerror,
                _onclose: onclose,
            })
        }
        
        /// Send a message
        pub fn send(&self, message: &str) -> Result<(), JsValue> {
            self.ws.send_with_str(message)
        }
        
        /// Close the connection
        pub fn close(&self) -> Result<(), JsValue> {
            self.ws.close()
        }
        
        /// Get ready state
        pub fn ready_state(&self) -> u16 {
            self.ws.ready_state()
        }
    }
}

/// Performance utilities
#[cfg(feature = "wasm-runtime")]
pub mod performance {
    use super::*;
    
    /// Performance timer
    pub struct Timer {
        start_time: f64,
        label: String,
    }
    
    impl Timer {
        /// Start a new timer
        pub fn start(label: &str) -> Self {
            Timer {
                start_time: utils::now(),
                label: label.to_string(),
            }
        }
        
        /// Stop the timer and return elapsed time
        pub fn stop(self) -> f64 {
            let elapsed = utils::now() - self.start_time;
            utils::log(&format!("Timer '{}': {:.2}ms", self.label, elapsed));
            elapsed
        }
        
        /// Get elapsed time without stopping
        pub fn elapsed(&self) -> f64 {
            utils::now() - self.start_time
        }
    }
    
    /// Measure function execution time
    pub async fn measure<F, T>(label: &str, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let timer = Timer::start(label);
        let result = f.await;
        timer.stop();
        result
    }
    
    /// Get performance navigation timing
    pub fn get_navigation_timing() -> Result<js_sys::Object, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let performance = window.performance().ok_or_else(|| JsValue::from_str("No performance object"))?;
        let timing = performance.timing();
        
        let result = js_sys::Object::new();
        js_sys::Reflect::set(&result, &"domLoading".into(), &timing.dom_loading().into())?;
        js_sys::Reflect::set(&result, &"domInteractive".into(), &timing.dom_interactive().into())?;
        js_sys::Reflect::set(&result, &"domContentLoadedEventEnd".into(), &timing.dom_content_loaded_event_end().into())?;
        js_sys::Reflect::set(&result, &"loadEventEnd".into(), &timing.load_event_end().into())?;
        
        Ok(result)
    }
}

/// Error handling utilities
#[cfg(feature = "wasm-runtime")]
pub mod error {
    use super::*;
    
    /// Convert JsValue to readable error string
    pub fn js_value_to_string(value: &JsValue) -> String {
        if let Some(s) = value.as_string() {
            s
        } else if let Some(obj) = value.dyn_ref::<js_sys::Object>() {
            js_sys::JSON::stringify(obj)
                .map(|s| s.into())
                .unwrap_or_else(|_| "Unknown error".to_string())
        } else {
            "Unknown error".to_string()
        }
    }
    
    /// Log error to console
    pub fn log_error(error: &JsValue, context: &str) {
        let error_str = js_value_to_string(error);
        utils::error(&format!("{}: {}", context, error_str));
    }
    
    /// Create a JavaScript Error object
    pub fn create_error(message: &str) -> JsValue {
        js_sys::Error::new(message).into()
    }
}

// Re-export commonly used types for convenience
#[cfg(feature = "wasm-runtime")]
pub use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm-runtime")]
pub use web_sys;
#[cfg(feature = "wasm-runtime")]
pub use js_sys;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn test_wasm_module_compilation() {
        // This test just ensures the module compiles
        // Actual functionality would need to be tested in a browser environment
    }
}