use ratatui::{
    layout::Rect,
    widgets::Paragraph,
    style::Style,
    text::{Line, Span},
};
use anyhow::Result;

use crate::{
    app::AppState,
    renderer::Renderer,
    theme::Theme,
};

/// Status bar component showing application information
pub struct StatusBar {
    theme: Box<dyn Theme + Send + Sync>,
    mode: String,
    file_path: Option<String>,
    cursor_position: (u16, u16),
    selection_info: Option<String>,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new(theme: &dyn Theme) -> Self {
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            mode: "NORMAL".to_string(),
            file_path: None,
            cursor_position: (0, 0),
            selection_info: None,
        }
    }
    
    /// Update the status bar
    pub async fn update(&mut self, state: &AppState) -> Result<()> {
        self.mode = match state {
            AppState::Running => "NORMAL".to_string(),
            AppState::Chat => "CHAT".to_string(),
            AppState::FileViewer => "FILE".to_string(),
            AppState::CommandPalette => "COMMAND".to_string(),
            AppState::Dialog => "DIALOG".to_string(),
            AppState::Help => "HELP".to_string(),
            AppState::Quitting => "QUIT".to_string(),
        };
        Ok(())
    }
    
    /// Set the current file path
    pub fn set_file_path(&mut self, path: Option<String>) {
        self.file_path = path;
    }
    
    /// Set cursor position
    pub fn set_cursor_position(&mut self, x: u16, y: u16) {
        self.cursor_position = (x, y);
    }
    
    /// Set selection information
    pub fn set_selection_info(&mut self, info: Option<String>) {
        self.selection_info = info;
    }
    
    /// Update theme
    pub fn update_theme(&mut self, theme: &dyn Theme) {
        // In a real implementation, we'd clone or recreate the theme
        // For now, this is a placeholder
    }
    
    /// Render the status bar
    pub fn render(&self, renderer: &mut Renderer, area: Rect) {
        let mode_style = Style::default()
            .fg(self.theme.background())
            .bg(self.theme.primary());
            
        let file_style = Style::default()
            .fg(self.theme.text())
            .bg(self.theme.background_panel());
            
        let cursor_style = Style::default()
            .fg(self.theme.text_muted())
            .bg(self.theme.background_panel());
        
        // Build status line spans
        let mut spans = vec![
            Span::styled(format!(" {} ", self.mode), mode_style),
            Span::raw(" "),
        ];
        
        if let Some(ref path) = self.file_path {
            spans.push(Span::styled(format!(" {} ", path), file_style));
            spans.push(Span::raw(" "));
        }
        
        // Add cursor position
        spans.push(Span::styled(
            format!(" {}:{} ", self.cursor_position.0, self.cursor_position.1),
            cursor_style,
        ));
        
        if let Some(ref selection) = self.selection_info {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(format!(" {} ", selection), cursor_style));
        }
        
        let status_line = Line::from(spans);
        
        let paragraph = Paragraph::new(vec![status_line])
            .style(Style::default().bg(self.theme.background_panel()));
            
        renderer.render_widget(paragraph, area);
    }
}