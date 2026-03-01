# Architecture

## Overview

SentinelRS is a distributed monitoring system composed of four independently deployable binaries that communicate through gRPC, NATS JetStream and a shared TimescaleDB database.

```
                              ┌─────────────────────────────────┐
                              │           Server                │
Agent 1 ──gRPC/TLS──────────▶│  ┌─────────┐  ┌──────────┐     │
Agent 2 ──gRPC/TLS──────────▶│  │ gRPC    │  │ REST API │     │    NATS JetStream
Agent N ──gRPC/TLS──────────▶│  │ Gateway │  │ :8080    │     │───────────────────┐
                              │  │ :50051  │  └──────────┘     │                   │
                              │  └────┬────┘                   │                   ▼
                              │       │ validate + publish     │           ┌──────────────┐
          CLI ──REST─────────▶│       └────────────────────────│──────────▶│   Workers    │
                              └─────────────────────────────────┘           │              │
                                                                           │ Consumer     │
                                                                           │ Aggregator   │
                                                                           │ Alert Engine │
                                                                           │ Notifiers    │
                                                                           └──────┬───────┘
                                                                                  │
                                                                           TimescaleDB
```

## Data Flow

1. **Collection** — The agent collects system metrics (CPU, memory, disk) via built-in collectors and optional WASM plugins on a configurable interval.

2. **Batching & Signing** — Metrics are grouped into protobuf `Batch` messages. Each batch is signed with HMAC-SHA256 using the agent's secret key, then written to the local append-only WAL.

3. **Transmission** — The exporter reads unacked WAL entries and sends them to the server over gRPC (with optional TLS/mTLS). On failure, the WAL retains the data for retry. An HTTP fallback exporter is available.

4. **Ingestion** — The server validates the HMAC signature, checks for replay attacks (timestamp window), deduplicates via the idempotency store, then publishes the batch to NATS JetStream.

5. **Processing** — Workers pull batches from NATS JetStream via a durable pull consumer. Each batch is decoded, ingested into the rolling aggregator, evaluated against alert rules, and persisted to TimescaleDB.

6. **Alerting** — The alert engine evaluates rules against aggregated metrics. When a threshold is breached (optionally after a `for_duration` hold), a `Firing` event is generated. When the condition clears, a `Resolved` event follows. Events are persisted and dispatched to configured notifiers.

7. **Query** — The REST API and CLI provide read access to agents, metrics, alert rules and system health.

## Crate Responsibilities

### sentinel_common

Shared library used by all other crates.

- **Protobuf definitions** — `Metric`, `Batch`, `RegisterRequest/Response`, `Heartbeat`, `PushResponse`, `AgentService` (generated via `tonic-build` + `prost-build`)
- **Crypto** — HMAC-SHA256 signing/verification, secret generation
- **NATS config** — Stream name, subjects, retention settings
- **Utilities** — Batch ID generation, sequence numbers, trace IDs, canonical path resolution, retry helpers

### sentinel_agent

Standalone binary that runs on each monitored host.

| Module | Purpose |
|---|---|
| `collector/` | System metrics collection via `sysinfo` (CPU, memory, disk) |
| `plugin/` | WASM plugin runtime (wasmtime) — manifest loading, sandboxed execution, host functions |
| `buffer/` | Append-only WAL with segmented files, CRC32 integrity, compaction |
| `scheduler/` | Periodic collection scheduling |
| `exporter/` | gRPC exporter (primary) + HTTP fallback |
| `batch.rs` | Batching collected metrics into protobuf messages |
| `security/` | Encrypted key store (AES-256-GCM), HMAC signer, compression |
| `config/` | YAML config loading and validation |

### sentinel_server

Stateless ingestion gateway. Runs two listeners concurrently:

| Component | Port | Purpose |
|---|---|---|
| gRPC server | 50051 | Agent registration, metric push, heartbeats |
| REST API | 8080 | Admin endpoints — agents, rules, notifiers, metrics, health |

Internal components:
- **Auth** — Hand-rolled JWT (HMAC-SHA256) for REST, HMAC signature verification for gRPC batches
- **Middleware** — Rate limiting (configurable RPS), replay window enforcement
- **Broker** — In-memory NATS publisher (publishes validated batches to JetStream)
- **Stores** — Agent store, idempotency store, rule store (in-memory, behind `DashMap`)

### sentinel_workers

Background processing service. Connects to both NATS and TimescaleDB.

| Module | Purpose |
|---|---|
| `consumer/` | NATS JetStream pull consumer — durable, explicit ack, max 5 redeliveries |
| `aggregator/` | Rolling time-series windows — avg, min, max, last, count per (agent, metric) |
| `alert/` | Rule evaluation engine with FSM state tracking (Ok → Pending → Firing → Resolved) |
| `dedup/` | Batch deduplication |
| `metrics/` | Worker-level Prometheus metrics |
| `api/` | Health/metrics HTTP endpoint |

### sentinel_cli

Admin command-line tool. Communicates with the server REST API and reads local agent config/WAL.

Commands: `register`, `config`, `wal`, `force-send`, `agents`, `rules`, `notifiers`, `key`, `health`, `status`, `tail-logs`, `version`.

Full reference: [cli.md](cli.md)

## Database Schema

TimescaleDB (PostgreSQL 14+) with the following tables:

| Table | Type | Purpose |
|---|---|---|
| `metrics_time` | Hypertable (1-day chunks) | Structured metric storage with labels |
| `metrics_raw` | Hypertable (1-day chunks) | Raw batch payloads (JSONB) |
| `alerts` | Regular table | Alert events (firing/resolved) |
| `alert_rules` | Regular table | Alert rule definitions |
| `notifications_dlq` | Regular table | Failed notification dead-letter queue |
| `mv_metrics_1h` | Continuous aggregate | 1-hour rollups (avg, min, max, count) |

Views: `v_top_metrics`, `v_recent_values`, `v_active_alerts`.

Retention policies: 7 days for raw data, 90 days for structured metrics.

Full schema in `migrations/`.

## Protocol

Communication between agent and server uses Protocol Buffers over gRPC.

### Services

```protobuf
service AgentService {
  rpc Register(RegisterRequest) returns (RegisterResponse);
  rpc PushMetrics(Batch) returns (PushResponse);
  rpc SendHeartbeat(Heartbeat) returns (PushResponse);
}
```

### Key Messages

- **Batch** — Container for metrics with `agent_id`, `batch_id`, sequence range, timestamp and metadata
- **Metric** — Name, labels (key-value), type (gauge/counter/histogram), value and timestamp
- **RegisterRequest/Response** — Hardware ID exchange for agent ID + secret provisioning

Full definition: `crates/common/proto/sentinel.proto`
