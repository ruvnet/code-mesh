//! CLI utilities and helpers

use anyhow::Result;
use console::{style, Color};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use termcolor::{Color as TermColor, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// CLI output utilities
pub struct Output {
    stdout: StandardStream,
    stderr: StandardStream,
}

impl Output {
    /// Create a new output handler
    pub fn new() -> Self {
        Output {
            stdout: StandardStream::stdout(ColorChoice::Auto),
            stderr: StandardStream::stderr(ColorChoice::Auto),
        }
    }
    
    /// Print a success message
    pub fn success(&mut self, message: &str) -> Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(TermColor::Green)))?;
        writeln!(self.stdout, "✓ {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }
    
    /// Print an error message
    pub fn error(&mut self, message: &str) -> Result<()> {
        self.stderr.set_color(ColorSpec::new().set_fg(Some(TermColor::Red)))?;
        writeln!(self.stderr, "✗ {}", message)?;
        self.stderr.reset()?;
        Ok(())
    }
    
    /// Print a warning message
    pub fn warning(&mut self, message: &str) -> Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(TermColor::Yellow)))?;
        writeln!(self.stdout, "⚠ {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }
    
    /// Print an info message
    pub fn info(&mut self, message: &str) -> Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(TermColor::Blue)))?;
        writeln!(self.stdout, "ℹ {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }
    
    /// Print a regular message
    pub fn println(&mut self, message: &str) -> Result<()> {
        writeln!(self.stdout, "{}", message)?;
        Ok(())
    }
    
    /// Print without newline
    pub fn print(&mut self, message: &str) -> Result<()> {
        write!(self.stdout, "{}", message)?;
        self.stdout.flush()?;
        Ok(())
    }
    
    /// Print a table header
    pub fn table_header(&mut self, columns: &[&str]) -> Result<()> {
        self.stdout.set_color(ColorSpec::new().set_bold(true))?;
        let header = columns.join(" | ");
        writeln!(self.stdout, "{}", header)?;
        writeln!(self.stdout, "{}", "-".repeat(header.len()))?;
        self.stdout.reset()?;
        Ok(())
    }
    
    /// Print a table row
    pub fn table_row(&mut self, columns: &[&str]) -> Result<()> {
        writeln!(self.stdout, "{}", columns.join(" | "))?;
        Ok(())
    }
    
    /// Print JSON output
    pub fn json(&mut self, value: &serde_json::Value) -> Result<()> {
        let json_str = serde_json::to_string_pretty(value)?;
        writeln!(self.stdout, "{}", json_str)?;
        Ok(())
    }
    
    /// Create a progress bar
    pub fn progress_bar(&self, length: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(length);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos:>7}/{len:7} {percent:>3}%")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏  ")
        );
        pb.set_message(message.to_string());
        pb
    }
    
    /// Create a spinner
    pub fn spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a duration in a human-readable way
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if secs >= 60 {
        let mins = secs / 60;
        let secs = secs % 60;
        if mins >= 60 {
            let hours = mins / 60;
            let mins = mins % 60;
            format!("{}h {}m {}s", hours, mins, secs)
        } else {
            format!("{}m {}s", mins, secs)
        }
    } else if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Format file size in a human-readable way
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format a number with thousand separators
pub fn format_number(num: u64) -> String {
    let s = num.to_string();
    let len = s.len();
    let mut result = String::new();
    
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    
    result
}

/// Read a line from stdin
pub fn read_line() -> Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Read a line with a prompt
pub fn read_line_with_prompt(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    read_line()
}

/// Ask for confirmation
pub fn confirm(message: &str) -> Result<bool> {
    loop {
        let input = read_line_with_prompt(&format!("{} (y/n): ", message))?;
        match input.to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Please enter 'y' or 'n'"),
        }
    }
}

/// Ask for input with a default value
pub fn input_with_default(prompt: &str, default: &str) -> Result<String> {
    let input = read_line_with_prompt(&format!("{} [{}]: ", prompt, default))?;
    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input)
    }
}

/// Style text with color
pub fn style_text(text: &str, color: Color) -> String {
    style(text).color(color).to_string()
}

/// Create a styled header
pub fn header(text: &str) -> String {
    style(text).bold().underlined().to_string()
}

/// Create a styled subheader
pub fn subheader(text: &str) -> String {
    style(text).bold().to_string()
}

/// Create a styled success message
pub fn success_text(text: &str) -> String {
    style(text).green().to_string()
}

/// Create a styled error message
pub fn error_text(text: &str) -> String {
    style(text).red().to_string()
}

/// Create a styled warning message
pub fn warning_text(text: &str) -> String {
    style(text).yellow().to_string()
}

/// Create a styled info message
pub fn info_text(text: &str) -> String {
    style(text).blue().to_string()
}

/// Create a dimmed text
pub fn dimmed_text(text: &str) -> String {
    style(text).dim().to_string()
}

/// Print a separator line
pub fn separator() -> String {
    "─".repeat(80)
}

/// Print a box around text
pub fn boxed_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let max_width = lines.iter().map(|line| line.len()).max().unwrap_or(0);
    let width = max_width + 4; // padding
    
    let mut result = String::new();
    result.push_str(&format!("┌{}┐\n", "─".repeat(width - 2)));
    
    for line in lines {
        result.push_str(&format!("│ {:<width$} │\n", line, width = max_width));
    }
    
    result.push_str(&format!("└{}┘", "─".repeat(width - 2)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_duration() {
        let duration = std::time::Duration::from_millis(500);
        assert_eq!(format_duration(duration), "500ms");
        
        let duration = std::time::Duration::from_secs(65);
        assert_eq!(format_duration(duration), "1m 5s");
        
        let duration = std::time::Duration::from_secs(3661);
        assert_eq!(format_duration(duration), "1h 1m 1s");
    }
    
    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
    }
    
    #[test]
    fn test_format_number() {
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }
    
    #[test]
    fn test_styling() {
        let text = "Hello, world!";
        let styled = style_text(text, Color::Green);
        assert!(!styled.is_empty());
        
        let header = header(text);
        assert!(!header.is_empty());
        
        let boxed = boxed_text(text);
        assert!(boxed.contains("┌"));
        assert!(boxed.contains("└"));
    }
}