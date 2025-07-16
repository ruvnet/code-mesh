//! Web interface components
//!
//! This module contains reusable UI components for the OpenCode web interface.

mod chat_input;
mod chat_message;
mod agent_card;
mod provider_card;
mod memory_item;
mod loading_spinner;
mod error_message;
mod modal;
mod button;
mod input;
mod textarea;
mod select;
mod table;
mod status_indicator;
mod progress_bar;
mod navigation;
mod sidebar;
mod header;
mod footer;

// Re-export components
pub use chat_input::ChatInput;
pub use chat_message::ChatMessage;
pub use agent_card::AgentCard;
pub use provider_card::ProviderCard;
pub use memory_item::MemoryItem;
pub use loading_spinner::LoadingSpinner;
pub use error_message::ErrorMessage;
pub use modal::Modal;
pub use button::Button;
pub use input::Input;
pub use textarea::Textarea;
pub use select::Select;
pub use table::Table;
pub use status_indicator::StatusIndicator;
pub use progress_bar::ProgressBar;
pub use navigation::Navigation;
pub use sidebar::Sidebar;
pub use header::Header;
pub use footer::Footer;

// Component utilities
pub mod utils {
    use web_sys::HtmlElement;
    use yew::prelude::*;
    
    /// Focus an element by ID
    pub fn focus_element(id: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id(id) {
                    if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                        let _ = html_element.focus();
                    }
                }
            }
        }
    }
    
    /// Scroll to bottom of an element
    pub fn scroll_to_bottom(id: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id(id) {
                    if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                        html_element.set_scroll_top(html_element.scroll_height());
                    }
                }
            }
        }
    }
    
    /// Get element dimensions
    pub fn get_element_dimensions(id: &str) -> Option<(i32, i32)> {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id(id) {
                    if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                        return Some((html_element.offset_width(), html_element.offset_height()));
                    }
                }
            }
        }
        None
    }
    
    /// Copy text to clipboard
    pub fn copy_to_clipboard(text: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(navigator) = window.navigator() {
                if let Some(clipboard) = navigator.clipboard() {
                    let _ = clipboard.write_text(text);
                }
            }
        }
    }
    
    /// Format timestamp for display
    pub fn format_timestamp(timestamp: chrono::DateTime<chrono::Utc>) -> String {
        let local: chrono::DateTime<chrono::Local> = timestamp.into();
        local.format("%H:%M:%S").to_string()
    }
    
    /// Format duration for display
    pub fn format_duration(duration: chrono::Duration) -> String {
        let seconds = duration.num_seconds();
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        }
    }
    
    /// Truncate text to specified length
    pub fn truncate_text(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            text.to_string()
        } else {
            format!("{}...", &text[..max_length.saturating_sub(3)])
        }
    }
    
    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }
    
    /// Generate random ID
    pub fn generate_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    /// Get current timestamp
    pub fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
    
    /// Check if element is visible
    pub fn is_element_visible(id: &str) -> bool {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id(id) {
                    if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                        return html_element.offset_height() > 0 && html_element.offset_width() > 0;
                    }
                }
            }
        }
        false
    }
}

// Component types and traits
pub trait Component {
    type Message;
    type Properties;
    
    fn create(ctx: &Context<Self>) -> Self;
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool;
    fn view(&self, ctx: &Context<Self>) -> Html;
}

// Common component properties
#[derive(Properties, PartialEq)]
pub struct BaseProps {
    pub id: Option<String>,
    pub class: Option<String>,
    pub style: Option<String>,
    pub disabled: Option<bool>,
    pub children: Children,
}

#[derive(Properties, PartialEq)]
pub struct ClickableProps {
    pub onclick: Option<Callback<MouseEvent>>,
    pub onmouseenter: Option<Callback<MouseEvent>>,
    pub onmouseleave: Option<Callback<MouseEvent>>,
}

#[derive(Properties, PartialEq)]
pub struct InputProps {
    pub value: String,
    pub placeholder: Option<String>,
    pub oninput: Option<Callback<InputEvent>>,
    pub onkeypress: Option<Callback<KeyboardEvent>>,
    pub onfocus: Option<Callback<FocusEvent>>,
    pub onblur: Option<Callback<FocusEvent>>,
}

// Component themes
#[derive(Clone, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    Auto,
}

#[derive(Clone, PartialEq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

#[derive(Clone, PartialEq)]
pub enum Variant {
    Primary,
    Secondary,
    Success,
    Warning,
    Error,
    Info,
}

// Animation utilities
pub mod animations {
    use yew::prelude::*;
    
    /// Fade in animation class
    pub fn fade_in() -> &'static str {
        "animate-fade-in"
    }
    
    /// Fade out animation class
    pub fn fade_out() -> &'static str {
        "animate-fade-out"
    }
    
    /// Slide in animation class
    pub fn slide_in() -> &'static str {
        "animate-slide-in"
    }
    
    /// Slide out animation class
    pub fn slide_out() -> &'static str {
        "animate-slide-out"
    }
    
    /// Pulse animation class
    pub fn pulse() -> &'static str {
        "animate-pulse"
    }
    
    /// Bounce animation class
    pub fn bounce() -> &'static str {
        "animate-bounce"
    }
    
    /// Shake animation class
    pub fn shake() -> &'static str {
        "animate-shake"
    }
}

// Responsive utilities
pub mod responsive {
    /// Check if screen is mobile
    pub fn is_mobile() -> bool {
        if let Some(window) = web_sys::window() {
            window.inner_width().map(|w| w.as_f64().unwrap_or(0.0) < 768.0).unwrap_or(false)
        } else {
            false
        }
    }
    
    /// Check if screen is tablet
    pub fn is_tablet() -> bool {
        if let Some(window) = web_sys::window() {
            if let Ok(width) = window.inner_width() {
                let w = width.as_f64().unwrap_or(0.0);
                return w >= 768.0 && w < 1024.0;
            }
        }
        false
    }
    
    /// Check if screen is desktop
    pub fn is_desktop() -> bool {
        if let Some(window) = web_sys::window() {
            window.inner_width().map(|w| w.as_f64().unwrap_or(0.0) >= 1024.0).unwrap_or(true)
        } else {
            true
        }
    }
    
    /// Get screen width
    pub fn get_screen_width() -> f64 {
        if let Some(window) = web_sys::window() {
            window.inner_width().map(|w| w.as_f64().unwrap_or(0.0)).unwrap_or(0.0)
        } else {
            0.0
        }
    }
    
    /// Get screen height
    pub fn get_screen_height() -> f64 {
        if let Some(window) = web_sys::window() {
            window.inner_height().map(|h| h.as_f64().unwrap_or(0.0)).unwrap_or(0.0)
        } else {
            0.0
        }
    }
}