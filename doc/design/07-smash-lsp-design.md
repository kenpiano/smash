# smash-lsp — Module Design

## 1. Overview

`smash-lsp` implements the Language Server Protocol (LSP) client. It manages the lifecycle of language servers, handles JSON-RPC communication over stdio, and dispatches responses to the editor core for display.

**Phase**: 2

**Requirements Covered**: REQ-LSP-001 – 012, REQ-NAV-001 – 005

---

## 2. Public API Surface

### 2.1 Core Types

```rust
LspClient            — manages a single LSP server process
LspClientId          — opaque handle for a running server
LspRegistry          — manages multiple LspClient instances by language
LspServerConfig      — server command, args, initialization options
LspCapabilities      — negotiated capability set for a server

// Request/Response types (mirrors LSP spec)
CompletionItem       — { label, detail, kind, insert_text, ... }
Diagnostic           — { range, severity, message, source, code }
Hover                — { contents: MarkupContent, range }
Location             — { uri, range }
SymbolInformation    — { name, kind, location }
CodeAction           — { title, kind, edit, command }
TextEdit             — { range, new_text }
```

### 2.2 Key Functions

```rust
/// Start a language server for a given language
pub async fn start_server(&mut self, config: LspServerConfig)
    -> Result<LspClientId, LspError>

/// Send textDocument/didOpen notification
pub async fn did_open(&self, id: LspClientId, uri: &str, text: &str, language: &str)
    -> Result<(), LspError>

/// Send textDocument/didChange notification
pub async fn did_change(&self, id: LspClientId, uri: &str, changes: Vec<TextEdit>)
    -> Result<(), LspError>

/// Request completions
pub async fn completion(&self, id: LspClientId, uri: &str, position: Position)
    -> Result<Vec<CompletionItem>, LspError>

/// Request hover info
pub async fn hover(&self, id: LspClientId, uri: &str, position: Position)
    -> Result<Option<Hover>, LspError>

/// Request go-to-definition
pub async fn goto_definition(&self, id: LspClientId, uri: &str, position: Position)
    -> Result<Vec<Location>, LspError>

/// Request find-references
pub async fn find_references(&self, id: LspClientId, uri: &str, position: Position)
    -> Result<Vec<Location>, LspError>

/// Request code actions
pub async fn code_action(&self, id: LspClientId, uri: &str, range: Range, diagnostics: Vec<Diagnostic>)
    -> Result<Vec<CodeAction>, LspError>

/// Request document formatting
pub async fn format(&self, id: LspClientId, uri: &str)
    -> Result<Vec<TextEdit>, LspError>

/// Request rename
pub async fn rename(&self, id: LspClientId, uri: &str, position: Position, new_name: &str)
    -> Result<WorkspaceEdit, LspError>

/// Shutdown a language server
pub async fn shutdown(&mut self, id: LspClientId) -> Result<(), LspError>
```

---

## 3. Internal Architecture

```
┌──────────────────────────────────────────────────────┐
│                    smash-lsp                         │
│                                                      │
│  ┌──────────────────────────────────────────────┐    │
│  │              LspRegistry                      │    │
│  │                                               │    │
│  │  lang_id → LspClient                         │    │
│  │  ┌─────────────────────────────────────────┐  │    │
│  │  │           LspClient                     │  │    │
│  │  │                                         │  │    │
│  │  │  ┌──────────┐   ┌───────────────────┐   │  │    │
│  │  │  │ Process  │   │ JSON-RPC Transport │   │  │    │
│  │  │  │ (child)  │◀─▶│ (stdio reader/     │   │  │    │
│  │  │  │          │   │  writer tasks)     │   │  │    │
│  │  │  └──────────┘   └────────┬──────────┘   │  │    │
│  │  │                          │              │  │    │
│  │  │              ┌───────────┴───────────┐  │  │    │
│  │  │              │ Request/Response       │  │  │    │
│  │  │              │ Dispatcher             │  │  │    │
│  │  │              │ (id → pending oneshot) │  │  │    │
│  │  │              └───────────────────────┘  │  │    │
│  │  │                                         │  │    │
│  │  │  ┌────────────────────┐                 │  │    │
│  │  │  │ Capabilities       │                 │  │    │
│  │  │  │ (negotiated at     │                 │  │    │
│  │  │  │  initialize)       │                 │  │    │
│  │  │  └────────────────────┘                 │  │    │
│  │  └─────────────────────────────────────────┘  │    │
│  └──────────────────────────────────────────────┘    │
│                                                      │
│  ┌──────────────────────────────────────────────┐    │
│  │          Diagnostics Collector                │    │
│  │                                               │    │
│  │  Receives textDocument/publishDiagnostics     │    │
│  │  notifications → stores per-file → notifies   │    │
│  │  editor core via channel                      │    │
│  └──────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────┘
```

### 3.1 JSON-RPC Transport

- Two async tasks per server:
  - **Reader**: reads from server stdout, parses JSON-RPC messages, routes to dispatcher.
  - **Writer**: receives outbound messages from a channel, serializes, writes to server stdin.
- Uses `Content-Length` header framing per LSP spec.

### 3.2 Request/Response Dispatcher

- Each outbound request gets a unique integer ID and a `oneshot::Sender`.
- Incoming responses are matched by ID and sent to the waiting oneshot.
- Timeout: requests without a response within 10 seconds are canceled with an error.

### 3.3 Server Lifecycle

1. **Spawn**: `std::process::Command` with stdin/stdout pipes.
2. **Initialize**: send `initialize` request with client capabilities → receive server capabilities.
3. **Initialized**: send `initialized` notification.
4. **Running**: handle requests and notifications bidirectionally.
5. **Shutdown**: send `shutdown` request → `exit` notification → wait for process.

### 3.4 Diagnostic Handling

- Server sends asynchronous `textDocument/publishDiagnostics` notifications.
- The client stores diagnostics per-URI in a concurrent map.
- A channel pushes diagnostic updates to the editor core for rendering.

### 3.5 Completion Handling

- Triggered by user action or trigger characters (`.`, `::`, etc.).
- Supports `textDocument/completion` and `completionItem/resolve`.
- Results are passed to the TUI for rendering a completion menu.
- Fuzzy filtering is done client-side for responsive re-filtering.

### 3.6 Progress Reporting

- Handles `$/progress` notifications for long-running operations (indexing, building).
- Progress info is forwarded to the status bar via the editor core.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("server failed to start: {0}")]
    SpawnFailed(String),

    #[error("server initialization failed: {0}")]
    InitFailed(String),

    #[error("JSON-RPC error {code}: {message}")]
    Rpc { code: i32, message: String },

    #[error("request timed out after {0} seconds")]
    Timeout(u64),

    #[error("server process exited unexpectedly")]
    ServerCrashed,

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `serde_json` | JSON serialization for JSON-RPC |
| `serde` | Derive serialization |
| `tokio` | Async runtime (process I/O, channels, timeouts) |
| `lsp-types` | LSP type definitions (or custom if too heavy) |
| `smash-core` | Position, Range, BufferId |
| `thiserror` | Error derivation |

---

## 6. Module File Layout

```
crates/smash-lsp/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── client.rs           # LspClient struct and methods
    ├── registry.rs         # LspRegistry (multi-server management)
    ├── transport.rs        # JSON-RPC reader/writer tasks
    ├── dispatcher.rs       # Request ID → oneshot mapping
    ├── capabilities.rs     # Capability negotiation
    ├── diagnostics.rs      # Diagnostic collection and notification
    ├── completion.rs       # Completion request/resolve
    ├── navigation.rs       # Definition, references, symbols
    ├── formatting.rs       # Document/range formatting
    ├── progress.rs         # $/progress handling
    ├── types.rs            # LSP type definitions (or re-exports)
    └── error.rs            # LspError
```
