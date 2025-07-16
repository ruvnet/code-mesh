//! UI components for the TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};
use std::collections::VecDeque;

/// Chat message component
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Chat window component
pub struct ChatWindow {
    pub messages: VecDeque<ChatMessage>,
    pub selected: usize,
    pub scroll: usize,
}

impl ChatWindow {
    /// Create a new chat window
    pub fn new() -> Self {
        ChatWindow {
            messages: VecDeque::new(),
            selected: 0,
            scroll: 0,
        }
    }
    
    /// Add a message to the chat
    pub fn add_message(&mut self, role: String, content: String) {
        self.messages.push_back(ChatMessage {
            role,
            content,
            timestamp: chrono::Utc::now(),
        });
        self.selected = self.messages.len().saturating_sub(1);
    }
    
    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.selected = 0;
        self.scroll = 0;
    }
    
    /// Scroll up
    pub fn scroll_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    
    /// Scroll down
    pub fn scroll_down(&mut self) {
        if self.selected < self.messages.len().saturating_sub(1) {
            self.selected += 1;
        }
    }
    
    /// Render the chat window
    pub fn render<B: ratatui::backend::Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
        focused: bool,
    ) {
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let timestamp = msg.timestamp.format("%H:%M:%S");
                let header = if msg.role == "user" {
                    vec![
                        Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::DarkGray)),
                        Span::styled("You: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    ]
                } else {
                    vec![
                        Span::styled(format!("[{}] ", timestamp), Style::default().fg(Color::DarkGray)),
                        Span::styled("AI: ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                    ]
                };
                
                let mut lines = vec![Line::from(header)];
                
                // Wrap long messages
                let content_lines = wrap_text(&msg.content, area.width as usize - 4);
                for line in content_lines {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::raw(line),
                    ]));
                }
                
                ListItem::new(lines)
                    .style(if i == self.selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    })
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Chat ({})", self.messages.len()))
                .border_style(if focused {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        
        f.render_widget(list, area);
        
        // Render scrollbar if needed
        if self.messages.len() > area.height as usize - 2 {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(self.messages.len())
                .position(self.selected);
            
            f.render_stateful_widget(
                scrollbar,
                area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }
    }
}

/// Input box component
pub struct InputBox {
    pub content: String,
    pub cursor_position: usize,
    pub placeholder: String,
}

impl InputBox {
    /// Create a new input box
    pub fn new(placeholder: String) -> Self {
        InputBox {
            content: String::new(),
            cursor_position: 0,
            placeholder,
        }
    }
    
    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }
    
    /// Delete the character before the cursor
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.content.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }
    
    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }
    
    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.content.len() {
            self.cursor_position += 1;
        }
    }
    
    /// Move cursor to beginning
    pub fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
    }
    
    /// Move cursor to end
    pub fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.content.len();
    }
    
    /// Clear the input
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_position = 0;
    }
    
    /// Get the current content
    pub fn content(&self) -> &str {
        &self.content
    }
    
    /// Check if the input is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    
    /// Render the input box
    pub fn render<B: ratatui::backend::Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
        focused: bool,
        title: &str,
    ) {
        let content = if self.content.is_empty() && !focused {
            &self.placeholder
        } else {
            &self.content
        };
        
        let input = Paragraph::new(content)
            .style(if focused {
                Style::default().fg(Color::Yellow)
            } else if self.content.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            })
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(if focused {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
            );
        
        f.render_widget(input, area);
        
        // Set cursor position if focused
        if focused {
            f.set_cursor(
                area.x + self.cursor_position as u16 + 1,
                area.y + 1,
            );
        }
    }
}

/// Status bar component
pub struct StatusBar {
    pub status: String,
    pub is_thinking: bool,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        StatusBar {
            status: "Ready".to_string(),
            is_thinking: false,
        }
    }
    
    /// Set the status
    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }
    
    /// Set thinking state
    pub fn set_thinking(&mut self, thinking: bool) {
        self.is_thinking = thinking;
        if thinking {
            self.status = "Thinking...".to_string();
        } else {
            self.status = "Ready".to_string();
        }
    }
    
    /// Render the status bar
    pub fn render<B: ratatui::backend::Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let style = if self.is_thinking {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };
        
        let status = Paragraph::new(self.status.as_str())
            .style(style)
            .block(Block::default().borders(Borders::TOP));
        
        f.render_widget(status, area);
    }
}

/// Progress bar component
pub struct ProgressBar {
    pub progress: f64,
    pub label: String,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(label: String) -> Self {
        ProgressBar {
            progress: 0.0,
            label,
        }
    }
    
    /// Set the progress (0.0 to 1.0)
    pub fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }
    
    /// Render the progress bar
    pub fn render<B: ratatui::backend::Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(&self.label))
            .gauge_style(Style::default().fg(Color::Blue))
            .ratio(self.progress);
        
        f.render_widget(gauge, area);
    }
}

/// Helper function to wrap text
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    
    for word in text.split_whitespace() {
        let word_width = word.chars().count();
        
        if current_width + word_width + 1 <= width {
            if !current_line.is_empty() {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_width;
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            current_line = word.to_string();
            current_width = word_width;
        }
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    if lines.is_empty() {
        lines.push(String::new());
    }
    
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chat_window() {
        let mut chat = ChatWindow::new();
        assert_eq!(chat.messages.len(), 0);
        
        chat.add_message("user".to_string(), "Hello".to_string());
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.selected, 0);
        
        chat.add_message("assistant".to_string(), "Hi there!".to_string());
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.selected, 1);
        
        chat.clear();
        assert_eq!(chat.messages.len(), 0);
        assert_eq!(chat.selected, 0);
    }
    
    #[test]
    fn test_input_box() {
        let mut input = InputBox::new("Enter message...".to_string());
        assert!(input.is_empty());
        
        input.insert_char('H');
        input.insert_char('i');
        assert_eq!(input.content(), "Hi");
        assert_eq!(input.cursor_position, 2);
        
        input.delete_char();
        assert_eq!(input.content(), "H");
        assert_eq!(input.cursor_position, 1);
        
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_position, 0);
    }
    
    #[test]
    fn test_status_bar() {
        let mut status = StatusBar::new();
        assert_eq!(status.status, "Ready");
        assert!(!status.is_thinking);
        
        status.set_thinking(true);
        assert_eq!(status.status, "Thinking...");
        assert!(status.is_thinking);
        
        status.set_thinking(false);
        assert_eq!(status.status, "Ready");
        assert!(!status.is_thinking);
    }
    
    #[test]
    fn test_wrap_text() {
        let text = "This is a long line that should be wrapped";
        let wrapped = wrap_text(text, 10);
        assert!(wrapped.len() > 1);
        assert!(wrapped[0].len() <= 10);
        
        let short_text = "Short";
        let wrapped_short = wrap_text(short_text, 10);
        assert_eq!(wrapped_short.len(), 1);
        assert_eq!(wrapped_short[0], "Short");
    }
}