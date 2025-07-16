use ratatui::{style::Style, text::Span};
use syntect::{
    easy::HighlightLines,
    highlighting::{ThemeSet, Style as SyntectStyle},
    parsing::SyntaxSet,
};

use crate::theme::Theme;

/// Syntax highlighter for code files
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }
    
    /// Highlight a line of code
    pub fn highlight<'a>(&self, line: &'a str, language: &str) -> Vec<Span<'a>> {
        let syntax = match self.syntax_set.find_syntax_by_name(language) {
            Some(syntax) => syntax,
            None => {
                // Try by file extension
                let extension = match language {
                    "rust" => "rs",
                    "javascript" => "js",
                    "typescript" => "ts",
                    "python" => "py",
                    "bash" => "sh",
                    _ => language,
                };
                
                match self.syntax_set.find_syntax_by_extension(extension) {
                    Some(syntax) => syntax,
                    None => {
                        // Fallback to plain text
                        return vec![Span::raw(line)];
                    }
                }
            }
        };
        
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);
        
        match highlighter.highlight_line(line, &self.syntax_set) {
            Ok(ranges) => {
                ranges
                    .into_iter()
                    .map(|(style, text)| {
                        let ratatui_style = self.syntect_to_ratatui_style(style);
                        Span::styled(text, ratatui_style)
                    })
                    .collect()
            }
            Err(_) => vec![Span::raw(line)],
        }
    }
    
    /// Convert syntect style to ratatui style
    fn syntect_to_ratatui_style(&self, style: SyntectStyle) -> Style {
        let fg = style.foreground;
        let color = ratatui::style::Color::Rgb(fg.r, fg.g, fg.b);
        
        let mut ratatui_style = Style::default().fg(color);
        
        if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
            ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::BOLD);
        }
        
        if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
            ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::ITALIC);
        }
        
        if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
            ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::UNDERLINED);
        }
        
        ratatui_style
    }
    
    /// Get list of supported languages
    pub fn supported_languages(&self) -> Vec<String> {
        self.syntax_set
            .syntaxes()
            .iter()
            .map(|syntax| syntax.name.clone())
            .collect()
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new();
        let languages = highlighter.supported_languages();
        assert!(!languages.is_empty());
        assert!(languages.contains(&"Rust".to_string()));
    }

    #[test]
    fn test_basic_highlighting() {
        let highlighter = SyntaxHighlighter::new();
        let spans = highlighter.highlight("fn main() {}", "rust");
        assert!(!spans.is_empty());
    }
}