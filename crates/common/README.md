# sentinel_common

Shared types, Protobuf definitions and helper utilities for the SentinelRS workspace.

## What's inside

| Module | Description |
|---|---|
| `proto` | Auto-generated Rust types from `proto/sentinel.proto` (prost) |
| `canonicalize` | Deterministic serialization of `Batch` for HMAC signing |
| `batch_id` | UUID v4 batch identifier generator |
| `seq` | Atomic monotonic sequence counter |
| `metric_json` | Bidirectional `Metric` ↔ JSON conversion (serde) |

## Protobuf versioning

All `.proto` files live in `proto/` and follow these rules:

1. **Field numbers are permanent** — never reuse or reassign a field number.
2. **Additive only** — new fields get the next available number; existing fields are never removed in a minor version.
3. **`reserved` for deprecation** — deprecated fields must be marked `reserved` with a comment explaining the removal.
4. **Enum zero value** — every enum must have an `UNSPECIFIED = 0` variant.
5. **Backward compatibility** — consumers must tolerate unknown fields gracefully (default prost behavior).

Breaking changes require a major version bump of the crate and coordination across all workspace members.

## Building

The `build.rs` script runs `prost-build` automatically — no manual codegen step needed:

```bash
cargo build -p sentinel_common
```

Generated code lands in `$OUT_DIR/sentinel.common.rs` and is re-exported via `pub mod proto`.
