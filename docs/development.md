# Development

## Prerequisites

| Tool           | Version | Purpose                     |
| -------------- | ------- | --------------------------- |
| Rust           | stable  | `rustup default stable`     |
| protoc         | 3.x+    | Protobuf compiler           |
| Docker         | 20.x+   | Local stack for integration |
| Docker Compose | v2+     | Multi-service orchestration |

## Project Structure

```
SentinelRS/
├── Cargo.toml                  # Workspace definition
├── crates/
│   ├── common/                 # Shared library (proto, crypto, retry)
│   │   ├── proto/sentinel.proto
│   │   └── src/
│   ├── agent/                  # Metrics collector agent
│   │   └── src/
│   │       ├── collector/      # System metrics (sysinfo)
│   │       ├── plugin/         # WASM runtime (wasmtime)
│   │       ├── buffer/         # Write-Ahead Log
│   │       ├── security/       # HMAC, key store
│   │       ├── scheduler/      # Collection timer
│   │       ├── exporter/       # Batch builder + gRPC sender
│   │       └── config/         # YAML config
│   ├── server/                 # gRPC + REST server
│   │   └── src/
│   │       ├── stream/         # gRPC stream, sessions, registry
│   │       ├── rest/           # REST API handlers
│   │       └── auth/           # JWT + HMAC verification
│   ├── workers/                # Background processors
│   │   └── src/
│   │       └── notifier/       # 10 notification backends
│   └── cli/                    # Command-line tool
│       └── src/
│           ├── cmd/            # Subcommand implementations
│           ├── client/         # HTTP client
│           └── output/         # Tables, charts, themes
├── migrations/                 # SQL migration files
├── deploy/                     # Docker Compose + scripts
├── packaging/                  # deb, macOS, MSI packaging
├── scripts/                    # Dev utilities
├── tests/                      # E2E tests
└── docs/                       # Documentation
```

## Build

```bash
# Debug build (fast compilation)
cargo build

# Release build (optimized)
cargo build --release

# Single crate
cargo build -p sentinel_server
cargo build -p sentinel_cli
```

## Protobuf

Protobuf definitions live in `crates/common/proto/sentinel.proto`. Code generation is handled automatically by `build.rs` via `prost-build` — no manual step needed. Generated code is placed in `target/` and included via `tonic::include_proto!`.

## Testing

### Unit Tests

```bash
# All crates
cargo test --workspace --lib

# Single crate
cargo test -p sentinel_server --lib
cargo test -p sentinel_agent --lib
```

### Integration Tests

Integration tests require running NATS and TimescaleDB:

```bash
docker compose -f deploy/docker-compose.yml up -d timescaledb nats
cargo test --workspace
```

### E2E Tests

```bash
cd tests/e2e
./run_e2e.sh
```

Uses `docker-compose.e2e.yml` to spin up the full stack and run scenario scripts.

### Test Locations

| Crate   | Unit Tests          | Integration Tests      |
| ------- | ------------------- | ---------------------- |
| common  | `src/*.rs` (inline) | —                      |
| agent   | `src/*.rs` (inline) | `tests/integration.rs` |
| server  | `src/*.rs` (inline) | `tests/`               |
| workers | `src/*.rs` (inline) | `tests/`               |
| cli     | `src/tests/`        | —                      |

## Linting

```bash
# Clippy (must be zero warnings)
cargo clippy --workspace --all-targets

# Format check
cargo fmt --all --check

# Format fix
cargo fmt --all
```

CI enforces `clippy -- -D warnings` (warnings are errors).

## Makefile

```bash
make test          # cargo test --workspace
make lint          # cargo clippy --workspace -- -D warnings
make fmt           # cargo fmt --all --check
make build         # cargo build --release
make e2e           # docker compose E2E tests
make docker-build  # Build all Docker images
```

## Key Dependencies

| Crate       | Purpose                             |
| ----------- | ----------------------------------- |
| tokio       | Async runtime                       |
| tonic       | gRPC framework                      |
| prost       | Protobuf serialization              |
| axum        | REST API framework                  |
| async-nats  | NATS JetStream client               |
| sqlx        | PostgreSQL async driver             |
| wasmtime    | WASM plugin runtime                 |
| sysinfo     | System metrics collection           |
| dashmap     | Concurrent hash map (session store) |
| clap        | CLI argument parsing                |
| aes-gcm     | Key encryption at rest              |
| hmac + sha2 | Batch signing                       |
| lettre      | SMTP email sending                  |
| reqwest     | HTTP client (webhooks, notifiers)   |
| comfy-table | Table rendering                     |
| colored     | Terminal colors                     |
| dialoguer   | Interactive prompts                 |
| indicatif   | Spinners and progress bars          |

## Adding a New CLI Command

1. Create `crates/cli/src/cmd/<group>/<command>.rs`
2. Add the subcommand variant to the group's `enum` in `mod.rs`
3. Wire `execute()` match arm in `mod.rs`
4. Add `#[command(about = "...", visible_alias = "...")]` for help and aliases

Example structure:

```rust
// crates/cli/src/cmd/agents/health.rs
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct HealthArgs {
    pub id: Option<String>,
}

pub async fn run(args: HealthArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;
    let data = api.get_json(&format!("/v1/agents/{}/health", args.id.unwrap())).await?;
    // render...
    Ok(())
}
```

## Adding a New REST Endpoint

1. Create handler in `crates/server/src/rest/<handler>.rs`
2. Add `mod <handler>;` to `rest/mod.rs`
3. Add `.route(...)` to `rest/router.rs`

Handler pattern:

```rust
use axum::{extract::State, Json};
use crate::rest::AppState;

pub async fn my_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    // use state.pool, state.registry, etc.
    Json(serde_json::json!({"ok": true}))
}
```

## Adding a New WASM Host Function

1. Define the function signature in `crates/agent/src/plugin/`
2. Register it with the Wasmtime linker
3. Document in [Plugin Development](plugin-development.md)

## CI Pipeline

```
fmt → clippy → test → build → integration-smoke
```

All checks must pass before merge. See [CONTRIBUTING.md](../CONTRIBUTING.md) for the full workflow.

## Commit Convention

```
feat(cli): add agents health command
fix(server): handle missing session in health endpoint
docs: rewrite quickstart guide
refactor(agent): extract collector module
test(workers): add notification retry tests
```
