# Configuration

## Agent Configuration

The agent reads its configuration from a YAML file. Default path: `./sentinel-agent.yml`.

### Full Reference

```yaml
# Unique agent identifier. Set to null for auto-generation from hostname.
agent_id: null

# Server connection
server: "http://localhost:50051"

# Shared secret for HMAC signing (can also be set via env var)
secret: null

# Collection settings
collect:
    interval_seconds: 10 # How often to collect metrics
    cpu: true # Collect CPU usage
    memory: true # Collect memory usage
    disk: true # Collect disk usage

# WASM plugin directory
plugins_dir: "./plugins"

# Write-Ahead Log buffer
buffer:
    wal_dir: "./data/wal" # WAL storage directory
    segment_size_mb: 16 # Max segment file size
    max_retention_days: 7 # Auto-cleanup after N days

# Security settings
security:
    key_store: "auto" # "auto" | "file" | path to key file
    rotation_check_interval_hours: 24 # How often to check for key rotation

# Local API port (for health checks and debugging)
api_port: 9100
```

### Agent Secret Resolution

The agent resolves its HMAC secret in order:

1. `secret` field in the YAML config file
2. `SENTINEL_AGENT_SECRET` environment variable
3. `SENTINEL_MASTER_KEY` environment variable

If none is set, the agent cannot authenticate and will fail to connect.

### Environment Variables (Agent)

| Variable                | Description                   | Default                  |
| ----------------------- | ----------------------------- | ------------------------ |
| `SENTINEL_AGENT_SECRET` | Agent HMAC secret             | —                        |
| `SENTINEL_MASTER_KEY`   | Fallback shared key           | —                        |
| `SERVER_URL`            | gRPC server address           | `http://localhost:50051` |
| `COLLECT_INTERVAL`      | Collection interval (seconds) | `10`                     |
| `BOOTSTRAP_TOKEN`       | One-time provisioning token   | —                        |
| `RUST_LOG`              | Log level filter              | `info`                   |

## Server Configuration

The server is configured entirely through CLI flags and environment variables.

### CLI Flags

```bash
sentinel_server \
  --grpc-addr    0.0.0.0:50051 \
  --rest-addr    0.0.0.0:8080 \
  --jwt-secret   "your-jwt-secret" \
  --nats-url     nats://localhost:4222 \
  --database-url "postgres://sentinel:sentinel@localhost:5432/sentinel"
```

### Environment Variables (Server)

| Variable              | Flag             | Default                 | Description                   |
| --------------------- | ---------------- | ----------------------- | ----------------------------- |
| `GRPC_ADDR`           | `--grpc-addr`    | `0.0.0.0:50051`         | gRPC listen address           |
| `REST_ADDR`           | `--rest-addr`    | `0.0.0.0:8080`          | REST API listen address       |
| `JWT_SECRET`          | `--jwt-secret`   | —                       | Secret for JWT token signing  |
| `NATS_URL`            | `--nats-url`     | `nats://localhost:4222` | NATS connection URL           |
| `DATABASE_URL`        | `--database-url` | —                       | PostgreSQL connection string  |
| `RATE_LIMIT_RPS`      | —                | `100`                   | REST API rate limit (req/sec) |
| `KEY_GRACE_PERIOD_MS` | —                | `86400000` (24h)        | Key rotation grace period     |
| `REPLAY_WINDOW_MS`    | —                | `300000` (5min)         | Anti-replay timestamp window  |
| `RUST_LOG`            | —                | `info`                  | Log level filter              |

### TLS Configuration (Optional)

```bash
sentinel_server \
  --tls-cert /path/to/cert.pem \
  --tls-key  /path/to/key.pem \
  --tls-ca   /path/to/ca.pem     # Enables mTLS (client cert verification)
```

Generate development certificates:

```bash
./scripts/gen-dev-certs.sh
```

## Worker Configuration

Workers use environment variables only.

| Variable          | Default                 | Description                  |
| ----------------- | ----------------------- | ---------------------------- |
| `NATS_URL`        | `nats://localhost:4222` | NATS connection URL          |
| `DATABASE_URL`    | —                       | PostgreSQL connection string |
| `BATCH_SIZE`      | `100`                   | Batch processing size        |
| `WORKER_API_ADDR` | `0.0.0.0:9200`          | Health check endpoint        |
| `RUST_LOG`        | `info`                  | Log level filter             |

## CLI Configuration

The CLI stores its configuration in `~/.config/sentinel/config.toml`.

### Create with wizard

```bash
sentinel init
```

### Manual creation

```toml
[server]
url = "http://localhost:8080"

[output]
mode = "human"    # "human" or "json"
```

### CLI Global Flags

These override config file values:

| Flag       | Description                       |
| ---------- | --------------------------------- |
| `--json`   | Force JSON output mode            |
| `--server` | Override server URL for this call |
| `--config` | Use alternate config file path    |

### Manage with commands

```bash
# Show current config
sentinel config show

# Set a value
sentinel config set server.url http://my-server:8080

# Interactive editor
sentinel config edit

# Print config file path
sentinel config path

# Reset to defaults
sentinel config reset
```

## Alert Rules

Alert rules are managed via CLI or REST API. JSON schema:

```json
{
    "name": "High CPU",
    "agent_pattern": "*",
    "metric_name": "cpu.usage_percent",
    "condition": "gt",
    "threshold": 90.0,
    "for_duration_ms": 60000,
    "severity": "critical",
    "annotations": {
        "summary": "CPU usage exceeds 90%",
        "runbook": "https://wiki.example.com/high-cpu"
    },
    "enabled": true,
    "notifier_ids": ["uuid-of-discord-notifier"]
}
```

### Conditions

| Condition | Meaning               |
| --------- | --------------------- |
| `gt`      | Greater than          |
| `gte`     | Greater than or equal |
| `lt`      | Less than             |
| `lte`     | Less than or equal    |
| `eq`      | Equal to              |
| `ne`      | Not equal to          |

### Severity Levels

`info`, `warning`, `critical`

## WASM Plugin Manifest

Plugins are loaded from `plugins_dir`. Each plugin needs a WASM file and optional manifest:

```yaml
name: "custom-collector"
version: "1.0.0"
wasm_file: "custom_collector.wasm"
config:
    interval_seconds: 30
    custom_key: "custom_value"
```

See [Plugin Development](plugin-development.md) for writing plugins.

## Log Levels

All binaries use `RUST_LOG` for filtering. Examples:

```bash
# Default
RUST_LOG=info

# Debug for sentinel crates only
RUST_LOG=sentinel_server=debug,sentinel_agent=debug

# Trace everything (very verbose)
RUST_LOG=trace

# Specific module
RUST_LOG=sentinel_server::stream=debug,info
```
