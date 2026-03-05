# API Reference

Base URL: `http://localhost:8080`

All responses are JSON. Errors return `{"error": "message"}` with appropriate HTTP status codes.

---

## Health

### `GET /healthz`

Health check.

```bash
curl http://localhost:8080/healthz
```

```json
{ "status": "ok" }
```

### `GET /ready`

Readiness probe. Returns `200 OK` when the server is ready to accept traffic.

```bash
curl http://localhost:8080/ready
```

### `GET /metrics`

Prometheus-format metrics exposition.

```bash
curl http://localhost:8080/metrics
```

---

## Agents

### `GET /v1/agents`

List all registered agents with enriched session data.

```bash
curl http://localhost:8080/v1/agents
```

```json
[
    {
        "agent_id": "prod-db",
        "hw_id": "abc123",
        "agent_version": "0.1.0",
        "registered_at_ms": 1709568000000,
        "last_seen": "2026-03-04T12:00:00Z",
        "status": "online",
        "connected_since": "2026-03-02T10:30:00Z",
        "connection_duration_ms": 180000000,
        "latency_ms": 12.5,
        "cpu_percent": 23.4,
        "memory_percent": 72.1,
        "disk_percent": 45.0,
        "uptime_seconds": 864000,
        "os_name": "Ubuntu 22.04",
        "hostname": "prod-db-01",
        "connection_quality": "Excellent",
        "heartbeat_count": 15000
    }
]
```

**Status values:** `online`, `offline`, `stale`, `bootstrapping`

- `online`: Active gRPC stream with recent heartbeat
- `offline`: No active stream
- `stale`: Active stream but heartbeat missed (> 3× interval)
- `bootstrapping`: Provisioned but never connected

Fields `connected_since` through `heartbeat_count` are `null` for offline/bootstrapping agents.

### `GET /v1/agents/:agent_id`

Get a single agent by ID.

```bash
curl http://localhost:8080/v1/agents/prod-db
```

Same response shape as a single element from the list endpoint.

**Errors:**

- `404` — Agent not found

### `GET /v1/agents/:agent_id/health`

Detailed health information for a connected agent. Includes latency percentiles and full system stats.

```bash
curl http://localhost:8080/v1/agents/prod-db/health
```

```json
{
    "agent_id": "prod-db",
    "status": "online",
    "connected_since": "2026-03-02T10:30:00Z",
    "connection_duration_ms": 180000000,
    "connection_quality": "Excellent",
    "capabilities": ["metrics", "plugins"],
    "heartbeat_count": 15000,
    "heartbeat_interval_ms": 30000,
    "latency": {
        "avg_ms": 12.5,
        "min_ms": 8.0,
        "max_ms": 45.0,
        "p50_ms": 11.0,
        "p95_ms": 28.0,
        "p99_ms": 42.0,
        "jitter_ms": 3.2,
        "sample_count": 128
    },
    "system": {
        "cpu_percent": 23.4,
        "memory_used_bytes": 12345678912,
        "memory_total_bytes": 17179869184,
        "memory_percent": 72.1,
        "disk_used_bytes": 107374182400,
        "disk_total_bytes": 214748364800,
        "disk_percent": 50.0,
        "load_avg_1m": 1.23,
        "process_count": 342,
        "uptime_seconds": 864000,
        "os_name": "Ubuntu 22.04",
        "hostname": "prod-db-01"
    }
}
```

**Errors:**

- `404` — Agent not connected (no active session)

### `GET /v1/agents/:agent_id/live`

Live session snapshot for a connected agent. Raw session data from the registry.

```bash
curl http://localhost:8080/v1/agents/prod-db/live
```

**Errors:**

- `404` — Agent not connected

### `POST /v1/agents/:agent_id/rotate-key`

Rotate the HMAC signing key for an agent.

```bash
curl -X POST http://localhost:8080/v1/agents/prod-db/rotate-key
```

The server generates a new key and pushes it to the agent via the gRPC stream. The old key remains valid for the configured grace period (default: 24h).

### `POST /v1/agents/generate-install`

Generate a bootstrap token and one-liner install command.

```bash
curl -X POST http://localhost:8080/v1/agents/generate-install \
  -H "Content-Type: application/json" \
  -d '{"agent_id": "new-server", "server_url": "grpc://my-server:50051"}'
```

```json
{
    "agent_id": "new-server",
    "token": "bootstrap-token-value",
    "install_command": "curl -sSL ... | sh -s -- --token bootstrap-token-value --server grpc://my-server:50051"
}
```

---

## Fleet

### `GET /v1/fleet/overview`

Fleet-wide overview with status counts and resource averages.

```bash
curl http://localhost:8080/v1/fleet/overview
```

```json
{
    "total": 10,
    "online": 8,
    "offline": 1,
    "stale": 1,
    "bootstrapping": 0,
    "avg_cpu_percent": 34.5,
    "avg_memory_percent": 61.2,
    "avg_latency_ms": 15.3,
    "agents": [
        {
            "agent_id": "prod-db",
            "status": "online",
            "cpu_percent": 23.4,
            "...": "..."
        }
    ]
}
```

Averages are computed from online agents only.

---

## Alert Rules

### `GET /v1/rules`

List all alert rules.

```bash
curl http://localhost:8080/v1/rules
```

```json
[
    {
        "id": "uuid",
        "name": "High CPU",
        "agent_pattern": "*",
        "metric_name": "cpu.usage_percent",
        "condition": "gt",
        "threshold": 90.0,
        "for_duration_ms": 60000,
        "severity": "critical",
        "annotations": { "summary": "CPU > 90%" },
        "enabled": true,
        "notifier_ids": ["notifier-uuid"],
        "created_at": "2026-03-01T00:00:00Z",
        "updated_at": "2026-03-01T00:00:00Z"
    }
]
```

### `POST /v1/rules`

Create an alert rule.

```bash
curl -X POST http://localhost:8080/v1/rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "High CPU",
    "agent_pattern": "*",
    "metric_name": "cpu.usage_percent",
    "condition": "gt",
    "threshold": 90.0,
    "for_duration_ms": 60000,
    "severity": "critical",
    "annotations": {"summary": "CPU exceeds 90%"},
    "notifier_ids": []
  }'
```

**Required fields:** `name`, `metric_name`, `condition`, `threshold`, `severity`

**Conditions:** `gt`, `gte`, `lt`, `lte`, `eq`, `ne`

**Severity:** `info`, `warning`, `critical`

### `GET /v1/rules/:rule_id`

Get a single rule.

```bash
curl http://localhost:8080/v1/rules/uuid
```

### `PUT /v1/rules/:rule_id`

Update a rule. Send the full rule object.

```bash
curl -X PUT http://localhost:8080/v1/rules/uuid \
  -H "Content-Type: application/json" \
  -d '{"name": "High CPU (updated)", "threshold": 95.0, "..."}'
```

### `DELETE /v1/rules/:rule_id`

Delete a rule.

```bash
curl -X DELETE http://localhost:8080/v1/rules/uuid
```

---

## Alerts

### `GET /v1/alerts`

List fired alerts. Supports query filters.

```bash
# All alerts
curl http://localhost:8080/v1/alerts

# Filter by agent
curl "http://localhost:8080/v1/alerts?agent_id=prod-db"

# Filter by status
curl "http://localhost:8080/v1/alerts?status=firing"

# Filter by rule
curl "http://localhost:8080/v1/alerts?rule_id=uuid"
```

```json
[
    {
        "id": "alert-uuid",
        "fingerprint": "hash",
        "rule_id": "rule-uuid",
        "rule_name": "High CPU",
        "agent_id": "prod-db",
        "metric_name": "cpu.usage_percent",
        "severity": "critical",
        "status": "firing",
        "value": 95.2,
        "threshold": 90.0,
        "fired_at": "2026-03-04T12:00:00Z",
        "resolved_at": null,
        "annotations": { "summary": "CPU exceeds 90%" }
    }
]
```

### `GET /v1/alerts/:alert_id`

Get a single alert.

```bash
curl http://localhost:8080/v1/alerts/alert-uuid
```

---

## Notifiers

### `GET /v1/notifiers`

List all notification channel configurations.

```bash
curl http://localhost:8080/v1/notifiers
```

```json
[
    {
        "id": "uuid",
        "name": "alerts-discord",
        "ntype": "discord",
        "config": { "webhook_url": "https://discord.com/api/webhooks/..." },
        "enabled": true,
        "created_at": "2026-03-01T00:00:00Z"
    }
]
```

### `POST /v1/notifiers`

Create a notifier.

```bash
curl -X POST http://localhost:8080/v1/notifiers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "alerts-discord",
    "ntype": "discord",
    "config": {"webhook_url": "https://discord.com/api/webhooks/..."}
  }'
```

**Supported types and required config:**

| Type        | Required Fields        | Notes                                         |
| ----------- | ---------------------- | --------------------------------------------- |
| `webhook`   | `url`                  | Optional `secret` for HMAC signing            |
| `slack`     | `webhook_url`          | Must contain `hooks.slack.com`                |
| `discord`   | `webhook_url`          | Must contain `discord.com/api/webhooks`       |
| `smtp`      | `host`, `from`, `to`   | Optional `port` (587), `username`, `password` |
| `telegram`  | `bot_token`, `chat_id` |                                               |
| `pagerduty` | `routing_key`          | Min 10 chars, uses Events API v2              |
| `teams`     | `webhook_url`          | Sends Adaptive Cards v1.4                     |
| `opsgenie`  | `api_key`              | Creates/closes alerts via v2 API              |
| `gotify`    | `server_url`, `token`  |                                               |
| `ntfy`      | `server_url`, `topic`  | Optional `token` for Bearer auth              |

### `GET /v1/notifiers/:notifier_id`

Get a single notifier config.

### `PUT /v1/notifiers/:notifier_id`

Update a notifier.

```bash
curl -X PUT http://localhost:8080/v1/notifiers/uuid \
  -H "Content-Type: application/json" \
  -d '{"name": "alerts-discord-v2", "config": {"webhook_url": "https://..."}}'
```

### `DELETE /v1/notifiers/:notifier_id`

Delete a notifier.

```bash
curl -X DELETE http://localhost:8080/v1/notifiers/uuid
```

### `POST /v1/notifiers/:notifier_id/toggle`

Toggle a notifier enabled/disabled.

```bash
curl -X POST http://localhost:8080/v1/notifiers/uuid/toggle
```

### `POST /v1/notifiers/test`

Send a test notification to validate config.

```bash
curl -X POST http://localhost:8080/v1/notifiers/test \
  -H "Content-Type: application/json" \
  -d '{
    "ntype": "discord",
    "config": {"webhook_url": "https://discord.com/api/webhooks/..."}
  }'
```

Returns `200` on success, `400` with error details on failure.

---

## Notification History

### `GET /v1/notifications/history`

Delivery history with filters.

```bash
# All history
curl http://localhost:8080/v1/notifications/history

# Filter by notifier
curl "http://localhost:8080/v1/notifications/history?notifier_id=uuid"

# Filter by alert
curl "http://localhost:8080/v1/notifications/history?alert_id=uuid"
```

```json
[
    {
        "id": "uuid",
        "alert_id": "alert-uuid",
        "notifier_id": "notifier-uuid",
        "ntype": "discord",
        "status": "sent",
        "error": null,
        "attempts": 1,
        "duration_ms": 245,
        "sent_at": "2026-03-04T12:00:05Z"
    }
]
```

**Status values:** `sent`, `failed`

### `GET /v1/notifications/stats`

Aggregate notification statistics.

```bash
curl http://localhost:8080/v1/notifications/stats
```

```json
{
    "total": 1542,
    "sent": 1530,
    "failed": 12,
    "avg_duration_ms": 180.5
}
```

---

## Metrics

### `GET /v1/metrics/agents/:agent_id/latest`

Latest metric values for an agent.

```bash
curl http://localhost:8080/v1/metrics/agents/prod-db/latest
```

```json
[
    {
        "name": "cpu.usage_percent",
        "value": 23.4,
        "time": "2026-03-04T12:00:00Z"
    },
    {
        "name": "memory.used_bytes",
        "value": 12345678912,
        "time": "2026-03-04T12:00:00Z"
    }
]
```

### `GET /v1/metrics/agents/:agent_id/history`

Time-series history for a specific metric.

```bash
curl "http://localhost:8080/v1/metrics/agents/prod-db/history?metric=cpu.usage_percent&hours=24"
```

| Parameter | Required | Default | Description              |
| --------- | -------- | ------- | ------------------------ |
| `metric`  | yes      | —       | Metric name              |
| `hours`   | no       | `24`    | Lookback window in hours |

```json
[
    { "time": "2026-03-03T12:00:00Z", "value": 22.1 },
    { "time": "2026-03-03T12:05:00Z", "value": 24.3 }
]
```

### `GET /v1/metrics/agents/:agent_id/names`

List all metric names collected for an agent.

```bash
curl http://localhost:8080/v1/metrics/agents/prod-db/names
```

```json
[
    "cpu.usage_percent",
    "memory.used_bytes",
    "memory.total_bytes",
    "disk.usage_percent"
]
```

### `GET /v1/metrics/agents/:agent_id/aggregates`

Pre-computed aggregates from continuous aggregate views.

```bash
curl "http://localhost:8080/v1/metrics/agents/prod-db/aggregates?metric=cpu.usage_percent&bucket=5m&hours=6"
```

| Parameter | Required | Default | Description     |
| --------- | -------- | ------- | --------------- |
| `metric`  | yes      | —       | Metric name     |
| `bucket`  | no       | `5m`    | `5m` or `1h`    |
| `hours`   | no       | `6`     | Lookback window |

```json
[
    {
        "bucket": "2026-03-04T11:00:00Z",
        "avg": 23.5,
        "min": 18.0,
        "max": 31.2,
        "count": 60
    }
]
```

### `GET /v1/metrics/agents/:agent_id/percentiles`

Percentile analysis for a metric.

```bash
curl "http://localhost:8080/v1/metrics/agents/prod-db/percentiles?metric=cpu.usage_percent&hours=24"
```

```json
{
    "metric": "cpu.usage_percent",
    "agent_id": "prod-db",
    "p50": 23.0,
    "p90": 45.0,
    "p95": 62.0,
    "p99": 88.0,
    "min": 5.0,
    "max": 95.0,
    "count": 8640
}
```

### `GET /v1/metrics/agents/:agent_id/top`

Top metrics by sample count.

```bash
curl "http://localhost:8080/v1/metrics/agents/prod-db/top?limit=5&hours=24"
```

| Parameter | Default | Description       |
| --------- | ------- | ----------------- |
| `limit`   | `10`    | Number of metrics |
| `hours`   | `24`    | Lookback window   |

### `GET /v1/metrics/agents/:agent_id/export`

Export raw metrics as JSON or CSV.

```bash
# JSON (default)
curl "http://localhost:8080/v1/metrics/agents/prod-db/export?hours=24"

# CSV
curl "http://localhost:8080/v1/metrics/agents/prod-db/export?hours=24&format=csv"
```

| Parameter | Default | Description      |
| --------- | ------- | ---------------- |
| `hours`   | `24`    | Lookback window  |
| `metric`  | —       | Filter by metric |
| `format`  | `json`  | `json` or `csv`  |

### `GET /v1/metrics/summary`

Fleet-wide metrics summary (all agents).

```bash
curl http://localhost:8080/v1/metrics/summary
```

### `GET /v1/metrics/compare`

Compare a metric across multiple agents.

```bash
curl "http://localhost:8080/v1/metrics/compare?metric=cpu.usage_percent&agents=prod-db,staging-web&hours=6"
```

| Parameter | Required | Description               |
| --------- | -------- | ------------------------- |
| `metric`  | yes      | Metric name               |
| `agents`  | yes      | Comma-separated agent IDs |
| `hours`   | no       | Lookback window           |

### `GET /v1/metrics/heatmap`

Heatmap data for a metric across all agents.

```bash
curl "http://localhost:8080/v1/metrics/heatmap?metric=cpu.usage_percent&hours=6"
```

---

## Cluster

### `GET /v1/cluster/status`

Cluster-wide statistics.

```bash
curl http://localhost:8080/v1/cluster/status
```

```json
{
    "connected_agents": 8,
    "total_heartbeats": 45000,
    "avg_latency_ms": 15.3,
    "uptime_seconds": 864000
}
```

### `GET /v1/cluster/agents`

List IDs of currently connected agents.

```bash
curl http://localhost:8080/v1/cluster/agents
```

```json
["prod-db", "staging-web", "dev-worker"]
```

### `GET /v1/cluster/events`

Server-Sent Events (SSE) stream of agent presence events.

```bash
curl -N http://localhost:8080/v1/cluster/events
```

```
data: {"event":"connected","agent_id":"prod-db","timestamp":"2026-03-04T12:00:00Z"}

data: {"event":"disconnected","agent_id":"dev-worker","timestamp":"2026-03-04T12:01:00Z"}
```

Event types: `connected`, `disconnected`, `heartbeat`

Keep the connection open to receive real-time events. Use `Ctrl+C` to stop.
