# Release Notes — v2.0.0

## Overview

SentinelRS V2 is a major evolution of the distributed monitoring platform. It introduces persistent bidirectional gRPC streaming, zero-touch agent provisioning, real-time presence tracking and a Docker-first deployment strategy while maintaining full backward compatibility with V1 agents.

## New features

### Bidirectional gRPC streaming

- Persistent `SentinelStream.OpenStream` connection between agents and server
- Signed handshake with mutual authentication
- Per-batch acknowledgement with accept/reject/retry semantics
- Server-initiated config updates and remote commands (reload, restart, update interval)
- Heartbeat with live system stats (CPU, memory, disk, load, uptime)

### Zero-touch provisioning

- Bootstrap tokens generated via `sentinel agents generate-install`
- Agents auto-register on first start using `BOOTSTRAP_TOKEN` env var
- Server validates token, creates credentials and returns config YAML
- Token scrubbed from process environment after use

### Presence tracking

- `SessionRegistry` tracks all active streaming sessions
- `PresenceEventBus` emits real-time connect/disconnect events
- Watchdog detects silent agents and triggers disconnect events
- SSE endpoint for live cluster monitoring (`sentinel cluster watch`)

### Docker deployment

- 4 Docker images: server, worker, agent, cli
- Multi-stage builds with `rust:1.85-bookworm` / `debian:bookworm-slim`
- Full `docker-compose.yml` with TimescaleDB, NATS, server, worker, agent
- Health checks on all containers
- Worker horizontal scaling via replicas

### CLI V2

- `sentinel init` — interactive setup wizard
- `sentinel doctor` — system diagnostics
- `sentinel agents live` — real-time agent metrics via SSE
- `sentinel agents generate-install` — one-liner provisioning command
- `sentinel cluster status` — cluster overview
- `sentinel cluster agents` — connected agents
- `sentinel cluster watch` — live presence events

### Automatic database migrations

- Server applies SQL migrations on startup
- No manual `psql` commands required

### Structured logging

- JSON-formatted logs with correlation fields (`agent_id`, `batch_id`, `trace_id`)
- Configurable via `RUST_LOG`

### Agent resilience

- Automatic reconnection with exponential backoff (1s to 60s cap)
- WAL continues collecting during disconnection
- Unacked batches replayed after reconnection

## Migration from V1

### Agents

V1 unary RPCs (`AgentService.Register`, `PushMetrics`, `SendHeartbeat`) remain functional. Existing V1 agents continue to work without changes. To benefit from V2 features (streaming, heartbeat presence, server-push commands), update agents to V2 binaries.

### Server

The V2 server serves both V1 unary and V2 streaming RPCs on the same port. No configuration changes required. Migrations run automatically.

### Workers

Workers are unchanged. They continue consuming from NATS JetStream.

### CLI

V1 CLI commands remain available. New V2 commands (`agents generate-install`, `agents live`, `cluster *`) are additive.

### Docker

The recommended deployment method is now Docker Compose. See [docs/docker.md](docs/docker.md) for the full guide. Native binaries remain available for all platforms.

## Breaking changes

None. V2 is fully backward compatible with V1.

## Infrastructure requirements

| Component   | Minimum version  |
| ----------- | ---------------- |
| Rust        | 1.85 (stable)    |
| protoc      | 3.x              |
| Docker      | 24.x             |
| Compose     | v2               |
| NATS        | 2.10 (JetStream) |
| TimescaleDB | latest-pg14      |
