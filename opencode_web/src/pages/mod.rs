//! Web interface pages
//!
//! This module contains the main pages for the OpenCode web interface.

mod chat;
mod agents;
mod memory;
mod providers;
mod settings;
mod about;

// Re-export pages
pub use chat::ChatPage;
pub use agents::AgentsPage;
pub use memory::MemoryPage;
pub use providers::ProvidersPage;
pub use settings::SettingsPage;
pub use about::AboutPage;

// Page utilities
pub mod utils {
    use yew::prelude::*;
    use yew_router::prelude::*;
    
    /// Page route definitions
    #[derive(Clone, Routable, PartialEq)]
    pub enum Route {
        #[at("/")]
        Home,
        #[at("/chat")]
        Chat,
        #[at("/agents")]
        Agents,
        #[at("/memory")]
        Memory,
        #[at("/providers")]
        Providers,
        #[at("/settings")]
        Settings,
        #[at("/about")]
        About,
        #[not_found]
        #[at("/404")]
        NotFound,
    }
    
    /// Page metadata
    #[derive(Clone, PartialEq)]
    pub struct PageMeta {
        pub title: String,
        pub description: String,
        pub keywords: Vec<String>,
        pub canonical_url: Option<String>,
    }
    
    impl PageMeta {
        pub fn new(title: &str, description: &str) -> Self {
            Self {
                title: title.to_string(),
                description: description.to_string(),
                keywords: vec![],
                canonical_url: None,
            }
        }
        
        pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
            self.keywords = keywords;
            self
        }
        
        pub fn with_canonical_url(mut self, url: String) -> Self {
            self.canonical_url = Some(url);
            self
        }
    }
    
    /// Get page metadata for route
    pub fn get_page_meta(route: &Route) -> PageMeta {
        match route {
            Route::Home | Route::Chat => PageMeta::new(
                "OpenCode - AI Coding Assistant",
                "Chat with AI agents to get coding help, generate code, and solve problems."
            ).with_keywords(vec![
                "AI".to_string(),
                "coding".to_string(),
                "assistant".to_string(),
                "chat".to_string(),
                "programming".to_string(),
            ]),
            Route::Agents => PageMeta::new(
                "Agents - OpenCode",
                "Manage and configure AI agents for different coding tasks."
            ).with_keywords(vec![
                "agents".to_string(),
                "AI".to_string(),
                "management".to_string(),
                "configuration".to_string(),
            ]),
            Route::Memory => PageMeta::new(
                "Memory - OpenCode",
                "View and manage persistent memory storage across sessions."
            ).with_keywords(vec![
                "memory".to_string(),
                "storage".to_string(),
                "persistence".to_string(),
                "data".to_string(),
            ]),
            Route::Providers => PageMeta::new(
                "Providers - OpenCode",
                "Configure LLM providers like OpenAI, Anthropic, and local models."
            ).with_keywords(vec![
                "providers".to_string(),
                "LLM".to_string(),
                "OpenAI".to_string(),
                "Anthropic".to_string(),
                "models".to_string(),
            ]),
            Route::Settings => PageMeta::new(
                "Settings - OpenCode",
                "Configure application settings and preferences."
            ).with_keywords(vec![
                "settings".to_string(),
                "configuration".to_string(),
                "preferences".to_string(),
                "options".to_string(),
            ]),
            Route::About => PageMeta::new(
                "About - OpenCode",
                "Learn about OpenCode, its features, and development."
            ).with_keywords(vec![
                "about".to_string(),
                "information".to_string(),
                "features".to_string(),
                "development".to_string(),
            ]),
            Route::NotFound => PageMeta::new(
                "Page Not Found - OpenCode",
                "The requested page could not be found."
            ),
        }
    }
    
    /// Set page title
    pub fn set_page_title(title: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                document.set_title(title);
            }
        }
    }
    
    /// Set page description
    pub fn set_page_description(description: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(head) = document.head() {
                    // Remove existing description meta tag
                    if let Some(existing) = document.query_selector("meta[name='description']").unwrap_or(None) {
                        let _ = head.remove_child(&existing);
                    }
                    
                    // Create new description meta tag
                    if let Ok(meta) = document.create_element("meta") {
                        let _ = meta.set_attribute("name", "description");
                        let _ = meta.set_attribute("content", description);
                        let _ = head.append_child(&meta);
                    }
                }
            }
        }
    }
    
    /// Set page keywords
    pub fn set_page_keywords(keywords: &[String]) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(head) = document.head() {
                    // Remove existing keywords meta tag
                    if let Some(existing) = document.query_selector("meta[name='keywords']").unwrap_or(None) {
                        let _ = head.remove_child(&existing);
                    }
                    
                    // Create new keywords meta tag
                    if let Ok(meta) = document.create_element("meta") {
                        let _ = meta.set_attribute("name", "keywords");
                        let _ = meta.set_attribute("content", &keywords.join(", "));
                        let _ = head.append_child(&meta);
                    }
                }
            }
        }
    }
    
    /// Update page metadata
    pub fn update_page_meta(meta: &PageMeta) {
        set_page_title(&meta.title);
        set_page_description(&meta.description);
        set_page_keywords(&meta.keywords);
        
        if let Some(canonical_url) = &meta.canonical_url {
            set_canonical_url(canonical_url);
        }
    }
    
    /// Set canonical URL
    pub fn set_canonical_url(url: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(head) = document.head() {
                    // Remove existing canonical link
                    if let Some(existing) = document.query_selector("link[rel='canonical']").unwrap_or(None) {
                        let _ = head.remove_child(&existing);
                    }
                    
                    // Create new canonical link
                    if let Ok(link) = document.create_element("link") {
                        let _ = link.set_attribute("rel", "canonical");
                        let _ = link.set_attribute("href", url);
                        let _ = head.append_child(&link);
                    }
                }
            }
        }
    }
    
    /// Get current URL
    pub fn get_current_url() -> String {
        if let Some(window) = web_sys::window() {
            if let Some(location) = window.location() {
                return location.href().unwrap_or_default();
            }
        }
        String::new()
    }
    
    /// Navigate to URL
    pub fn navigate_to(url: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(location) = window.location() {
                let _ = location.assign(url);
            }
        }
    }
    
    /// Reload page
    pub fn reload_page() {
        if let Some(window) = web_sys::window() {
            if let Some(location) = window.location() {
                let _ = location.reload();
            }
        }
    }
    
    /// Go back in history
    pub fn go_back() {
        if let Some(window) = web_sys::window() {
            if let Some(history) = window.history() {
                let _ = history.back();
            }
        }
    }
    
    /// Go forward in history
    pub fn go_forward() {
        if let Some(window) = web_sys::window() {
            if let Some(history) = window.history() {
                let _ = history.forward();
            }
        }
    }
}

// Page base component
#[derive(Properties, PartialEq)]
pub struct PageProps {
    pub title: String,
    pub children: Children,
}

/// Base page component
pub struct BasePage {
    pub title: String,
}

/// Page component trait
pub trait PageComponent {
    fn get_title(&self) -> String;
    fn get_meta(&self) -> utils::PageMeta;
    fn render_content(&self, ctx: &Context<Self>) -> Html;
}

/// Page wrapper component
#[function_component(PageWrapper)]
pub fn page_wrapper(props: &PageProps) -> Html {
    use_effect_with_deps(
        {
            let title = props.title.clone();
            move |_| {
                utils::set_page_title(&title);
                || {}
            }
        },
        props.title.clone(),
    );
    
    html! {
        <div class="page-wrapper">
            <div class="page-content">
                { for props.children.iter() }
            </div>
        </div>
    }
}