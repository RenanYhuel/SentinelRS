# Deployment

## Docker Deployment (Recommended)

The full SentinelRS stack runs with a single command:

```bash
docker compose -f deploy/docker-compose.yml up -d
```

This starts:

| Service           | Image                               | Ports                            |
| ----------------- | ----------------------------------- | -------------------------------- |
| `timescaledb`     | `timescale/timescaledb:latest-pg14` | 5432                             |
| `nats`            | `nats:2.10` (JetStream)             | 4222 (client), 8222 (monitoring) |
| `sentinel-server` | `sentinelrs/server`                 | 50051 (gRPC), 8080 (REST)        |
| `sentinel-worker` | `sentinelrs/worker`                 | 9091                             |
| `sentinel-agent`  | `sentinelrs/agent` (profile: agent) | 9090                             |

Default credentials: `sentinel` / `sentinel_secret`, database: `sentinel`.

Database migrations run automatically on server startup.

> Full Docker guide: [docker.md](docker.md)

### Scaling workers

```bash
docker compose -f deploy/docker-compose.yml up -d --scale sentinel-worker=3
```

Workers join the same NATS consumer group for automatic load balancing.

### Deploying agents

**Via compose profile:**

```bash
docker compose -f deploy/docker-compose.yml --profile agent up -d
```

**Standalone container with bootstrap:**

```bash
docker run -d \
  -e SERVER_URL=https://server:50051 \
  -e BOOTSTRAP_TOKEN=<token> \
  -v sentinel-config:/etc/sentinel \
  sentinelrs/agent:latest
```

> See [provisioning.md](provisioning.md) for the zero-touch provisioning flow.

## Database

Migrations run automatically when the server starts. The SQL files are in `migrations/` for reference or manual use.

| Table               | Description                                                                  |
| ------------------- | ---------------------------------------------------------------------------- |
| `metrics_time`      | Hypertable — structured metrics with labels (1-day chunks, 90-day retention) |
| `metrics_raw`       | Hypertable — raw batch JSONB payloads (1-day chunks, 7-day retention)        |
| `alerts`            | Alert events (firing and resolved)                                           |
| `alert_rules`       | Alert rule definitions                                                       |
| `notifications_dlq` | Dead-letter queue for failed notifications                                   |
| `mv_metrics_1h`     | Continuous aggregate — 1-hour rollups                                        |

## Running the Binaries (native)

### Server

```bash
# Default ports (gRPC :50051, REST :8080)
RUST_LOG=info ./sentinel_server

# Custom ports via CLI flags
./sentinel_server --grpc-port 9051 --rest-port 3000

# Custom ports via environment variables
GRPC_PORT=9051 REST_PORT=3000 ./sentinel_server

# Full address override
./sentinel_server --grpc-addr 127.0.0.1:9051 --rest-addr 127.0.0.1:3000
```

The server starts two listeners concurrently:

- **gRPC** on `0.0.0.0:50051` (default) — V2 bidirectional streaming + V1 unary RPCs
- **REST** on `0.0.0.0:8080` (default) — admin API, health checks, SSE events, Prometheus metrics

Both ports are configurable via CLI flags (`--grpc-port`, `--rest-port`) or environment variables (`GRPC_PORT`, `REST_PORT`, `GRPC_ADDR`, `REST_ADDR`). CLI flags take precedence over env vars.

### Workers

```bash
NATS_URL=nats://127.0.0.1:4222 \
BATCH_SIZE=50 \
WORKER_API_ADDR=0.0.0.0:9090 \
RUST_LOG=info \
./sentinel_workers
```

| Variable          | Default                 | Description                  |
| ----------------- | ----------------------- | ---------------------------- |
| `NATS_URL`        | `nats://127.0.0.1:4222` | NATS server URL              |
| `BATCH_SIZE`      | `50`                    | Messages per pull batch      |
| `WORKER_API_ADDR` | `0.0.0.0:9090`          | Health/metrics HTTP endpoint |

### Agent

```bash
# Start with a config file (required)
./sentinel_agent --config /etc/sentinel/agent.yml

# With explicit secret via environment
SENTINEL_MASTER_KEY="your-32-byte-key" ./sentinel_agent --config agent.yml

# Version and help
./sentinel_agent --version
./sentinel_agent --help
```

The agent:

- If no config file exists and `BOOTSTRAP_TOKEN` is set, enters zero-touch provisioning mode
- Loads and validates the YAML config
- Opens a persistent gRPC stream to the server (V2) with signed handshake
- Starts periodic system metric collection (CPU, memory, disk)
- Writes batches to the local WAL, sends via stream with per-batch acknowledgement
- Sends heartbeat pings with live system stats for presence tracking
- Reconnects automatically with exponential backoff on disconnection
- Exposes `/healthz`, `/ready` and `/metrics` on the configured `api_port` (default: 9090)
- Shuts down gracefully on SIGTERM / SIGINT

See [configuration.md](configuration.md) for all agent settings.

## TLS Setup

### Development Certificates

Generate self-signed certificates for local development:

```bash
./scripts/gen-dev-certs.sh [output_dir]
```

Default output: `certs/`. Generates:

- `ca-key.pem` / `ca-cert.pem` — Certificate Authority (RSA-4096)
- `server-key.pem` / `server-cert.pem` — Server certificate signed by CA
- SAN entries: `localhost`, `127.0.0.1`, `::1`
- Validity: 365 days

### Production TLS

For production, provide certificates from a trusted CA or your internal PKI.

The server accepts TLS configuration through its config:

```
tls:
  cert_path: /etc/sentinel/tls/server-cert.pem
  key_path: /etc/sentinel/tls/server-key.pem
  ca_path: /etc/sentinel/tls/ca-cert.pem    # enables mTLS
```

When `ca_path` is set, the server requires client certificates (mutual TLS).

## Production Checklist

### Security

- [ ] Replace the default `jwt_secret` — the hardcoded default is **not safe for production**
- [ ] Set `SENTINEL_MASTER_KEY` for agent key encryption (32+ bytes)
- [ ] Enable TLS on the gRPC listener
- [ ] Consider enabling mTLS for agent authentication
- [ ] Restrict network access to NATS and TimescaleDB ports

### Reliability

- [ ] Configure WAL retention (`max_retention_days`) appropriate for your network reliability
- [ ] Set up TimescaleDB retention policies (defaults: 7 days raw, 90 days structured)
- [ ] Monitor NATS JetStream consumer lag
- [ ] Set `RUST_LOG=warn` or `RUST_LOG=info` in production (avoid `debug`/`trace`)

### Monitoring

- [ ] Scrape `/metrics` from the server (`:8080/metrics` or configured REST port) for Prometheus
- [ ] Scrape agent health endpoint (`:9090/metrics` or configured `api_port`)
- [ ] Scrape worker health endpoint (`:9090`)
- [ ] Monitor the `notifications_dlq` table for failed alert deliveries
- [ ] Set up alerts on SentinelRS's own health endpoints (`/healthz`, `/ready`)

### Packaging

Pre-built packages are available:

- **Debian** — `packaging/deb/build-deb.sh` generates a `.deb` with systemd services
- **macOS** — `packaging/macos/bundle.sh`
- **Windows** — `packaging/wix/build-msi.bat` generates an MSI installer

Systemd service files for all three binaries are in `packaging/deb/`.

## NATS Configuration

The workers expect a JetStream stream. On first run, the worker binary creates the stream automatically with default settings. For custom configuration:

```bash
# Example manual stream creation
nats stream add sentinel-metrics \
  --subjects "sentinel.metrics.>" \
  --storage file \
  --retention limits \
  --max-bytes 1073741824 \
  --max-age 72h
```

The NATS setup script at `deploy/nats-setup.sh` can also be used for initial configuration.
