# Configuration

## Agent Configuration

The agent reads a YAML configuration file. Default lookup paths:

1. Path provided via `--config` CLI flag
2. `~/.config/sentinel/agent.yml`
3. `/etc/sentinel/agent.yml`

### Full Reference

```yaml
# Assigned by the server during registration.
# Leave null to auto-detect from hostname.
agent_id: null

# Server URL for gRPC communication.
server: https://sentinel.example.com:8443

# Agent secret (base64-encoded). Used for HMAC batch signing.
# Can also be set via SENTINEL_AGENT_SECRET or SENTINEL_MASTER_KEY env vars.
secret: "base64-encoded-secret-here"

# Metric collection settings.
collect:
    # Interval between collection cycles (seconds). Must be > 0.
    interval_seconds: 10

    # Toggle individual metric collectors.
    metrics:
        cpu: true
        mem: true
        disk: true

# Directory containing WASM plugin files (.wasm + .manifest.yml).
plugins_dir: /var/lib/sentinel/plugins

# Write-Ahead Log settings.
buffer:
    # Directory for WAL segment files. Required.
    wal_dir: /var/lib/sentinel/wal

    # Maximum size of a single WAL segment (MB).
    segment_size_mb: 16

    # Automatically discard WAL data older than this (days).
    max_retention_days: 7

# Security and key management.
security:
    # Key store path. "auto" resolves to ~/.local/share/sentinel/keys.
    key_store: auto

    # How often to check for key rotation (hours).
    rotation_check_interval_hours: 24

# Local HTTP API port for health checks and Prometheus metrics.
# Endpoints: /healthz, /ready, /metrics
api_port: 9090
```

### Validation Rules

| Field                      | Rule              |
| -------------------------- | ----------------- |
| `server`                   | Must be non-empty |
| `collect.interval_seconds` | Must be > 0       |
| `buffer.wal_dir`           | Must be non-empty |

Use `sentinel config validate` to check a config file before deploying.

### Agent CLI Options

```
Usage: sentinel_agent [OPTIONS]

Options:
  -c, --config <PATH>  Configuration file path (required)
  -V, --version        Print version
  -h, --help           Print help
```

### Agent Secret Resolution

The agent resolves its signing secret with the following precedence:

1. `secret` field in the config file (base64-encoded)
2. `SENTINEL_AGENT_SECRET` environment variable
3. `SENTINEL_MASTER_KEY` environment variable

## Server Configuration

The server uses a `ServerConfig` struct with defaults that can be overridden via CLI flags or environment variables.

| Setting               | Default                   | CLI Flag       | Env Var      | Description                        |
| --------------------- | ------------------------- | -------------- | ------------ | ---------------------------------- |
| `grpc_addr`           | `0.0.0.0:50051`           | `--grpc-addr`  | `GRPC_ADDR`  | gRPC listener address              |
| `rest_addr`           | `0.0.0.0:8080`            | `--rest-addr`  | `REST_ADDR`  | REST API listener address          |
| `jwt_secret`          | `change-me-in-production` | `--jwt-secret` | `JWT_SECRET` | Secret for JWT HMAC-SHA256 signing |
| `rate_limit_rps`      | `100`                     | —              | —            | Maximum requests per second        |
| `key_grace_period_ms` | `86400000` (24h)          | —              | —            | Grace period after key rotation    |
| `replay_window_ms`    | `300000` (5min)           | —              | —            | Time window for replay detection   |

**Precedence:** CLI flags > environment variables > defaults.

### Server CLI Options

```
Usage: sentinel_server [OPTIONS]

Options:
      --grpc-addr <ADDR>   gRPC listen address  (default: 0.0.0.0:50051)
      --grpc-port <PORT>   gRPC listen port     (default: 50051)
      --rest-addr <ADDR>   REST listen address  (default: 0.0.0.0:8080)
      --rest-port <PORT>   REST listen port     (default: 8080)
      --jwt-secret <KEY>   JWT signing secret
      --tls-cert <PATH>    TLS certificate path
      --tls-key <PATH>     TLS private key path
      --tls-ca <PATH>      TLS CA certificate path
  -V, --version            Print version
  -h, --help               Print help
```

### TLS

TLS is optional. When configured, both gRPC and REST listeners use TLS.

```yaml
tls:
    cert_path: /path/to/server-cert.pem
    key_path: /path/to/server-key.pem
    ca_path: /path/to/ca-cert.pem # optional — enables mutual TLS
```

### Logging

All binaries use `tracing` with JSON-formatted output. Control verbosity with `RUST_LOG`:

```bash
# Examples
RUST_LOG=info ./sentinel_server
RUST_LOG=sentinel_server=debug,tower=warn ./sentinel_server
RUST_LOG=trace ./sentinel_server    # very verbose — not recommended in production
```

## Worker Configuration

Workers are configured entirely through environment variables.

| Variable          | Default                 | Description                                   |
| ----------------- | ----------------------- | --------------------------------------------- |
| `NATS_URL`        | `nats://127.0.0.1:4222` | NATS server URL                               |
| `BATCH_SIZE`      | `50`                    | Number of messages per JetStream pull         |
| `WORKER_API_ADDR` | `0.0.0.0:9090`          | Bind address for health/metrics HTTP endpoint |
| `RUST_LOG`        | `info`                  | Log verbosity                                 |

### NATS Stream Settings

Default stream configuration used by the workers:

| Setting        | Value                                     |
| -------------- | ----------------------------------------- |
| Stream name    | Defined in `sentinel_common::nats_config` |
| Consumer       | Durable pull consumer with explicit ack   |
| Max deliveries | 5 (after which the message is discarded)  |

## Environment Variables Summary

| Variable                | Used By      | Description                                                          |
| ----------------------- | ------------ | -------------------------------------------------------------------- |
| `RUST_LOG`              | All binaries | Tracing filter (e.g., `info`, `debug`, `sentinel_agent=trace`)       |
| `NATS_URL`              | Workers      | NATS connection URL                                                  |
| `BATCH_SIZE`            | Workers      | Messages per pull batch                                              |
| `WORKER_API_ADDR`       | Workers      | Worker HTTP bind address                                             |
| `SENTINEL_MASTER_KEY`   | CLI / Agent  | AES-256-GCM master key for encrypted key store (first 32 bytes used) |
| `SENTINEL_AGENT_SECRET` | Agent        | Agent signing secret (alternative to config `secret` field)          |
| `GRPC_ADDR`             | Server       | Full gRPC listen address (e.g. `0.0.0.0:50051`)                      |
| `GRPC_PORT`             | Server       | gRPC port only (e.g. `50051`)                                        |
| `REST_ADDR`             | Server       | Full REST listen address (e.g. `0.0.0.0:8080`)                       |
| `REST_PORT`             | Server       | REST port only (e.g. `8080`)                                         |
| `JWT_SECRET`            | Server       | JWT signing secret                                                   |

## WASM Plugin Manifest

Each plugin requires a `.manifest.yml` alongside its `.wasm` file in the plugins directory.

```yaml
name: my-plugin
version: "1.0.0"
entry_fn: collect # exported WASM function name
capabilities: # optional
    - http_get
    - read_file
    - metric_builder
resource_limits:
    max_memory_mb: 64 # default
    timeout_ms: 5000 # default
    max_metrics: 1000 # default
metadata: # arbitrary key-value pairs
    author: "Example"
```

See [security.md](security.md) for details on plugin sandboxing and signing.

## Alert Rule Schema

Rules are managed via the REST API or CLI. JSON format:

```json
{
    "name": "High CPU",
    "agent_pattern": "*",
    "metric_name": "cpu_usage",
    "condition": "GreaterThan",
    "threshold": 90.0,
    "for_duration_ms": 60000,
    "severity": "Critical",
    "annotations": {
        "description": "CPU usage exceeds 90% for 1 minute"
    }
}
```

| Field             | Required | Default   | Description                                                         |
| ----------------- | -------- | --------- | ------------------------------------------------------------------- |
| `name`            | Yes      | —         | Human-readable rule name                                            |
| `metric_name`     | Yes      | —         | Metric to evaluate                                                  |
| `condition`       | Yes      | —         | `GreaterThan`, `LessThan`, `GreaterOrEqual`, `LessOrEqual`, `Equal` |
| `threshold`       | Yes      | —         | Numeric threshold                                                   |
| `agent_pattern`   | No       | `*`       | Agent ID pattern (`*` = all, `prefix*` = prefix match)              |
| `for_duration_ms` | No       | `0`       | Hold duration before firing (0 = immediate)                         |
| `severity`        | No       | `Warning` | `Info`, `Warning`, `Critical`                                       |
| `annotations`     | No       | `{}`      | Arbitrary key-value metadata                                        |
