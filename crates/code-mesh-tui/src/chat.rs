use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    style::Style,
    text::{Line, Span},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use tui_textarea::TextArea;

use crate::{
    config::ChatConfig,
    renderer::Renderer,
    theme::Theme,
};

/// Chat component for interactive messaging
pub struct ChatComponent {
    theme: Box<dyn Theme + Send + Sync>,
    config: ChatConfig,
    messages: Vec<ChatMessage>,
    input_area: TextArea<'static>,
    scroll_offset: usize,
    auto_scroll: bool,
}

/// A chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: std::time::SystemTime,
    pub attachments: Vec<MessageAttachment>,
}

/// Message role enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Message attachment
#[derive(Debug, Clone)]
pub struct MessageAttachment {
    pub name: String,
    pub path: String,
    pub mime_type: String,
    pub size: u64,
}

impl ChatComponent {
    /// Create a new chat component
    pub fn new(config: &ChatConfig, theme: &dyn Theme) -> Self {
        let mut input_area = TextArea::default();
        input_area.set_placeholder_text("Type your message...");
        
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            config: config.clone(),
            messages: Vec::new(),
            input_area,
            scroll_offset: 0,
            auto_scroll: true,
        }
    }
    
    /// Add a new message
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        
        // Limit message history
        if self.messages.len() > self.config.max_messages {
            self.messages.remove(0);
        }
        
        // Auto-scroll to bottom if enabled
        if self.config.auto_scroll && self.auto_scroll {
            self.scroll_to_bottom();
        }
    }
    
    /// Send the current message
    pub async fn send_message(&mut self) -> Result<()> {
        let content = self.input_area.lines().join("\n");
        if content.trim().is_empty() {
            return Ok(());
        }
        
        // Create user message
        let message = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: content.clone(),
            timestamp: std::time::SystemTime::now(),
            attachments: Vec::new(),
        };
        
        self.add_message(message);
        self.clear_input();
        
        // TODO: Send to LLM service and handle response
        
        Ok(())
    }
    
    /// Clear the input area
    pub fn clear_input(&mut self) {
        self.input_area = TextArea::default();
        self.input_area.set_placeholder_text("Type your message...");
    }
    
    /// Insert a newline in the input
    pub fn insert_newline(&mut self) {
        self.input_area.insert_newline();
    }
    
    /// Handle paste event
    pub async fn handle_paste(&mut self, data: String) -> Result<()> {
        self.input_area.insert_str(&data);
        Ok(())
    }
    
    /// Handle key events
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter if key.modifiers.is_empty() => {
                self.send_message().await?;
            }
            KeyCode::Enter if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => {
                self.insert_newline();
            }
            _ => {
                self.input_area.input(key);
            }
        }
        Ok(())
    }
    
    /// Scroll up in the message history
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
            self.auto_scroll = false;
        }
    }
    
    /// Scroll down in the message history
    pub fn scroll_down(&mut self) {
        let max_offset = self.messages.len().saturating_sub(1);
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        } else {
            self.auto_scroll = true;
        }
    }
    
    /// Page up in the message history
    pub fn page_up(&mut self) {
        let page_size = 10; // Could be based on visible area
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
        self.auto_scroll = false;
    }
    
    /// Page down in the message history
    pub fn page_down(&mut self) {
        let page_size = 10;
        let max_offset = self.messages.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_offset);
        if self.scroll_offset >= max_offset {
            self.auto_scroll = true;
        }
    }
    
    /// Scroll to the bottom of the message history
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
        self.auto_scroll = true;
    }
    
    /// Clear all messages
    pub async fn clear(&mut self) -> Result<()> {
        self.messages.clear();
        self.scroll_offset = 0;
        self.auto_scroll = true;
        Ok(())
    }
    
    /// Update the component
    pub async fn update(&mut self) -> Result<()> {
        // Handle any periodic updates
        Ok(())
    }
    
    /// Update theme
    pub fn update_theme(&mut self, theme: &dyn Theme) {
        // In a real implementation, we'd clone or recreate the theme
        // For now, this is a placeholder
    }
    
    /// Render the main chat area (messages)
    pub fn render(&mut self, renderer: &Renderer, area: Rect) {
        let block = Block::default()
            .title("Chat")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border()));
        
        renderer.render_widget(block.clone(), area);
        
        let inner_area = block.inner(area);
        self.render_messages(renderer, inner_area);
    }
    
    /// Render the input area
    pub fn render_input(&mut self, renderer: &Renderer, area: Rect) {
        let block = Block::default()
            .title("Message")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_active()));
        
        renderer.render_widget(block.clone(), area);
        
        let inner_area = block.inner(area);
        
        // Apply theme to input area
        let mut input_style = self.input_area.style();
        input_style = input_style
            .fg(self.theme.text())
            .bg(self.theme.background_element());
        self.input_area.set_style(input_style);
        
        // Render the text area
        let widget = &self.input_area;
        renderer.render_widget(widget, inner_area);
    }
    
    /// Render the message list
    fn render_messages(&self, renderer: &Renderer, area: Rect) {
        if self.messages.is_empty() {
            let empty_msg = Paragraph::new("No messages yet. Start a conversation!")
                .style(Style::default().fg(self.theme.text_muted()));
            renderer.render_widget(empty_msg, area);
            return;
        }
        
        // Calculate visible messages based on area height and scroll offset
        let visible_height = area.height as usize;
        let start_index = self.scroll_offset;
        let end_index = (start_index + visible_height).min(self.messages.len());
        
        let visible_messages = &self.messages[start_index..end_index];
        
        // Convert messages to renderable lines
        let mut lines = Vec::new();
        for message in visible_messages {
            lines.extend(self.format_message(message));
            lines.push(Line::raw("")); // Empty line between messages
        }
        
        let paragraph = Paragraph::new(lines)
            .style(Style::default().bg(self.theme.background()));
        
        renderer.render_widget(paragraph, area);
        
        // Render scrollbar if needed
        if self.messages.len() > visible_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(self.messages.len())
                .position(self.scroll_offset);
            
            // Note: scrollbar rendering would need proper state management
            // renderer.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }
    
    /// Format a message for display
    fn format_message<'a>(&self, message: &'a ChatMessage) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        
        // Message header with role and timestamp
        let timestamp = message.timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let time_str = format_timestamp(timestamp);
        
        let (role_text, role_style) = match message.role {
            MessageRole::User => ("You", Style::default().fg(self.theme.primary())),
            MessageRole::Assistant => ("Assistant", Style::default().fg(self.theme.secondary())),
            MessageRole::System => ("System", Style::default().fg(self.theme.accent())),
        };
        
        let header = Line::from(vec![
            Span::styled(role_text, role_style),
            Span::raw(" • "),
            Span::styled(time_str, Style::default().fg(self.theme.text_muted())),
        ]);
        
        lines.push(header);
        
        // Message content
        let content_lines: Vec<&str> = message.content.lines().collect();
        for line in content_lines {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(self.theme.text()),
            )));
        }
        
        // Attachments
        for attachment in &message.attachments {
            lines.push(Line::from(vec![
                Span::raw("📎 "),
                Span::styled(
                    &attachment.name,
                    Style::default().fg(self.theme.accent()),
                ),
                Span::styled(
                    format!(" ({})", format_file_size(attachment.size)),
                    Style::default().fg(self.theme.text_muted()),
                ),
            ]));
        }
        
        lines
    }
}

/// Format a timestamp for display
fn format_timestamp(timestamp: u64) -> String {
    match chrono::NaiveDateTime::from_timestamp_opt(timestamp as i64, 0) {
        Some(dt) => dt.format("%H:%M:%S").to_string(),
        None => "??:??:??".to_string(),
    }
}

/// Format file size for display
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChatConfig;
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }
    
    #[test]
    fn test_message_management() {
        let config = ChatConfig::default();
        let theme = crate::theme::DefaultTheme;
        let mut chat = ChatComponent::new(&config, &theme);
        
        // Add a message
        let message = ChatMessage {
            id: "test".to_string(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: std::time::SystemTime::now(),
            attachments: Vec::new(),
        };
        
        chat.add_message(message);
        assert_eq!(chat.messages.len(), 1);
    }
}