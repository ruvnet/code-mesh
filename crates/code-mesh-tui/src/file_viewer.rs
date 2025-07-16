use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    style::Style,
    text::{Line, Span},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::Path;

use crate::{
    config::{FileViewerConfig, DiffStyle},
    renderer::Renderer,
    theme::Theme,
    utils::syntax_highlighter::SyntaxHighlighter,
};

/// File viewer component for displaying and editing files
pub struct FileViewer {
    theme: Box<dyn Theme + Send + Sync>,
    config: FileViewerConfig,
    current_file: Option<FileContent>,
    scroll_offset: usize,
    syntax_highlighter: SyntaxHighlighter,
    diff_style: DiffStyle,
    is_visible: bool,
}

/// File content representation
#[derive(Debug, Clone)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub lines: Vec<String>,
    pub language: Option<String>,
    pub is_diff: bool,
    pub file_type: FileType,
}

/// File type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Text,
    Binary,
    Image,
    Archive,
    Unknown,
}

impl FileViewer {
    /// Create a new file viewer
    pub fn new(config: &FileViewerConfig, theme: &dyn Theme) -> Self {
        Self {
            theme: Box::new(crate::theme::DefaultTheme), // Temporary
            config: config.clone(),
            current_file: None,
            scroll_offset: 0,
            syntax_highlighter: SyntaxHighlighter::new(),
            diff_style: config.default_style,
            is_visible: false,
        }
    }
    
    /// Open a file for viewing
    pub async fn open_file(&mut self, path: &str) -> Result<()> {
        let file_content = self.load_file(path).await?;
        self.current_file = Some(file_content);
        self.scroll_offset = 0;
        self.is_visible = true;
        Ok(())
    }
    
    /// Close the current file
    pub fn close_file(&mut self) {
        self.current_file = None;
        self.is_visible = false;
        self.scroll_offset = 0;
    }
    
    /// Check if the file viewer is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
    
    /// Get the current file path
    pub fn current_file_path(&self) -> Option<&str> {
        self.current_file.as_ref().map(|f| f.path.as_str())
    }
    
    /// Toggle diff style between unified and side-by-side
    pub fn toggle_diff_style(&mut self) {
        self.diff_style = match self.diff_style {
            DiffStyle::Unified => DiffStyle::SideBySide,
            DiffStyle::SideBySide => DiffStyle::Unified,
        };
    }
    
    /// Handle key events
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.close_file();
            }
            KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.toggle_diff_style();
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Scroll up
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }
    
    /// Scroll down
    pub fn scroll_down(&mut self) {
        if let Some(ref file) = self.current_file {
            let max_offset = file.lines.len().saturating_sub(1);
            if self.scroll_offset < max_offset {
                self.scroll_offset += 1;
            }
        }
    }
    
    /// Page up
    pub fn page_up(&mut self) {
        let page_size = 20; // Could be based on visible area
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }
    
    /// Page down
    pub fn page_down(&mut self) {
        if let Some(ref file) = self.current_file {
            let page_size = 20;
            let max_offset = file.lines.len().saturating_sub(page_size);
            self.scroll_offset = (self.scroll_offset + page_size).min(max_offset);
        }
    }
    
    /// Update the component
    pub async fn update(&mut self) -> Result<()> {
        // Handle any periodic updates
        Ok(())
    }
    
    /// Update theme
    pub fn update_theme(&mut self, theme: &dyn Theme) {
        // In a real implementation, we'd clone or recreate the theme
        // For now, this is a placeholder
    }
    
    /// Load file content from disk
    async fn load_file(&self, path: &str) -> Result<FileContent> {
        let _path_obj = Path::new(path);
        
        // Check file size
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > self.config.max_file_size as u64 {
            return Err(anyhow::anyhow!("File too large: {} bytes", metadata.len()));
        }
        
        // Determine file type
        let file_type = self.detect_file_type(path);
        
        // Read file content
        let content = match file_type {
            FileType::Binary | FileType::Image | FileType::Archive => {
                format!("Binary file: {} ({} bytes)", path, metadata.len())
            }
            _ => {
                match std::fs::read_to_string(path) {
                    Ok(content) => content,
                    Err(_e) => {
                        // Try reading as binary and show hex dump
                        let binary_data = std::fs::read(path)?;
                        self.format_hex_dump(&binary_data)
                    }
                }
            }
        };
        
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        // Detect language for syntax highlighting
        let language = if self.config.syntax_highlighting {
            self.detect_language(path, &content)
        } else {
            None
        };
        
        // Check if this is a diff file
        let is_diff = content.starts_with("diff --git") || 
                     content.starts_with("--- ") ||
                     path.ends_with(".diff") || 
                     path.ends_with(".patch");
        
        Ok(FileContent {
            path: path.to_string(),
            content,
            lines,
            language,
            is_diff,
            file_type,
        })
    }
    
    /// Detect file type based on extension and content
    fn detect_file_type(&self, path: &str) -> FileType {
        let path = Path::new(path);
        
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension.to_lowercase().as_str() {
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" => FileType::Image,
                "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => FileType::Archive,
                "exe" | "dll" | "so" | "dylib" | "bin" => FileType::Binary,
                _ => FileType::Text,
            }
        } else {
            FileType::Unknown
        }
    }
    
    /// Detect programming language for syntax highlighting
    fn detect_language(&self, path: &str, content: &str) -> Option<String> {
        let path = Path::new(path);
        
        // First try by file extension
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            let language = match extension.to_lowercase().as_str() {
                "rs" => Some("rust"),
                "js" | "mjs" => Some("javascript"),
                "ts" => Some("typescript"),
                "py" => Some("python"),
                "go" => Some("go"),
                "java" => Some("java"),
                "c" => Some("c"),
                "cpp" | "cxx" | "cc" => Some("cpp"),
                "h" | "hpp" => Some("c"),
                "cs" => Some("csharp"),
                "php" => Some("php"),
                "rb" => Some("ruby"),
                "swift" => Some("swift"),
                "kt" => Some("kotlin"),
                "scala" => Some("scala"),
                "r" => Some("r"),
                "sql" => Some("sql"),
                "sh" | "bash" => Some("bash"),
                "ps1" => Some("powershell"),
                "html" | "htm" => Some("html"),
                "css" => Some("css"),
                "scss" | "sass" => Some("scss"),
                "xml" => Some("xml"),
                "json" => Some("json"),
                "yaml" | "yml" => Some("yaml"),
                "toml" => Some("toml"),
                "md" | "markdown" => Some("markdown"),
                "tex" => Some("latex"),
                "vim" => Some("vim"),
                "lua" => Some("lua"),
                "pl" => Some("perl"),
                "clj" | "cljs" => Some("clojure"),
                "hs" => Some("haskell"),
                "ml" => Some("ocaml"),
                "elm" => Some("elm"),
                "ex" | "exs" => Some("elixir"),
                "erl" => Some("erlang"),
                "dart" => Some("dart"),
                _ => None,
            };
            
            if language.is_some() {
                return language.map(|s| s.to_string());
            }
        }
        
        // Try to detect by filename
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            match filename.to_lowercase().as_str() {
                "dockerfile" => return Some("dockerfile".to_string()),
                "makefile" => return Some("makefile".to_string()),
                "rakefile" => return Some("ruby".to_string()),
                "gemfile" => return Some("ruby".to_string()),
                "cargo.toml" => return Some("toml".to_string()),
                "package.json" => return Some("json".to_string()),
                _ => {}
            }
        }
        
        // Try to detect by shebang
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!") {
                if first_line.contains("python") {
                    return Some("python".to_string());
                } else if first_line.contains("bash") || first_line.contains("sh") {
                    return Some("bash".to_string());
                } else if first_line.contains("node") {
                    return Some("javascript".to_string());
                } else if first_line.contains("ruby") {
                    return Some("ruby".to_string());
                } else if first_line.contains("perl") {
                    return Some("perl".to_string());
                }
            }
        }
        
        None
    }
    
    /// Format binary data as hex dump
    fn format_hex_dump(&self, data: &[u8]) -> String {
        let mut result = String::new();
        
        for (i, chunk) in data.chunks(16).enumerate() {
            // Address
            result.push_str(&format!("{:08x}  ", i * 16));
            
            // Hex bytes
            for (j, byte) in chunk.iter().enumerate() {
                if j == 8 {
                    result.push(' ');
                }
                result.push_str(&format!("{:02x} ", byte));
            }
            
            // Padding for incomplete lines
            if chunk.len() < 16 {
                for j in chunk.len()..16 {
                    if j == 8 {
                        result.push(' ');
                    }
                    result.push_str("   ");
                }
            }
            
            // ASCII representation
            result.push_str(" |");
            for byte in chunk {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    result.push(*byte as char);
                } else {
                    result.push('.');
                }
            }
            result.push_str("|\n");
        }
        
        result
    }
    
    /// Render the file viewer
    pub fn render(&mut self, renderer: &Renderer, area: Rect) {
        if !self.is_visible {
            return;
        }
        
        let title = if let Some(ref file) = self.current_file {
            format!("File: {}", file.path)
        } else {
            "No file open".to_string()
        };
        
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border()));
        
        renderer.render_widget(block.clone(), area);
        
        let inner_area = block.inner(area);
        
        if let Some(ref file) = self.current_file {
            if file.is_diff {
                self.render_diff(renderer, inner_area, file);
            } else {
                self.render_text_file(renderer, inner_area, file);
            }
        } else {
            let empty_msg = Paragraph::new("No file open")
                .style(Style::default().fg(self.theme.text_muted()));
            renderer.render_widget(empty_msg, inner_area);
        }
    }
    
    /// Render a text file
    fn render_text_file(&self, renderer: &Renderer, area: Rect, file: &FileContent) {
        let visible_height = area.height as usize;
        let start_line = self.scroll_offset;
        let end_line = (start_line + visible_height).min(file.lines.len());
        
        let visible_lines = &file.lines[start_line..end_line];
        
        let mut lines = Vec::new();
        for (i, line) in visible_lines.iter().enumerate() {
            let line_number = start_line + i + 1;
            
            let formatted_line = if self.config.show_line_numbers {
                let line_num_style = Style::default().fg(self.theme.text_muted());
                let line_content = if self.config.syntax_highlighting && file.language.is_some() {
                    // Apply syntax highlighting
                    self.syntax_highlighter.highlight(line, file.language.as_ref().unwrap())
                } else {
                    vec![Span::styled(line, Style::default().fg(self.theme.text()))]
                };
                
                let mut spans = vec![
                    Span::styled(format!("{:4} ", line_number), line_num_style),
                ];
                spans.extend(line_content);
                Line::from(spans)
            } else {
                if self.config.syntax_highlighting && file.language.is_some() {
                    let highlighted = self.syntax_highlighter.highlight(line, file.language.as_ref().unwrap());
                    Line::from(highlighted)
                } else {
                    Line::from(Span::styled(line, Style::default().fg(self.theme.text())))
                }
            };
            
            lines.push(formatted_line);
        }
        
        let paragraph = Paragraph::new(lines)
            .style(Style::default().bg(self.theme.background()));
        
        renderer.render_widget(paragraph, area);
        
        // Render scrollbar if needed
        if file.lines.len() > visible_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(file.lines.len())
                .position(self.scroll_offset);
            
            // Note: scrollbar rendering would need proper state management
            // renderer.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }
    
    /// Render a diff file
    fn render_diff(&self, renderer: &Renderer, area: Rect, file: &FileContent) {
        // This is a simplified diff renderer
        // A full implementation would parse the diff and show changes
        let lines: Vec<Line> = file.lines
            .iter()
            .skip(self.scroll_offset)
            .take(area.height as usize)
            .map(|line| {
                let style = if line.starts_with('+') {
                    Style::default()
                        .fg(self.theme.diff_added())
                        .bg(self.theme.background_element())
                } else if line.starts_with('-') {
                    Style::default()
                        .fg(self.theme.diff_removed())
                        .bg(self.theme.background_element())
                } else if line.starts_with("@@") {
                    Style::default()
                        .fg(self.theme.diff_context())
                        .bg(self.theme.background_panel())
                } else {
                    Style::default().fg(self.theme.text())
                };
                
                Line::from(Span::styled(line, style))
            })
            .collect();
        
        let paragraph = Paragraph::new(lines)
            .style(Style::default().bg(self.theme.background()));
        
        renderer.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FileViewerConfig;
    
    #[test]
    fn test_file_type_detection() {
        let config = FileViewerConfig::default();
        let theme = crate::theme::DefaultTheme;
        let viewer = FileViewer::new(&config, &theme);
        
        assert_eq!(viewer.detect_file_type("test.rs"), FileType::Text);
        assert_eq!(viewer.detect_file_type("image.jpg"), FileType::Image);
        assert_eq!(viewer.detect_file_type("archive.zip"), FileType::Archive);
        assert_eq!(viewer.detect_file_type("binary.exe"), FileType::Binary);
    }
    
    #[test]
    fn test_language_detection() {
        let config = FileViewerConfig::default();
        let theme = crate::theme::DefaultTheme;
        let viewer = FileViewer::new(&config, &theme);
        
        assert_eq!(viewer.detect_language("test.rs", ""), Some("rust".to_string()));
        assert_eq!(viewer.detect_language("script.py", "#!/usr/bin/env python"), Some("python".to_string()));
        assert_eq!(viewer.detect_language("Dockerfile", "FROM ubuntu"), Some("dockerfile".to_string()));
    }
}