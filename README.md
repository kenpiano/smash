# SMASH

A high-performance terminal text editor for software developers, written in Rust.

## Goals

- **Fast**: ≤ 200 ms startup, < 500 KB RSS, ≤ 16 ms frame rendering
- **Smart**: LSP support, Tree-sitter syntax highlighting, fuzzy file finder
- **Flexible**: Vim mode, configurable keybindings, per-project settings
- **Modern**: Integrated terminal emulator, real-time collaboration (CRDT), remote development (SSH/WSL), DAP debugging

## Features

### Core Editing
- Rope-backed buffer with unlimited undo/redo tree
- Multi-cursor editing with add-cursor-at-next-match
- Column (rectangular) selection
- Find & replace with regex support
- Bracket matching and auto-close
- UTF-8 with CJK wide-character support

### Language Intelligence (LSP)
- Diagnostics, completion, hover information
- Go to definition, find references, rename symbol
- Code actions, signature help, formatting
- Supports multiple concurrent LSP servers

### Syntax Highlighting
- Tree-sitter–based incremental parsing
- 16+ languages out of the box

### Integrated Terminal
- Embedded terminal emulator pane (xterm-256color)
- VT escape sequence parser (CSI, SGR, OSC)
- Hyperlink detection (URLs and file paths)
- Multiple simultaneous terminal instances
- Clipboard integration between editor and terminal

### Debugging (DAP)
- Debug Adapter Protocol client
- Breakpoints: line, conditional, hit-count, logpoints
- Step over / into / out, continue, pause
- Stack trace, scopes, and variable inspection
- Expression evaluation

### Reliability
- Crash recovery via swap files (`.smash-swap`)
- Auto-save with configurable interval (default 30 s)
- Structured logging with file rotation

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
│   ├── smash-core/         # Buffer (rope), undo tree, cursor, selection,
│   │                       #   multi-cursor, column selection, recovery, logging
│   ├── smash-syntax/       # Tree-sitter integration, grammar loading
│   ├── smash-lsp/          # LSP client, JSON-RPC transport
│   ├── smash-terminal/     # Embedded terminal emulator (VT parser, grid, PTY)
│   ├── smash-dap/          # Debug Adapter Protocol client
│   ├── smash-config/       # Config parsing, validation, live reload
│   ├── smash-tui/          # TUI renderer (crossterm backend)
│   ├── smash-platform/     # OS abstraction (clipboard, paths, signals)
│   └── smash-input/        # Keybinding engine, command dispatch
├── src/main.rs             # Binary entry point
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
cargo test -p smash-terminal
cargo test -p smash-dap

# Code coverage (requires cargo-llvm-cov)
cargo llvm-cov --workspace --summary-only

# Check docs build cleanly
cargo doc --no-deps --document-private-items
```

## Phased Delivery

| Phase | Status | Scope |
|---|---|---|
| 1 — Core Editor (MVP) | ✅ Done | Rope buffer, undo tree, TUI renderer, syntax highlighting, file I/O, find & replace, splits |
| 2 — LSP & Navigation | ✅ Done | LSP client, diagnostics, completion, hover, go-to-def, fuzzy finder, Vim/Emacs modes |
| 3 — Terminal & Debugging | ✅ Done | Embedded terminal, DAP client, multi-cursor, column selection, crash recovery, logging |
| 4 — Collaboration & Remote | Planned | CRDT engine, collaboration signaling, SSH remote agent, WSL |
| 5 — Plugins & Polish | Planned | WASM plugin host, GUI mode prototype, accessibility, encoding detection |

## License

MIT OR Apache-2.0
