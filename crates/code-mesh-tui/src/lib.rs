/*!
# Code Mesh TUI

Terminal User Interface for Code Mesh - A modern, responsive terminal interface
that enhances the user experience with interactive chat, file viewing, diff
comparison, and more.

## Architecture

The TUI is built using ratatui and crossterm for cross-platform terminal handling,
with a component-based architecture similar to React components.

### Core Components

- **App**: Main application state and event handling
- **Chat**: Interactive chat interface with message history and markdown rendering
- **FileViewer**: File browser and viewer with syntax highlighting
- **DiffViewer**: Side-by-side and unified diff comparison
- **StatusBar**: System information and mode indicators
- **CommandPalette**: Quick action interface
- **Dialog**: Modal dialogs for forms and confirmations

### Features

- Responsive layout system with panels and windows
- Keyboard navigation and shortcuts
- Mouse support where appropriate
- Theme system with customizable colors
- Syntax highlighting for code blocks
- Real-time log streaming
- Progress indicators for long operations
- Accessibility features (high contrast, etc.)

## Usage

```rust
use code_mesh_tui::{App, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::default();
    let mut app = App::new(config).await?;
    app.run().await
}
```
*/

pub mod app;
pub mod chat;
pub mod components;
pub mod config;
pub mod diff;
pub mod events;
pub mod file_viewer;
pub mod layout;
pub mod renderer;
pub mod status;
pub mod theme;
pub mod utils;

pub use app::App;
pub use config::Config;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to normal mode
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}