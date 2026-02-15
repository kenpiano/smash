# smash-dap — Module Design

## 1. Overview

`smash-dap` implements a Debug Adapter Protocol (DAP) client that communicates with debug adapters to provide breakpoints, stepping, variable inspection, and call-stack display inside the SMASH editor.

**Phase**: 3

**Requirements Covered**: REQ-DEBUG-001 – 005

---

## 2. Public API Surface

### 2.1 Core Types

```rust
DapClient            — represents a connection to a single debug adapter
DapSession           — active debug session state
Breakpoint           — { id, source_path, line, condition?, hit_condition?, log_message? }
StackFrame           — { id, name, source, line, column, module? }
Variable             — { name, value, type?, variables_reference }
Scope                — { name, variables_reference, expensive? }
Thread               — { id, name }
StopReason           — enum { Breakpoint, Step, Exception, Pause, Entry }
DapEvent             — enum { Stopped, Continued, Exited, Output, Breakpoint, Thread, ... }
LaunchConfig         — adapter launch/attach configuration
```

### 2.2 Key Functions

```rust
/// Start a debug adapter process and initialize the session
pub async fn launch(config: &LaunchConfig) -> Result<DapSession, DapError>

/// Attach to a running process
pub async fn attach(config: &LaunchConfig) -> Result<DapSession, DapError>

/// Set breakpoints for a source file
pub async fn set_breakpoints(
    &mut self,
    source: &Path,
    breakpoints: &[Breakpoint],
) -> Result<Vec<Breakpoint>, DapError>

/// Continue execution
pub async fn continue_execution(&mut self, thread_id: i64) -> Result<(), DapError>

/// Step over / into / out
pub async fn step_over(&mut self, thread_id: i64) -> Result<(), DapError>
pub async fn step_into(&mut self, thread_id: i64) -> Result<(), DapError>
pub async fn step_out(&mut self, thread_id: i64) -> Result<(), DapError>

/// Pause execution
pub async fn pause(&mut self, thread_id: i64) -> Result<(), DapError>

/// Get all threads
pub async fn threads(&mut self) -> Result<Vec<Thread>, DapError>

/// Get stack trace for a thread
pub async fn stack_trace(&mut self, thread_id: i64) -> Result<Vec<StackFrame>, DapError>

/// Get scopes for a stack frame
pub async fn scopes(&mut self, frame_id: i64) -> Result<Vec<Scope>, DapError>

/// Get variables for a scope or parent variable
pub async fn variables(&mut self, reference: i64) -> Result<Vec<Variable>, DapError>

/// Evaluate an expression in the current context
pub async fn evaluate(
    &mut self,
    expression: &str,
    frame_id: Option<i64>,
) -> Result<Variable, DapError>

/// Stop the debug session
pub async fn disconnect(&mut self) -> Result<(), DapError>

/// Subscribe to debug events
pub fn events(&self) -> tokio::sync::broadcast::Receiver<DapEvent>
```

---

## 3. Internal Architecture

```
┌─────────────────────────────────────────────────────┐
│                     DapSession                       │
│                                                      │
│  ┌───────────────┐     ┌──────────────────────────┐  │
│  │  RequestQueue  │     │    Event Stream           │  │
│  │                │     │                           │  │
│  │  seq tracking  │     │  broadcast::Sender<Event> │  │
│  │  pending map   │     │                           │  │
│  └──────┬────────┘     └───────────▲───────────────┘  │
│         │                          │                  │
│         ▼                          │                  │
│  ┌──────────────────────────────────────────────┐    │
│  │            DapTransport                       │    │
│  │                                               │    │
│  │  stdin writer ──▶ Debug Adapter ──▶ stdout    │    │
│  │                     (child process)    reader  │    │
│  │                                               │    │
│  │  Content-Length framing (DAP base protocol)   │    │
│  └──────────────────────────────────────────────┘    │
│                                                      │
│  ┌───────────────────────────────────────────┐       │
│  │         Capability Negotiator              │       │
│  │                                            │       │
│  │  Tracks adapter capabilities from          │       │
│  │  initialize response                       │       │
│  └───────────────────────────────────────────┘       │
│                                                      │
│  ┌───────────────────────────────────────────┐       │
│  │         Breakpoint Manager                 │       │
│  │                                            │       │
│  │  Local cache → verified breakpoint sync    │       │
│  └───────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────┘
```

### 3.1 DAP Transport

- Debug adapters communicate over stdin/stdout using the DAP base protocol (Content-Length headers + JSON body).
- Two tokio tasks: **reader** (parses incoming JSON) and **writer** (serializes outgoing JSON).
- Sequence numbers track request ↔ response pairing; a `HashMap<i64, oneshot::Sender<ResponseBody>>` resolves pending requests.
- Timeout: 10 seconds default on requests.

### 3.2 Session Lifecycle

1. **Initialize**: Send `initialize` request → negotiate capabilities.
2. **Launch/Attach**: Send `launch` or `attach` request.
3. **Configure Breakpoints**: Send `setBreakpoints` for each source file.
4. **ConfigurationDone**: Signal adapter that configuration is complete.
5. **Running**: Respond to `stopped` events, allow step/continue/evaluate.
6. **Disconnect**: Send `disconnect` to terminate.

### 3.3 Breakpoint Manager

- Maintains a local set of breakpoints per source file.
- Syncs with adapter after set operations; updates `verified` status from adapter response.
- Supports conditional breakpoints, hit-count breakpoints, and logpoints.

### 3.4 Event Handling

- The transport reader dispatches events to a `broadcast::Sender<DapEvent>`.
- UI components subscribe to events for updating debug panels.
- Key events: `stopped` (hit breakpoint), `continued`, `exited`, `output` (debug console output).

### 3.5 Adapter Discovery

- Launch configurations are defined in `.smash/launch.json` (or embedded in `config.toml`).
- Specifies adapter executable, argument list, and request type (launch/attach).
- Phase 3 supports manually configured adapters; Phase 5 could add auto-detection.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum DapError {
    #[error("adapter failed to start: {0}")]
    AdapterSpawnFailed(#[from] std::io::Error),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("request timed out: {command}")]
    Timeout { command: String },

    #[error("adapter rejected request: {message}")]
    Rejected { message: String },

    #[error("adapter sent invalid response: {0}")]
    InvalidResponse(String),

    #[error("session not initialized")]
    NotInitialized,

    #[error("session already terminated")]
    Terminated,
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `serde`, `serde_json` | DAP JSON serialization |
| `tokio` | Async I/O, channels |
| `thiserror` | Error derivation |
| `tracing` | Logging |

---

## 6. Module File Layout

```
crates/smash-dap/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── client.rs           # DapClient — high-level API
    ├── session.rs          # DapSession state machine
    ├── transport.rs        # DAP base protocol (Content-Length framing)
    ├── protocol.rs         # DAP request/response/event types (serde)
    ├── breakpoint.rs       # Breakpoint manager
    ├── capabilities.rs     # Capability negotiation
    └── error.rs            # DapError
```
