# smash-collab — Module Design

## 1. Overview

`smash-collab` implements real-time collaborative editing using CRDT (Conflict-free Replicated Data Type) algorithms. When collaboration is inactive, this crate imposes zero overhead on buffer operations.

**Phase**: 4

**Requirements Covered**: REQ-COLLAB-001 – 006

---

## 2. Public API Surface

### 2.1 Core Types

```rust
CollabSession        — active collaboration session
PeerId               — unique peer identifier
PeerCursor           — { peer_id, position, selection?, display_name, color }
CollabDocument       — CRDT-backed document state
CollabChange         — serialized CRDT operation for network transfer
SyncMessage          — enum { Full(Vec<u8>), Incremental(Vec<u8>) }
SessionInfo          — { session_id, peers, document_id }
SignalingMessage     — enum { Join, Leave, Offer, Answer, IceCandidate }
```

### 2.2 Key Functions

```rust
/// Create a new collaboration session from an existing buffer
pub fn host_session(
    document: &Buffer,
    display_name: &str,
) -> Result<CollabSession, CollabError>

/// Join an existing collaboration session
pub async fn join_session(
    session_id: &str,
    display_name: &str,
    signaling_url: &str,
) -> Result<CollabSession, CollabError>

/// Apply a local edit to the CRDT document
pub fn apply_local_edit(
    &mut self,
    position: usize,
    delete_len: usize,
    insert_text: &str,
) -> Result<CollabChange, CollabError>

/// Receive and apply a remote change
pub fn apply_remote_change(
    &mut self,
    change: &CollabChange,
) -> Result<(), CollabError>

/// Get current peer cursors
pub fn peer_cursors(&self) -> &[PeerCursor]

/// Update local cursor position (broadcast to peers)
pub fn update_cursor(&mut self, position: usize, selection: Option<Range<usize>>)

/// Generate sync state for a new peer
pub fn sync_state(&self) -> Result<SyncMessage, CollabError>

/// Merge sync state from a peer
pub fn merge_sync(&mut self, msg: &SyncMessage) -> Result<Vec<CollabChange>, CollabError>

/// Leave the session
pub async fn leave(&mut self) -> Result<(), CollabError>
```

---

## 3. Internal Architecture

```
┌───────────────────────────────────────────────────────────┐
│                     CollabSession                          │
│                                                            │
│  ┌─────────────────────┐    ┌───────────────────────────┐  │
│  │   CrdtDocument       │    │   PeerManager             │  │
│  │                      │    │                            │  │
│  │  automerge::AutoCommit│   │  peer_id → PeerCursor     │  │
│  │  or custom Yata impl │    │  presence tracking         │  │
│  │                      │    │  heartbeat / timeout       │  │
│  └──────────┬───────────┘    └───────────────────────────┘  │
│             │                                               │
│             ▼                                               │
│  ┌────────────────────────────────────────────────────┐    │
│  │              NetworkTransport                       │    │
│  │                                                     │    │
│  │  ┌─────────────┐    ┌────────────────────────────┐  │    │
│  │  │  Signaling   │    │  Data Channel (WebRTC or   │  │    │
│  │  │  (WebSocket) │    │  WebSocket relay)           │  │    │
│  │  │              │    │                             │  │    │
│  │  │  Join/Leave  │    │  CRDT ops, cursor updates   │  │    │
│  │  │  Peer disc.  │    │  E2EE encrypted             │  │    │
│  │  └─────────────┘    └────────────────────────────┘  │    │
│  └────────────────────────────────────────────────────┘    │
│                                                            │
│  ┌────────────────────────────────────────────────────┐    │
│  │              Encryption Layer (E2EE)                │    │
│  │                                                     │    │
│  │  Session key exchange (X25519 + AES-256-GCM)        │    │
│  │  Per-message encrypt/decrypt                        │    │
│  └────────────────────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────┘
```

### 3.1 CRDT Engine

- Uses the **Automerge** crate as the CRDT backend.
- The `CrdtDocument` wraps `automerge::AutoCommit` and exposes text-editing operations.
- Local edits produce `CollabChange` (serialized Automerge changes) for broadcast.
- Remote changes are applied via `automerge::AutoCommit::apply_changes()`.
- When collaboration is inactive, `CollabSession` is `None` and the buffer uses the plain rope with zero overhead.

### 3.2 Buffer Integration

- `smash-core::Buffer` has an optional `CollabSession` field.
- When active, every edit operation:
  1. Applies to the rope (local view).
  2. Applies to the CRDT document (produces `CollabChange`).
  3. Broadcasts the change via the network transport.
- Remote changes:
  1. Applied to CRDT document.
  2. Translated to rope operations (insert/delete at position).

### 3.3 Peer Management

- Each peer has a unique `PeerId` (UUID v4).
- Cursor positions are broadcast periodically (debounced, ~100ms).
- Peers are displayed with colored cursors/selections in the TUI.
- Heartbeat every 5 seconds; peers removed after 15 seconds of silence.

### 3.4 Network Transport

- **Signaling**: WebSocket connection to a signaling server for peer discovery.
- **Data channel**: Peer-to-peer (WebRTC) preferred; fallback to relayed WebSocket.
- Phase 4 MVP: WebSocket relay only (simpler). P2P in Phase 5.

### 3.5 Encryption (E2EE)

- Session key established via X25519 Diffie-Hellman key exchange.
- All data-channel messages encrypted with AES-256-GCM.
- Signaling messages are not encrypted (contain no document content).

### 3.6 Conflict Resolution

- CRDT guarantees eventual consistency — no manual conflict resolution needed.
- Concurrent edits at the same position are resolved by Automerge's YATA algorithm (left-to-right insertion order by peer ID).

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CollabError {
    #[error("signaling connection failed: {0}")]
    SignalingFailed(String),

    #[error("peer connection failed: {peer_id}")]
    PeerConnectionFailed { peer_id: String },

    #[error("CRDT operation failed: {0}")]
    CrdtError(String),

    #[error("sync failed: {0}")]
    SyncFailed(String),

    #[error("encryption error: {0}")]
    EncryptionError(String),

    #[error("session not active")]
    NotActive,
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `automerge` | CRDT engine |
| `tokio` | Async networking |
| `tokio-tungstenite` | WebSocket client |
| `x25519-dalek` | Key exchange |
| `aes-gcm` | Symmetric encryption |
| `uuid` | Peer ID generation |
| `serde`, `serde_json` | Message serialization |
| `thiserror` | Error derivation |
| `tracing` | Logging |

---

## 6. Module File Layout

```
crates/smash-collab/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── session.rs          # CollabSession — high-level API
    ├── crdt.rs             # CrdtDocument wrapper over automerge
    ├── peer.rs             # PeerManager, PeerCursor
    ├── transport.rs        # NetworkTransport trait
    ├── signaling.rs        # WebSocket signaling client
    ├── relay.rs            # WebSocket relay data channel
    ├── encryption.rs       # E2EE (X25519 + AES-256-GCM)
    ├── sync.rs             # State sync protocol
    └── error.rs            # CollabError
```
