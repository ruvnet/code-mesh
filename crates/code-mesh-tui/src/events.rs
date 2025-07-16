use crossterm::event::{Event as CrosstermEvent, KeyEvent, MouseEvent};
use futures_util::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Terminal input event
    Input(InputEvent),
    /// Application tick for animations/updates
    Tick,
    /// Application should quit
    Quit,
    /// Resize terminal
    Resize(u16, u16),
    /// Custom application event
    Custom(String),
}

/// Input events from the terminal
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
    /// Paste event
    Paste(String),
}

/// Event handler for managing terminal and application events
pub struct EventHandler {
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    _event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // Spawn terminal event listener
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let mut event_stream = crossterm::event::EventStream::new();
            
            loop {
                use futures_util::StreamExt;
                
                if let Some(Ok(event)) = event_stream.next().await {
                    let app_event = match event {
                        CrosstermEvent::Key(key) => {
                            // Handle quit shortcut early
                            if key.code == crossterm::event::KeyCode::Char('c')
                                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                AppEvent::Quit
                            } else {
                                AppEvent::Input(InputEvent::Key(key))
                            }
                        }
                        CrosstermEvent::Mouse(mouse) => AppEvent::Input(InputEvent::Mouse(mouse)),
                        CrosstermEvent::Resize(width, height) => AppEvent::Resize(width, height),
                        CrosstermEvent::FocusGained => AppEvent::Input(InputEvent::FocusGained),
                        CrosstermEvent::FocusLost => AppEvent::Input(InputEvent::FocusLost),
                        CrosstermEvent::Paste(data) => AppEvent::Input(InputEvent::Paste(data)),
                    };
                    
                    if tx.send(app_event).is_err() {
                        break;
                    }
                }
            }
        });
        
        Self {
            event_rx,
            _event_tx: event_tx,
        }
    }
    
    /// Receive the next event
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.event_rx.recv().await
    }
    
    /// Try to receive an event without blocking
    pub fn try_next(&mut self) -> Option<AppEvent> {
        self.event_rx.try_recv().ok()
    }
}

impl Stream for EventHandler {
    type Item = AppEvent;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.event_rx.poll_recv(cx)
    }
}

/// Key binding handler for mapping keys to actions
#[derive(Debug, Clone)]
pub struct KeybindHandler {
    bindings: std::collections::HashMap<String, String>,
    leader_key: Option<String>,
    leader_sequence: bool,
}

impl KeybindHandler {
    /// Create a new keybind handler
    pub fn new() -> Self {
        Self {
            bindings: std::collections::HashMap::new(),
            leader_key: None,
            leader_sequence: false,
        }
    }
    
    /// Set the leader key
    pub fn set_leader_key(&mut self, key: String) {
        self.leader_key = Some(key);
    }
    
    /// Add a key binding
    pub fn bind(&mut self, key: String, action: String) {
        self.bindings.insert(key, action);
    }
    
    /// Handle a key event and return the bound action if any
    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<String> {
        let key_string = self.key_to_string(key);
        
        // Check if this is the leader key
        if let Some(leader) = &self.leader_key {
            if key_string == *leader && !self.leader_sequence {
                self.leader_sequence = true;
                return None;
            }
        }
        
        // If we're in a leader sequence, check for leader bindings
        if self.leader_sequence {
            self.leader_sequence = false;
            let leader_binding = format!("leader+{}", key_string);
            return self.bindings.get(&leader_binding).cloned();
        }
        
        // Check for direct bindings
        self.bindings.get(&key_string).cloned()
    }
    
    /// Convert a key event to a string representation
    fn key_to_string(&self, key: &KeyEvent) -> String {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        let mut parts = Vec::new();
        
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("ctrl");
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            parts.push("alt");
        }
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("shift");
        }
        
        let key_part = match key.code {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "enter".to_string(),
            KeyCode::Tab => "tab".to_string(),
            KeyCode::Backspace => "backspace".to_string(),
            KeyCode::Delete => "delete".to_string(),
            KeyCode::Insert => "insert".to_string(),
            KeyCode::Home => "home".to_string(),
            KeyCode::End => "end".to_string(),
            KeyCode::PageUp => "pageup".to_string(),
            KeyCode::PageDown => "pagedown".to_string(),
            KeyCode::Up => "up".to_string(),
            KeyCode::Down => "down".to_string(),
            KeyCode::Left => "left".to_string(),
            KeyCode::Right => "right".to_string(),
            KeyCode::Esc => "esc".to_string(),
            KeyCode::F(n) => format!("f{}", n),
            _ => "unknown".to_string(),
        };
        
        parts.push(&key_part);
        parts.join("+")
    }
}

impl Default for KeybindHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Mouse handler for processing mouse events
#[derive(Debug, Clone)]
pub struct MouseHandler {
    last_position: (u16, u16),
    drag_state: Option<DragState>,
}

#[derive(Debug, Clone)]
struct DragState {
    start_position: (u16, u16),
    current_position: (u16, u16),
}

impl MouseHandler {
    /// Create a new mouse handler
    pub fn new() -> Self {
        Self {
            last_position: (0, 0),
            drag_state: None,
        }
    }
    
    /// Handle a mouse event
    pub fn handle_mouse(&mut self, event: &MouseEvent) -> MouseAction {
        use crossterm::event::{MouseButton, MouseEventKind};
        
        match event.kind {
            MouseEventKind::Down(button) => {
                self.last_position = (event.column, event.row);
                match button {
                    MouseButton::Left => {
                        self.drag_state = Some(DragState {
                            start_position: (event.column, event.row),
                            current_position: (event.column, event.row),
                        });
                        MouseAction::LeftClick(event.column, event.row)
                    }
                    MouseButton::Right => MouseAction::RightClick(event.column, event.row),
                    MouseButton::Middle => MouseAction::MiddleClick(event.column, event.row),
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(drag) = self.drag_state.take() {
                    if drag.start_position != drag.current_position {
                        MouseAction::DragEnd(drag.start_position, drag.current_position)
                    } else {
                        MouseAction::LeftClick(event.column, event.row)
                    }
                } else {
                    MouseAction::LeftClick(event.column, event.row)
                }
            }
            MouseEventKind::Up(_) => MouseAction::None,
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(ref mut drag) = self.drag_state {
                    drag.current_position = (event.column, event.row);
                    MouseAction::Drag(drag.start_position, drag.current_position)
                } else {
                    MouseAction::None
                }
            }
            MouseEventKind::Moved => {
                self.last_position = (event.column, event.row);
                MouseAction::Move(event.column, event.row)
            }
            MouseEventKind::ScrollDown => MouseAction::ScrollDown(event.column, event.row),
            MouseEventKind::ScrollUp => MouseAction::ScrollUp(event.column, event.row),
            MouseEventKind::ScrollLeft => MouseAction::ScrollLeft(event.column, event.row),
            MouseEventKind::ScrollRight => MouseAction::ScrollRight(event.column, event.row),
            _ => MouseAction::None,
        }
    }
}

impl Default for MouseHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Mouse actions that can be performed
#[derive(Debug, Clone)]
pub enum MouseAction {
    None,
    LeftClick(u16, u16),
    RightClick(u16, u16),
    MiddleClick(u16, u16),
    Move(u16, u16),
    Drag((u16, u16), (u16, u16)), // start, current
    DragEnd((u16, u16), (u16, u16)), // start, end
    ScrollUp(u16, u16),
    ScrollDown(u16, u16),
    ScrollLeft(u16, u16),
    ScrollRight(u16, u16),
}

/// Event dispatcher for routing events to components
pub struct EventDispatcher {
    handlers: Vec<Box<dyn EventConsumer>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    
    /// Add an event handler
    pub fn add_handler(&mut self, handler: Box<dyn EventConsumer>) {
        self.handlers.push(handler);
    }
    
    /// Dispatch an event to all handlers
    pub fn dispatch(&mut self, event: &AppEvent) -> bool {
        for handler in &mut self.handlers {
            if handler.handle_event(event) {
                return true; // Event was consumed
            }
        }
        false
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for components that can consume events
pub trait EventConsumer {
    /// Handle an event, returning true if the event was consumed
    fn handle_event(&mut self, event: &AppEvent) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_key_to_string() {
        let handler = KeybindHandler::new();
        
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(handler.key_to_string(&key), "a");
        
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(handler.key_to_string(&key), "ctrl+a");
        
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(handler.key_to_string(&key), "enter");
    }
    
    #[test]
    fn test_keybind_handler() {
        let mut handler = KeybindHandler::new();
        handler.bind("q".to_string(), "quit".to_string());
        handler.bind("ctrl+c".to_string(), "interrupt".to_string());
        
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key(&key), Some("quit".to_string()));
        
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key(&key), Some("interrupt".to_string()));
    }
    
    #[test]
    fn test_leader_sequence() {
        let mut handler = KeybindHandler::new();
        handler.set_leader_key("ctrl+x".to_string());
        handler.bind("leader+s".to_string(), "save".to_string());
        
        // First press the leader key
        let leader_key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key(&leader_key), None);
        
        // Then press the bound key
        let bound_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key(&bound_key), Some("save".to_string()));
    }
}