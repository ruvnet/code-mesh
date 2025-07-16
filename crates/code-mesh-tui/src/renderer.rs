use ratatui::{
    Frame,
    layout::Rect,
    widgets::Widget,
};

use crate::theme::Theme;

/// Renderer wrapper for consistent styling and theme application
pub struct Renderer<'a> {
    frame: &'a mut Frame<'a>,
    theme: &'a dyn Theme,
}

impl<'a> Renderer<'a> {
    /// Create a new renderer
    pub fn new(frame: &'a mut Frame<'a>, theme: &'a dyn Theme) -> Self {
        Self { frame, theme }
    }
    
    /// Get the current theme
    pub fn theme(&self) -> &dyn Theme {
        self.theme
    }
    
    /// Render a widget in the specified area
    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        self.frame.render_widget(widget, area);
    }
    
    /// Render a stateful widget
    pub fn render_stateful_widget<W: ratatui::widgets::StatefulWidget>(
        &mut self, 
        widget: W, 
        area: Rect, 
        state: &mut W::State
    ) {
        self.frame.render_stateful_widget(widget, area, state);
    }
    
    /// Get the terminal size
    pub fn size(&self) -> Rect {
        self.frame.size()
    }
}