# smash-collab — Test Design

## 1. Overview

Test strategy for `smash-collab`. Requires ≥ 80% coverage (core crate).

Tests simulate multi-peer collaboration using in-process CRDT instances connected via `tokio::sync::mpsc` channels instead of real network connections.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-collab/tests/` | `cargo test` |
| Property tests | `src/crdt.rs` | `proptest` |

---

## 3. Unit Test Plan

### 3.1 `crdt.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `crdt_insert_text` | Insert "hello" at position 0 | Document text is "hello" |
| `crdt_delete_text` | Insert then delete 3 chars | Correct remaining text |
| `crdt_generate_change` | Local edit | Produces non-empty `CollabChange` |
| `crdt_apply_remote_change` | Apply change from another instance | Text matches |
| `crdt_concurrent_insert_same_position` | Two peers insert at same pos | Both texts present, deterministic order |
| `crdt_concurrent_delete_overlap` | Two peers delete overlapping ranges | Correct final text |
| `crdt_empty_document` | New document | Empty text |
| `crdt_large_document_sync` | 10,000 char document synced | Both peers match |

### 3.2 `peer.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `peer_add_and_list` | Add 3 peers | List returns 3 |
| `peer_remove` | Add then remove | List returns empty |
| `peer_cursor_update` | Update cursor position | `peer_cursors()` reflects change |
| `peer_heartbeat_timeout` | No heartbeat for 15s | Peer removed |
| `peer_duplicate_id` | Add same peer ID twice | Updated, not duplicated |

### 3.3 `sync.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `sync_full_state` | Generate full sync | Contains all document history |
| `sync_merge_from_behind` | Peer joins after 100 edits | Catches up fully |
| `sync_incremental` | Exchange incremental syncs | Both converge |
| `sync_empty_document` | Sync empty doc | No errors |

### 3.4 `encryption.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `encryption_roundtrip` | Encrypt then decrypt | Original data restored |
| `encryption_wrong_key` | Decrypt with wrong key | Error |
| `encryption_key_exchange` | Two peers exchange keys | Shared secret matches |
| `encryption_different_messages` | Encrypt two messages | Different ciphertexts |

### 3.5 `signaling.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `signaling_join_message` | Create join message | Valid JSON with session_id |
| `signaling_leave_message` | Create leave message | Valid JSON |
| `signaling_parse_peer_joined` | Parse incoming peer join | Correct peer info |

### 3.6 `session.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `session_host_creates_document` | Host session | Session active, document created |
| `session_edit_when_not_active` | Edit without session | `NotActive` error |
| `session_leave_cleans_up` | Leave session | State cleared |

---

## 4. Integration Tests (Multi-Peer Simulation)

Use in-process channel-based transport instead of real WebSocket:

```rust
struct MockTransport {
    tx: mpsc::Sender<Vec<u8>>,
    rx: mpsc::Receiver<Vec<u8>>,
}
```

| Test Name | Scenario | Expected |
|---|---|---|
| `integration_two_peers_converge` | 2 peers make concurrent edits | Final documents match |
| `integration_three_peers_converge` | 3 peers, interleaved edits | All documents match |
| `integration_peer_join_midway` | Peer joins after 50 edits | New peer catches up |
| `integration_peer_disconnect` | 1 of 3 peers disconnects | Remaining 2 continue |
| `integration_rapid_edits` | 1000 rapid edits from 2 peers | Convergence within 1s |
| `integration_cursor_broadcast` | Peer moves cursor | Other peers see update |

---

## 5. Property Tests

```rust
proptest! {
    #[test]
    fn crdt_convergence(
        ops_a in vec(edit_operation(), 1..100),
        ops_b in vec(edit_operation(), 1..100),
    ) {
        // Apply ops_a to peer A, ops_b to peer B
        // Exchange all changes
        // Assert: peer_a.text() == peer_b.text()
    }
}
```

---

## 6. Benchmarks

| Benchmark | Target |
|---|---|
| `bench_apply_local_edit` | < 100 µs |
| `bench_apply_remote_change` | < 200 µs |
| `bench_sync_10k_chars` | < 50 ms |
| `bench_1000_concurrent_edits` | < 2 s convergence |

---

## 7. Coverage Target

**Minimum: 80%** line coverage.

Priority for near-100%:
- `crdt.rs` — all edit and merge code paths
- `sync.rs` — state synchronization
- `encryption.rs` — encrypt/decrypt round-trips
