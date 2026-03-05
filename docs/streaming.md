# gRPC Streaming Protocol

SentinelRS uses a bidirectional gRPC stream between agents and the server.
This document specifies the V2 streaming protocol defined in `sentinel.proto`.

## Service Definition

```protobuf
service SentinelStream {
  rpc OpenStream(stream AgentMessage) returns (stream ServerMessage);
}
```

A single persistent TCP connection carries all communication:
handshake, metrics, heartbeats, bootstrap, config updates, and commands.

---

## Message Envelope

### Agent → Server

```protobuf
message AgentMessage {
  oneof payload {
    HandshakeRequest handshake = 1;
    MetricsBatch metrics_batch = 2;
    HeartbeatPing heartbeat_ping = 3;
    BootstrapRequest bootstrap_request = 4;
  }
}
```

### Server → Agent

```protobuf
message ServerMessage {
  oneof payload {
    HandshakeAck handshake_ack = 1;
    BatchAck batch_ack = 2;
    HeartbeatPong heartbeat_pong = 3;
    BootstrapResponse bootstrap_response = 4;
    ConfigUpdate config_update = 5;
    Command command = 6;
    ServerError error = 7;
  }
}
```

---

## Connection Lifecycle

```
Agent                              Server
  │                                  │
  │─── OpenStream ──────────────────→│
  │                                  │
  │─── HandshakeRequest ───────────→│   (agent_id, key_id, signature)
  │←── HandshakeAck ───────────────│   (OK / REJECTED / UPGRADE_REQUIRED)
  │                                  │
  │─── MetricsBatch ───────────────→│   (signed, sequenced)
  │←── BatchAck ───────────────────│   (ACCEPTED / REJECTED / RETRY)
  │                                  │
  │─── HeartbeatPing ──────────────→│   (system stats)
  │←── HeartbeatPong ──────────────│   (next interval)
  │                                  │
  │←── ConfigUpdate ───────────────│   (server-initiated)
  │←── Command ────────────────────│   (server-initiated)
  │←── ServerError ────────────────│   (fatal = disconnect)
  │                                  │
```

---

## Handshake

The first message on every stream must be a `HandshakeRequest`.

### Request

```protobuf
message HandshakeRequest {
  string agent_id = 1;
  string agent_version = 2;
  repeated string capabilities = 3;
  string key_id = 4;
  int64 timestamp_ms = 5;
  string signature = 6;
}
```

| Field           | Description                                    |
| --------------- | ---------------------------------------------- | ------------- |
| `agent_id`      | Registered agent identifier                    |
| `agent_version` | Agent binary version                           |
| `capabilities`  | Supported features (`metrics`, `plugins`, etc) |
| `key_id`        | HMAC key identifier                            |
| `timestamp_ms`  | Current timestamp (replay protection)          |
| `signature`     | HMAC-SHA256 of `agent_id                       | timestamp_ms` |

### Response

```protobuf
message HandshakeAck {
  HandshakeStatus status = 1;
  string message = 2;
  int64 server_time_ms = 3;
  int64 heartbeat_interval_ms = 4;
}
```

| Status                       | Action                            |
| ---------------------------- | --------------------------------- |
| `HANDSHAKE_OK`               | Stream authenticated, proceed     |
| `HANDSHAKE_REJECTED`         | Invalid credentials, disconnect   |
| `HANDSHAKE_UPGRADE_REQUIRED` | Agent version too old, disconnect |

### Signature Computation

```
signature = HMAC-SHA256(secret, agent_id + "|" + timestamp_ms)
```

The server verifies the signature using the key referenced by `key_id`.
Timestamps outside the replay window (`REPLAY_WINDOW_MS`) are rejected.

---

## Metrics Batching

### MetricsBatch

```protobuf
message MetricsBatch {
  string batch_id = 1;
  uint64 seq_start = 2;
  uint64 seq_end = 3;
  int64 created_at_ms = 4;
  repeated Metric metrics = 5;
  map<string, string> meta = 6;
  string signature = 7;
}
```

| Field           | Description                             |
| --------------- | --------------------------------------- |
| `batch_id`      | Unique batch identifier (UUID)          |
| `seq_start`     | First sequence number in batch          |
| `seq_end`       | Last sequence number in batch           |
| `created_at_ms` | Agent-side creation timestamp           |
| `metrics`       | Array of metric samples                 |
| `meta`          | Arbitrary metadata                      |
| `signature`     | HMAC-SHA256 of serialized batch payload |

### BatchAck

```protobuf
message BatchAck {
  string batch_id = 1;
  BatchAckStatus status = 2;
  string message = 3;
}
```

| Status           | Agent Action                    |
| ---------------- | ------------------------------- |
| `BATCH_ACCEPTED` | Remove batch from WAL           |
| `BATCH_REJECTED` | Log error, do not retry         |
| `BATCH_RETRY`    | Keep in WAL, retry with backoff |

### Metric Format

```protobuf
message Metric {
  string name = 1;
  map<string, string> labels = 2;
  MetricType rtype = 3;
  oneof value {
    double value_double = 4;
    int64 value_int = 5;
    Histogram histogram = 6;
  }
  int64 timestamp_ms = 7;
}

enum MetricType {
  METRIC_TYPE_UNSPECIFIED = 0;
  GAUGE = 1;
  COUNTER = 2;
  HISTOGRAM = 3;
}

message Histogram {
  repeated double boundaries = 1;
  repeated uint64 counts = 2;
  uint64 count = 3;
  double sum = 4;
}
```

---

## Heartbeat

Agents send periodic heartbeats with live system statistics.

### HeartbeatPing

```protobuf
message HeartbeatPing {
  int64 timestamp_ms = 1;
  SystemStats system_stats = 2;
}
```

### SystemStats

```protobuf
message SystemStats {
  double cpu_percent = 1;
  uint64 memory_used_bytes = 2;
  uint64 memory_total_bytes = 3;
  uint64 disk_used_bytes = 4;
  uint64 disk_total_bytes = 5;
  double load_avg_1m = 6;
  uint32 process_count = 7;
  uint64 uptime_seconds = 8;
  string os_name = 9;
  string hostname = 10;
}
```

### HeartbeatPong

```protobuf
message HeartbeatPong {
  int64 server_time_ms = 1;
  int64 next_heartbeat_interval_ms = 2;
}
```

The server can adjust the heartbeat interval dynamically via `next_heartbeat_interval_ms`.

---

## Bootstrap (Zero-Touch Provisioning)

New agents can self-register using a bootstrap token.

### BootstrapRequest

```protobuf
message BootstrapRequest {
  string bootstrap_token = 1;
  string hw_id = 2;
  string agent_version = 3;
}
```

### BootstrapResponse

```protobuf
message BootstrapResponse {
  BootstrapStatus status = 1;
  string agent_id = 2;
  string secret = 3;
  string key_id = 4;
  bytes config_yaml = 5;
  string message = 6;
}
```

| Status                    | Description                   |
| ------------------------- | ----------------------------- |
| `BOOTSTRAP_OK`            | Agent provisioned, save creds |
| `BOOTSTRAP_INVALID_TOKEN` | Token not found               |
| `BOOTSTRAP_EXPIRED_TOKEN` | Token has expired             |

After receiving `BOOTSTRAP_OK`:

1. Agent saves `agent_id`, `secret`, `key_id`
2. Agent writes `config_yaml` to disk
3. Agent disconnects and reconnects with a normal `HandshakeRequest`

---

## Server-Initiated Messages

### ConfigUpdate

```protobuf
message ConfigUpdate {
  int64 version = 1;
  bytes config_yaml = 2;
}
```

Pushed when server-side config changes. Agent applies the new config and reloads.

### Command

```protobuf
message Command {
  string command_id = 1;
  oneof action {
    ReloadConfig reload_config = 2;
    RestartCollector restart_collector = 3;
    UpdateInterval update_interval = 4;
  }
}
```

| Action              | Effect                              |
| ------------------- | ----------------------------------- |
| `reload_config`     | Agent reloads config from disk      |
| `restart_collector` | Agent restarts the metric collector |
| `update_interval`   | Agent changes collection interval   |

### ServerError

```protobuf
message ServerError {
  uint32 code = 1;
  string message = 2;
  bool fatal = 3;
}
```

When `fatal = true`, the agent must disconnect and reconnect with backoff.

---

## Reconnection Strategy

When the stream disconnects:

1. Wait with exponential backoff: 1s, 2s, 4s, 8s ... max 60s
2. Jitter ±25% to avoid thundering herd
3. Re-open stream and send `HandshakeRequest`
4. Resume sending unsent WAL batches from `seq_start`
5. On repeated `HANDSHAKE_REJECTED`, stop reconnecting and log error

---

## Wire Format

- Transport: HTTP/2 (gRPC)
- Serialization: Protocol Buffers v3
- Default port: `50051`
- TLS: Optional (recommended for production)
- mTLS: Supported for mutual authentication

---

## V1 Legacy API

The V1 unary RPCs remain available for backward compatibility:

```protobuf
service AgentService {
  rpc Register(RegisterRequest) returns (RegisterResponse);
  rpc PushMetrics(Batch) returns (PushResponse);
  rpc SendHeartbeat(Heartbeat) returns (PushResponse);
}
```

V1 clients work with the same server. V2 streaming is preferred for:

- Lower latency (persistent connection)
- Server-initiated messages (config, commands)
- Richer heartbeat data (SystemStats)
- Better reconnection semantics
