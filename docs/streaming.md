# gRPC Streaming Protocol (V2)

SentinelRS V2 replaces the V1 unary RPCs with a persistent bidirectional gRPC stream between each agent and the server.

## Service definition

```protobuf
service SentinelStream {
  rpc OpenStream(stream AgentMessage) returns (stream ServerMessage);
}
```

A single `OpenStream` call carries all agent-server communication: handshake, metrics, heartbeats, bootstrap, config updates and commands.

## Message envelopes

### AgentMessage

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

### ServerMessage

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

## Connection lifecycle

```
Agent                                    Server
  │                                        │
  │──── HandshakeRequest ────────────────▶ │  (agent_id, version, key_id, signature)
  │                                        │  validate signature
  │◀──── HandshakeAck ────────────────── │  (status, heartbeat_interval)
  │                                        │
  │──── MetricsBatch ───────────────────▶ │  (batch_id, metrics, signature)
  │◀──── BatchAck ──────────────────────  │  (accepted / rejected / retry)
  │                                        │
  │──── HeartbeatPing ──────────────────▶ │  (timestamp, system_stats)
  │◀──── HeartbeatPong ────────────────── │  (server_time, next_interval)
  │                                        │
  │◀──── ConfigUpdate ────────────────── │  (server-initiated push)
  │◀──── Command ─────────────────────── │  (reload, restart_collector, update_interval)
  │                                        │
  │◀──── ServerError ─────────────────── │  (code, message, fatal)
  │      if fatal → close stream           │
```

## Handshake

The first message on every stream must be a `HandshakeRequest`. The agent signs the request with its HMAC key to prove identity.

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

The server responds with a `HandshakeAck`:

| Status                       | Meaning                                   |
| ---------------------------- | ----------------------------------------- |
| `HANDSHAKE_OK`               | Authenticated, stream is live             |
| `HANDSHAKE_REJECTED`         | Invalid credentials, stream will close    |
| `HANDSHAKE_UPGRADE_REQUIRED` | Agent version too old, must upgrade first |

On `HANDSHAKE_OK`, the server also sends the `heartbeat_interval_ms` the agent should use.

## Metrics streaming

Agents send batches as `MetricsBatch` messages through the open stream. Each batch includes an HMAC signature over its content.

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

The server acknowledges each batch with a `BatchAck`:

| Status           | Agent action                      |
| ---------------- | --------------------------------- |
| `BATCH_ACCEPTED` | Remove from WAL                   |
| `BATCH_REJECTED` | Log error, do not retry           |
| `BATCH_RETRY`    | Keep in WAL, resend after backoff |

## Heartbeats

The agent sends periodic `HeartbeatPing` messages containing a timestamp and live system stats.

```protobuf
message HeartbeatPing {
  int64 timestamp_ms = 1;
  SystemStats system_stats = 2;
}

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

The server uses heartbeats to track agent presence and detect disconnections. If no heartbeat arrives within the expected interval, the server's watchdog marks the agent as disconnected and fires a presence event.

The `HeartbeatPong` response can adjust the next heartbeat interval dynamically.

## Server-initiated messages

### ConfigUpdate

The server can push configuration changes to connected agents:

```protobuf
message ConfigUpdate {
  int64 version = 1;
  bytes config_yaml = 2;
}
```

### Command

Remote commands sent by the server:

| Command            | Effect                                |
| ------------------ | ------------------------------------- |
| `ReloadConfig`     | Agent reloads its config file         |
| `RestartCollector` | Agent restarts metric collection      |
| `UpdateInterval`   | Change collection interval on the fly |

### ServerError

```protobuf
message ServerError {
  uint32 code = 1;
  string message = 2;
  bool fatal = 3;
}
```

When `fatal` is true, the agent closes the stream and initiates reconnection.

## Reconnection

The agent implements automatic reconnection with exponential backoff:

1. Detect stream closure (server error, network failure, or timeout)
2. Wait with exponential backoff (starting at 1s, capped at 60s)
3. Re-establish the stream with a new `HandshakeRequest`
4. Resume sending unacked WAL entries

During disconnection, the agent continues collecting metrics locally. The WAL ensures no data loss.

## Server-side architecture

### Session registry

The `SessionRegistry` tracks all active streaming sessions. It provides:

- Connected agent count
- Per-agent session lookup
- Cluster-wide statistics (`ClusterStats`)

### Presence system

The `PresenceEventBus` emits events when agents connect or disconnect:

```
PresenceEvent {
  agent_id: String,
  event: Connected | Disconnected(reason),
}
```

Disconnect reasons include: `Timeout`, `StreamClosed`, `HandshakeFailure`.

These events are exposed via the REST API's SSE endpoint for real-time monitoring (`sentinel cluster watch`).

### Watchdog

The `spawn_watchdog` background task periodically checks session liveness. If an agent hasn't sent a heartbeat within the configured interval, the watchdog removes the session and emits a disconnect event.

### Stream dispatcher

Incoming `AgentMessage` frames are routed by the dispatcher to specialized handlers:

- `authenticator` — validates handshake signatures
- `metrics_handler` — processes `MetricsBatch`, publishes to NATS
- `heartbeat_handler` — updates presence, records latency

## V1 backward compatibility

The V1 unary `AgentService` RPCs remain available for agents that have not yet upgraded:

```protobuf
service AgentService {
  rpc Register(RegisterRequest) returns (RegisterResponse);
  rpc PushMetrics(Batch) returns (PushResponse);
  rpc SendHeartbeat(Heartbeat) returns (PushResponse);
}
```

V1 agents connect via unary calls and are not tracked in the session registry or presence system. Migration to V2 streaming is recommended.
