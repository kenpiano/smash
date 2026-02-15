use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How line endings are handled.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEndingSetting {
    /// Detect from file content.
    #[default]
    Auto,
    /// Unix-style LF.
    Lf,
    /// Windows-style CR+LF.
    CrLf,
}

/// How line numbers are displayed in the gutter.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineNumberMode {
    /// Show absolute line numbers.
    #[default]
    Absolute,
    /// Show relative line numbers from cursor.
    Relative,
    /// Hide line numbers.
    None,
}

/// Log verbosity level.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    /// Most verbose.
    Trace,
    /// Debug messages.
    Debug,
    /// Informational messages (default).
    #[default]
    Info,
    /// Warnings only.
    Warn,
    /// Errors only.
    Error,
}

/// Key-mapping configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeymapConfig {
    /// Name of the built-in keymap preset.
    #[serde(default = "default_keymap_name")]
    pub preset: String,
}

fn default_keymap_name() -> String {
    "default".to_string()
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            preset: default_keymap_name(),
        }
    }
}

/// Editor behaviour settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Number of spaces per tab stop (1–16).
    #[serde(default = "default_tab_size")]
    pub tab_size: u8,
    /// Insert spaces instead of tab characters.
    #[serde(default = "default_true")]
    pub insert_spaces: bool,
    /// Line ending style.
    #[serde(default)]
    pub line_ending: LineEndingSetting,
    /// Enable soft word-wrap.
    #[serde(default)]
    pub word_wrap: bool,
    /// Automatically indent new lines.
    #[serde(default = "default_true")]
    pub auto_indent: bool,
    /// Automatically close brackets and quotes.
    #[serde(default = "default_true")]
    pub auto_close_brackets: bool,
    /// Remove trailing whitespace on save.
    #[serde(default)]
    pub trim_trailing_whitespace: bool,
    /// Treat the macOS Option key as Alt for keybindings.
    /// When true, Unicode characters produced by Option+key on macOS
    /// (e.g. Option+f → ƒ) are normalised back to Alt+key.
    /// Defaults to `true` on macOS, `false` elsewhere.
    #[serde(default = "default_option_as_alt")]
    pub option_as_alt: bool,
}

fn default_tab_size() -> u8 {
    4
}
fn default_true() -> bool {
    true
}

fn default_option_as_alt() -> bool {
    cfg!(target_os = "macos")
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            line_ending: LineEndingSetting::Auto,
            word_wrap: false,
            auto_indent: true,
            auto_close_brackets: true,
            trim_trailing_whitespace: false,
            option_as_alt: default_option_as_alt(),
        }
    }
}

/// Display / UI settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Name of the colour theme.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Line-number display mode.
    #[serde(default)]
    pub line_numbers: LineNumberMode,
    /// Show minimap panel.
    #[serde(default)]
    pub show_minimap: bool,
    /// Blink the cursor.
    #[serde(default = "default_true")]
    pub cursor_blink: bool,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            line_numbers: LineNumberMode::Absolute,
            show_minimap: false,
            cursor_blink: true,
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log verbosity level.
    #[serde(default)]
    pub level: LogLevel,
    /// Optional path to a log file.
    pub file: Option<PathBuf>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            file: None,
        }
    }
}

/// Configuration for a single LSP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LspServerEntry {
    /// The command to run the server.
    pub command: String,
    /// Command-line arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// File extensions this server handles.
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// LSP configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LspConfig {
    /// Whether LSP is enabled globally.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Per-language server configurations, keyed by language ID.
    #[serde(default)]
    pub servers: HashMap<String, LspServerEntry>,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            servers: HashMap::new(),
        }
    }
}

/// Top-level SMASH configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Editor behaviour.
    #[serde(default)]
    pub editor: EditorConfig,
    /// Display / UI settings.
    #[serde(default)]
    pub display: DisplayConfig,
    /// Keymap settings.
    #[serde(default)]
    pub keymap: KeymapConfig,
    /// Override the terminal shell executable.
    #[serde(default)]
    pub terminal_shell: Option<String>,
    /// Logging settings.
    #[serde(default)]
    pub log: LogConfig,
    /// LSP configuration.
    #[serde(default)]
    pub lsp: LspConfig,
    /// Auto-save interval in seconds (0 = disabled, minimum 5).
    #[serde(default = "default_auto_save")]
    pub auto_save_interval_secs: u64,
}

fn default_auto_save() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: EditorConfig::default(),
            display: DisplayConfig::default(),
            keymap: KeymapConfig::default(),
            terminal_shell: None,
            log: LogConfig::default(),
            lsp: LspConfig::default(),
            auto_save_interval_secs: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = Config::default();
        assert_eq!(cfg.editor.tab_size, 4);
        assert!(cfg.editor.insert_spaces);
        assert_eq!(cfg.editor.line_ending, LineEndingSetting::Auto);
        assert!(!cfg.editor.word_wrap);
        assert!(cfg.editor.auto_indent);
        assert!(cfg.editor.auto_close_brackets);
        assert!(!cfg.editor.trim_trailing_whitespace);
        assert_eq!(cfg.editor.option_as_alt, cfg!(target_os = "macos"));
        assert_eq!(cfg.display.theme, "dark");
        assert_eq!(cfg.display.line_numbers, LineNumberMode::Absolute,);
        assert!(!cfg.display.show_minimap);
        assert!(cfg.display.cursor_blink);
        assert_eq!(cfg.keymap.preset, "default");
        assert!(cfg.terminal_shell.is_none());
        assert_eq!(cfg.log.level, LogLevel::Info);
        assert!(cfg.log.file.is_none());
        assert_eq!(cfg.auto_save_interval_secs, 30);
    }

    #[test]
    fn serde_roundtrip_preserves_values() {
        let cfg = Config {
            editor: EditorConfig {
                tab_size: 2,
                insert_spaces: false,
                line_ending: LineEndingSetting::Lf,
                word_wrap: true,
                auto_indent: false,
                auto_close_brackets: false,
                trim_trailing_whitespace: true,
                option_as_alt: true,
            },
            display: DisplayConfig {
                theme: "light".into(),
                line_numbers: LineNumberMode::Relative,
                show_minimap: true,
                cursor_blink: false,
            },
            keymap: KeymapConfig {
                preset: "emacs".into(),
            },
            terminal_shell: Some("/bin/bash".into()),
            log: LogConfig {
                level: LogLevel::Debug,
                file: Some(PathBuf::from("/tmp/smash.log")),
            },
            lsp: LspConfig {
                enabled: false,
                servers: HashMap::new(),
            },
            auto_save_interval_secs: 60,
        };

        let toml_str = toml::to_string(&cfg).expect("serialize");
        let deserialized: Config = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(cfg, deserialized);
    }

    #[test]
    fn parse_from_toml_string() {
        let input = r#"
[editor]
tab_size = 8
insert_spaces = false

[display]
theme = "monokai"
line_numbers = "relative"
"#;
        let cfg: Config = toml::from_str(input).expect("parse toml");
        assert_eq!(cfg.editor.tab_size, 8);
        assert!(!cfg.editor.insert_spaces);
        assert_eq!(cfg.display.theme, "monokai");
        assert_eq!(cfg.display.line_numbers, LineNumberMode::Relative,);
        // Unspecified fields keep defaults via serde(default)
        assert!(cfg.editor.auto_indent);
        assert_eq!(cfg.auto_save_interval_secs, 30);
    }

    #[test]
    fn empty_toml_gives_defaults() {
        let cfg: Config = toml::from_str("").expect("parse empty toml");
        assert_eq!(cfg, Config::default());
    }
}
