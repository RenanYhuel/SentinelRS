# Architecture

## Overview

SentinelRS is a distributed monitoring system composed of four independently deployable binaries that communicate through gRPC (V1 unary + V2 bidirectional streaming), NATS JetStream and a shared TimescaleDB database.

```
                              ┌─────────────────────────────────┐
                              │           Server                │
Agent 1 ══gRPC Stream════════▶│  ┌─────────────┐ ┌──────────┐  │
Agent 2 ══gRPC Stream════════▶│  │ Stream      │ │ REST API │  │    NATS JetStream
Agent N ══gRPC Stream════════▶│  │ Service     │ │ :8080    │  │───────────────────┐
                              │  │ :50051      │ └──────────┘  │                   │
                              │  └──────┬──────┘               │                   ▼
                              │         │ validate + publish   │           ┌──────────────┐
          CLI ──REST─────────▶│  ┌──────┴──────┐               │           │   Workers    │
                              │  │ Session     │               │──────────▶│              │
                              │  │ Registry    │               │           │ Consumer     │
                              │  │ Presence    │               │           │ Aggregator   │
                              │  │ Provisioning│               │           │ Alert Engine │
                              │  └─────────────┘               │           │ Notifiers    │
                              └─────────────────────────────────┘           └──────┬───────┘
                                                                                  │
                                                                           TimescaleDB
```

## Data Flow

1. **Collection** — The agent collects system metrics (CPU, memory, disk) via built-in collectors and optional WASM plugins on a configurable interval.

2. **Batching & Signing** — Metrics are grouped into protobuf `MetricsBatch` messages. Each batch is signed with HMAC-SHA256 using the agent's secret key, then written to the local append-only WAL.

3. **Streaming** — The agent maintains a persistent bidirectional gRPC stream (`SentinelStream.OpenStream`) to the server. On connection, it performs a signed handshake. Unacked WAL entries are sent as `MetricsBatch` frames and acknowledged individually by the server. Periodic `HeartbeatPing` messages carry live system stats for presence tracking.

4. **Ingestion** — The server's stream dispatcher validates the HMAC signature, checks for replay attacks, deduplicates via the idempotency store, then publishes the batch to NATS JetStream. The session registry tracks all active streams.

5. **Processing** — Workers pull batches from NATS JetStream via a durable pull consumer. Each batch is decoded, ingested into the rolling aggregator, evaluated against alert rules, and persisted to TimescaleDB.

6. **Alerting** — The alert engine evaluates rules against aggregated metrics. When a threshold is breached (optionally after a `for_duration` hold), a `Firing` event is generated. When the condition clears, a `Resolved` event follows. Events are persisted and dispatched to configured notifiers.

7. **Presence** — The server's watchdog monitors heartbeat intervals. Missing heartbeats trigger disconnect events on the `PresenceEventBus`, exposed via SSE for real-time cluster monitoring.

8. **Query** — The REST API and CLI provide read access to agents, metrics, alert rules, cluster status and live presence events.

## Crate Responsibilities

### sentinel_common

Shared library used by all other crates.

- **Protobuf definitions** — V1 (`Batch`, `AgentService`) and V2 (`AgentMessage`, `ServerMessage`, `SentinelStream`) generated via `tonic-build` + `prost-build`
- **Crypto** — HMAC-SHA256 signing/verification, secret generation
- **NATS config** — Stream name, subjects, retention settings
- **Utilities** — Batch ID generation, sequence numbers, trace IDs, canonical path resolution, retry helpers

### sentinel_agent

Standalone binary that runs on each monitored host.

| Module        | Purpose                                                                                |
| ------------- | -------------------------------------------------------------------------------------- |
| `collector/`  | System metrics collection via `sysinfo` (CPU, memory, disk)                            |
| `plugin/`     | WASM plugin runtime (wasmtime) — manifest loading, sandboxed execution, host functions |
| `buffer/`     | Append-only WAL with segmented files, CRC32 integrity, compaction                      |
| `scheduler/`  | Periodic collection scheduling                                                         |
| `stream/`     | V2 bidirectional gRPC client — connection, handshake, sender, receiver, reconnect      |
| `bootstrap/`  | Zero-touch provisioning — token detection, negotiation, config writing                 |
| `exporter/`   | V1 gRPC exporter (unary) + HTTP fallback                                               |
| `batch.rs`    | Batching collected metrics into protobuf messages                                      |
| `security/`   | Encrypted key store (AES-256-GCM), HMAC signer, compression                            |
| `config/`     | YAML config loading and validation                                                     |
| `cli.rs`      | CLI argument parsing (`--config`, `--help`, `--version`)                               |
| `run.rs`      | Async orchestration — wires all modules together                                       |
| `shutdown.rs` | Graceful shutdown on SIGTERM/SIGINT                                                    |
| `api/`        | Local HTTP API — health checks and Prometheus metrics                                  |

### sentinel_server

Stateless ingestion gateway. Runs two listeners concurrently:

| Component   | Default Port | Purpose                                                           |
| ----------- | ------------ | ----------------------------------------------------------------- |
| gRPC server | 50051        | V2 streaming (`SentinelStream`) + V1 unary (`AgentService`)       |
| REST API    | 8080         | Admin endpoints — agents, rules, notifiers, metrics, cluster, SSE |

Both ports are configurable via CLI flags (`--grpc-port`, `--rest-port`) or environment variables (`GRPC_ADDR`, `REST_ADDR`).

Internal components:

- **Stream service** — `StreamService` implementing `SentinelStream.OpenStream`, routing frames to specialized handlers
- **Session registry** — `SessionRegistry` tracking all active streaming sessions with `ClusterStats`
- **Presence** — `PresenceEventBus` emitting connect/disconnect events, `Watchdog` detecting silent agents
- **Provisioning** — `TokenStore`, `BootstrapOutcome` handling zero-touch agent setup
- **Auth** — HMAC-SHA256 for gRPC stream handshake, JWT for REST API
- **Middleware** — Rate limiting, replay window enforcement
- **Broker** — NATS publisher (publishes validated batches to JetStream)
- **Stores** — Agent store, idempotency store, rule store (in-memory, behind `DashMap`)

### sentinel_workers

Background processing service. Connects to both NATS and TimescaleDB.

| Module        | Purpose                                                                           |
| ------------- | --------------------------------------------------------------------------------- |
| `consumer/`   | NATS JetStream pull consumer — durable, explicit ack, max 5 redeliveries          |
| `aggregator/` | Rolling time-series windows — avg, min, max, last, count per (agent, metric)      |
| `alert/`      | Rule evaluation engine with FSM state tracking (Ok → Pending → Firing → Resolved) |
| `dedup/`      | Batch deduplication                                                               |
| `metrics/`    | Worker-level Prometheus metrics                                                   |
| `api/`        | Health/metrics HTTP endpoint                                                      |

### sentinel_cli

Admin command-line tool. Communicates with the server REST API and reads local agent config/WAL.

Commands: `init`, `doctor`, `completions`, `agents` (list, get, live, delete, generate-install), `cluster` (status, agents, watch), `config`, `rules`, `notifiers`, `key`, `wal`, `metrics`, `health`, `status`, `register`, `force-send`, `version`.

Full reference: [cli-v2.md](cli-v2.md)

## Database Schema

TimescaleDB (PostgreSQL 14+) with the following tables:

| Table               | Type                      | Purpose                               |
| ------------------- | ------------------------- | ------------------------------------- |
| `metrics_time`      | Hypertable (1-day chunks) | Structured metric storage with labels |
| `metrics_raw`       | Hypertable (1-day chunks) | Raw batch payloads (JSONB)            |
| `alerts`            | Regular table             | Alert events (firing/resolved)        |
| `alert_rules`       | Regular table             | Alert rule definitions                |
| `notifications_dlq` | Regular table             | Failed notification dead-letter queue |
| `mv_metrics_1h`     | Continuous aggregate      | 1-hour rollups (avg, min, max, count) |

Views: `v_top_metrics`, `v_recent_values`, `v_active_alerts`.

Retention policies: 7 days for raw data, 90 days for structured metrics.

Full schema in `migrations/`.

## Protocol

Communication between agent and server uses Protocol Buffers over gRPC.

### Services

```protobuf
// V2 — Persistent bidirectional stream (preferred)
service SentinelStream {
  rpc OpenStream(stream AgentMessage) returns (stream ServerMessage);
}

// V1 — Legacy unary RPCs (backward compatible)
service AgentService {
  rpc Register(RegisterRequest) returns (RegisterResponse);
  rpc PushMetrics(Batch) returns (PushResponse);
  rpc SendHeartbeat(Heartbeat) returns (PushResponse);
}
```

### Key Messages (V2)

- **AgentMessage** — Envelope carrying `HandshakeRequest`, `MetricsBatch`, `HeartbeatPing` or `BootstrapRequest`
- **ServerMessage** — Envelope carrying `HandshakeAck`, `BatchAck`, `HeartbeatPong`, `BootstrapResponse`, `ConfigUpdate`, `Command` or `ServerError`
- **MetricsBatch** — Container for metrics with batch ID, sequence range, HMAC signature
- **SystemStats** — Live system telemetry (CPU, memory, disk, load, uptime, hostname)

### Key Messages (V1)

- **Batch** — Container for metrics with `agent_id`, `batch_id`, sequence range, timestamp and metadata
- **Metric** — Name, labels (key-value), type (gauge/counter/histogram), value and timestamp
- **RegisterRequest/Response** — Hardware ID exchange for agent ID + secret provisioning

Full definition: `crates/common/proto/sentinel.proto`

Detailed streaming protocol: [streaming.md](streaming.md)
