# smash-remote — Module Design

## 1. Overview

`smash-remote` enables remote development by running a lightweight agent on a remote host (via SSH) while the local SMASH instance handles rendering. It also supports WSL integration on Windows.

**Phase**: 4

**Requirements Covered**: REQ-REMOTE-001 – 005

---

## 2. Public API Surface

### 2.1 Core Types

```rust
RemoteSession        — active connection to a remote host
RemoteConfig         — { host, port?, user?, identity_file?, agent_path? }
RemoteAgent          — the agent binary running on the remote side
AgentMessage         — enum of request/response messages between local ↔ agent
FileContent          — { path, content: Vec<u8>, encoding }
RemoteFileSystem     — trait for remote FS operations
RemoteStatus         — enum { Connecting, Connected, Reconnecting, Disconnected }
```

### 2.2 Key Functions

```rust
/// Connect to a remote host via SSH
pub async fn connect(config: &RemoteConfig) -> Result<RemoteSession, RemoteError>

/// Deploy or update the remote agent binary
pub async fn ensure_agent(&mut self) -> Result<(), RemoteError>

/// Read a file from the remote host
pub async fn read_file(&self, path: &Path) -> Result<FileContent, RemoteError>

/// Write a file to the remote host
pub async fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), RemoteError>

/// List directory contents
pub async fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, RemoteError>

/// Execute a command on the remote host
pub async fn exec(&self, command: &str) -> Result<ExecResult, RemoteError>

/// Open a port-forwarded connection (for LSP, DAP, etc.)
pub async fn forward_port(&self, remote_port: u16) -> Result<u16, RemoteError>

/// Get the remote agent's status
pub fn status(&self) -> RemoteStatus

/// Start a terminal on the remote host
pub async fn open_terminal(&self, shell: Option<&str>) -> Result<RemoteTerminal, RemoteError>

/// Disconnect gracefully
pub async fn disconnect(&mut self) -> Result<(), RemoteError>
```

---

## 3. Internal Architecture

```
LOCAL (SMASH TUI)                          REMOTE HOST
┌─────────────────────┐                   ┌──────────────────────┐
│                     │    SSH tunnel      │                      │
│  RemoteSession      │◀══════════════════▶│  smash-agent         │
│                     │   multiplexed      │                      │
│  ┌───────────────┐  │   binary channel   │  ┌────────────────┐  │
│  │ ChannelMux    │  │                   │  │ RequestHandler │  │
│  │               │  │                   │  │                │  │
│  │ FS channel    │◀─┼───────────────────┼─▶│ FS ops         │  │
│  │ LSP channel   │◀─┼───────────────────┼─▶│ LSP proxy      │  │
│  │ DAP channel   │◀─┼───────────────────┼─▶│ DAP proxy      │  │
│  │ Terminal ch.  │◀─┼───────────────────┼─▶│ PTY manager    │  │
│  │ Sync channel  │◀─┼───────────────────┼─▶│ File watcher   │  │
│  └───────────────┘  │                   │  └────────────────┘  │
│                     │                   │                      │
│  ┌───────────────┐  │                   │  ┌────────────────┐  │
│  │ Reconnection  │  │                   │  │ Agent Manager  │  │
│  │ Manager       │  │                   │  │                │  │
│  │               │  │                   │  │ Health check   │  │
│  │ Exponential   │  │                   │  │ Auto-restart   │  │
│  │ backoff       │  │                   │  │                │  │
│  └───────────────┘  │                   │  └────────────────┘  │
└─────────────────────┘                   └──────────────────────┘
```

### 3.1 SSH Transport

- Uses `russh` (async SSH2 library) for the SSH connection.
- A single SSH connection with multiplexed channels for different purposes.
- Authentication: key-based (identity file, SSH agent), password (fallback).
- Keep-alive packets every 30 seconds.

### 3.2 Remote Agent

- A small Rust binary (`smash-agent`) that runs on the remote host.
- Deployed automatically via SCP on first connection or version mismatch.
- The agent handles:
  - File system operations (read, write, list, watch).
  - Spawning and proxying LSP servers.
  - Spawning and proxying DAP adapters.
  - Managing remote terminal PTYs.
- Communication: length-prefixed binary messages over the SSH channel.

### 3.3 Channel Multiplexing

- All communication is multiplexed over the single SSH connection.
- Each logical channel (FS, LSP, DAP, terminal) has a channel ID.
- Messages: `{ channel_id: u32, payload_len: u32, payload: [u8] }`.

### 3.4 Reconnection

- On connection loss, automatic reconnection with exponential backoff (1s, 2s, 4s, … 60s max).
- Pending requests are queued and replayed after reconnection.
- The remote agent persists and is reattached on reconnect.

### 3.5 WSL Integration

- On Windows, WSL paths are transparently translated (`/home/user` ↔ `\\wsl.localhost\...`).
- Connection uses `wsl.exe` invocation instead of SSH.
- The same `smash-agent` protocol is used.

### 3.6 File Synchronization

- The remote agent watches files for external changes (via `inotify`/`kqueue` on remote side).
- Change notifications are pushed to the local SMASH instance.
- Conflict detection: if a file is modified both locally and remotely, the user is prompted.

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("SSH connection failed: {0}")]
    ConnectionFailed(String),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("agent deployment failed: {0}")]
    AgentDeployFailed(String),

    #[error("remote I/O error: {0}")]
    RemoteIo(String),

    #[error("channel error: {0}")]
    ChannelError(String),

    #[error("connection lost")]
    Disconnected,

    #[error("command execution failed: {command}: {message}")]
    ExecFailed { command: String, message: String },

    #[error("WSL not available")]
    WslNotAvailable,
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `russh` | Async SSH client |
| `russh-keys` | SSH key parsing |
| `tokio` | Async runtime |
| `serde`, `bincode` | Message serialization |
| `thiserror` | Error derivation |
| `tracing` | Logging |

---

## 6. Module File Layout

```
crates/smash-remote/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── session.rs          # RemoteSession — high-level API
    ├── ssh.rs              # SSH connection management
    ├── agent.rs            # Remote agent protocol, deployment
    ├── channel.rs          # Channel multiplexer
    ├── fs.rs               # Remote filesystem operations
    ├── reconnect.rs        # Reconnection manager
    ├── wsl.rs              # WSL-specific transport
    └── error.rs            # RemoteError
```

### 6.1 Agent Binary

```
crates/smash-remote/
└── agent/
    ├── Cargo.toml          # Separate binary crate
    └── src/
        ├── main.rs         # Agent entry point
        ├── handler.rs      # Request handler
        ├── fs.rs           # FS operations
        ├── lsp_proxy.rs    # LSP server proxy
        ├── dap_proxy.rs    # DAP adapter proxy
        └── pty.rs          # Remote PTY management
```
