# Architecture

## Overview

SentinelRS is a distributed monitoring system composed of 4 independently deployable binaries and 2 infrastructure services.

```
┌──────────────────────────────────────────────────────────────────────┐
│                          SentinelRS                                  │
│                                                                      │
│  ┌─────────┐   gRPC bidi    ┌──────────┐   NATS     ┌──────────┐   │
│  │  Agent   │ ─────────────▶ │  Server  │ ─────────▶ │ Workers  │   │
│  │ (N hosts)│ ◀───────────── │          │            │ (N inst) │   │
│  └─────────┘   stream       │          │            └────┬─────┘   │
│       │                      │  REST API│                 │         │
│       │ collect              │  :8080   │                 │ write   │
│       ▼                      └────┬─────┘                 ▼         │
│  ┌─────────┐                      │              ┌──────────────┐   │
│  │ sysinfo │                      │ query        │ TimescaleDB  │   │
│  │ plugins │                      └─────────────▶│  (Postgres)  │   │
│  └─────────┘                                     └──────────────┘   │
│                                                                      │
│  ┌─────────┐                                                        │
│  │   CLI   │  REST ──▶ Server :8080                                 │
│  └─────────┘                                                        │
└──────────────────────────────────────────────────────────────────────┘
```

## Components

### Agent (`sentinel_agent`)

Runs on every monitored host. Collects system metrics (CPU, memory, disk, load, processes) via `sysinfo`, optionally augmented by WASM plugins. Sends signed metric batches to the server over a persistent gRPC bidirectional stream.

**Key modules:**

| Module       | Path                   | Purpose                                   |
| ------------ | ---------------------- | ----------------------------------------- |
| Collector    | `agent/src/collector/` | System metrics collection via sysinfo     |
| Plugin       | `agent/src/plugin/`    | WASM plugin loader and runtime (wasmtime) |
| Buffer (WAL) | `agent/src/buffer/`    | Write-Ahead Log for crash resilience      |
| Security     | `agent/src/security/`  | HMAC signing, key rotation, key store     |
| Scheduler    | `agent/src/scheduler/` | Collection interval timer                 |
| Exporter     | `agent/src/exporter/`  | Batch building, signing, gRPC send        |
| Config       | `agent/src/config/`    | YAML config loader and validation         |

**Data flow:**

1. Scheduler triggers collection at configured interval (default: 10s)
2. Collector gathers system metrics + plugin metrics
3. Metrics batched, HMAC-SHA256 signed, optionally gzip compressed
4. Sent over the gRPC stream; if server is unreachable, written to WAL
5. WAL replays unacknowledged batches on reconnection

### Server (`sentinel_server`)

Single binary serving two protocols:

- **gRPC** (port 50051): Bidirectional streaming with agents — handshake, metrics ingestion, heartbeats, commands
- **REST** (port 8080): JSON API for the CLI, dashboards, and integrations

**Key modules:**

| Module        | Path                            | Purpose                                  |
| ------------- | ------------------------------- | ---------------------------------------- |
| Stream        | `server/src/stream/`            | gRPC stream handler, session management  |
| Registry      | `server/src/stream/registry.rs` | In-memory agent sessions (DashMap)       |
| REST handlers | `server/src/rest/`              | All REST endpoints (agents, rules, etc.) |
| Auth          | `server/src/auth/`              | JWT validation, HMAC verification        |
| TLS           | `server/src/tls.rs`             | Optional TLS/mTLS configuration          |

**Session tracking:**

The `SessionRegistry` maintains a `DashMap<String, Session>` of all connected agents with:

- Connection timestamp, last heartbeat, heartbeat count
- Live system stats (CPU, memory, disk, load, uptime)
- Latency tracker (sliding window of 128 samples, percentiles)
- Connection quality assessment (Excellent/Good/Fair/Poor)

### Workers (`sentinel_workers`)

Consume metric batches from NATS JetStream, write to TimescaleDB, evaluate alert rules, and dispatch notifications.

**Key modules:**

| Module          | Path                                 | Purpose                                |
| --------------- | ------------------------------------ | -------------------------------------- |
| Consumer        | `workers/src/`                       | NATS JetStream consumer                |
| Alert evaluator | `workers/src/`                       | Rule matching against incoming metrics |
| Notifier        | `workers/src/notifier/`              | 10 notification backends + retry + DLQ |
| Dispatcher      | `workers/src/notifier/dispatcher.rs` | Routes alerts to configured notifiers  |

### CLI (`sentinel_cli`)

Interactive command-line tool communicating exclusively with the server's REST API. No direct database or NATS access.

**Key modules:**

| Module   | Path              | Purpose                                |
| -------- | ----------------- | -------------------------------------- |
| Commands | `cli/src/cmd/`    | All subcommand implementations         |
| Client   | `cli/src/client/` | HTTP client wrapper (reqwest)          |
| Output   | `cli/src/output/` | Tables, bar charts, sparklines, themes |

### Common (`sentinel_common`)

Shared library used by all crates:

| Module           | Purpose                                     |
| ---------------- | ------------------------------------------- |
| `proto/`         | Protobuf definitions (compiled via `prost`) |
| `crypto.rs`      | HMAC-SHA256 signing and verification        |
| `retry.rs`       | Generic retry with exponential backoff      |
| `batch_id.rs`    | Batch ID generation                         |
| `trace_id.rs`    | Distributed trace ID generation             |
| `seq.rs`         | Monotonic sequence numbers                  |
| `nats_config.rs` | NATS connection helpers                     |
| `metric_json.rs` | Metric serialization                        |

## Data Flow

```
Agent                    Server                NATS             Worker            DB
  │                        │                    │                 │                │
  │── HandshakeRequest ──▶ │                    │                 │                │
  │◀── HandshakeAck ────── │                    │                 │                │
  │                        │                    │                 │                │
  │── MetricsBatch ──────▶ │                    │                 │                │
  │                        │── publish ────────▶│                 │                │
  │                        │                    │── deliver ─────▶│                │
  │                        │                    │                 │── INSERT ─────▶│
  │                        │                    │                 │── evaluate ──▶ │
  │                        │                    │                 │── notify ─────▶│
  │◀── BatchAck ────────── │                    │                 │                │
  │                        │                    │                 │                │
  │── HeartbeatPing ─────▶ │                    │                 │                │
  │◀── HeartbeatPong ───── │                    │                 │                │
```

1. **Handshake**: Agent authenticates with HMAC-signed request; server verifies and creates session
2. **Metrics streaming**: Agent sends batches; server publishes to NATS; replies with ACK/REJECT/RETRY
3. **Processing**: Workers consume from NATS, write to TimescaleDB, evaluate alert rules
4. **Alerting**: Matched rules trigger notifications via configured channels (with retry + DLQ)
5. **Heartbeat**: Periodic ping/pong with system stats for presence tracking and latency measurement
6. **Commands**: Server can push config updates, restart collectors, or update intervals

## Database Schema

TimescaleDB (PostgreSQL + time-series extensions).

### Tables

| Table                  | Purpose                             | Partitioning |
| ---------------------- | ----------------------------------- | ------------ |
| `metrics_time`         | Parsed metrics (hypertable)         | 1-day chunks |
| `metrics_raw`          | Raw batch payloads (hypertable)     | 1-day chunks |
| `alerts`               | Fired alert instances               | —            |
| `alert_rules`          | Alert rule definitions              | —            |
| `agents`               | Registered agent records            | —            |
| `notifier_configs`     | Notification channel configurations | —            |
| `notification_history` | Notification delivery log           | —            |
| `notifications_dlq`    | Dead-letter queue for failed notifs | —            |

### Continuous Aggregates

| View            | Bucket | Columns                         |
| --------------- | ------ | ------------------------------- |
| `mv_metrics_5m` | 5 min  | avg, min, max, count per metric |
| `mv_metrics_1h` | 1 hour | avg, min, max, count per metric |

### Retention Policies

| Table          | Retention |
| -------------- | --------- |
| `metrics_raw`  | 7 days    |
| `metrics_time` | 90 days   |

## Protocol

### gRPC Service

```protobuf
service SentinelStream {
  rpc OpenStream(stream AgentMessage) returns (stream ServerMessage);
}
```

### Message Types

**Agent → Server:**

- `HandshakeRequest` — authentication (agent_id, signature, timestamp, capabilities)
- `MetricsBatch` — signed metric payload
- `HeartbeatPing` — system stats + sequence number

**Server → Agent:**

- `HandshakeAck` — session ID + status
- `BatchAck` — accepted/rejected/retry per batch
- `HeartbeatPong` — latency measurement
- `ConfigUpdate` — push new configuration
- `Command` — reload config, restart collector, update interval
- `ServerError` — error with code and message

See [Streaming Protocol](streaming.md) for the complete protocol specification.

## Network Ports

| Service         | Port  | Protocol | Purpose               |
| --------------- | ----- | -------- | --------------------- |
| Server gRPC     | 50051 | HTTP/2   | Agent streaming       |
| Server REST     | 8080  | HTTP/1.1 | API for CLI/dashboard |
| TimescaleDB     | 5432  | TCP      | PostgreSQL            |
| NATS            | 4222  | TCP      | JetStream messaging   |
| NATS Monitoring | 8222  | HTTP     | NATS health/stats     |
