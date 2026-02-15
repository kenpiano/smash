# smash-terminal — Module Design

## 1. Overview

`smash-terminal` provides an embedded terminal emulator. It manages PTY processes, parses VT escape sequences, and maintains a character grid that the TUI renderer draws into a pane.

**Phase**: 3

**Requirements Covered**: REQ-TERM-001 – 006

---

## 2. Public API Surface

### 2.1 Core Types

```rust
TerminalPane         — a single terminal instance (PTY + grid)
TerminalId           — opaque handle
TerminalGrid         — 2D grid of terminal cells
TerminalCell         — { char, fg, bg, attrs }
TerminalSize         — { rows, cols }
PtyProcess           — child process connected via PTY
TerminalEvent        — enum { Output(Vec<u8>), Exited(ExitStatus), Bell, TitleChange(String) }
```

### 2.2 Key Functions

```rust
/// Spawn a new terminal pane with a shell
pub fn spawn(shell: &Path, size: TerminalSize) -> Result<TerminalPane, TerminalError>

/// Write input (keystrokes) to the terminal
pub fn write_input(&mut self, data: &[u8]) -> Result<(), TerminalError>

/// Process pending output from the PTY, update grid
pub fn process_output(&mut self) -> Result<Vec<TerminalEvent>, TerminalError>

/// Get the current grid state for rendering
pub fn grid(&self) -> &TerminalGrid

/// Resize the terminal
pub fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError>

/// Get the current working directory (if detectable)
pub fn cwd(&self) -> Option<&Path>

/// Close the terminal (send SIGHUP, wait)
pub fn close(&mut self) -> Result<(), TerminalError>
```

---

## 3. Internal Architecture

```
┌──────────────────────────────────────────────────┐
│                  TerminalPane                    │
│                                                  │
│  ┌──────────────┐    ┌────────────────────────┐  │
│  │  PtyProcess   │    │  VT Parser             │  │
│  │               │    │                        │  │
│  │  stdin ◀──────┤    │  Raw bytes →           │  │
│  │  stdout ──────┼───▶│  escape sequences →    │  │
│  │               │    │  grid mutations        │  │
│  └──────────────┘    └───────────┬────────────┘  │
│                                  │               │
│                                  ▼               │
│                    ┌─────────────────────────┐   │
│                    │     TerminalGrid        │   │
│                    │                         │   │
│                    │  rows × cols cells      │   │
│                    │  cursor position        │   │
│                    │  scroll region          │   │
│                    │  alternate screen buf   │   │
│                    └─────────────────────────┘   │
│                                                  │
│  ┌──────────────────────────────────────────┐    │
│  │       Hyperlink Detector (optional)       │    │
│  │                                           │    │
│  │  Scans grid for URLs, file paths          │    │
│  │  Annotates cells with hyperlink metadata  │    │
│  └──────────────────────────────────────────┘    │
└──────────────────────────────────────────────────┘
```

### 3.1 PTY Management

- On Unix: `openpty()` or `posix_openpt()` via the `rustix` or `nix` crate.
- On Windows: `ConPTY` via `windows-sys`.
- The PTY master side is held by the `TerminalPane`; the slave side is given to the child shell process.
- Async reading from PTY stdout via a tokio task.

### 3.2 VT Escape Sequence Parser

- Based on `vte` crate (used by Alacritty) or forked/vendored `alacritty_terminal`.
- Supports: xterm-256color, cursor movement (CSI), SGR attributes, scroll regions, alternate screen buffer, mouse reporting, OSC sequences (title, hyperlinks).

### 3.3 Terminal Grid

- `TerminalGrid` stores a 2D array of `TerminalCell` plus scrollback buffer.
- Alternate screen buffer supported (used by `vim`, `less`, etc.).
- Scrollback: configurable size (default: 10,000 lines).
- All cursor operations (move, insert, delete lines, erase) mutate the grid.

### 3.4 Input Forwarding

- Key events in the terminal pane are translated to escape sequences and written to the PTY stdin.
- Mouse events are forwarded if the terminal application has enabled mouse reporting.

### 3.5 Clipboard Integration

- Copy: select text from grid → send to `smash-platform` clipboard.
- Paste: read from clipboard → write to PTY stdin (with bracketed paste mode if supported).

### 3.6 Hyperlink Detection

- Regex-based scanner for URLs (`https?://...`) and file paths.
- Grid cells are annotated with hyperlink metadata for click-to-open.
- OSC 8 hyperlinks (explicit terminal hyperlinks) are also supported.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("PTY creation failed: {0}")]
    PtyFailed(String),

    #[error("shell process failed to start: {0}")]
    ShellSpawnFailed(#[from] std::io::Error),

    #[error("terminal I/O error: {0}")]
    Io(String),

    #[error("terminal process exited: {0}")]
    Exited(i32),

    #[error("resize failed: {0}")]
    ResizeFailed(String),
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `vte` | VT escape sequence parser |
| `rustix` or `nix` | PTY creation (Unix) |
| `windows-sys` | ConPTY (Windows, conditional) |
| `tokio` | Async PTY I/O |
| `regex` | Hyperlink detection |
| `smash-platform` | Clipboard, default shell |
| `thiserror` | Error derivation |

---

## 6. Module File Layout

```
crates/smash-terminal/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── pane.rs             # TerminalPane struct
    ├── grid.rs             # TerminalGrid, TerminalCell
    ├── pty.rs              # PTY creation abstraction
    ├── pty_unix.rs         # Unix PTY implementation
    ├── pty_windows.rs      # Windows ConPTY implementation
    ├── parser.rs           # VT parser integration
    ├── input.rs            # Keystroke → escape sequence translation
    ├── hyperlink.rs        # URL/path detection
    └── error.rs            # TerminalError
```
