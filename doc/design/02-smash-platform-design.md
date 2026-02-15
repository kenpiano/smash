# smash-platform — Module Design

## 1. Overview

`smash-platform` provides the OS abstraction layer. All platform-specific code (clipboard, file paths, process spawning, signal handling) is isolated behind traits so that the rest of the codebase is platform-independent.

**Phase**: 1 (MVP)

**Requirements Covered**: REQ-PLAT-001, REQ-PLAT-002, REQ-PLAT-010

---

## 2. Public API Surface

### 2.1 Core Traits

```rust
pub trait Platform: Send + Sync {
    fn clipboard(&self) -> &dyn Clipboard;
    fn paths(&self) -> &dyn PlatformPaths;
    fn process(&self) -> &dyn ProcessSpawner;
    fn signals(&self) -> &dyn SignalHandler;
    fn system_info(&self) -> SystemInfo;
}

pub trait Clipboard: Send + Sync {
    fn get(&self) -> Result<String, PlatformError>;
    fn set(&self, content: &str) -> Result<(), PlatformError>;
}

pub trait PlatformPaths: Send + Sync {
    fn config_dir(&self) -> PathBuf;          // ~/.config/smash/  (all platforms)
    fn data_dir(&self) -> PathBuf;            // ~/.local/share/smash/  (all platforms)
    fn cache_dir(&self) -> PathBuf;           // ~/.cache/smash/
    fn log_dir(&self) -> PathBuf;             // ~/.local/share/smash/logs/
    fn default_shell(&self) -> PathBuf;       // /bin/zsh, cmd.exe, etc.
    fn home_dir(&self) -> PathBuf;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, PlatformError>;
}

pub trait ProcessSpawner: Send + Sync {
    fn spawn(&self, cmd: &str, args: &[&str]) -> Result<Child, PlatformError>;
    fn spawn_shell(&self, command: &str) -> Result<Child, PlatformError>;
}

pub trait SignalHandler: Send + Sync {
    fn on_resize(&self) -> Receiver<(u16, u16)>;
    fn on_suspend(&self) -> Receiver<()>;
    fn on_interrupt(&self) -> Receiver<()>;
}
```

### 2.2 Types

```rust
SystemInfo           — { os: OsKind, arch: Arch, terminal: TerminalKind }
OsKind               — enum { Linux, MacOS, Windows }
Arch                 — enum { X86_64, Aarch64 }
TerminalKind         — enum { Xterm, Alacritty, WindowsTerminal, Unknown(String) }
PlatformError        — error type for all platform operations
```

---

## 3. Internal Architecture

```
┌────────────────────────────────────────────────────┐
│                  smash-platform                    │
│                                                    │
│  ┌──────────────────────────────────────────────┐  │
│  │          Platform trait (public API)          │  │
│  └─────────────────┬────────────────────────────┘  │
│                    │                               │
│     ┌──────────────┼──────────────┐                │
│     ▼              ▼              ▼                │
│  ┌────────┐  ┌──────────┐  ┌──────────┐           │
│  │ Linux  │  │  macOS   │  │ Windows  │           │
│  │ Impl   │  │  Impl    │  │  Impl    │           │
│  └────────┘  └──────────┘  └──────────┘           │
│                                                    │
│  Each impl provides:                               │
│  - Clipboard (xclip/pbcopy/win32)                 │
│  - PlatformPaths (XDG/Library/AppData)            │
│  - ProcessSpawner (fork+exec / CreateProcess)     │
│  - SignalHandler (SIGWINCH/SIGTSTP / Win events)  │
└────────────────────────────────────────────────────┘
```

### 3.1 Platform Selection

- Compile-time: `#[cfg(target_os = "...")]` selects the implementation.
- A `create_platform()` factory function returns `Box<dyn Platform>`.

### 3.2 Clipboard Backends

| OS | Primary | Fallback |
|---|---|---|
| Linux | `xclip` / `xsel` (X11) or `wl-copy` (Wayland) | OSC 52 escape if over SSH |
| macOS | `pbcopy` / `pbpaste` | — |
| Windows | Win32 clipboard API | — |

### 3.3 Path Conventions

All platforms use the **same config directory layout** (`~/.config/smash`) for consistency and ease of dotfile syncing.

| OS | Config Dir | Data Dir |
|---|---|---|
| Linux | `$XDG_CONFIG_HOME/smash` or `~/.config/smash` | `$XDG_DATA_HOME/smash` or `~/.local/share/smash` |
| macOS | `~/.config/smash` | `~/.local/share/smash` |
| Windows | `C:/Users/<user>/.config/smash` | `C:/Users/<user>/.local/share/smash` |

> **Rationale**: Using a unified `~/.config/smash` across all three OSes (instead of macOS `~/Library/Application Support` or Windows `%APPDATA%`) means users can share their config via a dotfiles repo without per-OS path mapping. The `~` on Windows resolves to `C:/Users/<user>` via `std::env::var("USERPROFILE")` or `dirs::home_dir()`.

### 3.4 Signal Handling

- **SIGWINCH** (terminal resize): detected via `crossterm::event::Event::Resize` or platform signal.
- **SIGTSTP** (Ctrl-Z suspend): restore terminal state → suspend → re-enter raw mode on resume.
- **SIGINT** (Ctrl-C): graceful shutdown sequence.
- Windows: uses `SetConsoleCtrlHandler` for equivalent signals.

### 3.5 Keybinding Portability Across Platforms

Keybindings are **identical on all platforms** — modifiers (`Ctrl`, `Alt`, `Shift`) and key names are normalized by `smash-input` so that the same keymap config works everywhere. Platform-specific notes:

| Concern | Behaviour |
|---|---|
| macOS `Cmd` key | Mapped to `Super` modifier. The default keymap does **not** use `Super` — it uses `Ctrl` for all shortcuts so that keybindings are cross-platform. Users can remap `Super` bindings in their config if desired. |
| macOS `Option` key | Mapped to `Alt`. Works for `Alt-` chords. Unicode input via Option (e.g., `Option-e` for accented chars) is **not** intercepted when it produces a printable character. |
| Windows `Win` key | Not intercepted (reserved by the OS). |
| Terminal limitations | Some key combinations (e.g., `Ctrl-Shift-<letter>`) may not be reported by all terminals. The default keymap avoids unreliable combos. |

> **Note**: SMASH does **not** use Emacs-style keybindings. The default keymap uses standard `Ctrl-` shortcuts common in modern editors (VS Code, Sublime, etc.). Only a Vim mode preset is planned (Phase 2). Users who want custom bindings can define their own `KeymapLayer` in config.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("clipboard operation failed: {0}")]
    Clipboard(String),

    #[error("path error: {0}")]
    Path(String),

    #[error("process spawn failed: {0}")]
    ProcessSpawn(#[from] std::io::Error),

    #[error("signal handler error: {0}")]
    Signal(String),

    #[error("unsupported operation on {os}: {detail}")]
    Unsupported { os: String, detail: String },
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `dirs` | Standard directory lookup |
| `crossterm` | Terminal events (resize) — shared with smash-tui |
| `thiserror` | Error derivation |

Platform-specific (conditional):
| Crate | OS | Purpose |
|---|---|---|
| `windows-sys` | Windows | Win32 API bindings |

---

## 6. Module File Layout

```
crates/smash-platform/
├── Cargo.toml
└── src/
    ├── lib.rs              # Platform trait, create_platform(), re-exports
    ├── clipboard.rs        # Clipboard trait
    ├── paths.rs            # PlatformPaths trait
    ├── process.rs          # ProcessSpawner trait
    ├── signals.rs          # SignalHandler trait
    ├── system_info.rs      # SystemInfo, OsKind, Arch
    ├── error.rs            # PlatformError
    ├── linux/
    │   ├── mod.rs
    │   ├── clipboard.rs
    │   ├── paths.rs
    │   └── signals.rs
    ├── macos/
    │   ├── mod.rs
    │   ├── clipboard.rs
    │   ├── paths.rs
    │   └── signals.rs
    └── windows/
        ├── mod.rs
        ├── clipboard.rs
        ├── paths.rs
        └── signals.rs
```
