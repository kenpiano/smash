# smash-config — Test Design

## 1. Overview

Test strategy for `smash-config`. Coverage target: ≥ 70%.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-config/tests/` | `cargo test` |

---

## 3. Unit Test Plan

### 3.1 `config.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `config_default_values_correct` | Create `Config::default()` | All fields match documented defaults |
| `config_deserialize_minimal_toml` | Minimal TOML (just `theme = "dark"`) | Rest are defaults |
| `config_deserialize_full_toml` | All fields specified | All fields populated |
| `config_unknown_keys_ignored` | TOML with extra unknown keys | Parses successfully |

### 3.2 `load.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `load_global_config` | Config file at standard location | Parsed Config |
| `load_project_config` | `.smash/config.toml` in temp dir | Parsed Config |
| `load_missing_global_uses_defaults` | No global config file | Default Config |
| `load_project_walk_finds_parent` | CWD is `project/src/`, config at `project/.smash/` | Found |
| `load_invalid_toml_returns_error` | Malformed TOML | `ConfigError::Parse` |
| `load_io_error` | Unreadable file | `ConfigError::Io` |

### 3.3 `merge.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `merge_project_overrides_global` | Global: tab_size=4, Project: tab_size=2 | Result: tab_size=2 |
| `merge_global_fills_default` | Global: theme="monokai", Default: theme="dark" | Result: theme="monokai" |
| `merge_project_only_overrides_specified` | Project sets 1 field | All others from global/default |
| `merge_lsp_configs_combined` | Global: rust-analyzer, Project: pyright | Both present |
| `merge_lsp_config_override_by_key` | Both define rust-analyzer | Project's config wins |

### 3.4 `validate.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `validate_tab_size_in_range` | tab_size = 4 | Ok |
| `validate_tab_size_zero_fails` | tab_size = 0 | `ConfigError::Validation` |
| `validate_tab_size_too_large_fails` | tab_size = 99 | `ConfigError::Validation` |
| `validate_log_level_valid` | log_level = "debug" | Ok |
| `validate_empty_theme_fails` | theme = "" | `ConfigError::Validation` |
| `validate_default_config_passes` | `Config::default()` | Ok |

### 3.5 `watcher.rs` (Phase 4)

| Test Name | Scenario | Expected |
|---|---|---|
| `watcher_detects_file_change` | Modify config file | Change event received |
| `watcher_debounces_rapid_changes` | 10 changes in 100 ms | Single reload event |
| `watcher_handles_file_deletion` | Delete config file | Graceful handling, revert to defaults |

### 3.6 `defaults.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `defaults_all_fields_set` | `Config::default()` | No `None` for required fields |
| `defaults_tab_size_is_4` | Check tab_size | `4` |
| `defaults_theme_is_dark` | Check theme | `"dark"` |

---

## 4. Integration Tests

| Test Name | Scenario | Requirements |
|---|---|---|
| `full_load_merge_validate` | Create global + project TOML files, load | Merged config, valid |
| `roundtrip_serialize_deserialize` | Serialize Config → TOML → deserialize | Identical Config |
| `config_file_template_generation` | Generate example config | Valid TOML that parses back |

---

## 5. Test Fixtures

```
crates/smash-config/tests/fixtures/
├── minimal.toml          # Bare minimum config
├── full.toml             # Every field specified
├── invalid_syntax.toml   # Malformed TOML
├── invalid_values.toml   # Valid TOML, invalid field values
├── global_sample.toml    # Simulated global config
└── project_sample.toml   # Simulated per-project config
```

---

## 6. Coverage Target

**Minimum: 70%** line coverage.

Priority for near-100%:
- `validate.rs` — all validation branches
- `merge.rs` — all merge scenarios
- `load.rs` — all error paths
