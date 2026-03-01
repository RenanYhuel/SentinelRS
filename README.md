# SentinelRS

Lightweight distributed monitoring system written in Rust.

Collect, sign and stream metrics from edge agents to a central ingestion service.
Built for reliability (append-only WAL), safe extensibility (WASM plugins) and scalable ingestion (NATS JetStream).

## Architecture

```
Agent (Rust)                    Server (Rust)               Workers (Rust)
┌──────────────┐   gRPC/TLS    ┌──────────────┐   NATS     ┌──────────────┐
│ Collectors   │──────────────▶│ Validation   │──────────▶│ Consumer     │
│ WASM Plugins │               │ Auth (HMAC)  │           │ Transform    │
│ WAL buffer   │               │ REST API     │           │ Alert Engine │
│ Batch+Sign   │               │ Rate Limit   │           │ Notifiers    │
└──────────────┘               └──────────────┘           └──────┬───────┘
                                                                 │
                                                          TimescaleDB
```

## Crates

| Crate            | Description                                                    |
| ---------------- | -------------------------------------------------------------- |
| `crates/common`  | Shared protobuf types, crypto helpers, NATS config             |
| `crates/agent`   | Agent binary — collectors, WAL, scheduler, gRPC exporter       |
| `crates/server`  | Ingestion gateway — gRPC/REST, HMAC validation, NATS publisher |
| `crates/workers` | JetStream consumers, DB writers, alert engine, notifiers       |
| `crates/cli`     | Admin CLI — register, rotate keys, inspect WAL                 |

## Prerequisites

- **Rust** stable toolchain
- **protoc** (Protocol Buffers compiler)
- **NATS** with JetStream enabled
- **PostgreSQL** with TimescaleDB extension

## Getting Started

```bash
# Build
cargo build --workspace --release

# Test
cargo test --workspace

# Format & lint
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

### Local development stack

```bash
docker compose -f deploy/docker-compose.yml up -d
```

Starts NATS and TimescaleDB locally. See `config.example.yml` for agent configuration.

### Generate dev TLS certificates

```bash
./scripts/gen-dev-certs.sh
```

## CI

GitHub Actions pipeline: **format** → **clippy** → **tests** → **build** (Linux, Windows, macOS) → **integration smoke**.

See [.github/workflows/ci.yml](.github/workflows/ci.yml).

## License

MIT
