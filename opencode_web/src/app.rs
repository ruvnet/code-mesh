//! Main application component

use yew::prelude::*;
use yew::platform::spawn_local;
use web_sys::window;
use opencode_core::wasm_bindings::*;

use crate::components::*;
use crate::pages::*;
use crate::services::*;

/// Main application state
pub struct App {
    /// Current route/page
    current_page: Page,
    
    /// OpenCode engine instance
    engine: Option<OpenCodeEngine>,
    
    /// Current agent
    current_agent: Option<OpenCodeAgent>,
    
    /// Application state
    app_state: AppState,
    
    /// Error message
    error_message: Option<String>,
}

/// Application state
#[derive(Clone, PartialEq)]
pub enum AppState {
    /// Application is initializing
    Initializing,
    
    /// Application is ready
    Ready,
    
    /// Application encountered an error
    Error(String),
}

/// Application pages
#[derive(Clone, PartialEq)]
pub enum Page {
    /// Main chat interface
    Chat,
    
    /// Settings page
    Settings,
    
    /// About page
    About,
    
    /// Agent management
    Agents,
    
    /// Memory management
    Memory,
    
    /// Provider configuration
    Providers,
}

/// Application messages
pub enum AppMsg {
    /// Initialize the application
    Initialize,
    
    /// Initialization completed successfully
    InitializeSuccess(OpenCodeEngine),
    
    /// Initialization failed
    InitializeFailed(String),
    
    /// Navigate to a page
    Navigate(Page),
    
    /// Create a new agent
    CreateAgent(String),
    
    /// Agent created successfully
    AgentCreated(OpenCodeAgent),
    
    /// Agent creation failed
    AgentCreationFailed(String),
    
    /// Clear error message
    ClearError,
}

impl Component for App {
    type Message = AppMsg;
    type Properties = ();
    
    fn create(ctx: &Context<Self>) -> Self {
        // Initialize the application
        ctx.link().send_message(AppMsg::Initialize);
        
        Self {
            current_page: Page::Chat,
            engine: None,
            current_agent: None,
            app_state: AppState::Initializing,
            error_message: None,
        }
    }
    
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::Initialize => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match OpenCodeEngine::init().await {
                        Ok(engine) => {
                            link.send_message(AppMsg::InitializeSuccess(engine));
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to initialize OpenCode: {:?}", e);
                            link.send_message(AppMsg::InitializeFailed(error_msg));
                        }
                    }
                });
                false
            }
            
            AppMsg::InitializeSuccess(engine) => {
                self.engine = Some(engine);
                self.app_state = AppState::Ready;
                self.error_message = None;
                
                // Create default agent
                ctx.link().send_message(AppMsg::CreateAgent("assistant".to_string()));
                true
            }
            
            AppMsg::InitializeFailed(error) => {
                self.app_state = AppState::Error(error.clone());
                self.error_message = Some(error);
                true
            }
            
            AppMsg::Navigate(page) => {
                self.current_page = page;
                true
            }
            
            AppMsg::CreateAgent(name) => {
                if let Some(engine) = &self.engine {
                    let engine_clone = engine.clone();
                    let name_clone = name.clone();
                    let link = ctx.link().clone();
                    
                    spawn_local(async move {
                        match engine_clone.create_agent(&name_clone).await {
                            Ok(agent) => {
                                link.send_message(AppMsg::AgentCreated(agent));
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to create agent: {:?}", e);
                                link.send_message(AppMsg::AgentCreationFailed(error_msg));
                            }
                        }
                    });
                }
                false
            }
            
            AppMsg::AgentCreated(agent) => {
                self.current_agent = Some(agent);
                true
            }
            
            AppMsg::AgentCreationFailed(error) => {
                self.error_message = Some(error);
                true
            }
            
            AppMsg::ClearError => {
                self.error_message = None;
                true
            }
        }
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        
        html! {
            <div class="app">
                { self.render_header(link) }
                { self.render_error_message(link) }
                { self.render_main_content(link) }
                { self.render_footer() }
            </div>
        }
    }
}

impl App {
    /// Render the application header
    fn render_header(&self, link: &Scope<Self>) -> Html {
        html! {
            <header class="app-header">
                <div class="header-content">
                    <h1 class="app-title">{ "OpenCode" }</h1>
                    <nav class="app-nav">
                        <button 
                            class={classes!("nav-button", self.current_page == Page::Chat)}
                            onclick={link.callback(|_| AppMsg::Navigate(Page::Chat))}
                        >
                            { "Chat" }
                        </button>
                        <button 
                            class={classes!("nav-button", self.current_page == Page::Agents)}
                            onclick={link.callback(|_| AppMsg::Navigate(Page::Agents))}
                        >
                            { "Agents" }
                        </button>
                        <button 
                            class={classes!("nav-button", self.current_page == Page::Memory)}
                            onclick={link.callback(|_| AppMsg::Navigate(Page::Memory))}
                        >
                            { "Memory" }
                        </button>
                        <button 
                            class={classes!("nav-button", self.current_page == Page::Providers)}
                            onclick={link.callback(|_| AppMsg::Navigate(Page::Providers))}
                        >
                            { "Providers" }
                        </button>
                        <button 
                            class={classes!("nav-button", self.current_page == Page::Settings)}
                            onclick={link.callback(|_| AppMsg::Navigate(Page::Settings))}
                        >
                            { "Settings" }
                        </button>
                        <button 
                            class={classes!("nav-button", self.current_page == Page::About)}
                            onclick={link.callback(|_| AppMsg::Navigate(Page::About))}
                        >
                            { "About" }
                        </button>
                    </nav>
                </div>
            </header>
        }
    }
    
    /// Render error message if present
    fn render_error_message(&self, link: &Scope<Self>) -> Html {
        if let Some(error) = &self.error_message {
            html! {
                <div class="error-banner">
                    <span class="error-text">{ error }</span>
                    <button 
                        class="error-close"
                        onclick={link.callback(|_| AppMsg::ClearError)}
                    >
                        { "×" }
                    </button>
                </div>
            }
        } else {
            html! {}
        }
    }
    
    /// Render the main content area
    fn render_main_content(&self, link: &Scope<Self>) -> Html {
        match &self.app_state {
            AppState::Initializing => {
                html! {
                    <main class="app-main">
                        <div class="loading-container">
                            <div class="loading-spinner"></div>
                            <p>{ "Initializing OpenCode..." }</p>
                        </div>
                    </main>
                }
            }
            
            AppState::Ready => {
                match &self.current_page {
                    Page::Chat => {
                        html! {
                            <main class="app-main">
                                <ChatPage 
                                    agent={self.current_agent.clone()}
                                    engine={self.engine.clone()}
                                />
                            </main>
                        }
                    }
                    
                    Page::Agents => {
                        html! {
                            <main class="app-main">
                                <AgentsPage 
                                    engine={self.engine.clone()}
                                />
                            </main>
                        }
                    }
                    
                    Page::Memory => {
                        html! {
                            <main class="app-main">
                                <MemoryPage 
                                    engine={self.engine.clone()}
                                />
                            </main>
                        }
                    }
                    
                    Page::Providers => {
                        html! {
                            <main class="app-main">
                                <ProvidersPage 
                                    engine={self.engine.clone()}
                                />
                            </main>
                        }
                    }
                    
                    Page::Settings => {
                        html! {
                            <main class="app-main">
                                <SettingsPage 
                                    engine={self.engine.clone()}
                                />
                            </main>
                        }
                    }
                    
                    Page::About => {
                        html! {
                            <main class="app-main">
                                <AboutPage />
                            </main>
                        }
                    }
                }
            }
            
            AppState::Error(error) => {
                html! {
                    <main class="app-main">
                        <div class="error-container">
                            <h2>{ "Application Error" }</h2>
                            <p>{ error }</p>
                            <button 
                                class="retry-button"
                                onclick={link.callback(|_| AppMsg::Initialize)}
                            >
                                { "Retry" }
                            </button>
                        </div>
                    </main>
                }
            }
        }
    }
    
    /// Render the application footer
    fn render_footer(&self) -> Html {
        let runtime_info = get_runtime_info();
        
        html! {
            <footer class="app-footer">
                <div class="footer-content">
                    <span class="footer-info">
                        { "OpenCode v" }{ get_version() }
                        { " | Running on WebAssembly" }
                    </span>
                    <span class="footer-status">
                        { match &self.app_state {
                            AppState::Initializing => "Initializing...",
                            AppState::Ready => "Ready",
                            AppState::Error(_) => "Error",
                        }}
                    </span>
                </div>
            </footer>
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yew::platform::Runtime;
    
    #[test]
    fn test_app_creation() {
        let runtime = Runtime::new().unwrap();
        let _handle = runtime.spawn_pinned(async {
            let app = App::create(&Context::new());
            assert_eq!(app.app_state, AppState::Initializing);
            assert_eq!(app.current_page, Page::Chat);
        });
    }
}