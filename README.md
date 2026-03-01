<p align="center">
  <strong>SentinelRS</strong><br>
  <em>Lightweight distributed monitoring in Rust</em>
</p>

<p align="center">
  <a href="https://github.com/RenanYhuel/SentinelRS/actions/workflows/ci.yml"><img src="https://github.com/RenanYhuel/SentinelRS/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BSL_1.1-blue.svg" alt="License"></a>
  <img src="https://img.shields.io/badge/rust-stable-orange.svg" alt="Rust">
</p>

---

SentinelRS collects, signs and streams system metrics from edge agents to a central ingestion gateway, then into TimescaleDB for storage, alerting and dashboarding.
Built for **reliability** (append-only WAL), **safe extensibility** (sandboxed WASM plugins) and **scalable ingestion** (NATS JetStream).

## Architecture

```
Agent                          Server                      Workers
┌──────────────┐  gRPC/TLS    ┌──────────────┐   NATS     ┌──────────────┐
│ Collectors   │─────────────▶│ Auth (HMAC)  │──────────▶│ Consumer     │
│ WASM Plugins │              │ Validation   │           │ Aggregator   │
│ WAL buffer   │              │ REST API     │           │ Alert Engine │
│ Batch+Sign   │              │ Rate Limit   │           │ Notifiers    │
└──────────────┘              └──────────────┘           └──────┬───────┘
                                                                │
                                                         TimescaleDB
```

> Full architecture details: [docs/architecture.md](docs/architecture.md)

## Highlights

- **gRPC + TLS/mTLS** between agents and server, with HMAC-SHA256 batch signing
- **Append-only WAL** on the agent — no data loss on network failure
- **WASM plugin runtime** (wasmtime) — extend collection with sandboxed user code
- **NATS JetStream** for decoupled, at-least-once delivery to workers
- **TimescaleDB** with hypertables, continuous aggregates and retention policies
- **Alerting engine** with configurable rules, severity levels and multi-channel notifications (webhook, Slack, Discord, SMTP)
- **Admin CLI** for registration, key rotation, WAL inspection, rule management and more

## Workspace

| Crate            | Role                                                                   |
| ---------------- | ---------------------------------------------------------------------- |
| `crates/common`  | Protobuf types, crypto helpers, NATS config                            |
| `crates/agent`   | Agent binary — collectors, WAL, scheduler, gRPC exporter, WASM runtime |
| `crates/server`  | Ingestion gateway — gRPC + REST API, HMAC validation, NATS publisher   |
| `crates/workers` | JetStream consumers, DB writers, aggregator, alert engine, notifiers   |
| `crates/cli`     | Admin CLI — register, keys, WAL, rules, agents, notifiers              |

## Quick Start

### Prerequisites

| Dependency              | Purpose                                       |
| ----------------------- | --------------------------------------------- |
| Rust stable             | Build toolchain                               |
| `protoc`                | Protocol Buffers compiler (for `prost-build`) |
| Docker & Docker Compose | Local NATS + TimescaleDB                      |

### 1. Start infrastructure

```bash
docker compose -f deploy/docker-compose.yml up -d
```

### 2. Build

```bash
cargo build --workspace --release
```

### 3. Generate dev TLS certificates

```bash
./scripts/gen-dev-certs.sh
```

### 4. Run

```bash
# Server (gRPC :50051 + REST :8080 by default)
./target/release/sentinel_server

# Server with custom ports
./target/release/sentinel_server --grpc-port 9051 --rest-port 3000
# or via env: GRPC_PORT=9051 REST_PORT=3000 ./target/release/sentinel_server

# Workers (connects to NATS + TimescaleDB)
NATS_URL=nats://127.0.0.1:4222 ./target/release/sentinel_workers

# Agent
./target/release/sentinel_agent --config config.example.yml
```

### 5. Register an agent

```bash
sentinel register --hw-id my-host-01 --save
```

## Documentation

| Document                               | Description                                             |
| -------------------------------------- | ------------------------------------------------------- |
| [Architecture](docs/architecture.md)   | System design, data flow, crate responsibilities        |
| [CLI Reference](docs/cli.md)           | All commands with usage examples                        |
| [Configuration](docs/configuration.md) | Agent, server and worker settings                       |
| [Deployment](docs/deployment.md)       | Docker Compose, production checklist, TLS setup         |
| [Security](docs/security.md)           | HMAC signing, encryption, key rotation, WASM sandboxing |
| [Development](docs/development.md)     | Building from source, tests, CI pipeline                |
| [Contributing](CONTRIBUTING.md)        | How to contribute                                       |

## CI

GitHub Actions pipeline:

**format** → **clippy** → **tests** → **build** (Linux / Windows / macOS) → **integration smoke**

See [.github/workflows/ci.yml](.github/workflows/ci.yml).

## License

[BSL 1.1](LICENSE) — source-available. Converts to Apache 2.0 on 2030-03-01.
