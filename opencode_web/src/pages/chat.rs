//! Chat page component

use yew::prelude::*;
use yew::platform::spawn_local;
use opencode_core::wasm_bindings::*;
use crate::components::*;
use crate::services::*;
use crate::utils::*;

/// Chat page properties
#[derive(Properties, PartialEq)]
pub struct ChatPageProps {
    /// Current agent instance
    pub agent: Option<OpenCodeAgent>,
    /// OpenCode engine instance
    pub engine: Option<OpenCodeEngine>,
}

/// Chat message structure
#[derive(Clone, PartialEq)]
pub struct ChatMessage {
    pub id: String,
    pub content: String,
    pub is_user: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: MessageStatus,
}

/// Message status
#[derive(Clone, PartialEq)]
pub enum MessageStatus {
    Sending,
    Sent,
    Delivered,
    Error(String),
}

/// Chat page state
pub struct ChatPage {
    /// Chat messages history
    messages: Vec<ChatMessage>,
    /// Current input value
    input_value: String,
    /// Whether a message is being sent
    is_sending: bool,
    /// Current conversation ID
    conversation_id: Option<String>,
    /// Error message if any
    error_message: Option<String>,
    /// Whether the chat is initialized
    is_initialized: bool,
}

/// Chat page messages
pub enum ChatPageMsg {
    /// Initialize the chat
    Initialize,
    /// Update input value
    UpdateInput(String),
    /// Send a message
    SendMessage,
    /// Message sent successfully
    MessageSent(String),
    /// Message sending failed
    MessageError(String),
    /// Received a response
    ReceivedResponse(String),
    /// Clear chat history
    ClearChat,
    /// Load chat history
    LoadHistory,
    /// History loaded
    HistoryLoaded(Vec<ChatMessage>),
    /// Clear error
    ClearError,
    /// Handle keyboard input
    HandleKeyPress(KeyboardEvent),
}

impl Component for ChatPage {
    type Message = ChatPageMsg;
    type Properties = ChatPageProps;
    
    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(ChatPageMsg::Initialize);
        
        Self {
            messages: Vec::new(),
            input_value: String::new(),
            is_sending: false,
            conversation_id: None,
            error_message: None,
            is_initialized: false,
        }
    }
    
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ChatPageMsg::Initialize => {
                self.is_initialized = true;
                ctx.link().send_message(ChatPageMsg::LoadHistory);
                true
            }
            
            ChatPageMsg::UpdateInput(value) => {
                self.input_value = value;
                true
            }
            
            ChatPageMsg::SendMessage => {
                if self.input_value.trim().is_empty() || self.is_sending {
                    return false;
                }
                
                let message_content = self.input_value.trim().to_string();
                let message_id = generate_uuid();
                
                // Add user message to chat
                let user_message = ChatMessage {
                    id: message_id.clone(),
                    content: message_content.clone(),
                    is_user: true,
                    timestamp: chrono::Utc::now(),
                    status: MessageStatus::Sending,
                };
                
                self.messages.push(user_message);
                self.input_value.clear();
                self.is_sending = true;
                self.error_message = None;
                
                // Send message to agent
                if let Some(agent) = &ctx.props().agent {
                    let agent_clone = agent.clone();
                    let link = ctx.link().clone();
                    
                    spawn_local(async move {
                        match agent_clone.send_message(&message_content).await {
                            Ok(response) => {
                                link.send_message(ChatPageMsg::MessageSent(message_id));
                                link.send_message(ChatPageMsg::ReceivedResponse(response));
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to send message: {:?}", e);
                                link.send_message(ChatPageMsg::MessageError(error_msg));
                            }
                        }
                    });
                }
                
                true
            }
            
            ChatPageMsg::MessageSent(message_id) => {
                // Update message status to sent
                if let Some(message) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    message.status = MessageStatus::Sent;
                }
                true
            }
            
            ChatPageMsg::MessageError(error) => {
                self.is_sending = false;
                self.error_message = Some(error);
                true
            }
            
            ChatPageMsg::ReceivedResponse(response) => {
                self.is_sending = false;
                
                // Add agent response to chat
                let response_message = ChatMessage {
                    id: generate_uuid(),
                    content: response,
                    is_user: false,
                    timestamp: chrono::Utc::now(),
                    status: MessageStatus::Delivered,
                };
                
                self.messages.push(response_message);
                
                // Save to history
                self.save_chat_history();
                
                // Scroll to bottom
                spawn_local(async {
                    request_animation_frame().await;
                    scroll_to_bottom("chat-messages");
                });
                
                true
            }
            
            ChatPageMsg::ClearChat => {
                self.messages.clear();
                self.conversation_id = None;
                self.clear_chat_history();
                true
            }
            
            ChatPageMsg::LoadHistory => {
                let link = ctx.link().clone();
                
                spawn_local(async move {
                    match load_chat_history().await {
                        Ok(messages) => {
                            link.send_message(ChatPageMsg::HistoryLoaded(messages));
                        }
                        Err(e) => {
                            log(&format!("Failed to load chat history: {:?}", e));
                        }
                    }
                });
                
                false
            }
            
            ChatPageMsg::HistoryLoaded(messages) => {
                self.messages = messages;
                
                // Scroll to bottom after loading
                spawn_local(async {
                    request_animation_frame().await;
                    scroll_to_bottom("chat-messages");
                });
                
                true
            }
            
            ChatPageMsg::ClearError => {
                self.error_message = None;
                true
            }
            
            ChatPageMsg::HandleKeyPress(event) => {
                if event.key() == "Enter" && !event.shift_key() {
                    event.prevent_default();
                    ctx.link().send_message(ChatPageMsg::SendMessage);
                }
                false
            }
        }
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        
        html! {
            <div class="chat-page">
                { self.render_header(link) }
                { self.render_error_message(link) }
                { self.render_chat_messages(link) }
                { self.render_input_area(link) }
                { self.render_status_bar() }
            </div>
        }
    }
}

impl ChatPage {
    /// Render the chat header
    fn render_header(&self, link: &Scope<Self>) -> Html {
        html! {
            <div class="chat-header">
                <div class="chat-title">
                    <h2>{ "Chat with AI Assistant" }</h2>
                    <span class="chat-subtitle">
                        { format!("{} messages", self.messages.len()) }
                    </span>
                </div>
                <div class="chat-actions">
                    <button 
                        class="btn btn-secondary"
                        onclick={link.callback(|_| ChatPageMsg::ClearChat)}
                        disabled={self.messages.is_empty()}
                    >
                        { "Clear Chat" }
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
                        onclick={link.callback(|_| ChatPageMsg::ClearError)}
                    >
                        { "×" }
                    </button>
                </div>
            }
        } else {
            html! {}
        }
    }
    
    /// Render chat messages
    fn render_chat_messages(&self, _link: &Scope<Self>) -> Html {
        html! {
            <div class="chat-messages" id="chat-messages">
                if self.messages.is_empty() {
                    <div class="empty-state">
                        <div class="empty-icon">{ "💬" }</div>
                        <h3>{ "Start a conversation" }</h3>
                        <p>{ "Type a message below to begin chatting with the AI assistant." }</p>
                    </div>
                } else {
                    { for self.messages.iter().map(|msg| self.render_message(msg)) }
                }
            </div>
        }
    }
    
    /// Render a single message
    fn render_message(&self, message: &ChatMessage) -> Html {
        let message_class = classes!(
            "chat-message",
            if message.is_user { "user-message" } else { "agent-message" }
        );
        
        html! {
            <div class={message_class}>
                <div class="message-avatar">
                    { if message.is_user { "👤" } else { "🤖" } }
                </div>
                <div class="message-content">
                    <div class="message-header">
                        <span class="message-sender">
                            { if message.is_user { "You" } else { "AI Assistant" } }
                        </span>
                        <span class="message-timestamp">
                            { format_timestamp(message.timestamp) }
                        </span>
                    </div>
                    <div class="message-body">
                        { self.render_message_content(&message.content) }
                    </div>
                    <div class="message-status">
                        { self.render_message_status(&message.status) }
                    </div>
                </div>
            </div>
        }
    }
    
    /// Render message content with markdown support
    fn render_message_content(&self, content: &str) -> Html {
        // For now, just render as plain text
        // TODO: Add markdown parsing
        html! {
            <div class="message-text">
                { content }
            </div>
        }
    }
    
    /// Render message status
    fn render_message_status(&self, status: &MessageStatus) -> Html {
        match status {
            MessageStatus::Sending => html! {
                <span class="status-sending">{ "Sending..." }</span>
            },
            MessageStatus::Sent => html! {
                <span class="status-sent">{ "✓" }</span>
            },
            MessageStatus::Delivered => html! {
                <span class="status-delivered">{ "✓✓" }</span>
            },
            MessageStatus::Error(error) => html! {
                <span class="status-error" title={error.clone()}>{ "⚠" }</span>
            },
        }
    }
    
    /// Render input area
    fn render_input_area(&self, link: &Scope<Self>) -> Html {
        html! {
            <div class="chat-input-area">
                <div class="input-container">
                    <textarea
                        class="chat-input"
                        placeholder="Type your message here..."
                        value={self.input_value.clone()}
                        disabled={self.is_sending}
                        oninput={link.callback(|e: InputEvent| {
                            let input: HtmlTextAreaElement = e.target_unchecked_into();
                            ChatPageMsg::UpdateInput(input.value())
                        })}
                        onkeypress={link.callback(ChatPageMsg::HandleKeyPress)}
                    />
                    <button 
                        class={classes!("send-button", if self.is_sending { "sending" } else { "" })}
                        disabled={self.input_value.trim().is_empty() || self.is_sending}
                        onclick={link.callback(|_| ChatPageMsg::SendMessage)}
                    >
                        if self.is_sending {
                            { "Sending..." }
                        } else {
                            { "Send" }
                        }
                    </button>
                </div>
                <div class="input-hint">
                    { "Press Enter to send, Shift+Enter for new line" }
                </div>
            </div>
        }
    }
    
    /// Render status bar
    fn render_status_bar(&self) -> Html {
        html! {
            <div class="chat-status-bar">
                <div class="status-info">
                    if self.is_sending {
                        <span class="status-indicator sending">{ "AI is thinking..." }</span>
                    } else {
                        <span class="status-indicator ready">{ "Ready" }</span>
                    }
                </div>
                <div class="status-stats">
                    <span>{ format!("{} messages", self.messages.len()) }</span>
                    if let Some(conversation_id) = &self.conversation_id {
                        <span>{ format!("ID: {}", truncate_text(conversation_id, 8)) }</span>
                    }
                </div>
            </div>
        }
    }
    
    /// Save chat history to local storage
    fn save_chat_history(&self) {
        spawn_local({
            let messages = self.messages.clone();
            async move {
                if let Err(e) = save_chat_history(&messages).await {
                    error(&format!("Failed to save chat history: {:?}", e));
                }
            }
        });
    }
    
    /// Clear chat history from local storage
    fn clear_chat_history(&self) {
        spawn_local(async move {
            if let Err(e) = clear_chat_history().await {
                error(&format!("Failed to clear chat history: {:?}", e));
            }
        });
    }
}

// Helper functions for chat history management
async fn save_chat_history(messages: &[ChatMessage]) -> Result<(), JsValue> {
    let storage = web_sys::window()
        .unwrap()
        .local_storage()
        .unwrap()
        .unwrap();
    
    let serialized = serde_json::to_string(messages)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    storage.set_item("opencode_chat_history", &serialized)
        .map_err(|e| JsValue::from_str(&format!("Failed to save: {:?}", e)))?;
    
    Ok(())
}

async fn load_chat_history() -> Result<Vec<ChatMessage>, JsValue> {
    let storage = web_sys::window()
        .unwrap()
        .local_storage()
        .unwrap()
        .unwrap();
    
    match storage.get_item("opencode_chat_history").unwrap() {
        Some(data) => {
            let messages: Vec<ChatMessage> = serde_json::from_str(&data)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(messages)
        }
        None => Ok(Vec::new()),
    }
}

async fn clear_chat_history() -> Result<(), JsValue> {
    let storage = web_sys::window()
        .unwrap()
        .local_storage()
        .unwrap()
        .unwrap();
    
    storage.remove_item("opencode_chat_history")
        .map_err(|e| JsValue::from_str(&format!("Failed to clear: {:?}", e)))?;
    
    Ok(())
}

// Implement Serialize and Deserialize for ChatMessage
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SerializableChatMessage {
    id: String,
    content: String,
    is_user: bool,
    timestamp: String,
    status: String,
}

impl From<&ChatMessage> for SerializableChatMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            id: msg.id.clone(),
            content: msg.content.clone(),
            is_user: msg.is_user,
            timestamp: msg.timestamp.to_rfc3339(),
            status: match &msg.status {
                MessageStatus::Sending => "sending".to_string(),
                MessageStatus::Sent => "sent".to_string(),
                MessageStatus::Delivered => "delivered".to_string(),
                MessageStatus::Error(e) => format!("error:{}", e),
            },
        }
    }
}

impl TryFrom<SerializableChatMessage> for ChatMessage {
    type Error = String;
    
    fn try_from(msg: SerializableChatMessage) -> Result<Self, Self::Error> {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&msg.timestamp)
            .map_err(|e| e.to_string())?
            .with_timezone(&chrono::Utc);
        
        let status = if msg.status == "sending" {
            MessageStatus::Sending
        } else if msg.status == "sent" {
            MessageStatus::Sent
        } else if msg.status == "delivered" {
            MessageStatus::Delivered
        } else if msg.status.starts_with("error:") {
            MessageStatus::Error(msg.status[6..].to_string())
        } else {
            MessageStatus::Delivered // Default fallback
        };
        
        Ok(Self {
            id: msg.id,
            content: msg.content,
            is_user: msg.is_user,
            timestamp,
            status,
        })
    }
}

// Custom serialization for ChatMessage
impl Serialize for ChatMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializableChatMessage::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChatMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serializable = SerializableChatMessage::deserialize(deserializer)?;
        ChatMessage::try_from(serializable).map_err(serde::de::Error::custom)
    }
}