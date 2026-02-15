# smash-platform — Test Design

## 1. Overview

Test strategy for `smash-platform`. Coverage target: ≥ 70%.

Platform-specific code is challenging to test in CI across all OS targets. The strategy uses trait-based mocking for unit tests and OS-conditional integration tests.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-platform/tests/` | `cargo test` (OS-conditional) |

---

## 3. Unit Test Plan

### 3.1 `paths.rs` (trait-level)

| Test Name | Scenario | Expected |
|---|---|---|
| `paths_config_dir_not_empty` | Call `config_dir()` | Non-empty path |
| `paths_data_dir_not_empty` | Call `data_dir()` | Non-empty path |
| `paths_home_dir_exists` | Call `home_dir()` | Valid directory |
| `paths_canonicalize_resolves_symlink` | Symlink to a file | Resolved path |
| `paths_canonicalize_nonexistent_returns_error` | Non-existent path | `PlatformError::Path` |
| `paths_default_shell_exists` | Call `default_shell()` | Path to existing executable |

### 3.2 `clipboard.rs` (mock)

| Test Name | Scenario | Expected |
|---|---|---|
| `clipboard_set_then_get` | Set "hello", get | Returns "hello" |
| `clipboard_set_empty_string` | Set "" | Returns "" |
| `clipboard_set_unicode` | Set "日本語" | Returns "日本語" |
| `clipboard_set_multiline` | Set "a\nb\nc" | Returns "a\nb\nc" |

> Note: Real clipboard tests require platform integration tests (CI with display).

### 3.3 `process.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `process_spawn_echo` | Spawn `echo hello` | Child process exits 0, output "hello" |
| `process_spawn_nonexistent` | Spawn "nonexistent_cmd" | `PlatformError::ProcessSpawn` |
| `process_spawn_shell_command` | Shell command `echo $HOME` | Output contains home path |

### 3.4 `signals.rs` (mock)

| Test Name | Scenario | Expected |
|---|---|---|
| `signal_resize_receiver_receives` | Send resize event | Receiver gets (cols, rows) |
| `signal_interrupt_receiver_receives` | Send interrupt | Receiver gets event |

### 3.5 `system_info.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `system_info_os_matches_target` | Check `os` field | Matches `cfg!(target_os)` |
| `system_info_arch_matches_target` | Check `arch` field | Matches `cfg!(target_arch)` |

---

## 4. Integration Tests (OS-Conditional)

```rust
#[cfg(target_os = "linux")]
mod linux_tests { ... }

#[cfg(target_os = "macos")]
mod macos_tests { ... }

#[cfg(target_os = "windows")]
mod windows_tests { ... }
```

| Test Name | OS | Scenario |
|---|---|---|
| `linux_xdg_paths_respect_env` | Linux | Set `$XDG_CONFIG_HOME`, verify path |
| `macos_library_path_correct` | macOS | Config dir is under `~/Library` |
| `windows_appdata_path_correct` | Windows | Config dir is under `%APPDATA%` |
| `clipboard_roundtrip_real` | All (CI w/ display) | Clipboard set → get matches |

---

## 5. Coverage Target

**Minimum: 70%** line coverage.

Priority:
- Trait implementations (paths, clipboard)
- Error paths (process spawn failures, clipboard failures)
- Platform-conditional code on the CI OS
