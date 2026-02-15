# smash-input — Module Design

## 1. Overview

`smash-input` is the keybinding engine and command dispatch layer. It normalizes raw terminal events into commands, resolves keybindings (including multi-key chords and modal sequences), and dispatches commands to the editor core.

**Phase**: 1 (MVP — default keybindings), Phase 2 (Vim/Emacs presets)

**Requirements Covered**: REQ-KEY-001 – 004

---

## 2. Public API Surface

### 2.1 Core Types

```rust
KeyEvent             — normalized key press: { key: Key, modifiers: Modifiers }
Key                  — enum (Char(char), Enter, Esc, Tab, Backspace, Arrow, F1-F12, ...)
Modifiers            — bitflags (Ctrl, Alt, Shift, Super)
MouseEvent           — { kind: MouseKind, position: (u16, u16), modifiers: Modifiers }
InputEvent           — enum { Key(KeyEvent), Mouse(MouseEvent), Resize(u16, u16), Paste(String) }

Command              — enum of all editor commands (see §3.2)
CommandId            — string identifier for a command (e.g., "editor.save")

Keymap               — ordered map of key sequences → CommandId
KeymapLayer          — a named set of keybindings (e.g., "default", "vim-normal", "vim-insert")
KeySequence          — Vec<KeyEvent> for multi-key chords (e.g., Ctrl-K Ctrl-C)
KeyResolver          — stateful resolver tracking partial key sequences

CommandPalette       — fuzzy-searchable list of all registered commands
```

### 2.2 Key Functions

```rust
/// Create resolver with a keymap
pub fn new(keymap: Keymap) -> KeyResolver

/// Feed an input event; returns a resolved command or WaitingForMore
pub fn resolve(&mut self, event: InputEvent) -> ResolveResult

/// Register a command
pub fn register_command(id: CommandId, handler: Box<dyn CommandHandler>)

/// Open command palette with fuzzy search
pub fn search_commands(query: &str) -> Vec<CommandMatch>

/// Load keymap from configuration
pub fn load_keymap(config: &KeymapConfig) -> Result<Keymap, InputError>
```

---

## 3. Internal Architecture

### 3.1 Event Flow

```
Terminal raw event (crossterm)
    │
    ▼
InputEvent normalization (platform differences resolved)
    │
    ▼
KeyResolver
    ├─ Exact match found → Command
    ├─ Prefix match (partial chord) → WaitingForMore (with timeout)
    └─ No match → Fallback (e.g., insert character for typing)
    │
    ▼
Command dispatch → EditorCore / TUI / LSP / etc.
```

### 3.2 Command Enum

```rust
pub enum Command {
    // Buffer operations
    InsertChar(char),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    DeleteLine,
    
    // Cursor movement
    MoveLeft, MoveRight, MoveUp, MoveDown,
    MoveWordLeft, MoveWordRight,
    MoveLineStart, MoveLineEnd,
    MoveBufferStart, MoveBufferEnd,
    PageUp, PageDown,
    
    // Selection
    SelectAll,
    ExtendSelection(Direction),
    AddCursorAbove, AddCursorBelow,
    
    // File operations
    Save, SaveAs, Open, Close,
    
    // Search
    Find, FindReplace, FindNext, FindPrev,
    
    // Undo / redo
    Undo, Redo,
    
    // Pane management
    SplitVertical, SplitHorizontal,
    FocusNext, FocusPrev, ClosePane,
    
    // Navigation
    GoToDefinition, FindReferences, GoToLine,
    OpenCommandPalette, OpenFileFinder,
    
    // LSP
    Hover, Completion, CodeAction, Rename, Format,
    
    // Terminal
    ToggleTerminal, NewTerminal,
    
    // Editor lifecycle
    Quit, ForceQuit,
    
    // Custom / plugin
    Custom(String),
}
```

### 3.3 Keybinding Resolution

- **Trie-based lookup**: Keybindings are stored in a trie where each level is a `KeyEvent`.
- **Multi-key chords**: e.g., `Ctrl-K` → `Ctrl-C` for comment. Resolver holds partial state with a timeout (default: 1 second).
- **Modal support**: `KeymapLayer` stack. Pushing a new layer (e.g., Vim normal mode) overrides bindings; popping restores the previous layer.
- **Priority**: User overrides > layer defaults > global defaults.

### 3.4 Default Keymap

| Key | Command |
|---|---|
| `Ctrl-S` | Save |
| `Ctrl-Q` | Quit |
| `Ctrl-Z` | Undo |
| `Ctrl-Shift-Z` | Redo |
| `Ctrl-F` | Find |
| `Ctrl-H` | Find & Replace |
| `Ctrl-G` | Go to Line |
| `Ctrl-P` | Command Palette |
| `Ctrl-O` | Open File |
| `Ctrl-W` | Close Pane |
| `Ctrl-\` | Toggle Terminal |
| `Ctrl-D` | Add Cursor at Next Match |
| Arrow keys | Cursor movement |
| `Shift-Arrow` | Extend selection |
| `Ctrl-Arrow` | Move by word |
| `Home/End` | Line start/end |
| `Ctrl-Home/End` | Buffer start/end |

### 3.5 Vim Mode (Phase 2)

- Layers: `Normal`, `Insert`, `Visual`, `Command`.
- Mode transitions: `Esc` → Normal, `i/a/o` → Insert, `v/V` → Visual, `:` → Command.
- Normal mode bindings: `hjkl`, `w/b/e`, `dd`, `yy`, `p`, etc.
- Vim mode is a `KeymapLayer` pushed on top of the default.

### 3.6 Command Palette

- All registered commands are searchable via fuzzy matching.
- Each command has: `CommandId`, display name, optional keybinding hint.
- Palette UI is rendered by `smash-tui`; the input layer provides the data.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("unknown command: {0}")]
    UnknownCommand(String),

    #[error("invalid key sequence: {0}")]
    InvalidKeySequence(String),

    #[error("keymap parse error: {0}")]
    KeymapParse(String),

    #[error("duplicate binding for {key} in layer {layer}")]
    DuplicateBinding { key: String, layer: String },
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `crossterm` | Raw terminal event input |
| `fuzzy-matcher` | Fuzzy matching for command palette |
| `smash-core` | Command targets (Buffer, EditCommand) |
| `smash-config` | Keymap configuration loading |

---

## 6. Module File Layout

```
crates/smash-input/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── event.rs            # InputEvent, KeyEvent, MouseEvent normalization
    ├── command.rs          # Command enum, CommandId, CommandHandler trait
    ├── keymap.rs           # Keymap, KeymapLayer, KeySequence
    ├── resolver.rs         # KeyResolver (trie-based, stateful)
    ├── palette.rs          # CommandPalette, fuzzy search
    ├── default_keymap.rs   # Built-in default keybindings
    ├── vim.rs              # Vim mode layer (Phase 2)
    └── error.rs            # InputError
```
