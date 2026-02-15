# smash-config — Module Design

## 1. Overview

`smash-config` handles parsing, validation, live reload, and merging of configuration files. It supports global config (`~/.config/smash/config.toml`) and per-project config (`.smash/config.toml`), with project settings overriding global ones.

**Phase**: 1 (MVP — file loading), Phase 4 (live reload)

**Requirements Covered**: REQ-CONF-001 – 004

---

## 2. Public API Surface

### 2.1 Core Types

```rust
Config               — fully resolved configuration struct (all fields typed)
ConfigSource         — enum { Global, Project(PathBuf), Default }
ConfigError          — validation and parsing errors
ConfigWatcher        — file system watcher for live reload
```

### 2.2 Config Struct (Selected Fields)

```rust
pub struct Config {
    // Editor behavior
    pub tab_size: u8,                    // default: 4
    pub insert_spaces: bool,             // default: true
    pub line_ending: LineEndingSetting,   // default: auto-detect
    pub word_wrap: bool,                 // default: false
    pub auto_indent: bool,              // default: true
    pub auto_close_brackets: bool,      // default: true
    pub trim_trailing_whitespace: bool, // default: false

    // Display
    pub theme: String,                   // default: "dark"
    pub line_numbers: LineNumberMode,    // default: Absolute
    pub show_minimap: bool,             // default: false
    pub cursor_blink: bool,             // default: true

    // Keybindings
    pub keymap: KeymapConfig,           // default: "default"

    // LSP
    pub lsp: HashMap<String, LspServerConfig>,

    // Terminal
    pub terminal_shell: Option<String>,

    // Debug
    pub debug_configs: Vec<DebugConfig>,

    // Logging
    pub log_level: LogLevel,            // default: Info
    pub log_file: Option<PathBuf>,

    // Recovery
    pub auto_save_interval_secs: u64,   // default: 30
}
```

### 2.3 Key Functions

```rust
/// Load config from default locations with merge
pub fn load() -> Result<Config, ConfigError>

/// Load from a specific path
pub fn load_from(path: &Path) -> Result<Config, ConfigError>

/// Merge project config over global config
pub fn merge(global: Config, project: Config) -> Config

/// Validate a config (schema check, value ranges)
pub fn validate(config: &Config) -> Result<(), ConfigError>

/// Start watching config files for changes
pub fn watch(paths: &[PathBuf]) -> Result<ConfigWatcher, ConfigError>
```

---

## 3. Internal Architecture

```
┌──────────────────────────────────────────────┐
│                smash-config                  │
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │         Config Loading Pipeline        │  │
│  │                                        │  │
│  │  1. Locate config files                │  │
│  │     - Global: ~/.config/smash/config.toml │
│  │     - Project: .smash/config.toml      │  │
│  │                                        │  │
│  │  2. Parse TOML → RawConfig             │  │
│  │                                        │  │
│  │  3. Merge: default ← global ← project  │  │
│  │                                        │  │
│  │  4. Validate (ranges, paths, enums)    │  │
│  │                                        │  │
│  │  5. Produce typed Config struct        │  │
│  └────────────────────────────────────────┘  │
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │         Config Watcher (Phase 4)       │  │
│  │                                        │  │
│  │  - File system notify (notify crate)   │  │
│  │  - Re-parse on change                  │  │
│  │  - Send updated Config via channel     │  │
│  └────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

### 3.1 Config File Location

1. **Global**: `platform.paths().config_dir() / "config.toml"`
2. **Project**: Walk from CWD upward looking for `.smash/config.toml`.
3. **Default**: Hardcoded defaults in Rust (no file needed).

### 3.2 Auto-Creation of Default Config

If the global config file does **not** exist on first launch, SMASH creates a minimal seed file automatically:

1. Ensure the parent directory exists (`~/.config/smash/`), creating it recursively if necessary.
2. Write a minimal `config.toml` containing only a commented-out skeleton with the most common settings and their defaults:

```toml
# SMASH configuration — https://github.com/smash-editor/smash
# Uncomment and edit settings below to override defaults.

# [editor]
# tab_size = 4
# insert_spaces = true
# word_wrap = false
# auto_indent = true
# trim_trailing_whitespace = false

# [display]
# theme = "dark"
# line_numbers = "absolute"   # absolute | relative | none
# cursor_blink = true

# [terminal]
# shell = "/bin/zsh"          # default: $SHELL

# [log]
# level = "info"              # trace | debug | info | warn | error
```

3. Log an info-level message: `"Created default config at {path}"`.
4. Continue the normal load pipeline — the freshly created file will be parsed (all comments, so no overrides).

This ensures users always have a discoverable config file they can edit, while keeping the default behaviour identical to "no config file".

### 3.3 Merge Strategy

- Three-layer merge: Default → Global → Project.
- Each field is independently overridden (project takes priority).
- `Option` fields: `Some` overrides `None`; explicit `None` in TOML can reset.
- Collections (LSP configs): merged by key (language name); project adds/overrides entries.

### 3.4 Validation

- `tab_size`: 1–16.
- `theme`: must correspond to a file in themes directory.
- `lsp.*.command`: must be a non-empty string (existence check is deferred to runtime).
- `log_level`: must be a valid level name.
- Unknown keys: warn but don't fail (forward compatibility).

### 3.5 Live Reload (Phase 4)

- Uses `notify` crate to watch config files.
- On file change: debounce 500 ms → re-parse → validate → emit new `Config` on an `mpsc` channel.
- The editor main loop receives the channel and applies hot-reloadable settings (theme, tab size, keybindings).
- Non-hot-reloadable settings (e.g., terminal shell) log a message: "restart required".

### 3.6 Schema Documentation

- A `config-schema.toml` reference file is generated from the `Config` struct's doc comments.
- `cargo doc` for this crate serves as the configuration reference.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(PathBuf),

    #[error("failed to create default config: {0}")]
    CreateDefault(String),

    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("validation error: {field}: {message}")]
    Validation { field: String, message: String },

    #[error("file watch error: {0}")]
    Watch(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `toml` | TOML parsing and deserialization |
| `serde` + `serde_derive` | Deserialization framework |
| `notify` | File system change notifications (Phase 4) |
| `smash-platform` | Path resolution |
| `thiserror` | Error derivation |

---

## 6. Module File Layout

```
crates/smash-config/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── config.rs           # Config struct with serde derives
    ├── load.rs             # File location, loading, parsing
    ├── merge.rs            # Three-layer merge logic
    ├── validate.rs         # Validation rules
    ├── watcher.rs          # ConfigWatcher (Phase 4)
    ├── defaults.rs         # Default values
    └── error.rs            # ConfigError
```
