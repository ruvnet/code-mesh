//! Agents page component

use yew::prelude::*;
use yew::platform::spawn_local;
use opencode_core::wasm_bindings::*;
use crate::components::*;
use crate::services::*;
use crate::utils::*;

/// Agents page properties
#[derive(Properties, PartialEq)]
pub struct AgentsPageProps {
    /// OpenCode engine instance
    pub engine: Option<OpenCodeEngine>,
}

/// Agent information structure
#[derive(Clone, PartialEq)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: AgentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub message_count: usize,
    pub provider: String,
    pub model: String,
    pub settings: std::collections::HashMap<String, String>,
}

/// Agent status
#[derive(Clone, PartialEq)]
pub enum AgentStatus {
    Active,
    Idle,
    Busy,
    Error(String),
    Offline,
}

/// Agents page state
pub struct AgentsPage {
    /// List of agents
    agents: Vec<AgentInfo>,
    /// Whether agents are loading
    is_loading: bool,
    /// Error message if any
    error_message: Option<String>,
    /// Selected agent ID
    selected_agent_id: Option<String>,
    /// Show create agent dialog
    show_create_dialog: bool,
    /// New agent form data
    new_agent_form: NewAgentForm,
    /// Search query
    search_query: String,
    /// Filter by status
    status_filter: Option<AgentStatus>,
}

/// New agent form data
#[derive(Clone, PartialEq)]
pub struct NewAgentForm {
    pub name: String,
    pub description: String,
    pub provider: String,
    pub model: String,
    pub settings: std::collections::HashMap<String, String>,
}

/// Agents page messages
pub enum AgentsPageMsg {
    /// Load agents list
    LoadAgents,
    /// Agents loaded successfully
    AgentsLoaded(Vec<AgentInfo>),
    /// Failed to load agents
    LoadAgentsError(String),
    /// Select an agent
    SelectAgent(String),
    /// Create new agent
    CreateAgent,
    /// Show create agent dialog
    ShowCreateDialog,
    /// Hide create agent dialog
    HideCreateDialog,
    /// Update new agent form
    UpdateNewAgentForm(NewAgentForm),
    /// Agent created successfully
    AgentCreated(AgentInfo),
    /// Failed to create agent
    CreateAgentError(String),
    /// Delete an agent
    DeleteAgent(String),
    /// Agent deleted successfully
    AgentDeleted(String),
    /// Failed to delete agent
    DeleteAgentError(String),
    /// Update search query
    UpdateSearchQuery(String),
    /// Update status filter
    UpdateStatusFilter(Option<AgentStatus>),
    /// Refresh agents
    RefreshAgents,
    /// Clear error
    ClearError,
    /// Test agent connection
    TestAgent(String),
    /// Agent test completed
    AgentTestResult(String, bool),
}

impl Component for AgentsPage {
    type Message = AgentsPageMsg;
    type Properties = AgentsPageProps;
    
    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(AgentsPageMsg::LoadAgents);
        
        Self {
            agents: Vec::new(),
            is_loading: true,
            error_message: None,
            selected_agent_id: None,
            show_create_dialog: false,
            new_agent_form: NewAgentForm {
                name: String::new(),
                description: String::new(),
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                settings: std::collections::HashMap::new(),
            },
            search_query: String::new(),
            status_filter: None,
        }
    }
    
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AgentsPageMsg::LoadAgents => {
                self.is_loading = true;
                self.error_message = None;
                
                if let Some(engine) = &ctx.props().engine {
                    let engine_clone = engine.clone();
                    let link = ctx.link().clone();
                    
                    spawn_local(async move {
                        match load_agents(&engine_clone).await {
                            Ok(agents) => {
                                link.send_message(AgentsPageMsg::AgentsLoaded(agents));
                            }
                            Err(e) => {
                                link.send_message(AgentsPageMsg::LoadAgentsError(e));
                            }
                        }
                    });
                }
                
                true
            }
            
            AgentsPageMsg::AgentsLoaded(agents) => {
                self.agents = agents;
                self.is_loading = false;
                true
            }
            
            AgentsPageMsg::LoadAgentsError(error) => {
                self.is_loading = false;
                self.error_message = Some(error);
                true
            }
            
            AgentsPageMsg::SelectAgent(agent_id) => {
                self.selected_agent_id = Some(agent_id);
                true
            }
            
            AgentsPageMsg::ShowCreateDialog => {
                self.show_create_dialog = true;
                true
            }
            
            AgentsPageMsg::HideCreateDialog => {
                self.show_create_dialog = false;
                self.new_agent_form = NewAgentForm {
                    name: String::new(),
                    description: String::new(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    settings: std::collections::HashMap::new(),
                };
                true
            }
            
            AgentsPageMsg::UpdateNewAgentForm(form) => {
                self.new_agent_form = form;
                true
            }
            
            AgentsPageMsg::CreateAgent => {
                if let Some(engine) = &ctx.props().engine {
                    let engine_clone = engine.clone();
                    let form = self.new_agent_form.clone();
                    let link = ctx.link().clone();
                    
                    spawn_local(async move {
                        match create_agent(&engine_clone, &form).await {
                            Ok(agent) => {
                                link.send_message(AgentsPageMsg::AgentCreated(agent));
                            }
                            Err(e) => {
                                link.send_message(AgentsPageMsg::CreateAgentError(e));
                            }
                        }
                    });
                }
                
                false
            }
            
            AgentsPageMsg::AgentCreated(agent) => {
                self.agents.push(agent);
                self.show_create_dialog = false;
                self.new_agent_form = NewAgentForm {
                    name: String::new(),
                    description: String::new(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    settings: std::collections::HashMap::new(),
                };
                true
            }
            
            AgentsPageMsg::CreateAgentError(error) => {
                self.error_message = Some(error);
                true
            }
            
            AgentsPageMsg::DeleteAgent(agent_id) => {
                if let Some(engine) = &ctx.props().engine {
                    let engine_clone = engine.clone();
                    let id = agent_id.clone();
                    let link = ctx.link().clone();
                    
                    spawn_local(async move {
                        match delete_agent(&engine_clone, &id).await {
                            Ok(_) => {
                                link.send_message(AgentsPageMsg::AgentDeleted(id));
                            }
                            Err(e) => {
                                link.send_message(AgentsPageMsg::DeleteAgentError(e));
                            }
                        }
                    });
                }
                
                false
            }
            
            AgentsPageMsg::AgentDeleted(agent_id) => {
                self.agents.retain(|agent| agent.id != agent_id);
                if self.selected_agent_id == Some(agent_id) {
                    self.selected_agent_id = None;
                }
                true
            }
            
            AgentsPageMsg::DeleteAgentError(error) => {
                self.error_message = Some(error);
                true
            }
            
            AgentsPageMsg::UpdateSearchQuery(query) => {
                self.search_query = query;
                true
            }
            
            AgentsPageMsg::UpdateStatusFilter(filter) => {
                self.status_filter = filter;
                true
            }
            
            AgentsPageMsg::RefreshAgents => {
                ctx.link().send_message(AgentsPageMsg::LoadAgents);
                false
            }
            
            AgentsPageMsg::ClearError => {
                self.error_message = None;
                true
            }
            
            AgentsPageMsg::TestAgent(agent_id) => {
                if let Some(engine) = &ctx.props().engine {
                    let engine_clone = engine.clone();
                    let id = agent_id.clone();
                    let link = ctx.link().clone();
                    
                    spawn_local(async move {
                        let result = test_agent(&engine_clone, &id).await;
                        link.send_message(AgentsPageMsg::AgentTestResult(id, result));
                    });
                }
                
                false
            }
            
            AgentsPageMsg::AgentTestResult(agent_id, success) => {
                if let Some(agent) = self.agents.iter_mut().find(|a| a.id == agent_id) {
                    agent.status = if success {
                        AgentStatus::Active
                    } else {
                        AgentStatus::Error("Test failed".to_string())
                    };
                }
                true
            }
        }
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        
        html! {
            <div class="agents-page">
                { self.render_header(link) }
                { self.render_error_message(link) }
                { self.render_toolbar(link) }
                { self.render_agents_list(link) }
                { self.render_create_dialog(link) }
            </div>
        }
    }
}

impl AgentsPage {
    /// Render the agents header
    fn render_header(&self, link: &Scope<Self>) -> Html {
        html! {
            <div class="agents-header">
                <div class="header-title">
                    <h2>{ "AI Agents" }</h2>
                    <span class="header-subtitle">
                        { format!("{} agents configured", self.agents.len()) }
                    </span>
                </div>
                <div class="header-actions">
                    <button 
                        class="btn btn-primary"
                        onclick={link.callback(|_| AgentsPageMsg::ShowCreateDialog)}
                    >
                        { "Create Agent" }
                    </button>
                    <button 
                        class="btn btn-secondary"
                        onclick={link.callback(|_| AgentsPageMsg::RefreshAgents)}
                        disabled={self.is_loading}
                    >
                        { "Refresh" }
                    </button>
                </div>
            </div>
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
                        onclick={link.callback(|_| AgentsPageMsg::ClearError)}
                    >
                        { "×" }
                    </button>
                </div>
            }
        } else {
            html! {}
        }
    }
    
    /// Render toolbar with search and filters
    fn render_toolbar(&self, link: &Scope<Self>) -> Html {
        html! {
            <div class="agents-toolbar">
                <div class="toolbar-search">
                    <input
                        type="text"
                        class="search-input"
                        placeholder="Search agents..."
                        value={self.search_query.clone()}
                        oninput={link.callback(|e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            AgentsPageMsg::UpdateSearchQuery(input.value())
                        })}
                    />
                </div>
                <div class="toolbar-filters">
                    <select 
                        class="status-filter"
                        onchange={link.callback(|e: Event| {
                            let select: HtmlSelectElement = e.target_unchecked_into();
                            let filter = match select.value().as_str() {
                                "active" => Some(AgentStatus::Active),
                                "idle" => Some(AgentStatus::Idle),
                                "busy" => Some(AgentStatus::Busy),
                                "offline" => Some(AgentStatus::Offline),
                                _ => None,
                            };
                            AgentsPageMsg::UpdateStatusFilter(filter)
                        })}
                    >
                        <option value="">{ "All Status" }</option>
                        <option value="active">{ "Active" }</option>
                        <option value="idle">{ "Idle" }</option>
                        <option value="busy">{ "Busy" }</option>
                        <option value="offline">{ "Offline" }</option>
                    </select>
                </div>
            </div>
        }
    }
    
    /// Render agents list
    fn render_agents_list(&self, link: &Scope<Self>) -> Html {
        if self.is_loading {
            html! {
                <div class="agents-loading">
                    <div class="loading-spinner"></div>
                    <p>{ "Loading agents..." }</p>
                </div>
            }
        } else if self.agents.is_empty() {
            html! {
                <div class="agents-empty">
                    <div class="empty-icon">{ "🤖" }</div>
                    <h3>{ "No agents configured" }</h3>
                    <p>{ "Create your first AI agent to get started." }</p>
                    <button 
                        class="btn btn-primary"
                        onclick={link.callback(|_| AgentsPageMsg::ShowCreateDialog)}
                    >
                        { "Create Agent" }
                    </button>
                </div>
            }
        } else {
            let filtered_agents = self.get_filtered_agents();
            
            html! {
                <div class="agents-grid">
                    { for filtered_agents.iter().map(|agent| self.render_agent_card(agent, link)) }
                </div>
            }
        }
    }
    
    /// Render a single agent card
    fn render_agent_card(&self, agent: &AgentInfo, link: &Scope<Self>) -> Html {
        let is_selected = self.selected_agent_id == Some(agent.id.clone());
        let card_class = classes!(
            "agent-card",
            if is_selected { "selected" } else { "" }
        );
        
        html! {
            <div class={card_class}>
                <div class="agent-header">
                    <div class="agent-avatar">
                        { "🤖" }
                    </div>
                    <div class="agent-info">
                        <h4 class="agent-name">{ &agent.name }</h4>
                        <p class="agent-description">{ &agent.description }</p>
                    </div>
                    <div class="agent-status">
                        { self.render_agent_status(&agent.status) }
                    </div>
                </div>
                
                <div class="agent-details">
                    <div class="agent-meta">
                        <span class="meta-item">
                            <strong>{ "Provider:" }</strong>
                            { &agent.provider }
                        </span>
                        <span class="meta-item">
                            <strong>{ "Model:" }</strong>
                            { &agent.model }
                        </span>
                        <span class="meta-item">
                            <strong>{ "Messages:" }</strong>
                            { agent.message_count }
                        </span>
                    </div>
                    
                    <div class="agent-timestamps">
                        <span class="timestamp-item">
                            <strong>{ "Created:" }</strong>
                            { format_timestamp(agent.created_at) }
                        </span>
                        if let Some(last_used) = agent.last_used {
                            <span class="timestamp-item">
                                <strong>{ "Last used:" }</strong>
                                { format_timestamp(last_used) }
                            </span>
                        }
                    </div>
                </div>
                
                <div class="agent-actions">
                    <button 
                        class="btn btn-sm btn-primary"
                        onclick={link.callback({
                            let agent_id = agent.id.clone();
                            move |_| AgentsPageMsg::SelectAgent(agent_id.clone())
                        })}
                    >
                        { "Select" }
                    </button>
                    <button 
                        class="btn btn-sm btn-secondary"
                        onclick={link.callback({
                            let agent_id = agent.id.clone();
                            move |_| AgentsPageMsg::TestAgent(agent_id.clone())
                        })}
                    >
                        { "Test" }
                    </button>
                    <button 
                        class="btn btn-sm btn-danger"
                        onclick={link.callback({
                            let agent_id = agent.id.clone();
                            move |_| AgentsPageMsg::DeleteAgent(agent_id.clone())
                        })}
                    >
                        { "Delete" }
                    </button>
                </div>
            </div>
        }
    }
    
    /// Render agent status indicator
    fn render_agent_status(&self, status: &AgentStatus) -> Html {
        match status {
            AgentStatus::Active => html! {
                <span class="status-indicator status-active">{ "Active" }</span>
            },
            AgentStatus::Idle => html! {
                <span class="status-indicator status-idle">{ "Idle" }</span>
            },
            AgentStatus::Busy => html! {
                <span class="status-indicator status-busy">{ "Busy" }</span>
            },
            AgentStatus::Error(error) => html! {
                <span class="status-indicator status-error" title={error.clone()}>{ "Error" }</span>
            },
            AgentStatus::Offline => html! {
                <span class="status-indicator status-offline">{ "Offline" }</span>
            },
        }
    }
    
    /// Render create agent dialog
    fn render_create_dialog(&self, link: &Scope<Self>) -> Html {
        if !self.show_create_dialog {
            return html! {};
        }
        
        html! {
            <div class="modal-overlay">
                <div class="modal-content">
                    <div class="modal-header">
                        <h3>{ "Create New Agent" }</h3>
                        <button 
                            class="modal-close"
                            onclick={link.callback(|_| AgentsPageMsg::HideCreateDialog)}
                        >
                            { "×" }
                        </button>
                    </div>
                    
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{ "Agent Name" }</label>
                            <input
                                type="text"
                                class="form-input"
                                placeholder="Enter agent name"
                                value={self.new_agent_form.name.clone()}
                                oninput={link.callback(|e: InputEvent| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    let mut form = self.new_agent_form.clone();
                                    form.name = input.value();
                                    AgentsPageMsg::UpdateNewAgentForm(form)
                                })}
                            />
                        </div>
                        
                        <div class="form-group">
                            <label>{ "Description" }</label>
                            <textarea
                                class="form-textarea"
                                placeholder="Describe what this agent does"
                                value={self.new_agent_form.description.clone()}
                                oninput={link.callback(|e: InputEvent| {
                                    let textarea: HtmlTextAreaElement = e.target_unchecked_into();
                                    let mut form = self.new_agent_form.clone();
                                    form.description = textarea.value();
                                    AgentsPageMsg::UpdateNewAgentForm(form)
                                })}
                            />
                        </div>
                        
                        <div class="form-group">
                            <label>{ "Provider" }</label>
                            <select 
                                class="form-select"
                                value={self.new_agent_form.provider.clone()}
                                onchange={link.callback(|e: Event| {
                                    let select: HtmlSelectElement = e.target_unchecked_into();
                                    let mut form = self.new_agent_form.clone();
                                    form.provider = select.value();
                                    AgentsPageMsg::UpdateNewAgentForm(form)
                                })}
                            >
                                <option value="openai">{ "OpenAI" }</option>
                                <option value="anthropic">{ "Anthropic" }</option>
                                <option value="local">{ "Local" }</option>
                            </select>
                        </div>
                        
                        <div class="form-group">
                            <label>{ "Model" }</label>
                            <input
                                type="text"
                                class="form-input"
                                placeholder="Enter model name"
                                value={self.new_agent_form.model.clone()}
                                oninput={link.callback(|e: InputEvent| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    let mut form = self.new_agent_form.clone();
                                    form.model = input.value();
                                    AgentsPageMsg::UpdateNewAgentForm(form)
                                })}
                            />
                        </div>
                    </div>
                    
                    <div class="modal-footer">
                        <button 
                            class="btn btn-secondary"
                            onclick={link.callback(|_| AgentsPageMsg::HideCreateDialog)}
                        >
                            { "Cancel" }
                        </button>
                        <button 
                            class="btn btn-primary"
                            onclick={link.callback(|_| AgentsPageMsg::CreateAgent)}
                            disabled={self.new_agent_form.name.trim().is_empty()}
                        >
                            { "Create Agent" }
                        </button>
                    </div>
                </div>
            </div>
        }
    }
    
    /// Get filtered agents based on search and status filter
    fn get_filtered_agents(&self) -> Vec<&AgentInfo> {
        self.agents
            .iter()
            .filter(|agent| {
                // Filter by search query
                let search_match = self.search_query.is_empty() || 
                    agent.name.to_lowercase().contains(&self.search_query.to_lowercase()) ||
                    agent.description.to_lowercase().contains(&self.search_query.to_lowercase());
                
                // Filter by status
                let status_match = self.status_filter.is_none() || 
                    std::mem::discriminant(&agent.status) == std::mem::discriminant(self.status_filter.as_ref().unwrap());
                
                search_match && status_match
            })
            .collect()
    }
}

// Helper functions for agent management
async fn load_agents(engine: &OpenCodeEngine) -> Result<Vec<AgentInfo>, String> {
    // Mock implementation - replace with actual engine calls
    Ok(vec![
        AgentInfo {
            id: "agent-1".to_string(),
            name: "Assistant".to_string(),
            description: "General purpose AI assistant".to_string(),
            status: AgentStatus::Active,
            created_at: chrono::Utc::now() - chrono::Duration::hours(24),
            last_used: Some(chrono::Utc::now() - chrono::Duration::minutes(30)),
            message_count: 42,
            provider: "OpenAI".to_string(),
            model: "gpt-4".to_string(),
            settings: std::collections::HashMap::new(),
        },
        AgentInfo {
            id: "agent-2".to_string(),
            name: "Code Reviewer".to_string(),
            description: "Specialized in code review and analysis".to_string(),
            status: AgentStatus::Idle,
            created_at: chrono::Utc::now() - chrono::Duration::hours(48),
            last_used: Some(chrono::Utc::now() - chrono::Duration::hours(2)),
            message_count: 15,
            provider: "Anthropic".to_string(),
            model: "claude-3-opus".to_string(),
            settings: std::collections::HashMap::new(),
        },
    ])
}

async fn create_agent(engine: &OpenCodeEngine, form: &NewAgentForm) -> Result<AgentInfo, String> {
    // Mock implementation - replace with actual engine calls
    Ok(AgentInfo {
        id: generate_uuid(),
        name: form.name.clone(),
        description: form.description.clone(),
        status: AgentStatus::Active,
        created_at: chrono::Utc::now(),
        last_used: None,
        message_count: 0,
        provider: form.provider.clone(),
        model: form.model.clone(),
        settings: form.settings.clone(),
    })
}

async fn delete_agent(engine: &OpenCodeEngine, agent_id: &str) -> Result<(), String> {
    // Mock implementation - replace with actual engine calls
    Ok(())
}

async fn test_agent(engine: &OpenCodeEngine, agent_id: &str) -> bool {
    // Mock implementation - replace with actual engine calls
    true
}