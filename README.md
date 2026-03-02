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
Agent                              Server                         Workers
┌──────────────────┐  gRPC bidi   ┌────────────────────┐  NATS   ┌──────────────┐
│ Collectors       │◀═══════════▶│ Stream Service     │────────▶│ Consumer     │
│ WASM Plugins     │  streaming   │ Presence Tracking  │         │ Aggregator   │
│ WAL buffer       │              │ REST API           │         │ Alert Engine │
│ Batch + Sign     │              │ Provisioning       │         │ Notifiers    │
│ Bootstrap        │              │ Session Registry   │         └──────┬───────┘
└──────────────────┘              └────────────────────┘                │
                                                                TimescaleDB
```

> Full architecture details: [docs/architecture.md](docs/architecture.md)

## Highlights

- **Bidirectional gRPC streaming** — persistent connection with handshake, heartbeats and server-push commands
- **Zero-touch provisioning** — bootstrap tokens for automated agent registration
- **Append-only WAL** on the agent — no data loss on network failure
- **Real-time presence tracking** — live agent status with watchdog and SSE events
- **Docker-first deployment** — full stack in one `docker compose up`
- **WASM plugin runtime** (wasmtime) — extend collection with sandboxed user code
- **NATS JetStream** for decoupled, at-least-once delivery to workers
- **TimescaleDB** with hypertables, continuous aggregates and automatic migrations
- **Alerting engine** with configurable rules, severity levels and multi-channel notifications (webhook, Slack, Discord, SMTP)
- **Admin CLI** — cluster monitoring, live metrics, provisioning, rule management and more

## Workspace

| Crate            | Role                                                                      |
| ---------------- | ------------------------------------------------------------------------- |
| `crates/common`  | Protobuf types (V1 + V2 streaming), crypto helpers, NATS config           |
| `crates/agent`   | Agent binary — collectors, WAL, streaming client, bootstrap, WASM runtime |
| `crates/server`  | Ingestion gateway — gRPC streaming + REST API, provisioning, presence     |
| `crates/workers` | JetStream consumers, DB writers, aggregator, alert engine, notifiers      |
| `crates/cli`     | Admin CLI — cluster, agents, provisioning, rules, WAL, keys               |

## Quick Start (Docker)

```bash
# 1. Start the full stack (server, worker, TimescaleDB, NATS)
docker compose -f deploy/docker-compose.yml up -d

# 2. Configure the CLI
sentinel init

# 3. Generate an install command with a bootstrap token
sentinel agents generate-install --server https://localhost:8080

# 4. Deploy an agent (Docker)
docker compose -f deploy/docker-compose.yml --profile agent up -d

# 5. Verify
sentinel agents list
sentinel cluster status
```

> Full quickstart guide: [docs/quickstart.md](docs/quickstart.md)

### Building from source

```bash
# Prerequisites: Rust stable, protoc, Docker
cargo build --workspace --release
```

## Documentation

| Document                               | Description                                          |
| -------------------------------------- | ---------------------------------------------------- |
| [Quick Start](docs/quickstart.md)      | 5-minute Docker-first setup guide                    |
| [Docker](docs/docker.md)               | Images, compose, scaling, production                 |
| [Architecture](docs/architecture.md)   | System design, data flow, crate responsibilities     |
| [Streaming](docs/streaming.md)         | gRPC V2 bidirectional protocol                       |
| [Provisioning](docs/provisioning.md)   | Bootstrap tokens and zero-touch agent setup          |
| [CLI Reference](docs/cli.md)           | All commands with usage examples                     |
| [CLI (V1 legacy)](docs/cli.md)         | V1 command reference                                 |
| [Configuration](docs/configuration.md) | Agent, server and worker settings                    |
| [Deployment](docs/deployment.md)       | Docker strategy, production checklist, TLS setup     |
| [Security](docs/security.md)           | HMAC signing, encryption, key rotation, WASM sandbox |
| [Development](docs/development.md)     | Building from source, tests, CI pipeline             |
| [Contributing](CONTRIBUTING.md)        | How to contribute                                    |

## CI

GitHub Actions pipeline:

**format** → **clippy** → **tests** → **build** (Linux / Windows / macOS) → **integration smoke**

See [.github/workflows/ci.yml](.github/workflows/ci.yml).

## License

[BSL 1.1](LICENSE) — source-available. Converts to Apache 2.0 on 2030-03-01.
