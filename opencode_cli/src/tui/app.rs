//! TUI application state and logic

use anyhow::Result;
use opencode_core::{Engine, agent::{AgentConfig, AgentHandle, AgentStats}};
use std::collections::VecDeque;
use std::time::Duration;

/// Input mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

/// Current panel focus
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Chat,
    AgentInfo,
    Stats,
    Help,
    Input,
}

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Application state
pub struct App {
    /// OpenCode engine
    pub engine: Engine,
    
    /// Current agent
    pub agent: AgentHandle,
    
    /// Agent name
    pub agent_name: String,
    
    /// Provider name
    pub provider_name: String,
    
    /// Model name
    pub model_name: String,
    
    /// Agent state
    pub agent_state: String,
    
    /// Input mode
    pub input_mode: InputMode,
    
    /// Current panel
    pub current_panel: Panel,
    
    /// Input buffer
    pub input: String,
    
    /// Chat messages
    pub messages: VecDeque<ChatMessage>,
    
    /// Selected message index
    pub selected_message: usize,
    
    /// Scroll position
    pub scroll: usize,
    
    /// Whether AI is thinking
    pub is_thinking: bool,
    
    /// Application statistics
    pub stats: AppStats,
}

/// Application statistics
#[derive(Debug, Clone)]
pub struct AppStats {
    pub messages_processed: u64,
    pub tokens_consumed: u64,
    pub uptime: Duration,
    pub start_time: std::time::Instant,
}

impl Default for AppStats {
    fn default() -> Self {
        AppStats {
            messages_processed: 0,
            tokens_consumed: 0,
            uptime: Duration::new(0, 0),
            start_time: std::time::Instant::now(),
        }
    }
}

impl App {
    /// Create a new app
    pub async fn new(
        engine: Engine,
        agent_name: &str,
        system_prompt: Option<&str>,
    ) -> Result<Self> {
        // Create agent configuration
        let mut config = AgentConfig::default();
        config.name = agent_name.to_string();
        
        if let Some(prompt) = system_prompt {
            config.system_prompt = Some(prompt.to_string());
        }
        
        // Get default provider
        let provider = engine.get_default_provider().await?;
        let provider_name = provider.name();
        let model_name = provider.default_model();
        
        // Create agent
        let agent = engine.agents().create_agent(agent_name, provider, Some(config)).await?;
        
        Ok(App {
            engine,
            agent,
            agent_name: agent_name.to_string(),
            provider_name,
            model_name,
            agent_state: "Idle".to_string(),
            input_mode: InputMode::Normal,
            current_panel: Panel::Chat,
            input: String::new(),
            messages: VecDeque::new(),
            selected_message: 0,
            scroll: 0,
            is_thinking: false,
            stats: AppStats::default(),
        })
    }
    
    /// Send a message to the agent
    pub async fn send_message(&mut self) -> Result<()> {
        if self.input.is_empty() {
            return Ok(());
        }
        
        let user_message = self.input.clone();
        self.input.clear();
        self.input_mode = InputMode::Normal;
        
        // Add user message to chat
        self.messages.push_back(ChatMessage {
            role: "user".to_string(),
            content: user_message.clone(),
            timestamp: chrono::Utc::now(),
        });
        
        // Update selected message
        self.selected_message = self.messages.len().saturating_sub(1);
        
        // Set thinking state
        self.is_thinking = true;
        self.agent_state = "Thinking".to_string();
        
        // Send message to agent
        let response = self.agent.send_message(&user_message).await?;
        
        // Add AI response to chat
        self.messages.push_back(ChatMessage {
            role: "assistant".to_string(),
            content: response.content,
            timestamp: chrono::Utc::now(),
        });
        
        // Update statistics
        self.stats.messages_processed += 1;
        if let Some(usage) = response.usage {
            self.stats.tokens_consumed += usage.total_tokens as u64;
        }
        
        // Update selected message
        self.selected_message = self.messages.len().saturating_sub(1);
        
        // Reset thinking state
        self.is_thinking = false;
        self.agent_state = "Idle".to_string();
        
        Ok(())
    }
    
    /// Handle periodic updates
    pub async fn on_tick(&mut self) -> Result<()> {
        // Update uptime
        self.stats.uptime = self.stats.start_time.elapsed();
        
        // Update agent state
        let agent_stats = self.agent.get_stats().await;
        self.update_agent_state(&agent_stats);
        
        Ok(())
    }
    
    /// Update agent state from statistics
    fn update_agent_state(&mut self, stats: &AgentStats) {
        if !self.is_thinking {
            if stats.messages_processed > 0 {
                self.agent_state = "Ready".to_string();
            } else {
                self.agent_state = "Idle".to_string();
            }
        }
    }
    
    /// Scroll up in chat
    pub fn scroll_up(&mut self) {
        if self.selected_message > 0 {
            self.selected_message -= 1;
        }
    }
    
    /// Scroll down in chat
    pub fn scroll_down(&mut self) {
        if self.selected_message < self.messages.len().saturating_sub(1) {
            self.selected_message += 1;
        }
    }
    
    /// Move to next panel
    pub fn next_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Chat => Panel::AgentInfo,
            Panel::AgentInfo => Panel::Stats,
            Panel::Stats => Panel::Help,
            Panel::Help => Panel::Input,
            Panel::Input => Panel::Chat,
        };
    }
    
    /// Move to previous panel
    pub fn previous_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Chat => Panel::Input,
            Panel::AgentInfo => Panel::Chat,
            Panel::Stats => Panel::AgentInfo,
            Panel::Help => Panel::Stats,
            Panel::Input => Panel::Help,
        };
    }
    
    /// Clear chat history
    pub async fn clear_chat(&mut self) -> Result<()> {
        self.messages.clear();
        self.selected_message = 0;
        self.scroll = 0;
        self.agent.clear_history().await;
        Ok(())
    }
    
    /// Reset agent
    pub async fn reset_agent(&mut self) -> Result<()> {
        self.agent.reset().await;
        self.clear_chat().await?;
        self.agent_state = "Idle".to_string();
        Ok(())
    }
    
    /// Get formatted chat history
    pub fn get_chat_history(&self) -> Vec<String> {
        self.messages
            .iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect()
    }
    
    /// Export chat to file
    pub async fn export_chat(&self, filename: &str) -> Result<()> {
        let content = self.messages
            .iter()
            .map(|msg| format!("[{}] {}: {}", 
                msg.timestamp.format("%Y-%m-%d %H:%M:%S"),
                msg.role, 
                msg.content
            ))
            .collect::<Vec<_>>()
            .join("\n");
        
        tokio::fs::write(filename, content).await?;
        Ok(())
    }
    
    /// Get display statistics
    pub fn get_display_stats(&self) -> String {
        format!(
            "Messages: {}\nTokens: {}\nUptime: {}\nAgent: {}\nProvider: {}\nModel: {}",
            self.stats.messages_processed,
            self.stats.tokens_consumed,
            format_duration(self.stats.uptime),
            self.agent_name,
            self.provider_name,
            self.model_name
        )
    }
}

/// Format duration for display
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_app_stats_default() {
        let stats = AppStats::default();
        assert_eq!(stats.messages_processed, 0);
        assert_eq!(stats.tokens_consumed, 0);
        assert_eq!(stats.uptime, Duration::new(0, 0));
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    }
    
    #[test]
    fn test_panel_navigation() {
        let mut app = App {
            engine: todo!(),
            agent: todo!(),
            agent_name: "test".to_string(),
            provider_name: "test".to_string(),
            model_name: "test".to_string(),
            agent_state: "idle".to_string(),
            input_mode: InputMode::Normal,
            current_panel: Panel::Chat,
            input: String::new(),
            messages: VecDeque::new(),
            selected_message: 0,
            scroll: 0,
            is_thinking: false,
            stats: AppStats::default(),
        };
        
        assert_eq!(app.current_panel, Panel::Chat);
        
        app.next_panel();
        assert_eq!(app.current_panel, Panel::AgentInfo);
        
        app.previous_panel();
        assert_eq!(app.current_panel, Panel::Chat);
    }
}