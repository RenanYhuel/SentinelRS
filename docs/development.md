# Development

## Prerequisites

| Tool             | Version         | Purpose                   |
| ---------------- | --------------- | ------------------------- |
| Rust             | Stable (latest) | Build toolchain           |
| `protoc`         | 3.x+            | Protocol Buffers compiler |
| Docker + Compose | Any recent      | Local NATS + TimescaleDB  |

### Install protoc

```bash
# Ubuntu/Debian
sudo apt-get install -y protobuf-compiler

# macOS
brew install protobuf

# Windows
choco install protoc
```

## Building

```bash
# Debug build (all crates)
cargo build --workspace

# Release build
cargo build --workspace --release

# Single crate
cargo build -p sentinel_cli --release
```

Binaries are output to `target/release/`:

- `sentinel_server`
- `sentinel_workers`
- `sentinel_agent`
- `sentinel_cli`

## Protobuf Generation

Protobuf types are generated at build time by `sentinel_common`'s `build.rs`:

```bash
cargo build -p sentinel_common
```

Source: `crates/common/proto/sentinel.proto`

The generated code is used by all crates via `sentinel_common::proto`.

## Testing

### Unit Tests

```bash
cargo test --workspace
```

### Integration Tests

Integration tests require NATS and TimescaleDB running locally:

```bash
docker compose -f deploy/docker-compose.yml up -d
cargo test --workspace -- --test-threads=1
```

`--test-threads=1` is required because integration tests share the database.

### Test Structure

| Location                | Coverage                                                |
| ----------------------- | ------------------------------------------------------- |
| `crates/agent/tests/`   | Plugin integration, agent integration                   |
| `crates/server/tests/`  | gRPC integration, REST integration, security acceptance |
| `crates/workers/tests/` | Storage integration, alert harness, aggregator          |
| `crates/cli/src/tests/` | CLI subcommand unit tests                               |

## Linting

```bash
# Clippy (treat warnings as errors)
cargo clippy --workspace --all-targets -- -D warnings

# Formatting check
cargo fmt --all -- --check

# Auto-format
cargo fmt --all
```

## Makefile

Common tasks are available via `make`:

```bash
make build-all    # cargo build --workspace
make fmt          # cargo fmt --all
make proto        # cargo build -p sentinel_common
make compose-up   # docker-compose up -d
make compose-down # docker-compose down
```

## CI Pipeline

The GitHub Actions pipeline (`.github/workflows/ci.yml`) runs on every push and PR:

```
fmt → clippy → test → build → integration-smoke
```

### Stages

| Stage                 | Description                                                                        |
| --------------------- | ---------------------------------------------------------------------------------- |
| **fmt**               | `cargo fmt --all -- --check`                                                       |
| **clippy**            | `cargo clippy --workspace --all-targets -- -D warnings`                            |
| **test**              | `cargo test --workspace`                                                           |
| **build**             | Release builds for Linux (x86_64), Windows (x86_64), macOS (aarch64)               |
| **integration-smoke** | Starts NATS + TimescaleDB services, runs integration tests with `--test-threads=1` |

Build artifacts are packaged as zip files per target and uploaded for each run.

## Project Structure

```
crates/
├── common/          Shared library (protobuf, crypto, NATS config)
│   ├── proto/       Protobuf definitions
│   └── src/
├── agent/           Agent binary
│   └── src/
│       ├── buffer/      WAL (segments, compaction, CRC)
│       ├── collector/   System metrics (sysinfo)
│       ├── config/      YAML config loading
│       ├── exporter/    gRPC + HTTP fallback exporters
│       ├── plugin/      WASM runtime (wasmtime)
│       ├── scheduler/   Collection scheduling
│       └── security/    Key store, HMAC signer, compression
├── server/          Ingestion gateway
│   └── src/
│       ├── auth/        JWT token creation/validation
│       ├── broker/      NATS JetStream publisher
│       ├── grpc/        gRPC service implementation
│       ├── metrics/     Prometheus metrics
│       ├── middleware/   Rate limiting, replay detection
│       ├── rest/        Axum REST API (agents, rules, notifiers, health)
│       └── store/       In-memory stores (agents, idempotency, rules)
├── workers/         Background processors
│   └── src/
│       ├── aggregator/  Rolling time-series (avg, min, max)
│       ├── alert/       Rule engine, FSM state, fingerprinting
│       ├── api/         Health/metrics HTTP endpoint
│       ├── consumer/    NATS JetStream pull consumer
│       ├── dedup/       Batch deduplication
│       └── metrics/     Worker-level Prometheus metrics
└── cli/             Admin CLI
    └── src/
        ├── cmd/         Subcommands (register, wal, rules, etc.)
        ├── output/      Human/JSON output formatting
        └── tests/       CLI unit tests
```

## Key Dependencies

| Dependency      | Version     | Used For                     |
| --------------- | ----------- | ---------------------------- |
| `tokio`         | 1           | Async runtime                |
| `tonic`         | 0.12        | gRPC framework               |
| `prost`         | 0.13        | Protobuf serialization       |
| `axum`          | 0.7         | REST API framework           |
| `async-nats`    | 0.38        | NATS JetStream client        |
| `sqlx`          | 0.8         | PostgreSQL driver (workers)  |
| `wasmtime`      | 29          | WASM plugin runtime (agent)  |
| `sysinfo`       | 0.35        | System metrics collection    |
| `dashmap`       | 6           | Concurrent hash maps         |
| `clap`          | 4           | CLI argument parsing         |
| `aes-gcm`       | 0.10        | Key encryption               |
| `hmac` + `sha2` | 0.12 / 0.10 | HMAC-SHA256 signing          |
| `lettre`        | 0.11        | SMTP notifications (workers) |
| `reqwest`       | 0.12        | HTTP client (webhooks, CLI)  |
