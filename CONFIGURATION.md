# SMASH Configuration Reference

SMASH is configured through TOML files. Settings cascade in this order (later overrides earlier):

1. **Built-in defaults** — hardcoded sensible values
2. **Global config** — `~/.config/smash/config.toml`
3. **Per-project config** — `.smash/config.toml` (searched upward from the working directory)

On first launch, SMASH creates a global config file with all options commented out. Uncomment and edit any line to override the default.

---

## File Locations

| Scope | Path |
|---|---|
| Global | `~/.config/smash/config.toml` (macOS/Linux) |
| Per-project | `<project-root>/.smash/config.toml` |

SMASH searches for a per-project config by walking upward from your current working directory until it finds a `.smash/config.toml` file or reaches the filesystem root.

---

## How to Edit

1. **Open your global config:**

   ```sh
   # The file is created automatically on first launch.
   # Open it in smash itself or any editor:
   smash ~/.config/smash/config.toml
   ```

2. **Create a per-project config:**

   ```sh
   mkdir -p .smash
   touch .smash/config.toml
   ```

   Then add only the settings you want to override for that project.

3. **Reload:** Settings are read at startup. Restart SMASH after editing a config file.

---

## Full Configuration Reference

Below is every available setting with its default value. Copy any section into your `config.toml` and change the values as needed.

### `[editor]` — Editor Behaviour

```toml
[editor]
# Number of spaces per tab stop (1–16).
tab_size = 4

# Insert spaces when pressing Tab. Set to false for hard tabs.
insert_spaces = true

# Line ending style: "auto", "lf", or "crlf".
# "auto" detects from file content.
line_ending = "auto"

# Enable soft word-wrap (long lines wrap visually).
word_wrap = false

# Automatically indent new lines to match the previous line.
auto_indent = true

# Automatically insert closing bracket/quote after opening one.
auto_close_brackets = true

# Remove trailing whitespace from lines when saving.
trim_trailing_whitespace = false

# Treat the macOS Option key as Alt for keybindings.
# When true, Unicode characters produced by Option+key (e.g.
# Option+f → ƒ) are mapped back to Alt+key so that
# Alt-based keybindings work on macOS without terminal config.
# Defaults to true on macOS, false on other platforms.
option_as_alt = true   # macOS default; set false to type special chars
```

| Key | Type | Default | Description |
|---|---|---|---|
| `tab_size` | integer (1–16) | `4` | Spaces per tab stop |
| `insert_spaces` | boolean | `true` | Insert spaces instead of tab characters |
| `line_ending` | `"auto"` \| `"lf"` \| `"crlf"` | `"auto"` | Line ending style |
| `word_wrap` | boolean | `false` | Soft word-wrap |
| `auto_indent` | boolean | `true` | Auto-indent new lines |
| `auto_close_brackets` | boolean | `true` | Auto-close brackets and quotes |
| `trim_trailing_whitespace` | boolean | `false` | Strip trailing whitespace on save |
| `option_as_alt` | boolean | `true` (macOS) / `false` (other) | Map macOS Option key to Alt |

---

### `[display]` — Display / UI

```toml
[display]
# Colour theme name. Built-in: "dark", "light".
theme = "dark"

# Line number mode: "absolute", "relative", or "none".
line_numbers = "absolute"

# Show a minimap panel on the right side.
show_minimap = false

# Blink the cursor.
cursor_blink = true
```

| Key | Type | Default | Description |
|---|---|---|---|
| `theme` | string | `"dark"` | Colour theme (must not be empty) |
| `line_numbers` | `"absolute"` \| `"relative"` \| `"none"` | `"absolute"` | Line number display mode |
| `show_minimap` | boolean | `false` | Show minimap panel |
| `cursor_blink` | boolean | `true` | Blink the cursor |

---

### `[keymap]` — Keybindings

```toml
[keymap]
# Built-in keymap preset: "default" or "emacs".
preset = "default"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `preset` | `"default"` \| `"emacs"` | `"default"` | Keymap preset to use |

Setting `preset = "emacs"` pushes an Emacs-style layer on top of the default keybindings. No modal switching is needed — all bindings use Ctrl or Alt modifiers.

> **macOS**: The Option key works as Alt when `editor.option_as_alt = true` (the default on macOS). No terminal configuration is required.

| Key | Action |
|---|---|
| `Ctrl-f` / `Ctrl-b` | Forward / backward character |
| `Ctrl-n` / `Ctrl-p` | Next / previous line |
| `Ctrl-a` / `Ctrl-e` | Beginning / end of line |
| `Alt-f` / `Alt-b` (Option on Mac) | Forward / backward word |
| `Ctrl-v` / `Alt-v` (Option on Mac) | Page down / page up |
| `Alt-<` / `Alt->` (Option on Mac) | Beginning / end of buffer |
| `Ctrl-d` | Delete forward character |
| `Ctrl-k` | Kill (delete) line |
| `Ctrl-s` | Incremental search |
| `Ctrl-r` | Reverse search |
| `Alt-x` (Option on Mac) | Command palette (M-x) |
| `Ctrl-x Ctrl-s` | Save |
| `Ctrl-x Ctrl-c` | Quit |
| `Ctrl-x Ctrl-f` | Open file |

---

### `[log]` — Logging

```toml
[log]
# Log level: "trace", "debug", "info", "warn", or "error".
level = "info"

# Optional path to a log file. Omit to use the default location.
# file = "/tmp/smash.log"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `level` | `"trace"` \| `"debug"` \| `"info"` \| `"warn"` \| `"error"` | `"info"` | Log verbosity |
| `file` | string (path) or omitted | *(none)* | Log file path (optional) |

Default log file locations:

| Platform | Path |
|---|---|
| macOS | `~/Library/Logs/smash/smash.log` |
| Linux | `~/.local/share/smash/smash.log` |
| Windows | `%APPDATA%/smash/logs/smash.log` |

Logs are rotated automatically when they exceed 10 MB. Up to 5 rotated files are kept (`smash.log.1` through `smash.log.5`).

---

### `[lsp]` — Language Server Protocol

```toml
[lsp]
# Enable or disable LSP globally.
enabled = true

# Per-language server definitions, keyed by language ID.
# Each server entry has: command, args, extensions.

[lsp.servers.rust]
command = "rust-analyzer"
args = []
extensions = ["rs"]

[lsp.servers.python]
command = "pylsp"
args = []
extensions = ["py"]

[lsp.servers.typescript]
command = "typescript-language-server"
args = ["--stdio"]
extensions = ["ts", "tsx", "js", "jsx"]

[lsp.servers.go]
command = "gopls"
args = []
extensions = ["go"]

[lsp.servers.c]
command = "clangd"
args = []
extensions = ["c", "h", "cpp", "hpp", "cc"]
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | boolean | `true` | Enable/disable LSP globally |
| `servers.<id>.command` | string | *(required)* | Server executable |
| `servers.<id>.args` | array of strings | `[]` | Command-line arguments |
| `servers.<id>.extensions` | array of strings | `[]` | File extensions this server handles |

The `<id>` (e.g., `rust`, `python`) is an arbitrary language identifier used internally.

---

### `terminal_shell` — Terminal Shell

```toml
# Override the shell used by the integrated terminal.
# Omit to use the system default ($SHELL).
# terminal_shell = "/bin/zsh"
```

| Key | Type | Default | Description |
|---|---|---|---|
| `terminal_shell` | string or omitted | *(system $SHELL)* | Shell executable for the integrated terminal |

---

### `auto_save_interval_secs` — Auto-Save & Crash Recovery

```toml
# Auto-save interval in seconds.
# Set to 0 to disable. Minimum value is 5.
auto_save_interval_secs = 30
```

| Key | Type | Default | Description |
|---|---|---|---|
| `auto_save_interval_secs` | integer (0 or ≥ 5) | `30` | Auto-save interval; `0` disables |

**Swap files:** SMASH automatically writes a swap file (`.smash-swap`) alongside each open file. The swap file records the original file hash and a log of edit commands since the last save. If SMASH crashes, the swap file can be used to recover unsaved work on the next startup.

- Swap file is written on each edit (debounced) or at the auto-save interval.
- On a clean save, the swap file is deleted.
- Swap file path: for `/path/to/file.rs`, the swap file is `/path/to/.file.rs.smash-swap`.

---

## Validation Rules

SMASH validates your configuration at startup. Invalid values produce an error message and the editor falls back to defaults.

| Rule | Error if violated |
|---|---|
| `editor.tab_size` must be 1–16 | `"must be 1–16, got X"` |
| `display.theme` must not be empty | `"must not be empty"` |
| `auto_save_interval_secs` must be 0 or ≥ 5 | `"must be 0 (disabled) or ≥ 5, got X"` |

---

## Example Configurations

### Minimal (just override what you need)

```toml
[editor]
tab_size = 2

[display]
theme = "dark"
```

### Rust Developer Setup

```toml
[editor]
tab_size = 4
insert_spaces = true
trim_trailing_whitespace = true

[display]
line_numbers = "relative"

[keymap]
preset = "emacs"

[lsp]
enabled = true

[lsp.servers.rust]
command = "rust-analyzer"
extensions = ["rs"]
```

### Web Development Setup

```toml
[editor]
tab_size = 2
insert_spaces = true
auto_close_brackets = true

[lsp]
enabled = true

[lsp.servers.typescript]
command = "typescript-language-server"
args = ["--stdio"]
extensions = ["ts", "tsx", "js", "jsx"]

[lsp.servers.css]
command = "vscode-css-language-server"
args = ["--stdio"]
extensions = ["css", "scss", "less"]
```

### Per-Project Override

Create `.smash/config.toml` in your project root to override global settings for that project only:

```toml
# .smash/config.toml
# This project uses tabs and 8-wide indents
[editor]
tab_size = 8
insert_spaces = false
line_ending = "lf"
```

Only the values you specify are overridden; everything else keeps its global (or default) value.

---

## Integrated Terminal

SMASH includes an embedded terminal emulator that supports xterm-256color escape sequences. The terminal runs in a split pane alongside your editor buffers.

- Use `Ctrl+\` to toggle the terminal pane.
- The terminal uses the shell specified by `terminal_shell` in your config, or `$SHELL` by default.
- Multiple terminal instances are supported simultaneously.
- URLs and file paths in terminal output are detected as clickable hyperlinks.
- Copy/paste works between editor buffers and the terminal.

---

## Debugging (DAP)

SMASH includes a Debug Adapter Protocol (DAP) client for integrated debugging. Debug configurations are defined per-project.

### Debug Session Lifecycle

1. **Configure** a debug adapter (specify the adapter executable and launch arguments).
2. **Set breakpoints** in your source files (line, conditional, hit-count, or logpoint).
3. **Launch** the debug session — SMASH spawns the adapter and initializes DAP.
4. **Debug** — step over/into/out, continue, pause, inspect variables and the call stack.
5. **Disconnect** to end the session.

### Supported Features

| Feature | Description |
|---|---|
| Breakpoints | Line, conditional (`x > 5`), hit-count, logpoints |
| Stepping | Step over, step into, step out, continue, pause |
| Inspection | Stack trace, scopes, local variables, watch expressions |
| Evaluation | Evaluate expressions in the current debug context |
| Events | Stopped, continued, exited, output events |
