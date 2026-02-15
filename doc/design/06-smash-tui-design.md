# smash-tui — Module Design

## 1. Overview

`smash-tui` is the terminal user interface renderer. It maintains a shadow screen buffer, computes visible line layouts with styled spans, diffs against the previous frame, and emits minimal escape sequences to the terminal.

**Phase**: 1 (MVP)

**Requirements Covered**: REQ-PERF-030, REQ-PERF-031, REQ-THEME-001 – 005, REQ-PLAT-020

---

## 2. Public API Surface

### 2.1 Core Types

```rust
TuiRenderer         — main renderer; owns the terminal backend and screen buffer
Screen              — 2D grid of styled cells (shadow buffer)
Cell                — { char, style: CellStyle }
CellStyle           — { fg: Color, bg: Color, modifiers: Modifiers }
Color               — enum (Rgb, Indexed256, Named16)
Modifiers           — bitflags (Bold, Italic, Underline, Strikethrough, Dim, Reverse)
Viewport            — { top_line, left_col, width, height }
RenderLine          — styled segments for one visible line
Theme               — color scheme loaded from TOML
LayoutPane          — a rectangular region with a buffer assignment
PaneTree            — binary tree of horizontal/vertical splits
```

### 2.2 Key Functions

```rust
/// Initialize the renderer, enter raw mode, alternate screen
pub fn new(backend: impl TerminalBackend) -> Result<TuiRenderer, RenderError>

/// Full render cycle: compute layout → gather styled lines → diff → flush
pub fn render(&mut self, state: &EditorState) -> Result<(), RenderError>

/// Resize handler
pub fn resize(&mut self, width: u16, height: u16)

/// Clean shutdown: restore terminal state
pub fn shutdown(&mut self) -> Result<(), RenderError>
```

---

## 3. Internal Architecture

```
┌────────────────────────────────────────────────────────┐
│                    TuiRenderer                         │
│                                                        │
│  ┌──────────────┐    ┌───────────────────────────────┐ │
│  │ PaneTree     │    │ Screen (current frame)        │ │
│  │              │    │  ┌─────────────────────────┐  │ │
│  │  ┌─────┐    │    │  │ Cell[][] (w × h)        │  │ │
│  │  │Pane │    │    │  └─────────────────────────┘  │ │
│  │  │  A  │    │    └───────────────────────────────┘ │
│  │  └──┬──┘    │                                      │
│  │   Split(V)  │    ┌───────────────────────────────┐ │
│  │  ┌──┴──┐    │    │ Screen (previous frame)       │ │
│  │  │Pane │    │    │  ┌─────────────────────────┐  │ │
│  │  │  B  │    │    │  │ Cell[][] (w × h)        │  │ │
│  │  └─────┘    │    │  └─────────────────────────┘  │ │
│  └──────────────┘    └───────────────────────────────┘ │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │              Render Pipeline                      │  │
│  │                                                   │  │
│  │  1. Layout panes → rectangles                     │  │
│  │  2. For each pane:                                │  │
│  │     a. Compute visible lines from viewport        │  │
│  │     b. Gather styled spans (syntax + diagnostics  │  │
│  │        + search + cursor)                         │  │
│  │     c. Render status bar, line numbers, gutter    │  │
│  │  3. Diff current vs previous screen               │  │
│  │  4. Emit escape sequences for changed cells       │  │
│  └──────────────────────────────────────────────────┘  │
│                                                        │
│  ┌──────────────┐    ┌──────────────┐                  │
│  │ Theme        │    │ Terminal     │                  │
│  │ (colors,     │    │ Backend     │                  │
│  │  styles)     │    │ (crossterm) │                  │
│  └──────────────┘    └──────────────┘                  │
└────────────────────────────────────────────────────────┘
```

### 3.1 Render Pipeline

Each `render()` call:

1. **Layout**: Traverse `PaneTree` to compute screen rectangles for each pane (including borders/dividers).
2. **Compose**: For each pane, read visible lines from the buffer through a `RenderContext` that overlays:
   - Syntax highlight spans
   - Diagnostic underlines
   - Search match highlights
   - Cursor(s) and selection background
   - Line numbers and gutter indicators
3. **Write to Screen**: Write `Cell` values into the current `Screen` grid.
4. **Diff**: Compare current `Screen` against previous `Screen`. Collect changed cells.
5. **Flush**: Move cursor to each changed cell, set style, write character. Use bulk writes for consecutive changed cells on the same line.
6. **Swap**: Previous screen = current screen.

### 3.2 Screen Buffer (Shadow Buffer)

- Two `Screen` instances: current and previous.
- `Screen` is a flat `Vec<Cell>` of size `width × height`.
- Diff is O(width × height) but typically only a small fraction of cells change per frame.

### 3.3 Pane Management

- `PaneTree` is a binary tree where leaves are buffer panes and internal nodes are splits (horizontal or vertical).
- Supports: split-horizontal, split-vertical, close-pane, resize-divider, focus-next/prev.
- Each pane has its own `Viewport` and focused buffer ID.

### 3.4 Theme System

- Themes are loaded from TOML files in `themes/`.
- A `Theme` maps scope names (from Tree-sitter) to `CellStyle`.
- Built-in themes: `dark.toml`, `light.toml`.
- Custom themes: placed in `~/.config/smash/themes/` or `.smash/themes/`.

### 3.5 Terminal Backend Trait

```rust
pub trait TerminalBackend {
    fn enter_raw_mode(&mut self) -> Result<(), RenderError>;
    fn leave_raw_mode(&mut self) -> Result<(), RenderError>;
    fn enter_alternate_screen(&mut self) -> Result<(), RenderError>;
    fn leave_alternate_screen(&mut self) -> Result<(), RenderError>;
    fn size(&self) -> Result<(u16, u16), RenderError>;
    fn write(&mut self, buf: &[u8]) -> Result<(), RenderError>;
    fn flush(&mut self) -> Result<(), RenderError>;
}
```

- Default implementation: `CrosstermBackend` wrapping `std::io::Stdout`.
- Testable via a mock backend that captures output.

### 3.6 Status Bar & UI Elements

- **Status bar** (bottom): file name, cursor position, encoding, line ending, mode, diagnostics summary.
- **Line numbers** (left gutter): configurable width, relative or absolute.
- **Gutter indicators**: Git diff markers, breakpoints, diagnostic icons.
- **Tab bar** (top, optional): open buffers.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("terminal I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("terminal size unavailable")]
    SizeUnavailable,

    #[error("theme not found: {0}")]
    ThemeNotFound(String),

    #[error("theme parse error: {0}")]
    ThemeParse(String),
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `crossterm` | Terminal abstraction (raw mode, escape sequences, events) |
| `toml` | Theme file parsing |
| `smash-core` | Buffer, Position, Range types |
| `smash-syntax` | HighlightSpan, ScopeId |

---

## 6. Performance Considerations

- Diff-based rendering: only changed cells are flushed → minimizes I/O.
- No allocations in the render hot path after initial screen buffer allocation.
- Screen buffer is reused across frames (just clear and overwrite).
- Styled span merging is done in a single pass per line.
- Target: ≤ 16 ms per `render()` call.

---

## 7. Module File Layout

```
crates/smash-tui/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── renderer.rs         # TuiRenderer, render pipeline
    ├── screen.rs           # Screen, Cell, CellStyle, diff
    ├── pane.rs             # PaneTree, LayoutPane, split logic
    ├── viewport.rs         # Viewport, scroll logic
    ├── theme.rs            # Theme loading and mapping
    ├── backend.rs          # TerminalBackend trait
    ├── crossterm_backend.rs # crossterm implementation
    ├── status_bar.rs       # Status bar rendering
    ├── gutter.rs           # Line numbers, gutter indicators
    ├── line_render.rs      # Styled line composition
    └── error.rs            # RenderError
```
