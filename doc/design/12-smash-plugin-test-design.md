# smash-plugin — Test Design

## 1. Overview

Test strategy for `smash-plugin`. Requires ≥ 70% coverage.

Tests use **pre-compiled WASM test modules** that exercise the host API. A minimal test plugin is written in Rust, compiled to WASM, and included as a test fixture.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-plugin/tests/` | `cargo test` |
| Test fixtures | `crates/smash-plugin/tests/fixtures/` | Pre-compiled WASM |

---

## 3. Test Fixture: Minimal Plugin

A tiny Rust crate compiled to `wasm32-wasi`:

```rust
// test-plugin/src/lib.rs
#[no_mangle]
pub extern "C" fn on_activate() { /* logs "activated" via smash_log */ }

#[no_mangle]
pub extern "C" fn on_command(name_ptr: i32, name_len: i32, args_ptr: i32, args_len: i32) {
    // Echo command args back
}

#[no_mangle]
pub extern "C" fn on_deactivate() { /* logs "deactivated" */ }
```

Also: a **malicious plugin** that tries to exceed resource limits, and a **permission-testing plugin** that calls APIs it doesn't have permission for.

---

## 4. Unit Test Plan

### 4.1 `manifest.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `manifest_parse_valid` | Parse well-formed plugin.toml | Correct fields |
| `manifest_parse_minimal` | Only required fields | Defaults for optional fields |
| `manifest_missing_id` | No `id` field | Parse error |
| `manifest_missing_entry` | No `entry` field | Parse error |
| `manifest_invalid_version` | Version "abc" | Parse error |
| `manifest_permissions_all_false` | No permissions declared | All permissions denied |
| `manifest_commands_parsed` | Two commands defined | Both in manifest |
| `manifest_version_compat_ok` | `min_smash_version = "0.1.0"` | Compatible |
| `manifest_version_compat_fail` | `min_smash_version = "99.0.0"` | `VersionIncompatible` |

### 4.2 `permissions.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `permission_allowed` | Plugin has ReadBuffer, calls read | Allowed |
| `permission_denied` | Plugin lacks FileSystem, calls read_file | `PermissionDenied` |
| `permission_log_always_allowed` | No permissions, calls smash_log | Allowed |
| `permission_check_all_variants` | Each PluginPermission variant | Correct permission checked |

### 4.3 `runtime.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `runtime_compile_valid_wasm` | Load test-plugin.wasm | Compiled module |
| `runtime_compile_invalid_wasm` | Load garbage bytes | `CompilationFailed` |
| `runtime_instantiate` | Instantiate compiled module | Instance created |
| `runtime_fuel_limit` | Plugin with infinite loop | `ResourceExceeded` |
| `runtime_memory_limit` | Plugin allocates 32 MB | `ResourceExceeded` (16 MB limit) |
| `runtime_call_export` | Call `on_activate` | No error, log produced |

### 4.4 `host_api.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `host_api_log` | Plugin calls smash_log | Message recorded |
| `host_api_read_buffer` | Plugin calls smash_read_buffer | Buffer content returned |
| `host_api_write_buffer` | Plugin calls smash_write_buffer | Buffer modified |
| `host_api_register_command` | Plugin registers command | Command in registry |
| `host_api_show_message` | Plugin calls smash_show_message | Message queued |
| `host_api_clipboard_read` | Plugin calls clipboard read | Content returned |

### 4.5 `manager.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `manager_discover_plugins` | Plugin dir with 2 plugins | 2 manifests returned |
| `manager_discover_empty_dir` | Empty dir | 0 manifests |
| `manager_load_start_stop` | Full lifecycle | No errors |
| `manager_load_nonexistent` | Load missing plugin | `NotFound` |
| `manager_double_load` | Load same plugin twice | Idempotent or error |
| `manager_stop_not_started` | Stop unstarted plugin | Error |
| `manager_broadcast_event` | Broadcast to 2 plugins | Both receive |
| `manager_commands_list` | 2 plugins with 3 commands total | 3 commands |

### 4.6 `events.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `event_serialize_buffer_change` | Serialize buffer change event | Valid bytes |
| `event_dispatch_to_subscriber` | Plugin subscribed to buffer events | Event delivered |
| `event_dispatch_no_subscribers` | No plugins running | No error |

---

## 5. Integration Tests

| Test Name | Scenario | Expected |
|---|---|---|
| `integration_load_and_run_plugin` | Discover → load → start → call command → stop | Full lifecycle works |
| `integration_permission_enforcement` | Plugin without FS permission tries file read | `PermissionDenied` |
| `integration_resource_limits` | Malicious plugin tries infinite loop | Killed after fuel exhaustion |
| `integration_multiple_plugins` | Load 3 plugins, broadcast event | All receive event |
| `integration_plugin_registers_command` | Plugin registers command, user invokes | Command executed |

---

## 6. Coverage Target

**Minimum: 70%** line coverage.

Priority:
- `permissions.rs` — all permission check paths (aim for 100%)
- `runtime.rs` — resource limit enforcement
- `manifest.rs` — all validation paths
- `host_api.rs` — every host function
