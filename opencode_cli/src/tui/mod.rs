//! Terminal User Interface for OpenCode

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use opencode_core::{Engine, agent::AgentConfig, agent::AgentHandle};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    collections::VecDeque,
    io::{self, Stdout},
    time::Duration,
};
use tokio::time::{interval, Instant};

mod app;
mod components;
mod events;

use app::App;
use events::Events;

/// Run the TUI
pub async fn run_tui(
    engine: Engine,
    agent_name: &str,
    system_prompt: Option<&str>,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app
    let mut app = App::new(engine, agent_name, system_prompt).await?;
    
    // Create event handler
    let events = Events::new(Duration::from_millis(100));
    
    // Run main loop
    let result = run_app(&mut terminal, &mut app, events, shutdown_signal).await;
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    result
}

/// Main application loop
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mut events: Events,
    shutdown_signal: impl std::future::Future<Output = ()>,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);
    
    // Pin the shutdown signal
    tokio::pin!(shutdown_signal);
    
    loop {
        // Draw UI
        terminal.draw(|f| ui(f, app))?;
        
        // Handle events
        tokio::select! {
            // Handle shutdown signal
            _ = &mut shutdown_signal => {
                break;
            }
            
            // Handle input events
            event = events.next() => {
                match event {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::Char('q') => {
                                    if app.input_mode == app::InputMode::Normal {
                                        break;
                                    }
                                }
                                KeyCode::Esc => {
                                    app.input_mode = app::InputMode::Normal;
                                    app.input.clear();
                                }
                                KeyCode::Enter => {
                                    if app.input_mode == app::InputMode::Editing {
                                        app.send_message().await?;
                                    } else {
                                        app.input_mode = app::InputMode::Editing;
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if app.input_mode == app::InputMode::Editing {
                                        app.input.push(c);
                                    }
                                }
                                KeyCode::Backspace => {
                                    if app.input_mode == app::InputMode::Editing {
                                        app.input.pop();
                                    }
                                }
                                KeyCode::Up => {
                                    if app.input_mode == app::InputMode::Normal {
                                        app.scroll_up();
                                    }
                                }
                                KeyCode::Down => {
                                    if app.input_mode == app::InputMode::Normal {
                                        app.scroll_down();
                                    }
                                }
                                KeyCode::Tab => {
                                    app.next_panel();
                                }
                                KeyCode::BackTab => {
                                    app.previous_panel();
                                }
                                _ => {}
                            }
                        }
                    }
                    Event::Mouse(_) => {}
                    Event::Resize(_, _) => {}
                    Event::FocusGained => {}
                    Event::FocusLost => {}
                    Event::Paste(_) => {}
                }
            }
            
            // Handle periodic updates
            _ = tokio::time::sleep_until(last_tick + tick_rate) => {
                app.on_tick().await?;
                last_tick = Instant::now();
            }
        }
    }
    
    Ok(())
}

/// Draw the UI
fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Status
        ])
        .split(f.size());
    
    // Header
    let header = Paragraph::new(format!(
        "OpenCode - Agent: {} | Provider: {}",
        app.agent_name,
        app.provider_name
    ))
    .block(Block::default().borders(Borders::ALL).title("OpenCode AI Assistant"))
    .style(Style::default().fg(Color::Cyan))
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);
    
    // Main content area
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75), // Chat area
            Constraint::Percentage(25), // Sidebar
        ])
        .split(chunks[1]);
    
    // Chat area
    let messages: Vec<ListItem> = app.messages
        .iter()
        .enumerate()
        .map(|(i, msg)| {
            let content = if msg.role == "user" {
                vec![
                    Span::styled("You: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(&msg.content),
                ]
            } else {
                vec![
                    Span::styled("AI: ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                    Span::raw(&msg.content),
                ]
            };
            
            ListItem::new(Line::from(content))
                .style(if i == app.selected_message {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                })
        })
        .collect();
    
    let chat_title = format!("Chat ({})", app.messages.len());
    let chat_list = List::new(messages)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(chat_title)
            .border_style(if app.current_panel == app::Panel::Chat {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");
    
    f.render_widget(chat_list, main_chunks[0]);
    
    // Sidebar
    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33), // Agent info
            Constraint::Percentage(33), // Statistics
            Constraint::Percentage(34), // Help
        ])
        .split(main_chunks[1]);
    
    // Agent info
    let agent_info = Paragraph::new(format!(
        "Agent: {}\nState: {}\nModel: {}",
        app.agent_name,
        app.agent_state,
        app.model_name
    ))
    .block(Block::default()
        .borders(Borders::ALL)
        .title("Agent Info")
        .border_style(if app.current_panel == app::Panel::AgentInfo {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        })
    )
    .wrap(Wrap { trim: true });
    f.render_widget(agent_info, sidebar_chunks[0]);
    
    // Statistics
    let stats_text = format!(
        "Messages: {}\nTokens: {}\nUptime: {}",
        app.stats.messages_processed,
        app.stats.tokens_consumed,
        format_duration(app.stats.uptime)
    );
    
    let stats = Paragraph::new(stats_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Statistics")
            .border_style(if app.current_panel == app::Panel::Stats {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
        )
        .wrap(Wrap { trim: true });
    f.render_widget(stats, sidebar_chunks[1]);
    
    // Help
    let help_text = if app.input_mode == app::InputMode::Editing {
        "Enter: Send | Esc: Cancel | Backspace: Delete"
    } else {
        "Enter: New message | q: Quit | Tab: Next panel | ↑↓: Scroll"
    };
    
    let help = Paragraph::new(help_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .border_style(if app.current_panel == app::Panel::Help {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
        )
        .wrap(Wrap { trim: true });
    f.render_widget(help, sidebar_chunks[2]);
    
    // Input area
    let input_title = match app.input_mode {
        app::InputMode::Normal => "Input (Press Enter to start typing)",
        app::InputMode::Editing => "Input (Press Enter to send, Esc to cancel)",
    };
    
    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            app::InputMode::Normal => Style::default(),
            app::InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default()
            .borders(Borders::ALL)
            .title(input_title)
            .border_style(if app.current_panel == app::Panel::Input {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
        );
    f.render_widget(input, chunks[2]);
    
    // Set cursor position
    if app.input_mode == app::InputMode::Editing {
        f.set_cursor(
            chunks[2].x + app.input.len() as u16 + 1,
            chunks[2].y + 1,
        );
    }
    
    // Status bar
    let status_text = if app.is_thinking {
        "Thinking..."
    } else {
        "Ready"
    };
    
    let status = Paragraph::new(status_text)
        .style(if app.is_thinking {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        })
        .alignment(Alignment::Right);
    f.render_widget(status, chunks[3]);
}

/// Format duration for display
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}