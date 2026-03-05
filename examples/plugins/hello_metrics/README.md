# hello_metrics — Example SentinelRS WASM Plugin

Minimal plugin that emits a single metric and logs a message each collection cycle.

## Files

- `hello_metrics.wat` — WebAssembly Text format source
- `manifest.yml` — Plugin manifest (name, version, capabilities, limits)

## Host API

Plugins communicate with the agent through two imported functions:

| Function                     | Signature           | Description                                  |
| ---------------------------- | ------------------- | -------------------------------------------- |
| `sentinel::emit_metric_json` | `(i32, i32) -> i32` | Emit a JSON metric string from linear memory |
| `sentinel::log`              | `(i32, i32) -> ()`  | Log a UTF-8 message                          |

## Metric JSON Format

```json
{
    "name": "hello",
    "value": 1,
    "labels": {
        "src": "wasm"
    }
}
```

The agent automatically adds a `plugin` label with the plugin name.

## Install

```bash
# Copy WAT file directly (wasmtime parses WAT)
sentinel plugins install hello_metrics.wat \
    --manifest manifest.yml \
    --dir /var/lib/sentinel/plugins

# Or compile to binary WASM first
wat2wasm hello_metrics.wat -o hello_metrics.wasm
sentinel plugins install hello_metrics.wasm \
    --manifest manifest.yml \
    --dir /var/lib/sentinel/plugins
```

## Agent Configuration

```yaml
plugins:
    enabled: true
    dir: /var/lib/sentinel/plugins
    interval_seconds: 30
```

## Writing Your Own Plugin

1. Create a `.wat` or compile Rust/C/Go to `.wasm`
2. Export a function matching `entry_fn` in the manifest (default: `collect`)
3. The function must return `i32`: `0` = success, non-zero = error
4. Use `sentinel::emit_metric_json` to send metrics as JSON strings
5. Use `sentinel::log` for debug logging
6. Create a `manifest.yml` with plugin metadata
7. Install with `sentinel plugins install`
