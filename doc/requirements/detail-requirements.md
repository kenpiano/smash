# SMASH — Detailed Requirements

This document expands the high-level requirements defined in `smash-requirements.md` into concrete, measurable specifications organized by functional area.

---

## 1. Performance Requirements

### 1.1 Startup Time
- **REQ-PERF-001**: Cold startup (no file argument) SHALL complete in ≤ 200 ms on a machine with ≥ 4 GB RAM and an SSD.
- **REQ-PERF-002**: Warm startup (file argument, file ≤ 1 MB) SHALL complete in ≤ 300 ms, with the first screen of content rendered and interactive.
- **REQ-PERF-003**: Startup SHALL NOT block on plugin loading or LSP server initialization; these SHOULD proceed asynchronously after the first frame is rendered.

### 1.2 Memory Usage
- **REQ-PERF-010**: Resident memory on startup (empty buffer, no plugins) SHALL be < 500 KB.
- **REQ-PERF-011**: Per-buffer overhead for a 1 MB file SHALL be ≤ 2× the file size (including undo history and syntax metadata).
- **REQ-PERF-012**: The editor SHALL use memory-mapped I/O or streaming for files > 10 MB to avoid loading the entire file into heap memory.

### 1.3 Large File Support
- **REQ-PERF-020**: The editor SHALL open files ≥ 100 MB without UI freeze (rendering the visible viewport within 500 ms).
- **REQ-PERF-021**: Basic operations (scroll, search, go-to-line) on 100 MB+ files SHALL respond within 100 ms.
- **REQ-PERF-022**: Syntax highlighting MAY be disabled or limited to the visible viewport for files > 50 MB to maintain responsiveness.

### 1.4 Rendering
- **REQ-PERF-030**: Frame rendering time for typical editing operations (typing, scrolling) SHALL be ≤ 16 ms (60 fps target).
- **REQ-PERF-031**: Input-to-screen latency SHALL be ≤ 50 ms on supported platforms.

---

## 2. Cross-Platform Requirements

### 2.1 Supported Platforms
- **REQ-PLAT-001**: The editor SHALL run natively on:
  - Linux (x86_64, aarch64) — glibc ≥ 2.17
  - macOS (x86_64, aarch64) — macOS 12+
  - Windows (x86_64) — Windows 10+
- **REQ-PLAT-002**: A single codebase SHALL be used for all platforms. Platform-specific code SHOULD be isolated behind abstraction layers.

### 2.2 Build & Distribution
- **REQ-PLAT-010**: The project SHALL produce statically-linked or self-contained binaries for each target platform.
- **REQ-PLAT-011**: No runtime dependency on Java, Python, Node.js, or other managed runtimes SHALL be required.
- **REQ-PLAT-012**: Package formats SHALL include at minimum: `.tar.gz` (Linux), `.dmg` or Homebrew formula (macOS), `.zip` and `.msi` (Windows).

### 2.3 Terminal & GUI Modes
- **REQ-PLAT-020**: The editor SHALL provide a terminal (TUI) mode that works over SSH and in any POSIX-compatible terminal emulator.
- **REQ-PLAT-021**: A native GUI mode SHOULD be provided as a secondary target, sharing the core editing engine with the TUI mode.

---

## 3. Text Editing Core

### 3.1 Buffer Model
- **REQ-EDIT-001**: The buffer data structure SHALL support efficient insert, delete, and random access (e.g., piece table, rope, or gap buffer).
- **REQ-EDIT-002**: Undo/redo SHALL support unlimited history (bounded by available memory) with branching (undo tree).
- **REQ-EDIT-003**: The editor SHALL support multiple simultaneous buffers (tabs / splits).

### 3.2 Encoding & Line Endings
- **REQ-EDIT-010**: The editor SHALL support UTF-8 as the primary encoding.
- **REQ-EDIT-011**: The editor SHALL detect and preserve line ending style (LF, CRLF, CR).
- **REQ-EDIT-012**: The editor SHOULD support other encodings (Latin-1, Shift-JIS, etc.) via explicit selection or auto-detection.

### 3.3 Editing Features
- **REQ-EDIT-019**: Vertical cursor movement (up, down, page-up, page-down) SHALL clamp the cursor column to the length of the target line to prevent out-of-bounds cursor positions.
- **REQ-EDIT-020**: Multiple cursors / multi-selection editing SHALL be supported.
- **REQ-EDIT-021**: Find & replace SHALL support plain text and regular expressions, with match highlighting.
- **REQ-EDIT-022**: The editor SHALL support rectangular (column) selection.
- **REQ-EDIT-023**: Auto-indent SHALL respect the detected or configured indentation style (spaces/tabs, width).
- **REQ-EDIT-024**: Bracket matching and auto-closing of paired characters SHALL be supported.

---

## 4. Syntax Highlighting

### 4.1 Highlighting Engine
- **REQ-SYN-001**: Syntax highlighting SHALL use Tree-sitter grammars for incremental, accurate parsing.
- **REQ-SYN-002**: Highlighting SHALL update incrementally on each edit without re-parsing the entire file.
- **REQ-SYN-003**: At minimum, the following languages SHALL be supported at launch: C, C++, Rust, Go, Python, JavaScript, TypeScript, Java, Ruby, Shell (bash/zsh), Markdown, JSON, YAML, TOML, HTML, CSS.

### 4.2 Theme Integration
- **REQ-SYN-010**: Highlight colors SHALL be driven by the active color theme (see §8 Theming).
- **REQ-SYN-011**: Users SHALL be able to override highlight rules per language via configuration.

---

## 5. Language Server Protocol (LSP)

### 5.1 Client Implementation
- **REQ-LSP-001**: The editor SHALL implement the LSP client specification (v3.17+).
- **REQ-LSP-002**: LSP servers SHALL be launched and managed automatically based on the detected file type and user configuration.
- **REQ-LSP-003**: The editor SHALL support multiple concurrent LSP servers (one per language / workspace).

### 5.2 Supported Capabilities
- **REQ-LSP-010**: The following LSP capabilities SHALL be supported:
  - Diagnostics (inline errors and warnings)
  - Completion (with snippet support)
  - Hover information
  - Go to Definition / Declaration / Implementation
  - Find References
  - Rename Symbol
  - Code Actions (quick fixes, refactors)
  - Signature Help
  - Document Symbols / Workspace Symbols
  - Formatting (document and range)
- **REQ-LSP-011**: Diagnostics SHALL be rendered inline with severity indicators and gutter icons.
- **REQ-LSP-012**: Completion SHALL display within 100 ms of trigger, with fuzzy matching.

---

## 6. Code Navigation

- **REQ-NAV-001**: Go to Definition SHALL open the target file and position the cursor, with breadcrumb support for navigating back.
- **REQ-NAV-002**: Find References SHALL display results in a searchable list with file path and line preview.
- **REQ-NAV-003**: Document Outline / Symbol List SHALL be available via a sidebar or command palette.
- **REQ-NAV-004**: File Finder (fuzzy file open) SHALL index the workspace and return results within 100 ms for projects with ≤ 100,000 files.
- **REQ-NAV-005**: Go to Line/Column SHALL be available via shortcut and command palette.

---

## 7. Integrated Terminal Emulator

- **REQ-TERM-001**: The editor SHALL embed a terminal emulator pane (split or panel).
- **REQ-TERM-002**: The terminal SHALL support VT100/xterm-256color escape sequences.
- **REQ-TERM-003**: The terminal SHALL use the system default shell or a user-configured shell.
- **REQ-TERM-004**: Multiple terminal instances SHALL be supported simultaneously.
- **REQ-TERM-005**: Copy/paste between editor buffers and the terminal SHALL work seamlessly.
- **REQ-TERM-006**: The terminal SHOULD support hyperlink detection (file paths, URLs) with click-to-open.

---

## 8. Theming & UI Customization

- **REQ-THEME-001**: The editor SHALL ship with at least one dark and one light built-in theme.
- **REQ-THEME-002**: Themes SHALL be defined in a standard format (TOML or JSON) and loaded at startup.
- **REQ-THEME-003**: Users SHALL be able to create and install custom themes.
- **REQ-THEME-004**: The editor SHALL support split panes (horizontal and vertical) with resizable dividers.
- **REQ-THEME-005**: Status bar, line numbers, minimap (optional), and gutter indicators SHALL be configurable.

---

## 9. Debugging Tools

- **REQ-DEBUG-001**: The editor SHALL implement the Debug Adapter Protocol (DAP) client.
- **REQ-DEBUG-002**: Breakpoint management (set, remove, conditional, hit-count) SHALL be supported in the gutter.
- **REQ-DEBUG-003**: During a debug session, the editor SHALL display:
  - Call stack
  - Local / watch variables
  - Inline variable values (optional)
- **REQ-DEBUG-004**: Step-over, step-into, step-out, continue, and pause controls SHALL be accessible via shortcuts and UI.
- **REQ-DEBUG-005**: Debug configurations SHALL be defined per-project in a configuration file.

---

## 10. Real-Time Collaboration

- **REQ-COLLAB-001**: The editor SHALL support real-time collaborative editing with ≥ 2 concurrent users on the same buffer.
- **REQ-COLLAB-002**: Conflict resolution SHALL use a CRDT or OT algorithm to guarantee eventual consistency.
- **REQ-COLLAB-003**: Each collaborator's cursor and selection SHALL be visible with a distinct color and name label.
- **REQ-COLLAB-004**: Collaboration sessions SHALL be encrypted end-to-end (TLS 1.3 minimum for transport, optional E2EE for content).
- **REQ-COLLAB-005**: A relay/signaling server component SHALL be provided; direct peer-to-peer fallback is OPTIONAL.
- **REQ-COLLAB-006**: Latency between an edit on one client and its appearance on another SHALL be ≤ 200 ms under typical network conditions (≤ 100 ms RTT).

---

## 11. Remote Development

- **REQ-REMOTE-001**: The editor SHALL support opening and editing files on a remote host via SSH.
- **REQ-REMOTE-002**: Remote mode SHALL proxy LSP, terminal, and file operations transparently.
- **REQ-REMOTE-003**: The editor SHOULD support WSL integration on Windows (launching and connecting to a WSL instance).
- **REQ-REMOTE-004**: The remote agent/component installed on the host SHALL be lightweight (< 5 MB binary, minimal dependencies).
- **REQ-REMOTE-005**: Connection loss SHALL be handled gracefully with automatic reconnection and unsaved-change preservation.

---

## 12. Plugin / Extension System (Optional)

- **REQ-PLUG-001**: The editor SHOULD expose a plugin API for extending functionality (commands, UI panels, language support).
- **REQ-PLUG-002**: Plugins SHOULD run in a sandboxed environment (e.g., WASM or separate process) to prevent crashes from affecting the core editor.
- **REQ-PLUG-003**: A plugin manifest format SHALL define metadata, activation events, and dependencies.
- **REQ-PLUG-004**: The editor SHOULD provide a built-in plugin manager for installing, updating, and removing plugins.

---

## 13. Keybindings & Input

- **REQ-KEY-001**: The editor SHALL provide a default keybinding set.
- **REQ-KEY-002**: Users SHALL be able to remap any command to any key combination via a configuration file.
- **REQ-KEY-003**: The editor SHOULD ship with built-in keymap presets for common editors (Vim, Emacs) as optional modes.
- **REQ-KEY-004**: A command palette (fuzzy-searchable list of all commands) SHALL be accessible via a keyboard shortcut.

---

## 13A. Japanese IME (Input Method Editor) Support

### 13A.1 Composition Lifecycle
- **REQ-IME-001**: The editor SHALL support IME composition events (preedit start, preedit update, preedit end / commit) on all supported platforms (macOS, Linux, Windows).
- **REQ-IME-002**: During IME composition (preedit state), the editor SHALL display the uncommitted composition string inline at the cursor position with a distinct visual style (e.g., underline, highlighted background) to distinguish it from committed text.
- **REQ-IME-003**: The editor SHALL NOT interpret raw keystrokes as editing commands while an IME composition session is active. Only the final committed string SHALL be inserted into the buffer.
- **REQ-IME-004**: Pressing Escape or other cancel keys during composition SHALL discard the preedit text without modifying the buffer.

### 13A.2 Candidate Window & Cursor Positioning
- **REQ-IME-010**: The editor SHALL report the correct cursor screen coordinates to the platform IME API so that the candidate/conversion window appears adjacent to the composition point.
- **REQ-IME-011**: Cursor coordinate reporting SHALL remain accurate during scrolling, window resizing, and split-pane layout changes.
- **REQ-IME-012**: The candidate window positioning SHALL work correctly in both TUI mode (via terminal IME passthrough) and a future GUI mode (via native platform IME APIs).

### 13A.3 Rendering & Performance
- **REQ-IME-020**: Preedit string rendering SHALL complete within the ≤ 16 ms frame budget (no visual lag during composition).
- **REQ-IME-021**: The preedit display SHALL support styled segments (e.g., unconverted kana underlined, converted kanji with solid underline, currently selected clause highlighted) when the platform provides segment attributes.
- **REQ-IME-022**: Wide (fullwidth) characters produced by IME input SHALL be rendered with correct double-column-width handling, maintaining alignment of subsequent text on the same line.

### 13A.4 Integration with Editor Features
- **REQ-IME-030**: IME composition SHALL work correctly with multi-cursor editing; each cursor SHALL maintain its own independent composition state.
- **REQ-IME-031**: Undo SHALL treat a single IME commit as one atomic operation (one undo step reverses the entire committed string, not individual keystrokes within composition).
- **REQ-IME-032**: IME-committed text SHALL trigger LSP `textDocument/didChange` notifications and Tree-sitter incremental re-parse, identical to regular typed text.
- **REQ-IME-033**: IME composition SHALL function correctly inside the command palette and find/replace input fields, not only in buffer editing mode.

### 13A.5 Platform-Specific Considerations
- **REQ-IME-040**: On macOS, the editor SHALL integrate with the macOS Input Method Kit (IMKit) or the Cocoa text input protocol when running in GUI mode.
- **REQ-IME-041**: On Linux, the editor SHALL support at least one major IME framework: IBus, Fcitx, or Fcitx5 (via XIM, Wayland text-input protocol, or DBus interface as appropriate).
- **REQ-IME-042**: On Windows, the editor SHALL support the TSF (Text Services Framework) or IMM32 API for IME interaction in GUI mode.
- **REQ-IME-043**: In TUI mode, the editor SHALL delegate IME handling to the host terminal emulator; it SHALL correctly process multi-byte UTF-8 character sequences that result from terminal-level IME composition.

---

## 14. Configuration

- **REQ-CONF-001**: Global configuration SHALL be stored in a single file (TOML or JSON) at a platform-standard location (e.g., `~/.config/smash/config.toml`).
- **REQ-CONF-002**: Per-project configuration SHALL be supported via a `.smash/` directory in the project root.
- **REQ-CONF-003**: Configuration changes SHOULD be applied in real-time without restart.
- **REQ-CONF-004**: The configuration schema SHALL be documented and validated on load with clear error messages.

---

## 15. Non-Functional Requirements

### 15.1 Reliability
- **REQ-NFR-001**: The editor SHALL implement crash recovery by auto-saving buffer state periodically (default: every 30 s or on every edit).
- **REQ-NFR-002**: A swap / recovery file mechanism SHALL allow restoring unsaved work after a crash.

### 15.2 Accessibility
- **REQ-NFR-010**: The TUI mode inherently supports screen readers via terminal accessibility.
- **REQ-NFR-011**: The GUI mode SHOULD implement platform accessibility APIs (e.g., macOS Accessibility, Windows UI Automation).

### 15.3 Logging & Diagnostics
- **REQ-NFR-020**: The editor SHALL support configurable log levels (error, warn, info, debug, trace).
- **REQ-NFR-021**: Logs SHALL be written to a file at a standard location and rotated to prevent unbounded growth.

### 15.4 Testing
- **REQ-NFR-030**: The project SHALL maintain ≥ 80% unit test coverage on core modules (buffer, LSP client, terminal, CRDT).
- **REQ-NFR-031**: Integration tests SHALL cover end-to-end scenarios (open file → edit → save → verify) on all supported platforms.

---

## Traceability Matrix

| Area | Requirement IDs | Source (smash-requirements.md) |
|---|---|---|
| Performance | REQ-PERF-001 – 031 | Fast startup, Low memory, Large files |
| Platform | REQ-PLAT-001 – 021 | Cross-platform |
| Editing | REQ-EDIT-001 – 024 | (core editor expectation) |
| Syntax | REQ-SYN-001 – 011 | Syntax highlights |
| LSP | REQ-LSP-001 – 012 | LSP support |
| Navigation | REQ-NAV-001 – 005 | Advanced code navigation |
| Terminal | REQ-TERM-001 – 006 | Integrated terminal emulator |
| Theming | REQ-THEME-001 – 005 | Customizable UI themes and layouts |
| Debugging | REQ-DEBUG-001 – 005 | Built-in debugging tools |
| Collaboration | REQ-COLLAB-001 – 006 | Real-time collaboration |
| Remote Dev | REQ-REMOTE-001 – 005 | Remote development (SSH, WSL) |
| Plugins | REQ-PLUG-001 – 004 | Extensible through plugins (optional) |
| Keybindings | REQ-KEY-001 – 004 | Customizable keybindings (optional) |
| Japanese IME | REQ-IME-001 – 043 | Japanese IME support |
| Configuration | REQ-CONF-001 – 004 | (implied by all configurable features) |
| Non-Functional | REQ-NFR-001 – 031 | (implied quality attributes) |
