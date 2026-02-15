# SMASH

A high-performance terminal text editor for software developers, written in Rust.

## Goals

- **Fast**: ≤ 200 ms startup, < 500 KB RSS, ≤ 16 ms frame rendering
- **Smart**: LSP support, Tree-sitter syntax highlighting, fuzzy file finder
- **Flexible**: Vim mode, configurable keybindings, per-project settings
- **Modern**: Integrated terminal emulator, real-time collaboration (CRDT), remote development (SSH/WSL), DAP debugging

## Quick Start

### Build

```sh
cargo build --release
```

### Run

```sh
# Open with no file (scratch buffer)
./target/release/smash

# Open a file
./target/release/smash path/to/file.rs
```

### Install (optional)

```sh
cargo install --path .
```

## Keybindings

### Default Keymap

| Key | Action |
|---|---|
| `Ctrl+S` | Save |
| `Ctrl+Q` | Quit |
| `Ctrl+O` | Open file |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |
| `Ctrl+F` | Find |
| `Ctrl+H` | Find & Replace |
| `Ctrl+G` | Go to line |
| `Ctrl+P` | Command palette |
| `Ctrl+N` | Find next |
| `Ctrl+Shift+N` | Find previous |
| `F3` / `Shift+F3` | Find next / previous |
| `Ctrl+A` | Select all |
| `Ctrl+D` | Add cursor below |
| `Ctrl+W` | Close pane |
| `Ctrl+\` | Toggle terminal |
| `Ctrl+Left/Right` | Word movement |
| `Ctrl+Home/End` | Buffer start / end |
| `Home` / `End` | Line start / end |
| `PageUp` / `PageDown` | Scroll page |
| `Arrow keys` | Cursor movement |
| `Backspace` / `Delete` | Delete backward / forward |
| `Enter` | Insert newline |
| `Tab` | Insert tab |
| `Esc` | Cancel prompt |

### Emacs Mode

Set `keymap.preset = "emacs"` in your config to enable Emacs-style keybindings. No modal switching — all bindings use modifiers:

| Key | Action |
|---|---|
| `Ctrl-f` / `Ctrl-b` | Forward / backward character |
| `Ctrl-n` / `Ctrl-p` | Next / previous line |
| `Ctrl-a` / `Ctrl-e` | Beginning / end of line |
| `Alt-f` / `Alt-b` | Forward / backward word |
| `Ctrl-v` / `Alt-v` | Page down / page up |
| `Alt-<` / `Alt->` | Beginning / end of buffer |
| `Ctrl-d` | Delete forward character |
| `Ctrl-k` | Kill (delete) line |
| `Ctrl-s` | Incremental search |
| `Ctrl-r` | Reverse search |
| `Alt-x` | Command palette (M-x) |
| `Ctrl-x Ctrl-s` | Save |
| `Ctrl-x Ctrl-c` | Quit |
| `Ctrl-x Ctrl-f` | Open file |

## Configuration

SMASH uses TOML configuration files. See [CONFIGURATION.md](CONFIGURATION.md) for the full reference.

- **Global config**: `~/.config/smash/config.toml` (created on first launch)
- **Per-project config**: `.smash/config.toml` in your project directory (overrides global)

Quick example:

```toml
[editor]
tab_size = 2
insert_spaces = true
trim_trailing_whitespace = true

[display]
theme = "dark"
line_numbers = "relative"

[keymap]
preset = "emacs"
```

## Repository Layout

```
smash/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── smash-core/         # Buffer (rope), undo tree, cursor, selection, edit ops
│   ├── smash-syntax/       # Tree-sitter integration, grammar loading
│   ├── smash-lsp/          # LSP client, JSON-RPC transport
│   ├── smash-terminal/     # Embedded terminal emulator
│   ├── smash-collab/       # CRDT engine, signaling protocol
│   ├── smash-remote/       # SSH tunneling, remote agent protocol
│   ├── smash-dap/          # Debug Adapter Protocol client
│   ├── smash-config/       # Config parsing, validation, live reload
│   ├── smash-plugin/       # WASM plugin host and API
│   ├── smash-tui/          # TUI renderer (crossterm backend)
│   ├── smash-platform/     # OS abstraction (clipboard, paths, signals)
│   └── smash-input/        # Keybinding engine, command dispatch
├── src/main.rs             # Binary entry point
├── grammars/               # Vendored Tree-sitter grammars
├── themes/                 # Built-in theme files (.toml)
├── doc/                    # Requirements, design plan, coding rules
└── tests/                  # Integration / end-to-end tests
```

## Development

```sh
# Format
cargo fmt --all

# Lint (zero warnings required)
cargo clippy --workspace --all-targets -- -D warnings

# Run all tests
cargo test --workspace

# Run tests for a single crate
cargo test -p smash-core

# Check docs build cleanly
cargo doc --no-deps --document-private-items
```

## License

MIT OR Apache-2.0
