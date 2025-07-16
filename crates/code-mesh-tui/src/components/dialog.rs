use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, List, ListItem},
    style::Style,
    text::{Line, Span},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    renderer::Renderer,
    theme::Theme,
};

/// Dialog result enumeration
#[derive(Debug, Clone)]
pub enum DialogResult {
    Confirmed(String),
    Cancelled,
}

/// Dialog types
#[derive(Debug, Clone)]
pub enum DialogType {
    /// Simple confirmation dialog
    Confirmation {
        title: String,
        message: String,
    },
    /// Input dialog with text field
    Input {
        title: String,
        prompt: String,
        default_value: String,
    },
    /// Selection dialog with list of options
    Selection {
        title: String,
        message: String,
        options: Vec<String>,
    },
    /// File picker dialog
    FilePicker {
        title: String,
        current_path: String,
        filter: Option<String>,
    },
}

/// Modal dialog component
pub struct Dialog {
    theme: Box<dyn Theme + Send + Sync>,
    dialog_type: DialogType,
    input_text: String,
    cursor_position: usize,
    selected_index: usize,
    width: u16,
    height: u16,
}

impl Dialog {
    /// Create a new confirmation dialog
    pub fn confirmation(title: String, message: String, theme: &dyn Theme) -> Self {
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            dialog_type: DialogType::Confirmation { title, message },
            input_text: String::new(),
            cursor_position: 0,
            selected_index: 0,
            width: 50,
            height: 8,
        }
    }
    
    /// Create a new input dialog
    pub fn input(title: String, prompt: String, default_value: String, theme: &dyn Theme) -> Self {
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            dialog_type: DialogType::Input { title, prompt, default_value: default_value.clone() },
            input_text: default_value,
            cursor_position: 0,
            selected_index: 0,
            width: 60,
            height: 10,
        }
    }
    
    /// Create a new selection dialog
    pub fn selection(title: String, message: String, options: Vec<String>, theme: &dyn Theme) -> Self {
        let height = 8 + options.len().min(10) as u16;
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            dialog_type: DialogType::Selection { title, message, options },
            input_text: String::new(),
            cursor_position: 0,
            selected_index: 0,
            width: 60,
            height,
        }
    }
    
    /// Create a new file picker dialog
    pub fn file_picker(title: String, current_path: String, filter: Option<String>, theme: &dyn Theme) -> Self {
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            dialog_type: DialogType::FilePicker { title, current_path, filter },
            input_text: String::new(),
            cursor_position: 0,
            selected_index: 0,
            width: 80,
            height: 20,
        }
    }
    
    /// Get dialog width
    pub fn width(&self) -> u16 {
        self.width
    }
    
    /// Get dialog height
    pub fn height(&self) -> u16 {
        self.height
    }
    
    /// Handle key events, returning the result if dialog is completed
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<DialogResult>> {
        match key.code {
            KeyCode::Esc => {
                return Ok(Some(DialogResult::Cancelled));
            }
            KeyCode::Enter => {
                return Ok(Some(self.get_result()));
            }
            _ => {}
        }
        
        match &self.dialog_type {
            DialogType::Confirmation { .. } => {
                // Handle Y/N for confirmation
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        return Ok(Some(DialogResult::Confirmed("yes".to_string())));
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        return Ok(Some(DialogResult::Cancelled));
                    }
                    _ => {}
                }
            }
            DialogType::Input { .. } => {
                self.handle_input_key(key);
            }
            DialogType::Selection { options, .. } => {
                match key.code {
                    KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected_index < options.len().saturating_sub(1) {
                            self.selected_index += 1;
                        }
                    }
                    _ => {}
                }
            }
            DialogType::FilePicker { .. } => {
                // Handle file picker navigation
                match key.code {
                    KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        self.selected_index += 1;
                        // Would be bounded by actual file list length
                    }
                    _ => {}
                }
            }
        }
        
        Ok(None)
    }
    
    /// Handle input key events for text input
    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.input_text.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.input_text.len() {
                    self.input_text.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.input_text.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.input_text.len();
            }
            KeyCode::Char(c) => {
                self.input_text.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            _ => {}
        }
    }
    
    /// Get the current result
    fn get_result(&self) -> DialogResult {
        match &self.dialog_type {
            DialogType::Confirmation { .. } => {
                DialogResult::Confirmed("yes".to_string())
            }
            DialogType::Input { .. } => {
                DialogResult::Confirmed(self.input_text.clone())
            }
            DialogType::Selection { options, .. } => {
                if let Some(option) = options.get(self.selected_index) {
                    DialogResult::Confirmed(option.clone())
                } else {
                    DialogResult::Cancelled
                }
            }
            DialogType::FilePicker { current_path, .. } => {
                DialogResult::Confirmed(current_path.clone())
            }
        }
    }
    
    /// Render the dialog
    pub fn render(&self, renderer: &Renderer, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_active()))
            .style(Style::default().bg(self.theme.background_panel()));
        
        renderer.render_widget(block.clone(), area);
        
        let inner_area = block.inner(area);
        
        match &self.dialog_type {
            DialogType::Confirmation { title, message } => {
                self.render_confirmation(renderer, inner_area, title, message);
            }
            DialogType::Input { title, prompt, .. } => {
                self.render_input(renderer, inner_area, title, prompt);
            }
            DialogType::Selection { title, message, options } => {
                self.render_selection(renderer, inner_area, title, message, options);
            }
            DialogType::FilePicker { title, current_path, .. } => {
                self.render_file_picker(renderer, inner_area, title, current_path);
            }
        }
    }
    
    /// Render confirmation dialog
    fn render_confirmation(&self, renderer: &Renderer, area: Rect, title: &str, message: &str) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1), // Title
                ratatui::layout::Constraint::Min(1),    // Message
                ratatui::layout::Constraint::Length(1), // Buttons
            ])
            .split(area);
        
        // Title
        let title_paragraph = Paragraph::new(title)
            .style(Style::default().fg(self.theme.primary()));
        renderer.render_widget(title_paragraph, chunks[0]);
        
        // Message
        let message_paragraph = Paragraph::new(message)
            .style(Style::default().fg(self.theme.text()));
        renderer.render_widget(message_paragraph, chunks[1]);
        
        // Buttons
        let buttons_line = Line::from(vec![
            Span::styled("[Y]es", Style::default().fg(self.theme.success())),
            Span::raw(" / "),
            Span::styled("[N]o", Style::default().fg(self.theme.error())),
        ]);
        let buttons_paragraph = Paragraph::new(vec![buttons_line]);
        renderer.render_widget(buttons_paragraph, chunks[2]);
    }
    
    /// Render input dialog
    fn render_input(&self, renderer: &Renderer, area: Rect, title: &str, prompt: &str) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1), // Title
                ratatui::layout::Constraint::Length(1), // Prompt
                ratatui::layout::Constraint::Length(1), // Input
                ratatui::layout::Constraint::Min(1),    // Spacer
                ratatui::layout::Constraint::Length(1), // Help
            ])
            .split(area);
        
        // Title
        let title_paragraph = Paragraph::new(title)
            .style(Style::default().fg(self.theme.primary()));
        renderer.render_widget(title_paragraph, chunks[0]);
        
        // Prompt
        let prompt_paragraph = Paragraph::new(prompt)
            .style(Style::default().fg(self.theme.text()));
        renderer.render_widget(prompt_paragraph, chunks[1]);
        
        // Input field
        let input_line = Line::from(vec![
            Span::raw("> "),
            Span::styled(&self.input_text, Style::default().fg(self.theme.text())),
        ]);
        let input_paragraph = Paragraph::new(vec![input_line])
            .style(Style::default().bg(self.theme.background_element()));
        renderer.render_widget(input_paragraph, chunks[2]);
        
        // Help
        let help_line = Line::from(vec![
            Span::styled("Enter", Style::default().fg(self.theme.success())),
            Span::raw(" to confirm, "),
            Span::styled("Esc", Style::default().fg(self.theme.error())),
            Span::raw(" to cancel"),
        ]);
        let help_paragraph = Paragraph::new(vec![help_line])
            .style(Style::default().fg(self.theme.text_muted()));
        renderer.render_widget(help_paragraph, chunks[4]);
    }
    
    /// Render selection dialog
    fn render_selection(&self, renderer: &Renderer, area: Rect, title: &str, message: &str, options: &[String]) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1), // Title
                ratatui::layout::Constraint::Length(1), // Message
                ratatui::layout::Constraint::Min(1),    // Options
                ratatui::layout::Constraint::Length(1), // Help
            ])
            .split(area);
        
        // Title
        let title_paragraph = Paragraph::new(title)
            .style(Style::default().fg(self.theme.primary()));
        renderer.render_widget(title_paragraph, chunks[0]);
        
        // Message
        let message_paragraph = Paragraph::new(message)
            .style(Style::default().fg(self.theme.text()));
        renderer.render_widget(message_paragraph, chunks[1]);
        
        // Options
        let list_items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(index, option)| {
                let style = if index == self.selected_index {
                    Style::default()
                        .fg(self.theme.background())
                        .bg(self.theme.primary())
                } else {
                    Style::default().fg(self.theme.text())
                };
                
                ListItem::new(Line::from(Span::styled(option, style)))
            })
            .collect();
        
        let list = List::new(list_items)
            .style(Style::default().bg(self.theme.background_element()));
        renderer.render_widget(list, chunks[2]);
        
        // Help
        let help_line = Line::from(vec![
            Span::styled("↑↓", Style::default().fg(self.theme.accent())),
            Span::raw(" to navigate, "),
            Span::styled("Enter", Style::default().fg(self.theme.success())),
            Span::raw(" to select, "),
            Span::styled("Esc", Style::default().fg(self.theme.error())),
            Span::raw(" to cancel"),
        ]);
        let help_paragraph = Paragraph::new(vec![help_line])
            .style(Style::default().fg(self.theme.text_muted()));
        renderer.render_widget(help_paragraph, chunks[3]);
    }
    
    /// Render file picker dialog
    fn render_file_picker(&self, renderer: &Renderer, area: Rect, title: &str, current_path: &str) {
        // This is a simplified file picker
        // A full implementation would show directory contents
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1), // Title
                ratatui::layout::Constraint::Length(1), // Current path
                ratatui::layout::Constraint::Min(1),    // File list
                ratatui::layout::Constraint::Length(1), // Help
            ])
            .split(area);
        
        // Title
        let title_paragraph = Paragraph::new(title)
            .style(Style::default().fg(self.theme.primary()));
        renderer.render_widget(title_paragraph, chunks[0]);
        
        // Current path
        let path_paragraph = Paragraph::new(current_path)
            .style(Style::default().fg(self.theme.accent()));
        renderer.render_widget(path_paragraph, chunks[1]);
        
        // Placeholder file list
        let placeholder = Paragraph::new("File picker implementation pending...")
            .style(Style::default().fg(self.theme.text_muted()));
        renderer.render_widget(placeholder, chunks[2]);
        
        // Help
        let help_line = Line::from(vec![
            Span::styled("Enter", Style::default().fg(self.theme.success())),
            Span::raw(" to select, "),
            Span::styled("Esc", Style::default().fg(self.theme.error())),
            Span::raw(" to cancel"),
        ]);
        let help_paragraph = Paragraph::new(vec![help_line])
            .style(Style::default().fg(self.theme.text_muted()));
        renderer.render_widget(help_paragraph, chunks[3]);
    }
}