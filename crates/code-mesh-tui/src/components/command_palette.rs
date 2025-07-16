use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    style::Style,
    text::{Line, Span},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    renderer::Renderer,
    theme::Theme,
};

/// Command palette for quick actions
pub struct CommandPalette {
    theme: Box<dyn Theme + Send + Sync>,
    input: String,
    cursor_position: usize,
    commands: Vec<Command>,
    filtered_commands: Vec<usize>,
    selected_index: usize,
    is_open: bool,
}

#[derive(Debug, Clone)]
struct Command {
    name: String,
    description: String,
    action: String,
    keywords: Vec<String>,
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new(theme: &dyn Theme) -> Self {
        let commands = vec![
            Command {
                name: "Open File".to_string(),
                description: "Open a file in the editor".to_string(),
                action: "open-file".to_string(),
                keywords: vec!["open".to_string(), "file".to_string()],
            },
            Command {
                name: "Toggle Theme".to_string(),
                description: "Switch between available themes".to_string(),
                action: "toggle-theme".to_string(),
                keywords: vec!["theme".to_string(), "color".to_string(), "appearance".to_string()],
            },
            Command {
                name: "Clear Chat".to_string(),
                description: "Clear the chat history".to_string(),
                action: "clear-chat".to_string(),
                keywords: vec!["clear".to_string(), "chat".to_string(), "history".to_string()],
            },
            Command {
                name: "Show Help".to_string(),
                description: "Show help information".to_string(),
                action: "show-help".to_string(),
                keywords: vec!["help".to_string(), "info".to_string(), "about".to_string()],
            },
            Command {
                name: "Quit".to_string(),
                description: "Exit the application".to_string(),
                action: "quit".to_string(),
                keywords: vec!["quit".to_string(), "exit".to_string(), "close".to_string()],
            },
        ];
        
        let filtered_commands: Vec<usize> = (0..commands.len()).collect();
        
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            input: String::new(),
            cursor_position: 0,
            commands,
            filtered_commands,
            selected_index: 0,
            is_open: false,
        }
    }
    
    /// Open the command palette
    pub fn open(&mut self) {
        self.is_open = true;
        self.input.clear();
        self.cursor_position = 0;
        self.selected_index = 0;
        self.update_filter();
    }
    
    /// Close the command palette
    pub fn close(&mut self) {
        self.is_open = false;
    }
    
    /// Check if the command palette is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }
    
    /// Handle key events, returning the selected command if one was executed
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<String>> {
        if !self.is_open {
            return Ok(None);
        }
        
        match key.code {
            KeyCode::Esc => {
                self.close();
                Ok(None)
            }
            KeyCode::Enter => {
                if let Some(command_index) = self.filtered_commands.get(self.selected_index) {
                    let action = self.commands[*command_index].action.clone();
                    self.close();
                    Ok(Some(action))
                } else {
                    Ok(None)
                }
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                Ok(None)
            }
            KeyCode::Down => {
                if self.selected_index < self.filtered_commands.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
                Ok(None)
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                    self.update_filter();
                }
                Ok(None)
            }
            KeyCode::Delete => {
                if self.cursor_position < self.input.len() {
                    self.input.remove(self.cursor_position);
                    self.update_filter();
                }
                Ok(None)
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                Ok(None)
            }
            KeyCode::Right => {
                if self.cursor_position < self.input.len() {
                    self.cursor_position += 1;
                }
                Ok(None)
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                Ok(None)
            }
            KeyCode::End => {
                self.cursor_position = self.input.len();
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                self.update_filter();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    
    /// Update the filtered commands based on input
    fn update_filter(&mut self) {
        let query = self.input.to_lowercase();
        
        if query.is_empty() {
            self.filtered_commands = (0..self.commands.len()).collect();
        } else {
            self.filtered_commands = self.commands
                .iter()
                .enumerate()
                .filter(|(_, command)| {
                    command.name.to_lowercase().contains(&query) ||
                    command.description.to_lowercase().contains(&query) ||
                    command.keywords.iter().any(|keyword| keyword.to_lowercase().contains(&query))
                })
                .map(|(index, _)| index)
                .collect();
        }
        
        // Reset selection if it's out of bounds
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = 0;
        }
    }
    
    /// Update theme
    pub fn update_theme(&mut self, theme: &dyn Theme) {
        // In a real implementation, we'd clone or recreate the theme
        // For now, this is a placeholder
    }
    
    /// Render the command palette
    pub fn render(&self, renderer: &Renderer, area: Rect) {
        if !self.is_open {
            return;
        }
        
        // Create a block for the command palette
        let block = Block::default()
            .title("Command Palette")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_active()));
        
        // Split area for input and list
        let inner_area = block.inner(area);
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1), // Input
                ratatui::layout::Constraint::Min(1),    // Command list
            ])
            .split(inner_area);
        
        // Render the block first
        renderer.render_widget(block, area);
        
        // Render input field
        let input_style = Style::default()
            .fg(self.theme.text())
            .bg(self.theme.background_element());
        
        let input_line = Line::from(vec![
            Span::raw("> "),
            Span::styled(&self.input, input_style),
        ]);
        
        let input_paragraph = Paragraph::new(vec![input_line])
            .style(Style::default().bg(self.theme.background_element()));
        
        renderer.render_widget(input_paragraph, chunks[0]);
        
        // Render command list
        let list_items: Vec<ListItem> = self.filtered_commands
            .iter()
            .enumerate()
            .map(|(index, &command_index)| {
                let command = &self.commands[command_index];
                let style = if index == self.selected_index {
                    Style::default()
                        .fg(self.theme.background())
                        .bg(self.theme.primary())
                } else {
                    Style::default().fg(self.theme.text())
                };
                
                let content = Line::from(vec![
                    Span::styled(&command.name, style),
                    Span::raw(" - "),
                    Span::styled(&command.description, Style::default().fg(self.theme.text_muted())),
                ]);
                
                ListItem::new(content)
            })
            .collect();
        
        let list = List::new(list_items)
            .style(Style::default().bg(self.theme.background_element()));
        
        renderer.render_widget(list, chunks[1]);
    }
}