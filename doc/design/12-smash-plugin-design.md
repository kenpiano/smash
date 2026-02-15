# smash-plugin — Module Design

## 1. Overview

`smash-plugin` provides a WASM-based plugin system that allows third-party extensions to add commands, syntax processors, UI elements, and integrations while running in a sandboxed environment with controlled access to the host editor.

**Phase**: 5

**Requirements Covered**: REQ-PLUG-001 – 004

---

## 2. Public API Surface

### 2.1 Core Types

```rust
PluginManager         — manages plugin lifecycle (load, start, stop, unload)
PluginId              — unique identifier (reverse-domain: "com.example.my-plugin")
PluginManifest        — parsed plugin.toml: name, version, permissions, entry point
PluginInstance        — a running WASM module instance
PluginPermission      — enum { ReadBuffer, WriteBuffer, FileSystem, Network, Clipboard, Terminal }
PluginEvent           — events sent to plugins (buffer change, command, timer, etc.)
PluginCommand         — a command registered by a plugin
HostApi               — trait defining functions plugins can call into the host
```

### 2.2 Key Functions

```rust
/// Initialize the plugin manager
pub fn new(plugin_dirs: &[PathBuf]) -> Result<PluginManager, PluginError>

/// Discover and load all plugins from plugin directories
pub fn discover(&mut self) -> Result<Vec<PluginManifest>, PluginError>

/// Load a specific plugin by ID
pub fn load(&mut self, id: &PluginId) -> Result<(), PluginError>

/// Start a loaded plugin (instantiate WASM module)
pub fn start(&mut self, id: &PluginId) -> Result<(), PluginError>

/// Stop a running plugin
pub fn stop(&mut self, id: &PluginId) -> Result<(), PluginError>

/// Unload a plugin
pub fn unload(&mut self, id: &PluginId) -> Result<(), PluginError>

/// Send an event to all active plugins
pub fn broadcast_event(&self, event: &PluginEvent) -> Result<(), PluginError>

/// Send an event to a specific plugin
pub fn send_event(&self, id: &PluginId, event: &PluginEvent) -> Result<(), PluginError>

/// Get commands registered by plugins
pub fn commands(&self) -> &[PluginCommand]

/// Execute a plugin command
pub fn execute_command(
    &self,
    id: &PluginId,
    command: &str,
    args: &[u8],
) -> Result<Vec<u8>, PluginError>

/// List all loaded plugins
pub fn loaded_plugins(&self) -> Vec<&PluginManifest>
```

---

## 3. Internal Architecture

```
┌───────────────────────────────────────────────────────────┐
│                     PluginManager                          │
│                                                            │
│  ┌──────────────────────┐    ┌──────────────────────────┐  │
│  │   ManifestRegistry    │    │   PermissionChecker       │  │
│  │                       │    │                           │  │
│  │   plugin.toml parsing │    │   Validates host API      │  │
│  │   version/compat      │    │   calls against declared  │  │
│  │   dependency graph    │    │   permissions              │  │
│  └──────────────────────┘    └──────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              WasmRuntime (Wasmtime)                    │  │
│  │                                                       │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │  │
│  │  │ Plugin A  │  │ Plugin B  │  │ Plugin C  │           │  │
│  │  │          │  │          │  │          │           │  │
│  │  │ WASM mod │  │ WASM mod │  │ WASM mod │           │  │
│  │  │ instance │  │ instance │  │ instance │           │  │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘           │  │
│  │       │              │              │                  │  │
│  │       ▼              ▼              ▼                  │  │
│  │  ┌───────────────────────────────────────────────┐    │  │
│  │  │           Host API (WASM imports)              │    │  │
│  │  │                                                │    │  │
│  │  │  smash_log()         smash_read_buffer()       │    │  │
│  │  │  smash_write_buffer() smash_register_command() │    │  │
│  │  │  smash_read_file()   smash_show_message()      │    │  │
│  │  │  smash_clipboard()   smash_get_config()        │    │  │
│  │  └───────────────────────────────────────────────┘    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              EventDispatcher                          │  │
│  │                                                       │  │
│  │  Routes editor events → subscribed plugins            │  │
│  │  Serializes events to WASM-compatible format          │  │
│  └──────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
```

### 3.1 WASM Runtime

- Uses **Wasmtime** as the WASM runtime.
- Each plugin runs in an isolated WASM instance with its own linear memory.
- Fuel metering limits CPU consumption per call (prevents infinite loops).
- Memory limits: configurable per-plugin (default 16 MB).

### 3.2 Plugin Manifest

```toml
# plugin.toml
[plugin]
id = "com.example.my-plugin"
name = "My Plugin"
version = "1.0.0"
description = "A sample plugin"
entry = "plugin.wasm"
min_smash_version = "0.5.0"

[permissions]
read_buffer = true
write_buffer = true
file_system = false
network = false
clipboard = true
terminal = false

[[commands]]
name = "my-plugin.do-something"
title = "Do Something"
keybinding = "ctrl+shift+d"
```

### 3.3 Host API (WASM Imports)

The host exposes functions that plugins can call via WASM imports:

| Function | Permission | Description |
|---|---|---|
| `smash_log(level, msg)` | None | Log a message |
| `smash_read_buffer(buf_id)` | `ReadBuffer` | Read buffer content |
| `smash_write_buffer(buf_id, pos, text)` | `WriteBuffer` | Insert text |
| `smash_delete_buffer(buf_id, pos, len)` | `WriteBuffer` | Delete text |
| `smash_cursor_position(buf_id)` | `ReadBuffer` | Get cursor position |
| `smash_register_command(name, callback)` | None | Register a command |
| `smash_read_file(path)` | `FileSystem` | Read a file |
| `smash_write_file(path, content)` | `FileSystem` | Write a file |
| `smash_clipboard_read()` | `Clipboard` | Read clipboard |
| `smash_clipboard_write(text)` | `Clipboard` | Write clipboard |
| `smash_show_message(level, msg)` | None | Show notification |
| `smash_get_config(key)` | None | Read plugin config |

### 3.4 Plugin Events (WASM Exports)

Plugins export handler functions that the host calls:

| Export | Trigger |
|---|---|
| `on_activate()` | Plugin started |
| `on_deactivate()` | Plugin stopping |
| `on_buffer_change(buf_id, change)` | Buffer edited |
| `on_command(name, args)` | Command invoked |
| `on_timer(timer_id)` | Timer fired |
| `on_event(event_json)` | Generic event |

### 3.5 Permission Model

- Permissions are declared in `plugin.toml` and shown to the user on first load.
- The `PermissionChecker` intercepts every host API call and verifies the plugin has the required permission.
- Undeclared permission calls return an error to the plugin without executing.

### 3.6 Resource Limits

| Resource | Default Limit | Configurable |
|---|---|---|
| Memory | 16 MB | Yes |
| Fuel (CPU) | 1,000,000 per call | Yes |
| Open files | 10 | Yes |
| Call timeout | 5 seconds | Yes |

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("manifest parse error in {path}: {message}")]
    ManifestError { path: String, message: String },

    #[error("WASM compilation failed: {0}")]
    CompilationFailed(String),

    #[error("WASM instantiation failed: {0}")]
    InstantiationFailed(String),

    #[error("plugin {id} not found")]
    NotFound { id: String },

    #[error("plugin {id} permission denied: {permission}")]
    PermissionDenied { id: String, permission: String },

    #[error("plugin {id} exceeded resource limit: {resource}")]
    ResourceExceeded { id: String, resource: String },

    #[error("plugin {id} execution failed: {message}")]
    ExecutionFailed { id: String, message: String },

    #[error("plugin {id} version incompatible: requires {required}, have {current}")]
    VersionIncompatible {
        id: String,
        required: String,
        current: String,
    },
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `wasmtime` | WASM runtime |
| `toml` | Plugin manifest parsing |
| `serde`, `serde_json` | Serialization |
| `semver` | Version compatibility checks |
| `thiserror` | Error derivation |
| `tracing` | Logging |

---

## 6. Module File Layout

```
crates/smash-plugin/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── manager.rs          # PluginManager — lifecycle
    ├── manifest.rs         # PluginManifest parsing + validation
    ├── runtime.rs          # Wasmtime setup, instance management
    ├── host_api.rs         # Host function implementations (WASM imports)
    ├── permissions.rs      # PermissionChecker
    ├── events.rs           # EventDispatcher, PluginEvent
    ├── commands.rs         # PluginCommand registry
    └── error.rs            # PluginError
```
