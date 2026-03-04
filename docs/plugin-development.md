# Plugin Development

Build custom metric collectors as WASM plugins that run inside the SentinelRS agent sandbox.

## Overview

Plugins are WebAssembly (WASM) modules executed by the [wasmtime](https://wasmtime.dev) runtime.
Each plugin:

- Runs in a sandboxed environment with memory and time limits
- Calls host functions (`log`, `emit_metric_json`) to produce output
- Is signed with HMAC-SHA256 for integrity verification
- Has a YAML manifest describing its entry point, capabilities, and resource limits

---

## Project Setup

### Rust (recommended)

```bash
cargo new --lib my-plugin
cd my-plugin
```

Add to `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "s"
lto = true
```

Install the WASM target:

```bash
rustup target add wasm32-unknown-unknown
```

### Other Languages

Any language that compiles to WASM works (C, Go via TinyGo, AssemblyScript, Zig).
The plugin must export a function matching the manifest `entry_fn` with signature `() -> i32`.

---

## Host Functions

The agent exposes two host functions under the `sentinel` module:

### `sentinel.log`

Write a log message. Appears in agent logs and in `ExecutionResult.logs`.

```
(import "sentinel" "log" (func $log (param i32 i32)))
```

| Parameter | Type  | Description               |
| --------- | ----- | ------------------------- |
| `ptr`     | `i32` | Pointer to UTF-8 string   |
| `len`     | `i32` | Byte length of the string |

### `sentinel.emit_metric_json`

Emit a metric as a JSON string. Returns `0` on success, `-1` on failure (limit reached).

```
(import "sentinel" "emit_metric_json" (func $emit (param i32 i32) (result i32)))
```

| Parameter | Type  | Description                  |
| --------- | ----- | ---------------------------- |
| `ptr`     | `i32` | Pointer to UTF-8 JSON string |
| `len`     | `i32` | Byte length of the string    |

The JSON must match the metric format:

```json
{
    "name": "my_plugin.temperature",
    "labels": { "sensor": "cpu0" },
    "value": 72.5
}
```

---

## Writing a Plugin (Rust)

### Minimal Example

```rust
extern "C" {
    fn log(ptr: *const u8, len: usize);
    fn emit_metric_json(ptr: *const u8, len: usize) -> i32;
}

fn host_log(msg: &str) {
    unsafe { log(msg.as_ptr(), msg.len()) }
}

fn host_emit(json: &str) -> i32 {
    unsafe { emit_metric_json(json.as_ptr(), json.len()) }
}

#[no_mangle]
pub extern "C" fn collect() -> i32 {
    host_log("plugin started");

    let metric = r#"{"name":"example.value","labels":{},"value":42.0}"#;

    if host_emit(metric) != 0 {
        host_log("failed to emit metric");
        return 1;
    }

    host_log("plugin finished");
    0
}
```

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

Output: `target/wasm32-unknown-unknown/release/my_plugin.wasm`

---

## Plugin Manifest

Every plugin needs a `manifest.yml`:

```yaml
name: my-plugin
version: "1.0.0"
entry_fn: collect
capabilities:
    - metric_builder
resource_limits:
    max_memory_mb: 64
    timeout_ms: 5000
    max_metrics: 1000
metadata:
    author: "Your Name"
    description: "Custom temperature collector"
```

### Fields

| Field                           | Required | Default | Description                        |
| ------------------------------- | -------- | ------- | ---------------------------------- |
| `name`                          | yes      |         | Plugin name (used for file naming) |
| `version`                       | yes      |         | Semver version                     |
| `entry_fn`                      | yes      |         | Exported WASM function name        |
| `capabilities`                  | no       | `[]`    | Declared capabilities              |
| `resource_limits.max_memory_mb` | no       | `64`    | Max WASM memory in MB              |
| `resource_limits.timeout_ms`    | no       | `5000`  | Max execution time in ms           |
| `resource_limits.max_metrics`   | no       | `1000`  | Max metrics per execution          |
| `metadata`                      | no       | `{}`    | Arbitrary key-value metadata       |

### Capabilities

| Capability       | Description                 |
| ---------------- | --------------------------- |
| `http_get`       | Plugin may perform HTTP GET |
| `read_file`      | Plugin may read files       |
| `metric_builder` | Plugin emits metrics        |

---

## Installing a Plugin

### File Layout

```
plugins/
  my-plugin.wasm
  my-plugin.manifest.yml
```

Place files in the agent's plugins directory (default: `./plugins/`).

### Programmatic API

```rust
use sentinel_agent::plugin::{store_blob, store_manifest, sign_blob, verify_blob};

let plugins_dir = Path::new("./plugins");

// Store WASM binary
store_blob(plugins_dir, "my-plugin", &wasm_bytes)?;

// Store manifest
store_manifest(plugins_dir, "my-plugin", &manifest_yaml)?;
```

### Signing

Plugins are signed to prevent tampering:

```rust
let key = b"signing-key";
let signature = sign_blob(&wasm_bytes, key);

// Verify before loading
assert!(verify_blob(&wasm_bytes, &signature, key));
```

---

## Agent Configuration

Enable plugins in the agent config:

```yaml
agent_id: my-server
server_addr: "http://localhost:50051"
interval_seconds: 10
plugins:
    - name: my-plugin
      path: plugins/my-plugin.wasm
      manifest: plugins/my-plugin.manifest.yml
```

---

## Execution Model

1. Agent scheduler triggers plugin execution at each collection interval
2. Agent loads WASM bytes and manifest
3. Wasmtime creates a sandboxed store with memory limits
4. Plugin entry function is called
5. Plugin calls `emit_metric_json` for each metric
6. Timeout watchdog thread increments epoch after `timeout_ms`
7. Results (metrics + logs) are collected
8. Metrics are merged with system metrics and sent in the next batch

### Error Handling

| Return Code | Meaning          |
| ----------- | ---------------- |
| `0`         | Success          |
| Non-zero    | Plugin error     |
| Epoch trap  | Timeout exceeded |
| Memory trap | Memory limit hit |

---

## Testing

### Unit Test (Rust)

```rust
use sentinel_agent::plugin::{PluginRuntime, PluginManifest};

#[test]
fn plugin_executes() {
    let wasm = std::fs::read("target/wasm32-unknown-unknown/release/my_plugin.wasm").unwrap();
    let manifest = PluginManifest {
        name: "my-plugin".into(),
        version: "1.0.0".into(),
        entry_fn: "collect".into(),
        capabilities: vec![],
        resource_limits: Default::default(),
        metadata: Default::default(),
    };
    let runtime = PluginRuntime::load(&wasm, manifest).unwrap();
    let result = runtime.execute().unwrap();

    assert!(!result.metrics_json.is_empty());
    assert!(result.logs.iter().any(|l| l.contains("started")));
}
```

### Integration Test

```bash
cargo test --package sentinel-agent --test plugin_integration
```

---

## Debugging

Enable trace logging to see plugin execution details:

```bash
RUST_LOG=sentinel_agent::plugin=trace sentinel-agent
```

Logs include:

- Plugin load/compile time
- Entry function call
- Each `log()` call from the plugin
- Each `emit_metric_json()` call
- Execution time and result

---

## Best Practices

- Keep plugins small (< 1 MB WASM binary)
- Use `opt-level = "s"` and LTO for smallest binary
- Emit metrics with consistent naming: `plugin_name.metric_name`
- Add meaningful labels for filtering
- Handle errors gracefully — return non-zero on failure
- Set reasonable `max_metrics` to prevent runaway plugins
- Test with reduced `timeout_ms` to catch performance issues early
- Sign all production plugins
