use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::time::{Duration, Instant};

use crate::{
    chat::ChatComponent,
    components::{CommandPalette, Dialog, StatusBar},
    config::Config,
    events::{AppEvent, EventHandler, InputEvent, KeybindHandler, MouseHandler},
    file_viewer::FileViewer,
    layout::{LayoutManager, PopupLayout},
    renderer::Renderer,
    theme::ThemeManager,
};

/// Main application state
pub struct App {
    /// Application configuration
    config: Config,
    /// Theme manager
    theme_manager: ThemeManager,
    /// Event handler
    event_handler: EventHandler,
    /// Keybind handler
    keybind_handler: KeybindHandler,
    /// Mouse handler
    mouse_handler: MouseHandler,
    /// Layout manager
    layout_manager: LayoutManager,
    /// Chat component
    chat: ChatComponent,
    /// File viewer component
    file_viewer: FileViewer,
    /// Status bar component
    status_bar: StatusBar,
    /// Command palette
    command_palette: CommandPalette,
    /// Active dialog
    active_dialog: Option<Dialog>,
    /// Application state
    state: AppState,
    /// Last render time
    last_render: Instant,
    /// Frame rate target
    target_fps: u64,
}

/// Application state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Normal operation
    Running,
    /// Application should quit
    Quitting,
    /// Showing help
    Help,
    /// Command palette is open
    CommandPalette,
    /// Dialog is open
    Dialog,
    /// File viewer is focused
    FileViewer,
    /// Chat is focused
    Chat,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        let theme_manager = ThemeManager::default();
        let event_handler = EventHandler::new();
        let mut keybind_handler = KeybindHandler::new();
        
        // Setup keybindings from config
        keybind_handler.set_leader_key(config.keybinds.leader.clone());
        for (action, key) in &config.keybinds.bindings {
            keybind_handler.bind(key.clone(), action.clone());
        }
        
        let mouse_handler = MouseHandler::new();
        
        // Initialize layout with default terminal size
        let layout_manager = LayoutManager::new(ratatui::layout::Rect::new(0, 0, 80, 24));
        
        // Initialize components
        let chat = ChatComponent::new(&config.chat, theme_manager.current_theme());
        let file_viewer = FileViewer::new(&config.file_viewer, theme_manager.current_theme());
        let status_bar = StatusBar::new(theme_manager.current_theme());
        let command_palette = CommandPalette::new(theme_manager.current_theme());
        
        Ok(Self {
            config,
            theme_manager,
            event_handler,
            keybind_handler,
            mouse_handler,
            layout_manager,
            chat,
            file_viewer,
            status_bar,
            command_palette,
            active_dialog: None,
            state: AppState::Running,
            last_render: Instant::now(),
            target_fps: 60,
        })
    }
    
    /// Run the application main loop
    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = crate::init_terminal()?;
        
        // Main application loop
        while self.state != AppState::Quitting {
            // Handle events
            if let Some(event) = self.event_handler.try_next() {
                self.handle_event(event).await?;
            }
            
            // Render at target FPS
            let now = Instant::now();
            let frame_duration = Duration::from_millis(1000 / self.target_fps);
            
            if now.duration_since(self.last_render) >= frame_duration {
                self.render(&mut terminal)?;
                self.last_render = now;
            }
            
            // Small sleep to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        
        crate::restore_terminal(&mut terminal)?;
        Ok(())
    }
    
    /// Handle an application event
    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(input_event) => {
                self.handle_input_event(input_event).await?;
            }
            AppEvent::Resize(width, height) => {
                let new_area = ratatui::layout::Rect::new(0, 0, width, height);
                self.layout_manager.resize(new_area);
            }
            AppEvent::Quit => {
                self.state = AppState::Quitting;
            }
            AppEvent::Tick => {
                // Handle periodic updates
                self.update_components().await?;
            }
            AppEvent::Custom(message) => {
                self.handle_custom_event(message).await?;
            }
        }
        Ok(())
    }
    
    /// Handle input events
    async fn handle_input_event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::Key(key_event) => {
                // Check for global keybindings first
                if let Some(action) = self.keybind_handler.handle_key(&key_event) {
                    self.execute_action(&action).await?;
                } else {
                    // Route to active component
                    self.route_key_event(key_event).await?;
                }
            }
            InputEvent::Mouse(mouse_event) => {
                let action = self.mouse_handler.handle_mouse(&mouse_event);
                self.handle_mouse_action(action).await?;
            }
            InputEvent::Paste(data) => {
                // Route paste to active component
                if self.state == AppState::Chat {
                    self.chat.handle_paste(data).await?;
                }
            }
            InputEvent::FocusGained | InputEvent::FocusLost => {
                // Handle focus changes if needed
            }
        }
        Ok(())
    }
    
    /// Execute a bound action
    async fn execute_action(&mut self, action: &str) -> Result<()> {
        match action {
            "quit" => {
                self.state = AppState::Quitting;
            }
            "help" => {
                self.toggle_help();
            }
            "command_palette" => {
                self.toggle_command_palette();
            }
            "send_message" => {
                if self.state == AppState::Chat {
                    self.chat.send_message().await?;
                }
            }
            "new_line" => {
                if self.state == AppState::Chat {
                    self.chat.insert_newline();
                }
            }
            "clear_input" => {
                if self.state == AppState::Chat {
                    self.chat.clear_input();
                }
            }
            "open_file" => {
                self.open_file_dialog();
            }
            "close_file" => {
                self.file_viewer.close_file();
                if self.state == AppState::FileViewer {
                    self.state = AppState::Chat;
                }
            }
            "toggle_diff" => {
                self.file_viewer.toggle_diff_style();
            }
            "scroll_up" => {
                self.handle_scroll(true).await?;
            }
            "scroll_down" => {
                self.handle_scroll(false).await?;
            }
            "page_up" => {
                self.handle_page_scroll(true).await?;
            }
            "page_down" => {
                self.handle_page_scroll(false).await?;
            }
            _ => {
                // Unknown action, ignore or log
            }
        }
        Ok(())
    }
    
    /// Route key events to the appropriate component
    async fn route_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> Result<()> {
        match self.state {
            AppState::Chat => {
                self.chat.handle_key_event(key_event).await?;
            }
            AppState::FileViewer => {
                self.file_viewer.handle_key_event(key_event).await?;
            }
            AppState::CommandPalette => {
                if let Some(result) = self.command_palette.handle_key_event(key_event).await? {
                    self.execute_command_palette_result(result).await?;
                    self.state = AppState::Chat;
                }
            }
            AppState::Dialog => {
                if let Some(ref mut dialog) = self.active_dialog {
                    if let Some(result) = dialog.handle_key_event(key_event).await? {
                        self.handle_dialog_result(result).await?;
                        self.active_dialog = None;
                        self.state = AppState::Chat;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle mouse actions
    async fn handle_mouse_action(&mut self, action: crate::events::MouseAction) -> Result<()> {
        use crate::events::MouseAction;
        
        match action {
            MouseAction::LeftClick(x, y) => {
                // Determine which component was clicked
                if self.layout_manager.main_area.intersects(ratatui::layout::Rect::new(x, y, 1, 1)) {
                    if self.file_viewer.is_visible() {
                        self.state = AppState::FileViewer;
                    } else {
                        self.state = AppState::Chat;
                    }
                }
            }
            MouseAction::ScrollUp(x, y) => {
                if self.is_in_scrollable_area(x, y) {
                    self.handle_scroll(true).await?;
                }
            }
            MouseAction::ScrollDown(x, y) => {
                if self.is_in_scrollable_area(x, y) {
                    self.handle_scroll(false).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Update components periodically
    async fn update_components(&mut self) -> Result<()> {
        self.chat.update().await?;
        self.file_viewer.update().await?;
        self.status_bar.update(&self.state).await?;
        Ok(())
    }
    
    /// Handle custom application events
    async fn handle_custom_event(&mut self, message: String) -> Result<()> {
        // Parse and handle custom events
        // This could be used for inter-component communication
        match message.as_str() {
            "theme_changed" => {
                self.update_theme();
            }
            "file_opened" => {
                self.state = AppState::FileViewer;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Render the application
    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let mut renderer = Renderer::new(frame, self.theme_manager.current_theme());
            
            // Render main layout
            self.render_main_layout(&mut renderer);
            
            // Render popups/dialogs on top
            self.render_overlays(&mut renderer);
        })?;
        Ok(())
    }
    
    /// Render the main application layout
    fn render_main_layout(&mut self, renderer: &mut Renderer) {
        // Render status bar
        self.status_bar.render(renderer, self.layout_manager.status_area);
        
        // Render main content area
        if self.file_viewer.is_visible() {
            // Show file viewer in side panel if available, or full area
            if let Some(side_panel) = self.layout_manager.side_panel {
                self.chat.render(renderer, self.layout_manager.main_area);
                self.file_viewer.render(renderer, side_panel);
            } else {
                self.file_viewer.render(renderer, self.layout_manager.main_area);
            }
        } else {
            self.chat.render(renderer, self.layout_manager.main_area);
        }
        
        // Render input area
        self.chat.render_input(renderer, self.layout_manager.input_area);
    }
    
    /// Render overlay components like dialogs and command palette
    fn render_overlays(&mut self, renderer: &mut Renderer) {
        match self.state {
            AppState::CommandPalette => {
                let popup_area = PopupLayout::centered(
                    self.layout_manager.terminal_area,
                    60,
                    15,
                );
                self.command_palette.render(renderer, popup_area);
            }
            AppState::Dialog => {
                if let Some(ref mut dialog) = self.active_dialog {
                    let popup_area = PopupLayout::centered(
                        self.layout_manager.terminal_area,
                        dialog.width(),
                        dialog.height(),
                    );
                    dialog.render(renderer, popup_area);
                }
            }
            AppState::Help => {
                let popup_area = PopupLayout::percentage(
                    self.layout_manager.terminal_area,
                    80,
                    80,
                );
                self.render_help(renderer, popup_area);
            }
            _ => {}
        }
    }
    
    /// Toggle help display
    fn toggle_help(&mut self) {
        self.state = if self.state == AppState::Help {
            AppState::Chat
        } else {
            AppState::Help
        };
    }
    
    /// Toggle command palette
    fn toggle_command_palette(&mut self) {
        self.state = if self.state == AppState::CommandPalette {
            AppState::Chat
        } else {
            AppState::CommandPalette
        };
    }
    
    /// Open file dialog
    fn open_file_dialog(&mut self) {
        // Implementation would create a file picker dialog
        // For now, this is a placeholder
    }
    
    /// Handle scrolling
    async fn handle_scroll(&mut self, up: bool) -> Result<()> {
        match self.state {
            AppState::Chat => {
                if up {
                    self.chat.scroll_up();
                } else {
                    self.chat.scroll_down();
                }
            }
            AppState::FileViewer => {
                if up {
                    self.file_viewer.scroll_up();
                } else {
                    self.file_viewer.scroll_down();
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle page scrolling
    async fn handle_page_scroll(&mut self, up: bool) -> Result<()> {
        match self.state {
            AppState::Chat => {
                if up {
                    self.chat.page_up();
                } else {
                    self.chat.page_down();
                }
            }
            AppState::FileViewer => {
                if up {
                    self.file_viewer.page_up();
                } else {
                    self.file_viewer.page_down();
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Check if coordinates are in a scrollable area
    fn is_in_scrollable_area(&self, x: u16, y: u16) -> bool {
        let point = ratatui::layout::Rect::new(x, y, 1, 1);
        self.layout_manager.main_area.intersects(point) ||
        self.layout_manager.side_panel.map_or(false, |area| area.intersects(point))
    }
    
    /// Execute command palette result
    async fn execute_command_palette_result(&mut self, result: String) -> Result<()> {
        // Parse and execute command palette commands
        match result.as_str() {
            "open-file" => self.open_file_dialog(),
            "toggle-theme" => self.cycle_theme(),
            "clear-chat" => self.chat.clear().await?,
            _ => {}
        }
        Ok(())
    }
    
    /// Handle dialog result
    async fn handle_dialog_result(&mut self, result: crate::components::DialogResult) -> Result<()> {
        use crate::components::DialogResult;
        
        match result {
            DialogResult::Confirmed(_data) => {
                // Handle confirmed dialog with data
            }
            DialogResult::Cancelled => {
                // Handle cancelled dialog
            }
        }
        Ok(())
    }
    
    /// Update theme for all components
    fn update_theme(&mut self) {
        let theme = self.theme_manager.current_theme();
        self.chat.update_theme(theme);
        self.file_viewer.update_theme(theme);
        self.status_bar.update_theme(theme);
        self.command_palette.update_theme(theme);
    }
    
    /// Cycle through available themes
    fn cycle_theme(&mut self) {
        let themes = self.theme_manager.available_themes();
        if !themes.is_empty() {
            let current_name = self.theme_manager.current_theme().name();
            let current_index = themes.iter().position(|name| name == current_name).unwrap_or(0);
            let next_index = (current_index + 1) % themes.len();
            let next_theme = &themes[next_index];
            
            if let Err(e) = self.theme_manager.set_theme(next_theme) {
                eprintln!("Failed to set theme {}: {}", next_theme, e);
            } else {
                self.update_theme();
            }
        }
    }
    
    /// Render help overlay
    fn render_help(&self, renderer: &mut Renderer, area: ratatui::layout::Rect) {
        // Implementation would render help content
        // This is a placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_app_creation() {
        let config = Config::default();
        let app = App::new(config).await;
        assert!(app.is_ok());
    }

    #[tokio::test]
    async fn test_app_state_transitions() {
        let config = Config::default();
        let mut app = App::new(config).await.unwrap();
        
        assert_eq!(app.state, AppState::Running);
        
        app.toggle_help();
        assert_eq!(app.state, AppState::Help);
        
        app.toggle_help();
        assert_eq!(app.state, AppState::Chat);
    }
}