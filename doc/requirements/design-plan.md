# SMASH — Design Plan

This document describes the high-level architecture, technology choices, module decomposition, and phased delivery plan for the SMASH text editor. It is derived from the detailed requirements in `detail-requirements.md`.

---

## 1. Guiding Principles

| # | Principle | Rationale |
|---|---|---|
| 1 | **Performance first** | Every architectural choice must preserve the ≤ 200 ms startup and ≤ 16 ms frame budget. |
| 2 | **Thin platform layer** | Core logic is platform-independent; platform specifics live behind narrow abstraction traits. |
| 3 | **Async by default** | Heavy work (LSP, file I/O, collaboration sync) runs off the main/render thread. |
| 4 | **Layered modularity** | Each subsystem (buffer, renderer, LSP, terminal, etc.) is a separate crate/module with explicit public APIs. |
| 5 | **Incremental delivery** | The editor is usable after Phase 1; advanced features are additive and never destabilize the core. |

---

## 2. Technology Selection

| Concern | Choice | Justification |
|---|---|---|
| Language | **Rust** | Memory safety without GC, low-level control, excellent cross-compilation support, strong ecosystem for TUI/parsing. |
| Buffer data structure | **Rope** (ropey crate as reference, custom if needed) | O(log n) insert/delete, efficient for large files, straightforward CRDT integration. |
| Syntax highlighting | **Tree-sitter** (tree-sitter Rust bindings) | Incremental parsing, broad language grammar ecosystem, proven performance. |
| TUI rendering | **crossterm** backend + custom immediate-mode renderer | Cross-platform terminal abstraction, no ncurses dependency, enables 60 fps rendering. |
| GUI (future) | **wgpu** or platform-native (optional Phase 5) | GPU-accelerated rendering, cross-platform. |
| Async runtime | **tokio** (multi-thread) | De-facto standard for async Rust; drives LSP, collaboration, and remote I/O. |
| LSP transport | **tower-lsp** or custom JSON-RPC over stdio/TCP | Proven LSP plumbing for Rust. |
| CRDT algorithm | **Automerge** or custom Yjs-style CRDT over rope | Mature CRDT library with Rust port; fits rope model. |
| Terminal emulator | **alacritty_terminal** crate (vendored/forked) | Battle-tested VT parser, already in Rust. |
| Serialization / config | **TOML** (toml crate) | Human-friendly, widely used in Rust ecosystem. |
| Plugin sandbox | **Wasmtime** (WASM runtime) | Secure sandboxing, language-agnostic plugin authoring. |
| Build / CI | **cargo** + **cross** + GitHub Actions | Native Rust toolchain; cross-compilation for all targets. |

---

## 3. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         SMASH Process                           │
│                                                                 │
│  ┌────────────┐   ┌────────────┐   ┌─────────────────────────┐ │
│  │  Platform   │   │   Input    │   │      Renderer           │ │
│  │  Abstraction│──▶│  Handler   │──▶│  (TUI / GUI frontend)   │ │
│  │  Layer      │   │            │   │                         │ │
│  └────────────┘   └─────┬──────┘   └──────────▲──────────────┘ │
│                         │                      │                │
│                         ▼                      │                │
│             ┌───────────────────────┐          │                │
│             │     Editor Core       │──────────┘                │
│             │  ┌───────┐ ┌───────┐  │                          │
│             │  │Buffer │ │ Undo  │  │                          │
│             │  │(Rope) │ │ Tree  │  │                          │
│             │  └───────┘ └───────┘  │                          │
│             │  ┌───────┐ ┌───────┐  │                          │
│             │  │Cursor │ │Select │  │                          │
│             │  │Manager│ │Engine │  │                          │
│             │  └───────┘ └───────┘  │                          │
│             └──────────┬────────────┘                          │
│                        │                                       │
│          ┌─────────────┼──────────────┐                        │
│          ▼             ▼              ▼                         │
│  ┌──────────────┐ ┌──────────┐ ┌───────────┐                  │
│  │  Tree-sitter  │ │   LSP    │ │  Terminal  │                  │
│  │  Highlighter  │ │  Client  │ │  Emulator  │                  │
│  └──────────────┘ └──────────┘ └───────────┘                  │
│          │             │              │                         │
│          ▼             ▼              ▼                         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Async Service Bus (tokio)                    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  │  │
│  │  │   File   │ │   LSP    │ │  Collab   │ │   Remote   │  │  │
│  │  │   I/O    │ │  Servers │ │  (CRDT)   │ │   Agent    │  │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Config      │  │   DAP Client │  │  Plugin Host (WASM)  │  │
│  │  Manager     │  │  (Debugger)  │  │                      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.1 Module Responsibilities

| Module | Responsibility | Key Interfaces |
|---|---|---|
| **Platform Abstraction** | OS-specific file paths, process spawning, clipboard, signal handling | `trait Platform` |
| **Input Handler** | Keyboard/mouse event normalization, keybinding resolution, command dispatch, IME composition lifecycle management (preedit/commit) | `Event → Command` mapping, `ImeState` |
| **Editor Core** | Buffer management (rope), cursor/selection state (with position clamping), undo tree, edit operations | `Buffer`, `EditCommand`, `UndoTree` |
| **Renderer** | Viewport calculation, styled line layout, diff-based terminal updates | `trait Renderer` (TUI impl, future GUI impl) |
| **Tree-sitter Highlighter** | Incremental parse, highlight query execution, token span output | `highlight(buffer, edit_range) → Spans` |
| **LSP Client** | JSON-RPC transport, capability negotiation, request/response dispatch | `LspClient` per server instance |
| **Terminal Emulator** | PTY management, VT sequence parsing, grid state | `TerminalPane` |
| **Collaboration (CRDT)** | Document state replication, operation transform, peer awareness | `CollabSession`, `CrdtDocument` |
| **Remote Agent** | SSH connection management, remote file ops, LSP/terminal proxying | `RemoteSession` |
| **DAP Client** | Debug Adapter Protocol transport, breakpoint/state management | `DebugSession` |
| **Config Manager** | TOML parsing, schema validation, live reload, merge (global + project) | `Config` struct with typed fields |
| **Plugin Host** | WASM runtime management, API surface exposition, lifecycle control | `PluginManifest`, `PluginApi` trait |

---

## 4. Data Flow

### 4.1 Edit Cycle (Main Loop)

```
1.  Platform polls input events (key, mouse, resize, IME composition)
2.  Input Handler checks IME state:
    2a. If IME preedit event → update ImeState (composition string + cursor + segments)
        → skip keybinding resolution, notify Renderer to display preedit overlay
    2b. If IME commit event → extract committed string, clear ImeState,
        proceed as a regular insert command
    2c. If regular key event and no active composition → resolve keybinding → Command
3.  Command dispatched to Editor Core
4.  Editor Core applies edit to Buffer (rope mutation)
5.  Undo Tree records operation (IME commit = single atomic undo entry)
6.  Tree-sitter Highlighter receives edit notification → incremental re-parse
7.  LSP Client notified of didChange → sends to language server
8.  Collaboration engine (if active) broadcasts CRDT operation
9.  Renderer reads buffer + highlights + diagnostics + ImeState → produces frame diff
    (preedit string rendered inline with composition styling)
10. Platform flushes frame diff to terminal
    (reports cursor position to OS IME API for candidate window placement)
```

### 4.2 LSP Request Cycle

```
User action (e.g., hover) → LSP Client sends request →
Language Server processes → LSP Client receives response →
Editor Core stores result → Renderer displays (tooltip / inline)
```

---

## 5. Directory / Crate Structure

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
├── src/
│   └── main.rs             # Binary entry point — wires crates together
├── grammars/               # Vendored or git-submoduled Tree-sitter grammars
├── themes/                 # Built-in theme files (.toml)
├── doc/
│   └── requirements/
│       ├── smash-requirements.md
│       ├── detail-requirements.md
│       └── design-plan.md
├── tests/                  # Integration / end-to-end tests
└── .github/
    └── workflows/          # CI pipelines
```

---

## 6. Key Design Decisions

### 6.1 Rope-Based Buffer
The core buffer uses a **rope** data structure:
- O(log n) inserts and deletes at arbitrary positions.
- Efficient line-index and byte-offset conversion.
- Natural integration with Tree-sitter (provide byte ranges to re-parse).
- Can be extended to support CRDT operations by tagging leaves with site-id/sequence.

### 6.2 Incremental Rendering
The TUI renderer maintains a **shadow screen buffer**. On each frame:
1. Compute the visible line range from viewport state.
2. Gather styled spans (syntax + diagnostics + search highlights + cursors).
3. Diff against the previous shadow buffer.
4. Emit only changed cells to the terminal.

This minimizes I/O and achieves ≤ 16 ms frame times even over SSH.

### 6.3 Async Architecture
- **Main thread**: input polling + rendering (tight synchronous loop).
- **Tokio runtime** (separate thread pool): all I/O-bound work — file reads/writes, LSP communication, collaboration sync, remote agent.
- Communication between main thread and async tasks uses **bounded channels** (`tokio::sync::mpsc`) to prevent backpressure issues.

### 6.4 CRDT Integration
- Each buffer can optionally be backed by a CRDT document.
- Local edits produce CRDT operations that are broadcast to peers.
- Remote operations are merged into the local rope, with cursor positions rebased.
- When collaboration is not active, the CRDT layer has zero overhead (trait-gated).

### 6.5 Plugin Sandboxing (WASM)
- Plugins compile to WASI and run inside Wasmtime.
- The host exposes a narrow API via WASM imports (read buffer, register commands, show UI).
- Plugins cannot access the filesystem or network unless explicitly granted permissions in the manifest.
- A crashed plugin is terminated without affecting the editor.

### 6.6 Japanese IME Handling
IME input follows a **composition lifecycle** distinct from regular keystroke processing:

1. **Preedit Start**: The platform signals that an IME composition session has begun. The Input Handler enters IME mode and suppresses normal keybinding resolution.
2. **Preedit Update**: The platform provides the current composition string (e.g., uncommitted hiragana/katakana), segment attributes (unconverted, converted, active clause), and an in-composition cursor offset. The Renderer displays this preedit text inline at the buffer cursor with distinct styling (underline, highlight per segment).
3. **Commit**: The platform delivers the final converted string (e.g., kanji). The Input Handler creates a single `InsertText` command. The Undo Tree records this as one atomic operation.
4. **Cancel**: The user cancels composition (e.g., Escape). The preedit overlay is removed; the buffer is unchanged.

**TUI mode**: IME composition is handled by the host terminal emulator. The editor receives already-committed multi-byte UTF-8 sequences via standard input. The editor must correctly handle variable-length UTF-8, fullwidth character column widths (`wcwidth`), and avoid splitting multi-byte sequences across reads.

**GUI mode (future)**: The editor integrates directly with platform IME APIs:
- **macOS**: NSTextInputClient protocol (IMKit).
- **Linux**: IBus / Fcitx via DBus or Wayland `text-input-v3` protocol.
- **Windows**: TSF (Text Services Framework) or legacy IMM32.

The `Renderer` reports the cursor's screen-space coordinates to the platform layer on every frame so the OS can position the IME candidate window correctly, even during scrolling or layout changes.

**Wide character handling**: CJK characters occupy two terminal columns. The Renderer's column-width calculation uses Unicode East Asian Width properties (UAX #11) to ensure correct alignment. The rope's column-index API accounts for double-width characters when computing cursor positions.

---

## 7. Phased Delivery Plan

### Phase 1 — Core Editor (MVP)
**Goal**: A usable terminal text editor with basic editing, syntax highlighting, and file I/O.

| Deliverable | Requirements Covered |
|---|---|
| Rope buffer with undo tree | REQ-EDIT-001, 002 |
| TUI renderer (crossterm) | REQ-PERF-030, 031 |
| UTF-8 file open / save | REQ-EDIT-010, 011 |
| Keyboard input + default keybindings | REQ-KEY-001, 004 |
| Tree-sitter syntax highlighting (5 languages) | REQ-SYN-001, 002, 003 (partial) |
| Config file loading (TOML) | REQ-CONF-001 |
| Large file support (mmap) | REQ-PERF-020, 021, 022 |
| Multiple buffers / splits | REQ-EDIT-003, REQ-THEME-004 |
| Find & replace (plain + regex) | REQ-EDIT-021 |
| Japanese IME support (TUI: terminal passthrough, wide-char rendering) | REQ-IME-001 – 004, 020 – 022, 031, 043 |
| Cross-platform builds (Linux, macOS, Windows) | REQ-PLAT-001, 010, 011 |

**Target**: 3 months

---

### Phase 2 — LSP & Navigation
**Goal**: Full language intelligence and code navigation.

| Deliverable | Requirements Covered |
|---|---|
| LSP client (stdio transport) | REQ-LSP-001, 002, 003 |
| Diagnostics, completion, hover | REQ-LSP-010, 011, 012 |
| Go to definition, find references | REQ-NAV-001, 002 |
| Document symbols / outline | REQ-NAV-003 |
| Fuzzy file finder | REQ-NAV-004 |
| Remaining Tree-sitter grammars | REQ-SYN-003 (complete) |
| Per-project config | REQ-CONF-002 |
| Additional keybinding presets (Vim mode) | REQ-KEY-003 |

**Target**: 2 months (cumulative: 5 months)

---

### Phase 3 — Terminal & Debugging
**Goal**: Integrated terminal and debug adapter support.

| Deliverable | Requirements Covered |
|---|---|
| Embedded terminal emulator | REQ-TERM-001 – 006 |
| DAP client | REQ-DEBUG-001 – 005 |
| Multi-cursor editing | REQ-EDIT-020 |
| Column selection | REQ-EDIT-022 |
| Crash recovery / swap files | REQ-NFR-001, 002 |
| Logging subsystem | REQ-NFR-020, 021 |

**Target**: 2 months (cumulative: 7 months)

---

### Phase 4 — Collaboration & Remote
**Goal**: Real-time collaboration and remote development.

| Deliverable | Requirements Covered |
|---|---|
| CRDT document engine | REQ-COLLAB-001, 002, 006 |
| Collaboration signaling server | REQ-COLLAB-005 |
| Peer cursor display | REQ-COLLAB-003 |
| E2EE transport | REQ-COLLAB-004 |
| SSH remote agent | REQ-REMOTE-001, 002, 004, 005 |
| WSL integration | REQ-REMOTE-003 |
| Custom themes | REQ-THEME-001 – 003 |
| Live config reload | REQ-CONF-003 |

**Target**: 3 months (cumulative: 10 months)

---

### Phase 5 — Plugins & Polish
**Goal**: Extension system, GUI mode exploration, and quality hardening.

| Deliverable | Requirements Covered |
|---|---|
| WASM plugin host | REQ-PLUG-001 – 004 |
| Plugin manager UI | REQ-PLUG-004 |
| GUI frontend prototype (wgpu) | REQ-PLAT-021 |
| Accessibility improvements | REQ-NFR-010, 011 |
| Encoding auto-detection | REQ-EDIT-012 |
| Config schema validation + docs | REQ-CONF-004 |
| Full keybinding customization | REQ-KEY-002 |
| Test coverage ≥ 80% | REQ-NFR-030, 031 |

**Target**: 3 months (cumulative: 13 months)

---

## 8. Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| Rope + CRDT integration complexity | High | Medium | Prototype in Phase 1; use proven Automerge library. |
| Tree-sitter grammar quality varies | Medium | Medium | Vendor grammars, pin versions, maintain patches. |
| 500 KB memory budget too tight with tokio runtime | High | Medium | Profile early; consider `smol` or manual async if needed. |
| Terminal emulator edge cases (escape sequences) | Medium | High | Fork alacritty_terminal; fuzz test. |
| WASM plugin API stability | Medium | Low | Defer to Phase 5; iterate API with internal plugins first. |
| Cross-platform CI flakiness | Low | High | Use deterministic container-based CI; gate merges on all 3 OS. |

---

## 9. Success Metrics

| Metric | Target | How Measured |
|---|---|---|
| Cold startup time | ≤ 200 ms | `hyperfine` benchmark on CI |
| RSS at startup (empty buffer) | < 500 KB | `/proc/self/status` / `mach_task_info` |
| Frame render time | ≤ 16 ms | Internal instrumentation + `perf` |
| Input latency | ≤ 50 ms | End-to-end measurement with `typometer` |
| 100 MB file open | viewport in ≤ 500 ms | Automated benchmark |
| LSP completion latency | ≤ 100 ms | Instrumented test with rust-analyzer |
| Test coverage | ≥ 80% on core crates | `cargo tarpaulin` |
| Collaboration edit latency | ≤ 200 ms (100 ms RTT) | Simulated network test |
