//! Browser-specific functionality for Code Mesh WASM

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Navigator, Clipboard, ClipboardPermissionDescriptor};
use js_sys::Promise;

/// Get clipboard content (requires user permission)
pub async fn get_clipboard() -> Result<String, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let navigator = window.navigator();
    let clipboard = navigator.clipboard().ok_or_else(|| JsValue::from_str("Clipboard API not available"))?;
    
    // Check permission first
    let permissions = navigator.permissions().ok_or_else(|| JsValue::from_str("Permissions API not available"))?;
    let mut permission_desc = ClipboardPermissionDescriptor::new();
    permission_desc.name("clipboard-read");
    
    let permission_promise = permissions.query(&permission_desc.into())?;
    let permission_result = JsFuture::from(permission_promise).await?;
    
    // Read from clipboard
    let read_promise = clipboard.read_text();
    let text = JsFuture::from(read_promise).await?;
    
    text.as_string().ok_or_else(|| JsValue::from_str("Failed to get clipboard text"))
}

/// Set clipboard content
pub async fn set_clipboard(text: String) -> Result<(), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let navigator = window.navigator();
    let clipboard = navigator.clipboard().ok_or_else(|| JsValue::from_str("Clipboard API not available"))?;
    
    let write_promise = clipboard.write_text(&text);
    JsFuture::from(write_promise).await?;
    
    Ok(())
}

/// Check if running in a secure context (required for many web APIs)
pub fn is_secure_context() -> bool {
    window()
        .and_then(|w| w.is_secure_context())
        .unwrap_or(false)
}

/// Get user agent string
pub fn get_user_agent() -> Option<String> {
    window()
        .and_then(|w| w.navigator().user_agent().ok())
}

/// Get browser language
pub fn get_language() -> Option<String> {
    window()
        .and_then(|w| w.navigator().language())
}

/// Get platform information
pub fn get_platform() -> Option<String> {
    window()
        .and_then(|w| w.navigator().platform().ok())
}

/// Check if service worker is supported
pub fn supports_service_worker() -> bool {
    window()
        .and_then(|w| w.navigator().service_worker())
        .is_some()
}

/// Check if web workers are supported
pub fn supports_web_workers() -> bool {
    js_sys::eval("typeof Worker !== 'undefined'")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Check if IndexedDB is supported
pub fn supports_indexed_db() -> bool {
    window()
        .and_then(|w| w.indexed_db().ok())
        .flatten()
        .is_some()
}

/// Check if localStorage is supported
pub fn supports_local_storage() -> bool {
    window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .is_some()
}

/// Get viewport dimensions
pub fn get_viewport_size() -> Result<(u32, u32), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let width = window.inner_width()?.as_f64().unwrap_or(0.0) as u32;
    let height = window.inner_height()?.as_f64().unwrap_or(0.0) as u32;
    Ok((width, height))
}

/// Check if the page is visible (Page Visibility API)
pub fn is_page_visible() -> bool {
    window()
        .and_then(|w| w.document())
        .and_then(|d| d.visibility_state())
        .map(|state| state == web_sys::VisibilityState::Visible)
        .unwrap_or(true)
}

/// Register visibility change callback
#[wasm_bindgen]
pub fn register_visibility_callback(callback: &js_sys::Function) -> Result<(), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
    
    document.add_event_listener_with_callback("visibilitychange", callback)?;
    Ok(())
}

/// Check if running in standalone mode (PWA)
pub fn is_standalone_mode() -> bool {
    window()
        .and_then(|w| w.navigator().standalone())
        .unwrap_or(false)
}

/// Get connection information
pub fn get_connection_info() -> Result<JsValue, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let navigator = window.navigator();
    
    // This is a non-standard API, so we need to be careful
    let connection = js_sys::Reflect::get(&navigator, &JsValue::from_str("connection"))
        .or_else(|_| js_sys::Reflect::get(&navigator, &JsValue::from_str("mozConnection")))
        .or_else(|_| js_sys::Reflect::get(&navigator, &JsValue::from_str("webkitConnection")))?;
    
    Ok(connection)
}

/// Create a notification (requires permission)
#[wasm_bindgen]
pub async fn create_notification(title: String, body: String) -> Result<(), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    
    // Check if Notification API is available
    let notification_constructor = js_sys::Reflect::get(&window, &JsValue::from_str("Notification"))?;
    if notification_constructor.is_undefined() {
        return Err(JsValue::from_str("Notification API not available"));
    }
    
    // Request permission
    let permission_promise = js_sys::Reflect::apply(
        &js_sys::Reflect::get(&notification_constructor, &JsValue::from_str("requestPermission"))?,
        &notification_constructor,
        &js_sys::Array::new(),
    )?;
    
    let permission = JsFuture::from(Promise::from(permission_promise)).await?;
    
    if permission.as_string().unwrap_or_default() != "granted" {
        return Err(JsValue::from_str("Notification permission denied"));
    }
    
    // Create notification
    let options = js_sys::Object::new();
    js_sys::Reflect::set(&options, &JsValue::from_str("body"), &JsValue::from_str(&body))?;
    
    let args = js_sys::Array::new();
    args.push(&JsValue::from_str(&title));
    args.push(&options);
    
    js_sys::Reflect::construct(&notification_constructor.into(), &args)?;
    
    Ok(())
}

/// Wake lock API for preventing screen sleep
#[wasm_bindgen]
pub async fn request_wake_lock() -> Result<JsValue, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let navigator = window.navigator();
    
    // Check if Wake Lock API is available
    let wake_lock = js_sys::Reflect::get(&navigator, &JsValue::from_str("wakeLock"))?;
    if wake_lock.is_undefined() {
        return Err(JsValue::from_str("Wake Lock API not available"));
    }
    
    let request_fn = js_sys::Reflect::get(&wake_lock, &JsValue::from_str("request"))?;
    let promise = js_sys::Reflect::apply(
        &request_fn.into(),
        &wake_lock,
        &js_sys::Array::of1(&JsValue::from_str("screen")),
    )?;
    
    JsFuture::from(Promise::from(promise)).await
}