# smash-lsp — Test Design

## 1. Overview

Test strategy for `smash-lsp`. Requires ≥ 80% coverage (core crate).

LSP tests use a **mock language server** — a child process or in-process stub that speaks JSON-RPC to validate the client's protocol handling without a real language server.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-lsp/tests/` | `cargo test` (with mock server) |

---

## 3. Unit Test Plan

### 3.1 `transport.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `transport_frame_message` | Serialize request | Valid `Content-Length: N\r\n\r\n{...}` |
| `transport_parse_response` | Read framed response | Correct JSON-RPC Response |
| `transport_parse_notification` | Read notification (no id) | Correct JSON-RPC Notification |
| `transport_malformed_header` | Missing Content-Length | Transport error |
| `transport_incomplete_body` | Body shorter than Content-Length | Transport error |

### 3.2 `dispatcher.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `dispatcher_register_and_resolve` | Register request ID, send response | oneshot receives value |
| `dispatcher_timeout_fires` | Register, no response | `LspError::Timeout` after deadline |
| `dispatcher_unknown_id_ignored` | Response with unregistered ID | Logged, no panic |
| `dispatcher_notification_routed` | Server notification | Delivered to notification handler |

### 3.3 `capabilities.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `capabilities_client_includes_completion` | Build client capabilities | `textDocument.completion` present |
| `capabilities_negotiation_subset` | Server supports only hover | `hover` enabled, `completion` disabled |
| `capabilities_full_server` | Server supports everything | All capabilities enabled |

### 3.4 `diagnostics.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `diagnostics_store_per_uri` | Receive diagnostics for 2 files | Stored separately |
| `diagnostics_update_replaces` | Second publish for same URI | Old diagnostics replaced |
| `diagnostics_clear_on_empty` | Publish empty diagnostic list | URI entry cleared |
| `diagnostics_notify_on_update` | Publish diagnostics | Channel receives update event |

### 3.5 `completion.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `completion_deserialize_items` | JSON response with 3 items | 3 `CompletionItem` structs |
| `completion_resolve_enriches` | Resolve adds documentation | Item has doc field |
| `completion_empty_response` | Server returns empty list | Empty Vec |

### 3.6 `navigation.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `goto_definition_single_location` | Server returns 1 location | Vec with 1 Location |
| `goto_definition_multiple` | Server returns 3 locations | Vec with 3 Locations |
| `find_references_includes_declaration` | includeDeclaration=true | Declaration in results |
| `document_symbols_nested` | Symbols with children | Hierarchical SymbolInformation |

### 3.7 `client.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `client_initialize_handshake` | Start mock server → initialize | Capabilities negotiated |
| `client_did_open_notification` | Open document | Notification sent to server |
| `client_did_change_notification` | Edit document | Change notification sent |
| `client_shutdown_sequence` | Shutdown → exit | Server process terminates |
| `client_server_crash_detected` | Kill server process | `LspError::ServerCrashed` |

---

## 4. Integration Tests

| Test Name | Scenario | Requirements |
|---|---|---|
| `mock_server_full_lifecycle` | Start → initialize → open → edit → completion → shutdown | All steps succeed |
| `mock_server_diagnostics_flow` | Open file with error → receive diagnostics | Diagnostics stored and notified |
| `mock_server_goto_definition` | Request goto-definition → receive location | Correct location returned |
| `mock_server_concurrent_requests` | Send 5 requests simultaneously | All resolve correctly |
| `mock_server_timeout_handling` | Server delays >10s | Timeout error returned |

---

## 5. Mock Server

```rust
/// A minimal JSON-RPC server for testing
struct MockLspServer {
    /// Pre-programmed responses keyed by method name
    responses: HashMap<String, serde_json::Value>,
    /// Captured requests for verification
    received: Vec<JsonRpcMessage>,
}
```

The mock server runs as a child process (or in-process with piped stdio) and responds to requests with pre-configured data.

---

## 6. Benchmarks

| Benchmark | Target |
|---|---|
| `bench_completion_response_parse_100_items` | < 1 ms |
| `bench_diagnostic_update_1000_items` | < 5 ms |
| `bench_json_rpc_roundtrip` | < 100 µs |

---

## 7. Coverage Target

**Minimum: 80%** line coverage.

Priority for near-100%:
- `transport.rs` — all framing/parsing paths
- `dispatcher.rs` — timeout, error, and normal paths
- `client.rs` — lifecycle state machine
