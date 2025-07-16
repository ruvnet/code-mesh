use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the TUI application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Theme configuration
    pub theme: ThemeConfig,
    /// Keybinding configuration
    pub keybinds: KeybindConfig,
    /// File viewer configuration
    pub file_viewer: FileViewerConfig,
    /// Chat configuration
    pub chat: ChatConfig,
    /// Diff viewer configuration
    pub diff: DiffConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            keybinds: KeybindConfig::default(),
            file_viewer: FileViewerConfig::default(),
            chat: ChatConfig::default(),
            diff: DiffConfig::default(),
        }
    }
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Current theme name
    pub name: String,
    /// Custom theme overrides
    pub custom: Option<CustomTheme>,
    /// Enable high contrast mode
    pub high_contrast: bool,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            custom: None,
            high_contrast: false,
        }
    }
}

/// Custom theme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub colors: HashMap<String, String>,
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindConfig {
    /// Leader key for command sequences
    pub leader: String,
    /// Custom keybindings
    pub bindings: HashMap<String, String>,
}

impl Default for KeybindConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        
        // Navigation
        bindings.insert("quit".to_string(), "q".to_string());
        bindings.insert("help".to_string(), "?".to_string());
        bindings.insert("command_palette".to_string(), "ctrl+p".to_string());
        
        // Chat
        bindings.insert("send_message".to_string(), "enter".to_string());
        bindings.insert("new_line".to_string(), "shift+enter".to_string());
        bindings.insert("clear_input".to_string(), "ctrl+l".to_string());
        
        // File viewer
        bindings.insert("open_file".to_string(), "o".to_string());
        bindings.insert("close_file".to_string(), "esc".to_string());
        bindings.insert("toggle_diff".to_string(), "d".to_string());
        
        // Navigation
        bindings.insert("scroll_up".to_string(), "k".to_string());
        bindings.insert("scroll_down".to_string(), "j".to_string());
        bindings.insert("page_up".to_string(), "ctrl+u".to_string());
        bindings.insert("page_down".to_string(), "ctrl+d".to_string());
        
        Self {
            leader: "ctrl+x".to_string(),
            bindings,
        }
    }
}

/// File viewer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileViewerConfig {
    /// Maximum file size to display (in bytes)
    pub max_file_size: usize,
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    /// Line number display
    pub show_line_numbers: bool,
    /// Word wrapping
    pub word_wrap: bool,
}

impl Default for FileViewerConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            syntax_highlighting: true,
            show_line_numbers: true,
            word_wrap: false,
        }
    }
}

/// Chat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Maximum number of messages to keep in memory
    pub max_messages: usize,
    /// Enable message history
    pub enable_history: bool,
    /// Auto-scroll to new messages
    pub auto_scroll: bool,
    /// Show typing indicators
    pub show_typing: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_messages: 1000,
            enable_history: true,
            auto_scroll: true,
            show_typing: true,
        }
    }
}

/// Diff viewer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffConfig {
    /// Default diff style (unified or side-by-side)
    pub default_style: DiffStyle,
    /// Enable intra-line highlighting
    pub intra_line_highlighting: bool,
    /// Context lines for unified diff
    pub context_lines: usize,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            default_style: DiffStyle::SideBySide,
            intra_line_highlighting: true,
            context_lines: 3,
        }
    }
}

/// Diff display style
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DiffStyle {
    Unified,
    SideBySide,
}

impl Config {
    /// Load configuration from file
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the configuration directory
    pub fn config_dir() -> anyhow::Result<std::path::PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("code-mesh");
        
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
        }
        
        Ok(config_dir)
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> anyhow::Result<std::path::PathBuf> {
        Ok(Self::config_dir()?.join("tui.json"))
    }

    /// Load configuration with fallback to defaults
    pub fn load_or_default() -> Self {
        match Self::default_config_path() {
            Ok(path) => {
                if path.exists() {
                    match Self::load_from_file(&path) {
                        Ok(config) => return config,
                        Err(e) => {
                            eprintln!("Failed to load config from {}: {}", path.display(), e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get config path: {}", e);
            }
        }
        Self::default()
    }
}