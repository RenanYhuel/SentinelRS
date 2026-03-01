# Deployment

## Quick Start (Docker Compose)

The provided `deploy/docker-compose.yml` starts the required infrastructure:

```bash
docker compose -f deploy/docker-compose.yml up -d
```

This launches:

| Service     | Image                               | Ports                            |
| ----------- | ----------------------------------- | -------------------------------- |
| NATS        | `nats:2.9.4`                        | 4222 (client), 8222 (monitoring) |
| TimescaleDB | `timescale/timescaledb:latest-pg14` | 5432                             |

Default database credentials: `postgres` / `postgres`, database name: `sentinel`.

## Database Setup

### Apply Migrations

Migrations are in `migrations/` and must be applied in order:

```bash
psql -h localhost -U postgres -d sentinel -f migrations/000_migration_tracking.sql
psql -h localhost -U postgres -d sentinel -f migrations/001_create_extensions.sql
psql -h localhost -U postgres -d sentinel -f migrations/002_create_metrics_time.sql
psql -h localhost -U postgres -d sentinel -f migrations/003_create_metrics_raw.sql
psql -h localhost -U postgres -d sentinel -f migrations/004_create_alerts.sql
psql -h localhost -U postgres -d sentinel -f migrations/005_retention_policies.sql
psql -h localhost -U postgres -d sentinel -f migrations/006_continuous_aggregates.sql
psql -h localhost -U postgres -d sentinel -f migrations/007_dashboard_views.sql
psql -h localhost -U postgres -d sentinel -f migrations/008_create_alert_rules.sql
psql -h localhost -U postgres -d sentinel -f migrations/009_create_notifications_dlq.sql
```

Or apply all at once:

```bash
for f in migrations/*.sql; do psql -h localhost -U postgres -d sentinel -f "$f"; done
```

### Schema Overview

| Table               | Description                                                                  |
| ------------------- | ---------------------------------------------------------------------------- |
| `metrics_time`      | Hypertable — structured metrics with labels (1-day chunks, 90-day retention) |
| `metrics_raw`       | Hypertable — raw batch JSONB payloads (1-day chunks, 7-day retention)        |
| `alerts`            | Alert events (firing and resolved)                                           |
| `alert_rules`       | Alert rule definitions                                                       |
| `notifications_dlq` | Dead-letter queue for failed notifications                                   |
| `mv_metrics_1h`     | Continuous aggregate — 1-hour rollups                                        |

## Running the Binaries

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

- **gRPC** on `0.0.0.0:50051` (default) — agent registration, metric push, heartbeats
- **REST** on `0.0.0.0:8080` (default) — admin API, health checks, Prometheus metrics

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

- Loads and validates the YAML config
- Starts periodic system metric collection (CPU, memory, disk)
- Writes batches to the local WAL
- Exports batches to the server via gRPC (with automatic retry and reconnection)
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
