# smash-remote — Test Design

## 1. Overview

Test strategy for `smash-remote`. Requires ≥ 70% coverage.

Tests use a **mock SSH server** and **mock agent** to avoid requiring real remote hosts. The mock agent runs in-process and communicates via `tokio::io::DuplexStream`.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-remote/tests/` | `cargo test` |

---

## 3. Unit Test Plan

### 3.1 `channel.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `channel_mux_create` | Create multiplexer | 0 active channels |
| `channel_open_and_send` | Open channel, send data | Data received on other side |
| `channel_multiplex_two` | Two channels interleaved | Correct routing |
| `channel_close` | Close one channel | Other channel unaffected |
| `channel_large_payload` | Send 1 MB payload | Received intact |
| `channel_frame_parsing` | Parse length-prefixed frame | Correct channel_id + payload |

### 3.2 `agent.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `agent_protocol_version_check` | Agent version matches | Connection proceeds |
| `agent_protocol_version_mismatch` | Agent version old | Re-deploy triggered |
| `agent_message_serialize` | Serialize FS read request | Valid bytes |
| `agent_message_deserialize` | Deserialize FS response | Correct content |
| `agent_deploy_path` | Compute deploy path | `~/.smash/agent` |

### 3.3 `fs.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `fs_read_file` | Request file content | Correct bytes returned |
| `fs_write_file` | Write bytes | Agent receives write request |
| `fs_list_dir` | List directory | Entries returned |
| `fs_read_nonexistent` | Read missing file | `RemoteIo` error |
| `fs_write_permission_denied` | Write to read-only path | `RemoteIo` error |

### 3.4 `reconnect.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `reconnect_exponential_backoff` | Connection fails 5 times | Delays: 1, 2, 4, 8, 16 seconds |
| `reconnect_max_backoff` | Many failures | Caps at 60 seconds |
| `reconnect_success_resets` | Reconnect succeeds | Backoff resets to 1s |
| `reconnect_pending_requests` | Request during disconnect | Replayed after reconnect |

### 3.5 `ssh.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `ssh_config_parse` | Parse RemoteConfig | Correct host, port, user |
| `ssh_auth_key_file` | Auth with identity file | Key loaded |
| `ssh_keepalive_interval` | Check keepalive config | 30 second interval |

### 3.6 `wsl.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `wsl_path_translate_to_windows` | `/home/user/file` | `\\wsl.localhost\...` |
| `wsl_path_translate_to_linux` | `\\wsl.localhost\Ubuntu\home\user` | `/home/user` |
| `wsl_detect_availability` | Mock `wsl.exe` present/absent | Correct boolean |

---

## 4. Integration Tests (Mock SSH + Agent)

```rust
struct MockSshServer {
    // Accepts connections, forwards to mock agent
}

struct MockAgent {
    // In-process agent that responds to protocol messages
    fs: HashMap<PathBuf, Vec<u8>>,  // virtual filesystem
}
```

| Test Name | Scenario | Expected |
|---|---|---|
| `integration_connect_and_read_file` | Connect → read file | Correct content |
| `integration_connect_and_write_file` | Connect → write file | File persisted in mock FS |
| `integration_list_directory` | Connect → list dir | Entries match mock FS |
| `integration_exec_command` | Run `echo hello` | Output: "hello" |
| `integration_port_forward` | Forward port | Local port maps to remote |
| `integration_reconnect_after_drop` | Drop connection → auto reconnect | Session continues |
| `integration_agent_deploy` | Empty remote → deploy agent | Agent binary transferred |

---

## 5. Coverage Target

**Minimum: 70%** line coverage.

Priority:
- `channel.rs` — multiplexing correctness
- `reconnect.rs` — all backoff and replay paths
- `agent.rs` — protocol serialization
- `fs.rs` — error handling paths
