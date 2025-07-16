//! User Interface utilities for the CLI

use crate::cmd::{CliError, Result};
use console::{Style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::fmt::Display;
use std::io::{self, Write};
use std::time::Duration;

/// Main UI handler for the CLI
pub struct UI {
    term: Term,
    theme: Theme,
}

/// Color theme for the UI
#[derive(Clone)]
pub struct Theme {
    pub primary: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub info: Style,
    pub dim: Style,
    pub bold: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Style::new().cyan().bold(),
            success: Style::new().green().bold(),
            warning: Style::new().yellow().bold(),
            error: Style::new().red().bold(),
            info: Style::new().blue().bold(),
            dim: Style::new().dim(),
            bold: Style::new().bold(),
        }
    }
}

impl UI {
    pub fn new() -> Self {
        Self {
            term: Term::stderr(),
            theme: Theme::default(),
        }
    }

    /// Print the Code Mesh logo
    pub fn print_logo(&mut self) -> Result<()> {
        let logo = vec![
            "╔═══════════════════════════════════════╗",
            "║           CODE MESH CLI               ║",
            "║     AI-Powered Coding Assistant       ║",
            "╚═══════════════════════════════════════╝",
        ];

        for line in logo {
            self.println(&self.theme.primary.apply_to(line).to_string())?;
        }
        self.println("")?;
        Ok(())
    }

    /// Print a line to stderr with newline
    pub fn println(&mut self, message: &str) -> Result<()> {
        writeln!(self.term, "{}", message)?;
        Ok(())
    }

    /// Print to stderr without newline
    pub fn print(&mut self, message: &str) -> Result<()> {
        write!(self.term, "{}", message)?;
        self.term.flush()?;
        Ok(())
    }

    /// Print success message
    pub fn success(&mut self, message: &str) -> Result<()> {
        self.println(&format!("✓ {}", self.theme.success.apply_to(message)))?;
        Ok(())
    }

    /// Print error message
    pub fn error(&mut self, message: &str) -> Result<()> {
        self.println(&format!("✗ {}", self.theme.error.apply_to(message)))?;
        Ok(())
    }

    /// Print warning message
    pub fn warning(&mut self, message: &str) -> Result<()> {
        self.println(&format!("⚠ {}", self.theme.warning.apply_to(message)))?;
        Ok(())
    }

    /// Print info message
    pub fn info(&mut self, message: &str) -> Result<()> {
        self.println(&format!("ℹ {}", self.theme.info.apply_to(message)))?;
        Ok(())
    }

    /// Print dimmed text
    pub fn dim(&mut self, message: &str) -> Result<()> {
        self.println(&self.theme.dim.apply_to(message).to_string())?;
        Ok(())
    }

    /// Create a confirmation prompt
    pub fn confirm(&self, message: &str, default: bool) -> Result<bool> {
        let theme = ColorfulTheme::default();
        Ok(Confirm::with_theme(&theme)
            .with_prompt(message)
            .default(default)
            .interact()
            .map_err(|e| CliError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?)
    }

    /// Create a text input prompt
    pub fn input(&self, message: &str, default: Option<&str>) -> Result<String> {
        let theme = ColorfulTheme::default();
        let mut input = Input::<String>::with_theme(&theme).with_prompt(message);
        
        if let Some(default) = default {
            input = input.default(default.to_string());
        }
        
        Ok(input.interact()
            .map_err(|e| CliError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?)
    }

    /// Create a password input prompt
    pub fn password(&self, message: &str) -> Result<String> {
        let theme = ColorfulTheme::default();
        Ok(Password::with_theme(&theme)
            .with_prompt(message)
            .interact()
            .map_err(|e| CliError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?)
    }

    /// Create a selection prompt
    pub fn select<T>(&self, message: &str, items: &[T]) -> Result<usize>
    where
        T: Display,
    {
        let theme = ColorfulTheme::default();
        Ok(Select::with_theme(&theme)
            .with_prompt(message)
            .items(items)
            .interact()
            .map_err(|e| CliError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?)
    }

    /// Create a progress bar
    pub fn progress_bar(&self, len: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏ "),
        );
        pb.set_message(message.to_string());
        pb
    }

    /// Create a spinner
    pub fn spinner(&self, message: &str) -> ProgressBar {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(80));
        spinner
    }

    /// Clear the current line
    pub fn clear_line(&mut self) -> Result<()> {
        self.term.clear_line()?;
        Ok(())
    }

    /// Check if we have a TTY
    pub fn is_tty(&self) -> bool {
        atty::is(atty::Stream::Stderr)
    }
}

/// Progress tracking utility
pub struct ProgressTracker {
    pb: ProgressBar,
    current: u64,
    total: u64,
}

impl ProgressTracker {
    pub fn new(total: u64, message: &str) -> Self {
        let ui = UI::new();
        let pb = ui.progress_bar(total, message);
        Self {
            pb,
            current: 0,
            total,
        }
    }

    pub fn inc(&mut self, delta: u64) {
        self.current += delta;
        self.pb.set_position(self.current);
    }

    pub fn set_message(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }

    pub fn finish(&self) {
        self.pb.finish_with_message("Complete");
    }

    pub fn finish_with_message(&self, message: &str) {
        self.pb.finish_with_message(message.to_string());
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        self.pb.finish_and_clear();
    }
}

/// Utility for displaying tables
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    max_widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<String>) -> Self {
        let max_widths = headers.iter().map(|h| h.len()).collect();
        Self {
            headers,
            rows: Vec::new(),
            max_widths,
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        for (i, cell) in row.iter().enumerate() {
            if i < self.max_widths.len() {
                self.max_widths[i] = self.max_widths[i].max(cell.len());
            }
        }
        self.rows.push(row);
    }

    pub fn print(&self, ui: &mut UI) -> Result<()> {
        // Print headers
        let header_line = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| format!("{:width$}", h, width = self.max_widths[i]))
            .collect::<Vec<_>>()
            .join(" │ ");
        
        ui.println(&ui.theme.bold.apply_to(&header_line).to_string())?;
        
        // Print separator
        let separator = self
            .max_widths
            .iter()
            .map(|&width| "─".repeat(width))
            .collect::<Vec<_>>()
            .join("─┼─");
        ui.println(&ui.theme.dim.apply_to(&separator).to_string())?;

        // Print rows
        for row in &self.rows {
            let row_line = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let width = self.max_widths.get(i).copied().unwrap_or(0);
                    format!("{:width$}", cell, width = width)
                })
                .collect::<Vec<_>>()
                .join(" │ ");
            ui.println(&row_line)?;
        }

        Ok(())
    }
}