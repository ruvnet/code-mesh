use anyhow::Result;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    style::Style,
    text::{Line, Span},
};

use crate::{
    renderer::Renderer,
    theme::Theme,
    config::DiffStyle,
};

/// Diff viewer component for showing file differences
pub struct DiffViewer {
    theme: Box<dyn Theme + Send + Sync>,
    current_diff: Option<DiffContent>,
    style: DiffStyle,
    scroll_offset: usize,
}

/// Diff content representation
#[derive(Debug, Clone)]
pub struct DiffContent {
    pub original_file: String,
    pub modified_file: String,
    pub hunks: Vec<DiffHunk>,
}

/// A diff hunk representing a section of changes
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub original_start: usize,
    pub original_count: usize,
    pub modified_start: usize,
    pub modified_count: usize,
    pub lines: Vec<DiffLine>,
}

/// A line in a diff
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub original_line_number: Option<usize>,
    pub modified_line_number: Option<usize>,
}

/// Type of diff line
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineType {
    Context,  // Unchanged line
    Added,    // Added line (+)
    Removed,  // Removed line (-)
    Header,   // Diff header
}

impl DiffViewer {
    /// Create a new diff viewer
    pub fn new(theme: &dyn Theme) -> Self {
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            current_diff: None,
            style: DiffStyle::SideBySide,
            scroll_offset: 0,
        }
    }
    
    /// Load diff content from a string
    pub fn load_diff(&mut self, diff_text: &str) -> Result<()> {
        let diff_content = self.parse_diff(diff_text)?;
        self.current_diff = Some(diff_content);
        self.scroll_offset = 0;
        Ok(())
    }
    
    /// Set the diff style
    pub fn set_style(&mut self, style: DiffStyle) {
        self.style = style;
    }
    
    /// Toggle between unified and side-by-side view
    pub fn toggle_style(&mut self) {
        self.style = match self.style {
            DiffStyle::Unified => DiffStyle::SideBySide,
            DiffStyle::SideBySide => DiffStyle::Unified,
        };
    }
    
    /// Scroll up
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }
    
    /// Scroll down
    pub fn scroll_down(&mut self) {
        if let Some(ref diff) = self.current_diff {
            let total_lines = diff.hunks.iter().map(|h| h.lines.len()).sum::<usize>();
            if self.scroll_offset < total_lines.saturating_sub(1) {
                self.scroll_offset += 1;
            }
        }
    }
    
    /// Parse unified diff format
    fn parse_diff(&self, diff_text: &str) -> Result<DiffContent> {
        let mut hunks = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        let mut original_file = String::new();
        let mut modified_file = String::new();
        
        for line in diff_text.lines() {
            if line.starts_with("--- ") {
                original_file = line[4..].to_string();
            } else if line.starts_with("+++ ") {
                modified_file = line[4..].to_string();
            } else if line.starts_with("@@ ") {
                // Save previous hunk if it exists
                if let Some(hunk) = current_hunk.take() {
                    hunks.push(hunk);
                }
                
                // Parse hunk header
                let hunk = self.parse_hunk_header(line)?;
                current_hunk = Some(hunk);
            } else if let Some(ref mut hunk) = current_hunk {
                // Parse diff line
                let diff_line = self.parse_diff_line(line, &hunk.lines)?;
                hunk.lines.push(diff_line);
            }
        }
        
        // Add the last hunk
        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }
        
        Ok(DiffContent {
            original_file,
            modified_file,
            hunks,
        })
    }
    
    /// Parse a hunk header line (e.g., "@@ -1,4 +1,6 @@")
    fn parse_hunk_header(&self, line: &str) -> Result<DiffHunk> {
        // Simple regex-like parsing for "@@ -start,count +start,count @@"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid hunk header: {}", line));
        }
        
        let original_part = parts[1];
        let modified_part = parts[2];
        
        let (original_start, original_count) = self.parse_range(original_part)?;
        let (modified_start, modified_count) = self.parse_range(modified_part)?;
        
        Ok(DiffHunk {
            original_start,
            original_count,
            modified_start,
            modified_count,
            lines: Vec::new(),
        })
    }
    
    /// Parse a range like "-1,4" or "+1,6"
    fn parse_range(&self, range: &str) -> Result<(usize, usize)> {
        let range = &range[1..]; // Remove +/- prefix
        
        if let Some(comma_pos) = range.find(',') {
            let start = range[..comma_pos].parse::<usize>()?;
            let count = range[comma_pos + 1..].parse::<usize>()?;
            Ok((start, count))
        } else {
            let start = range.parse::<usize>()?;
            Ok((start, 1))
        }
    }
    
    /// Parse a diff line
    fn parse_diff_line(&self, line: &str, existing_lines: &[DiffLine]) -> Result<DiffLine> {
        if line.is_empty() {
            return Ok(DiffLine {
                line_type: DiffLineType::Context,
                content: String::new(),
                original_line_number: None,
                modified_line_number: None,
            });
        }
        
        let line_type = match line.chars().next().unwrap_or(' ') {
            '+' => DiffLineType::Added,
            '-' => DiffLineType::Removed,
            ' ' => DiffLineType::Context,
            _ => DiffLineType::Header,
        };
        
        let content = if line.len() > 1 {
            line[1..].to_string()
        } else {
            String::new()
        };
        
        // Calculate line numbers (simplified)
        let (original_line_number, modified_line_number) = match line_type {
            DiffLineType::Added => (None, Some(existing_lines.len() + 1)),
            DiffLineType::Removed => (Some(existing_lines.len() + 1), None),
            DiffLineType::Context => (Some(existing_lines.len() + 1), Some(existing_lines.len() + 1)),
            DiffLineType::Header => (None, None),
        };
        
        Ok(DiffLine {
            line_type,
            content,
            original_line_number,
            modified_line_number,
        })
    }
    
    /// Render the diff viewer
    pub fn render(&self, renderer: &Renderer, area: Rect) {
        let title = if let Some(ref diff) = self.current_diff {
            format!("Diff: {} → {}", diff.original_file, diff.modified_file)
        } else {
            "No diff loaded".to_string()
        };
        
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border()));
        
        renderer.render_widget(block.clone(), area);
        
        let inner_area = block.inner(area);
        
        if let Some(ref diff) = self.current_diff {
            match self.style {
                DiffStyle::Unified => self.render_unified(renderer, inner_area, diff),
                DiffStyle::SideBySide => self.render_side_by_side(renderer, inner_area, diff),
            }
        } else {
            let empty_msg = Paragraph::new("No diff loaded")
                .style(Style::default().fg(self.theme.text_muted()));
            renderer.render_widget(empty_msg, inner_area);
        }
    }
    
    /// Render unified diff view
    fn render_unified(&self, renderer: &Renderer, area: Rect, diff: &DiffContent) {
        let mut lines = Vec::new();
        
        // Collect all lines from all hunks
        let mut all_lines = Vec::new();
        for hunk in &diff.hunks {
            for line in &hunk.lines {
                all_lines.push(line);
            }
        }
        
        // Show visible portion
        let visible_height = area.height as usize;
        let start_line = self.scroll_offset;
        let end_line = (start_line + visible_height).min(all_lines.len());
        let visible_lines = &all_lines[start_line..end_line];
        
        for line in visible_lines {
            let (prefix, style) = match line.line_type {
                DiffLineType::Added => ("+", Style::default().fg(self.theme.diff_added())),
                DiffLineType::Removed => ("-", Style::default().fg(self.theme.diff_removed())),
                DiffLineType::Context => (" ", Style::default().fg(self.theme.text())),
                DiffLineType::Header => ("@", Style::default().fg(self.theme.accent())),
            };
            
            let formatted_line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&line.content, style),
            ]);
            
            lines.push(formatted_line);
        }
        
        let paragraph = Paragraph::new(lines)
            .style(Style::default().bg(self.theme.background()));
        
        renderer.render_widget(paragraph, area);
    }
    
    /// Render side-by-side diff view
    fn render_side_by_side(&self, renderer: &Renderer, area: Rect, diff: &DiffContent) {
        // Split area into left and right halves
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(area);
        
        // Render original file (left side)
        self.render_side(renderer, chunks[0], diff, true);
        
        // Render modified file (right side)
        self.render_side(renderer, chunks[1], diff, false);
    }
    
    /// Render one side of the side-by-side view
    fn render_side(&self, renderer: &Renderer, area: Rect, diff: &DiffContent, is_original: bool) {
        let title = if is_original {
            &diff.original_file
        } else {
            &diff.modified_file
        };
        
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border()));
        
        renderer.render_widget(block.clone(), area);
        
        let inner_area = block.inner(area);
        
        let mut lines = Vec::new();
        
        // Collect relevant lines for this side
        let mut all_lines = Vec::new();
        for hunk in &diff.hunks {
            for line in &hunk.lines {
                match (&line.line_type, is_original) {
                    (DiffLineType::Context, _) => all_lines.push(line),
                    (DiffLineType::Added, false) => all_lines.push(line),
                    (DiffLineType::Removed, true) => all_lines.push(line),
                    _ => {
                        // Add empty line to maintain alignment
                        if !is_original && line.line_type == DiffLineType::Removed {
                            // Skip removed lines in modified view
                        } else if is_original && line.line_type == DiffLineType::Added {
                            // Skip added lines in original view
                        }
                    }
                }
            }
        }
        
        // Show visible portion
        let visible_height = inner_area.height as usize;
        let start_line = self.scroll_offset;
        let end_line = (start_line + visible_height).min(all_lines.len());
        let visible_lines = &all_lines[start_line..end_line];
        
        for line in visible_lines {
            let style = match line.line_type {
                DiffLineType::Added => Style::default().fg(self.theme.diff_added()),
                DiffLineType::Removed => Style::default().fg(self.theme.diff_removed()),
                DiffLineType::Context => Style::default().fg(self.theme.text()),
                DiffLineType::Header => Style::default().fg(self.theme.accent()),
            };
            
            let line_number = if is_original {
                line.original_line_number
            } else {
                line.modified_line_number
            };
            
            let formatted_line = if let Some(num) = line_number {
                Line::from(vec![
                    Span::styled(format!("{:4} ", num), Style::default().fg(self.theme.text_muted())),
                    Span::styled(&line.content, style),
                ])
            } else {
                Line::from(Span::styled(&line.content, style))
            };
            
            lines.push(formatted_line);
        }
        
        let paragraph = Paragraph::new(lines)
            .style(Style::default().bg(self.theme.background()));
        
        renderer.render_widget(paragraph, inner_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_diff_parsing() {
        let diff_text = r#"--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,4 @@
 line 1
-line 2
+line 2 modified
+line 2.5 added
 line 3"#;
        
        let theme = crate::theme::DefaultTheme;
        let mut viewer = DiffViewer::new(&theme);
        let result = viewer.load_diff(diff_text);
        assert!(result.is_ok());
        
        let diff = viewer.current_diff.unwrap();
        assert_eq!(diff.original_file, "a/file.txt");
        assert_eq!(diff.modified_file, "b/file.txt");
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].lines.len(), 4);
    }
    
    #[test]
    fn test_range_parsing() {
        let theme = crate::theme::DefaultTheme;
        let viewer = DiffViewer::new(&theme);
        
        assert_eq!(viewer.parse_range("-1,3").unwrap(), (1, 3));
        assert_eq!(viewer.parse_range("+5,2").unwrap(), (5, 2));
        assert_eq!(viewer.parse_range("-10").unwrap(), (10, 1));
    }
}