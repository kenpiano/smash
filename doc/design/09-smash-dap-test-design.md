# smash-dap — Test Design

## 1. Overview

Test strategy for `smash-dap`. Requires ≥ 70% coverage.

Tests use a **mock debug adapter** — a child process or in-process stub that speaks the DAP protocol and returns canned responses.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-dap/tests/` | `cargo test` |

---

## 3. Unit Test Plan

### 3.1 `transport.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `transport_serialize_request` | Serialize a launch request | Valid Content-Length header + JSON |
| `transport_parse_response` | Parse valid response bytes | Correct response body |
| `transport_parse_event` | Parse stopped event | Correct DapEvent |
| `transport_malformed_header` | Missing Content-Length | Transport error |
| `transport_incomplete_body` | Truncated JSON | Transport error |
| `transport_multiple_messages` | Stream with 3 sequential messages | All 3 parsed |

### 3.2 `protocol.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `protocol_initialize_request_serde` | Serialize/deserialize initialize | Round-trip matches |
| `protocol_launch_request_serde` | Serialize/deserialize launch | Round-trip matches |
| `protocol_stopped_event_serde` | Serialize/deserialize stopped event | Round-trip matches |
| `protocol_breakpoint_serde` | Serialize/deserialize breakpoint | Round-trip matches |
| `protocol_stack_frame_serde` | Serialize/deserialize stack frame | Round-trip matches |
| `protocol_variable_serde` | Serialize/deserialize variable | Round-trip matches |

### 3.3 `session.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `session_lifecycle_happy_path` | init → launch → configDone → disconnect | No errors |
| `session_operations_before_init` | Call continue before initialize | `NotInitialized` error |
| `session_operations_after_disconnect` | Call step after disconnect | `Terminated` error |
| `session_sequence_tracking` | Send 5 requests | Responses matched by seq |

### 3.4 `breakpoint.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `breakpoint_set_and_verify` | Set BP at line 10 | Adapter confirms verified |
| `breakpoint_conditional` | Set BP with condition `x > 5` | Condition included in request |
| `breakpoint_remove` | Set then remove BP | Empty breakpoint list sent |
| `breakpoint_unverified` | Adapter rejects location | BP marked unverified |
| `breakpoint_multiple_files` | BPs in 3 files | Each file gets separate request |

### 3.5 `client.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `client_step_over` | Mock adapter responds to next | StepOver request sent, response OK |
| `client_step_into` | Mock adapter responds | StepInto request sent |
| `client_threads` | Request threads | Thread list returned |
| `client_stack_trace` | Request stack trace | Frames returned |
| `client_scopes_and_variables` | Request scopes then variables | Nested data returned |
| `client_evaluate` | Evaluate expression | Result variable returned |
| `client_timeout` | Adapter does not respond | `Timeout` error |

---

## 4. Integration Tests (Mock Adapter)

A `MockDapAdapter` binary/struct that:
- Reads from stdin, writes to stdout using DAP base protocol.
- Returns canned responses for initialize, launch, setBreakpoints, etc.
- Emits stopped events on command.

| Test Name | Scenario | Expected |
|---|---|---|
| `integration_full_debug_session` | Launch → set BP → continue → stopped → stack trace → variables → disconnect | All data correct |
| `integration_adapter_crash` | Kill adapter mid-session | Transport error propagated |
| `integration_concurrent_requests` | Send 10 requests rapidly | All responses matched |

---

## 5. Coverage Target

**Minimum: 70%** line coverage.

Priority:
- `transport.rs` — message framing and parsing
- `session.rs` — state machine transitions
- `breakpoint.rs` — sync logic
