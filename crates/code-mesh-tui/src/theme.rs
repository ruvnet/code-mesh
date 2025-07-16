use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Theme trait for defining color schemes
pub trait Theme {
    fn name(&self) -> &str;
    
    // Background colors
    fn background(&self) -> Color;
    fn background_panel(&self) -> Color;
    fn background_element(&self) -> Color;
    
    // Text colors
    fn text(&self) -> Color;
    fn text_muted(&self) -> Color;
    fn text_secondary(&self) -> Color;
    
    // Brand colors
    fn primary(&self) -> Color;
    fn secondary(&self) -> Color;
    fn accent(&self) -> Color;
    
    // Status colors
    fn success(&self) -> Color;
    fn warning(&self) -> Color;
    fn error(&self) -> Color;
    fn info(&self) -> Color;
    
    // Border colors
    fn border(&self) -> Color;
    fn border_active(&self) -> Color;
    fn border_subtle(&self) -> Color;
    
    // Diff colors
    fn diff_added(&self) -> Color;
    fn diff_removed(&self) -> Color;
    fn diff_modified(&self) -> Color;
    fn diff_context(&self) -> Color;
    
    // Syntax highlighting colors
    fn syntax_keyword(&self) -> Color;
    fn syntax_string(&self) -> Color;
    fn syntax_comment(&self) -> Color;
    fn syntax_number(&self) -> Color;
    fn syntax_type(&self) -> Color;
    fn syntax_function(&self) -> Color;
    fn syntax_variable(&self) -> Color;
    fn syntax_operator(&self) -> Color;
}

/// Default theme with dark colors
#[derive(Debug, Clone)]
pub struct DefaultTheme;

impl Theme for DefaultTheme {
    fn name(&self) -> &str { "default" }
    
    fn background(&self) -> Color { Color::Rgb(16, 16, 16) }
    fn background_panel(&self) -> Color { Color::Rgb(24, 24, 24) }
    fn background_element(&self) -> Color { Color::Rgb(32, 32, 32) }
    
    fn text(&self) -> Color { Color::Rgb(240, 240, 240) }
    fn text_muted(&self) -> Color { Color::Rgb(160, 160, 160) }
    fn text_secondary(&self) -> Color { Color::Rgb(200, 200, 200) }
    
    fn primary(&self) -> Color { Color::Rgb(99, 179, 237) }
    fn secondary(&self) -> Color { Color::Rgb(158, 134, 200) }
    fn accent(&self) -> Color { Color::Rgb(255, 184, 108) }
    
    fn success(&self) -> Color { Color::Rgb(152, 195, 121) }
    fn warning(&self) -> Color { Color::Rgb(229, 192, 123) }
    fn error(&self) -> Color { Color::Rgb(224, 108, 117) }
    fn info(&self) -> Color { Color::Rgb(97, 175, 239) }
    
    fn border(&self) -> Color { Color::Rgb(64, 64, 64) }
    fn border_active(&self) -> Color { Color::Rgb(99, 179, 237) }
    fn border_subtle(&self) -> Color { Color::Rgb(48, 48, 48) }
    
    fn diff_added(&self) -> Color { Color::Rgb(152, 195, 121) }
    fn diff_removed(&self) -> Color { Color::Rgb(224, 108, 117) }
    fn diff_modified(&self) -> Color { Color::Rgb(229, 192, 123) }
    fn diff_context(&self) -> Color { Color::Rgb(160, 160, 160) }
    
    fn syntax_keyword(&self) -> Color { Color::Rgb(198, 120, 221) }
    fn syntax_string(&self) -> Color { Color::Rgb(152, 195, 121) }
    fn syntax_comment(&self) -> Color { Color::Rgb(92, 99, 112) }
    fn syntax_number(&self) -> Color { Color::Rgb(209, 154, 102) }
    fn syntax_type(&self) -> Color { Color::Rgb(86, 182, 194) }
    fn syntax_function(&self) -> Color { Color::Rgb(97, 175, 239) }
    fn syntax_variable(&self) -> Color { Color::Rgb(224, 108, 117) }
    fn syntax_operator(&self) -> Color { Color::Rgb(86, 182, 194) }
}

/// Gruvbox dark theme
#[derive(Debug, Clone)]
pub struct GruvboxTheme;

impl Theme for GruvboxTheme {
    fn name(&self) -> &str { "gruvbox" }
    
    fn background(&self) -> Color { Color::Rgb(40, 40, 40) }
    fn background_panel(&self) -> Color { Color::Rgb(50, 48, 47) }
    fn background_element(&self) -> Color { Color::Rgb(60, 56, 54) }
    
    fn text(&self) -> Color { Color::Rgb(235, 219, 178) }
    fn text_muted(&self) -> Color { Color::Rgb(168, 153, 132) }
    fn text_secondary(&self) -> Color { Color::Rgb(213, 196, 161) }
    
    fn primary(&self) -> Color { Color::Rgb(131, 165, 152) }
    fn secondary(&self) -> Color { Color::Rgb(211, 134, 155) }
    fn accent(&self) -> Color { Color::Rgb(250, 189, 47) }
    
    fn success(&self) -> Color { Color::Rgb(184, 187, 38) }
    fn warning(&self) -> Color { Color::Rgb(250, 189, 47) }
    fn error(&self) -> Color { Color::Rgb(204, 36, 29) }
    fn info(&self) -> Color { Color::Rgb(69, 133, 136) }
    
    fn border(&self) -> Color { Color::Rgb(80, 73, 69) }
    fn border_active(&self) -> Color { Color::Rgb(131, 165, 152) }
    fn border_subtle(&self) -> Color { Color::Rgb(60, 56, 54) }
    
    fn diff_added(&self) -> Color { Color::Rgb(184, 187, 38) }
    fn diff_removed(&self) -> Color { Color::Rgb(204, 36, 29) }
    fn diff_modified(&self) -> Color { Color::Rgb(250, 189, 47) }
    fn diff_context(&self) -> Color { Color::Rgb(168, 153, 132) }
    
    fn syntax_keyword(&self) -> Color { Color::Rgb(251, 73, 52) }
    fn syntax_string(&self) -> Color { Color::Rgb(184, 187, 38) }
    fn syntax_comment(&self) -> Color { Color::Rgb(146, 131, 116) }
    fn syntax_number(&self) -> Color { Color::Rgb(211, 134, 155) }
    fn syntax_type(&self) -> Color { Color::Rgb(250, 189, 47) }
    fn syntax_function(&self) -> Color { Color::Rgb(131, 165, 152) }
    fn syntax_variable(&self) -> Color { Color::Rgb(69, 133, 136) }
    fn syntax_operator(&self) -> Color { Color::Rgb(254, 128, 25) }
}

/// Dracula theme
#[derive(Debug, Clone)]
pub struct DraculaTheme;

impl Theme for DraculaTheme {
    fn name(&self) -> &str { "dracula" }
    
    fn background(&self) -> Color { Color::Rgb(40, 42, 54) }
    fn background_panel(&self) -> Color { Color::Rgb(68, 71, 90) }
    fn background_element(&self) -> Color { Color::Rgb(98, 114, 164) }
    
    fn text(&self) -> Color { Color::Rgb(248, 248, 242) }
    fn text_muted(&self) -> Color { Color::Rgb(98, 114, 164) }
    fn text_secondary(&self) -> Color { Color::Rgb(189, 147, 249) }
    
    fn primary(&self) -> Color { Color::Rgb(139, 233, 253) }
    fn secondary(&self) -> Color { Color::Rgb(189, 147, 249) }
    fn accent(&self) -> Color { Color::Rgb(241, 250, 140) }
    
    fn success(&self) -> Color { Color::Rgb(80, 250, 123) }
    fn warning(&self) -> Color { Color::Rgb(241, 250, 140) }
    fn error(&self) -> Color { Color::Rgb(255, 85, 85) }
    fn info(&self) -> Color { Color::Rgb(139, 233, 253) }
    
    fn border(&self) -> Color { Color::Rgb(68, 71, 90) }
    fn border_active(&self) -> Color { Color::Rgb(139, 233, 253) }
    fn border_subtle(&self) -> Color { Color::Rgb(98, 114, 164) }
    
    fn diff_added(&self) -> Color { Color::Rgb(80, 250, 123) }
    fn diff_removed(&self) -> Color { Color::Rgb(255, 85, 85) }
    fn diff_modified(&self) -> Color { Color::Rgb(241, 250, 140) }
    fn diff_context(&self) -> Color { Color::Rgb(98, 114, 164) }
    
    fn syntax_keyword(&self) -> Color { Color::Rgb(255, 121, 198) }
    fn syntax_string(&self) -> Color { Color::Rgb(241, 250, 140) }
    fn syntax_comment(&self) -> Color { Color::Rgb(98, 114, 164) }
    fn syntax_number(&self) -> Color { Color::Rgb(189, 147, 249) }
    fn syntax_type(&self) -> Color { Color::Rgb(139, 233, 253) }
    fn syntax_function(&self) -> Color { Color::Rgb(80, 250, 123) }
    fn syntax_variable(&self) -> Color { Color::Rgb(255, 184, 108) }
    fn syntax_operator(&self) -> Color { Color::Rgb(255, 121, 198) }
}

/// Theme manager for handling multiple themes
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current_theme: Box<dyn Theme + Send + Sync>,
    available_themes: HashMap<String, Box<dyn Theme + Send + Sync>>,
}

impl Default for ThemeManager {
    fn default() -> Self {
        let mut manager = Self {
            current_theme: Box::new(DefaultTheme),
            available_themes: HashMap::new(),
        };
        
        // Register built-in themes
        manager.register_theme(Box::new(DefaultTheme));
        manager.register_theme(Box::new(GruvboxTheme));
        manager.register_theme(Box::new(DraculaTheme));
        
        manager
    }
}

impl ThemeManager {
    /// Register a new theme
    pub fn register_theme(&mut self, theme: Box<dyn Theme + Send + Sync>) {
        let name = theme.name().to_string();
        self.available_themes.insert(name, theme);
    }
    
    /// Set the current theme by name
    pub fn set_theme(&mut self, name: &str) -> anyhow::Result<()> {
        if let Some(theme) = self.available_themes.get(name) {
            // Clone the theme (this requires implementing Clone for the theme)
            // For now, we'll recreate the theme based on name
            match name {
                "default" => self.current_theme = Box::new(DefaultTheme),
                "gruvbox" => self.current_theme = Box::new(GruvboxTheme),
                "dracula" => self.current_theme = Box::new(DraculaTheme),
                _ => return Err(anyhow::anyhow!("Unknown theme: {}", name)),
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Theme not found: {}", name))
        }
    }
    
    /// Get the current theme
    pub fn current_theme(&self) -> &dyn Theme {
        self.current_theme.as_ref()
    }
    
    /// Get list of available theme names
    pub fn available_themes(&self) -> Vec<String> {
        self.available_themes.keys().cloned().collect()
    }
}

/// Helper functions for creating styled components
pub struct Styles;

impl Styles {
    /// Create a style with the current theme's primary color
    pub fn primary(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.primary())
    }
    
    /// Create a style with the current theme's secondary color
    pub fn secondary(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.secondary())
    }
    
    /// Create a style with the current theme's accent color
    pub fn accent(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.accent())
    }
    
    /// Create a style for success messages
    pub fn success(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.success())
    }
    
    /// Create a style for warning messages
    pub fn warning(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.warning())
    }
    
    /// Create a style for error messages
    pub fn error(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.error())
    }
    
    /// Create a style for info messages
    pub fn info(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.info())
    }
    
    /// Create a style for muted text
    pub fn muted(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.text_muted())
    }
    
    /// Create a style for borders
    pub fn border(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.border())
    }
    
    /// Create a style for active borders
    pub fn border_active(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.border_active())
    }
    
    /// Create a bold style
    pub fn bold(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)
    }
    
    /// Create an italic style
    pub fn italic(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.text()).add_modifier(Modifier::ITALIC)
    }
    
    /// Create an underlined style
    pub fn underline(theme: &dyn Theme) -> Style {
        Style::default().fg(theme.text()).add_modifier(Modifier::UNDERLINED)
    }
}

/// Theme-aware color definitions for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub background_panel: String,
    pub background_element: String,
    pub text: String,
    pub text_muted: String,
    pub text_secondary: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,
    pub border: String,
    pub border_active: String,
    pub border_subtle: String,
    pub diff_added: String,
    pub diff_removed: String,
    pub diff_modified: String,
    pub diff_context: String,
}

impl ThemeColors {
    /// Convert hex color string to ratatui Color
    pub fn parse_color(hex: &str) -> anyhow::Result<Color> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(anyhow::anyhow!("Invalid hex color: {}", hex));
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;
        
        Ok(Color::Rgb(r, g, b))
    }
    
    /// Convert ratatui Color to hex string
    pub fn color_to_hex(color: Color) -> String {
        match color {
            Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
            _ => "#000000".to_string(), // Fallback for non-RGB colors
        }
    }
}