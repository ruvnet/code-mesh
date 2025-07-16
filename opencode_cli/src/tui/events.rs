//! Event handling for the TUI

use crossterm::event::{self, Event, KeyEvent, MouseEvent};
use std::time::Duration;

/// Event handler for the TUI
pub struct Events {
    /// Tick rate for the event loop
    tick_rate: Duration,
}

impl Events {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        Events { tick_rate }
    }
    
    /// Get the next event
    pub async fn next(&mut self) -> Event {
        // Check if an event is available
        if event::poll(self.tick_rate).unwrap_or(false) {
            match event::read() {
                Ok(event) => event,
                Err(_) => Event::Key(KeyEvent::from(crossterm::event::KeyCode::Null)),
            }
        } else {
            // Return a dummy event to keep the loop going
            Event::Key(KeyEvent::from(crossterm::event::KeyCode::Null))
        }
    }
}

/// Key event utilities
pub mod key {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    
    /// Check if a key event matches a specific key
    pub fn matches(event: &KeyEvent, code: KeyCode) -> bool {
        event.code == code
    }
    
    /// Check if a key event matches a specific key with modifiers
    pub fn matches_with_modifiers(event: &KeyEvent, code: KeyCode, modifiers: KeyModifiers) -> bool {
        event.code == code && event.modifiers == modifiers
    }
    
    /// Check if Ctrl+C was pressed
    pub fn is_ctrl_c(event: &KeyEvent) -> bool {
        matches_with_modifiers(event, KeyCode::Char('c'), KeyModifiers::CONTROL)
    }
    
    /// Check if Ctrl+D was pressed
    pub fn is_ctrl_d(event: &KeyEvent) -> bool {
        matches_with_modifiers(event, KeyCode::Char('d'), KeyModifiers::CONTROL)
    }
    
    /// Check if Ctrl+L was pressed (clear screen)
    pub fn is_ctrl_l(event: &KeyEvent) -> bool {
        matches_with_modifiers(event, KeyCode::Char('l'), KeyModifiers::CONTROL)
    }
    
    /// Check if Ctrl+R was pressed (reset)
    pub fn is_ctrl_r(event: &KeyEvent) -> bool {
        matches_with_modifiers(event, KeyCode::Char('r'), KeyModifiers::CONTROL)
    }
    
    /// Check if Ctrl+S was pressed (save)
    pub fn is_ctrl_s(event: &KeyEvent) -> bool {
        matches_with_modifiers(event, KeyCode::Char('s'), KeyModifiers::CONTROL)
    }
    
    /// Check if Alt+key was pressed
    pub fn is_alt(event: &KeyEvent, code: KeyCode) -> bool {
        matches_with_modifiers(event, code, KeyModifiers::ALT)
    }
    
    /// Check if Shift+key was pressed
    pub fn is_shift(event: &KeyEvent, code: KeyCode) -> bool {
        matches_with_modifiers(event, code, KeyModifiers::SHIFT)
    }
}

/// Mouse event utilities
pub mod mouse {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    
    /// Check if left mouse button was clicked
    pub fn is_left_click(event: &MouseEvent) -> bool {
        matches!(event.kind, MouseEventKind::Down(MouseButton::Left))
    }
    
    /// Check if right mouse button was clicked
    pub fn is_right_click(event: &MouseEvent) -> bool {
        matches!(event.kind, MouseEventKind::Down(MouseButton::Right))
    }
    
    /// Check if middle mouse button was clicked
    pub fn is_middle_click(event: &MouseEvent) -> bool {
        matches!(event.kind, MouseEventKind::Down(MouseButton::Middle))
    }
    
    /// Check if mouse was scrolled up
    pub fn is_scroll_up(event: &MouseEvent) -> bool {
        matches!(event.kind, MouseEventKind::ScrollUp)
    }
    
    /// Check if mouse was scrolled down
    pub fn is_scroll_down(event: &MouseEvent) -> bool {
        matches!(event.kind, MouseEventKind::ScrollDown)
    }
    
    /// Check if mouse was moved
    pub fn is_move(event: &MouseEvent) -> bool {
        matches!(event.kind, MouseEventKind::Moved)
    }
    
    /// Check if mouse was dragged
    pub fn is_drag(event: &MouseEvent) -> bool {
        matches!(event.kind, MouseEventKind::Drag(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    
    #[test]
    fn test_key_matching() {
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(key::matches(&event, KeyCode::Char('a')));
        assert!(!key::matches(&event, KeyCode::Char('b')));
    }
    
    #[test]
    fn test_key_with_modifiers() {
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(key::is_ctrl_c(&event));
        assert!(!key::is_ctrl_d(&event));
    }
    
    #[test]
    fn test_special_keys() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(key::is_ctrl_c(&ctrl_c));
        
        let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(key::is_ctrl_d(&ctrl_d));
        
        let ctrl_l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL);
        assert!(key::is_ctrl_l(&ctrl_l));
    }
}