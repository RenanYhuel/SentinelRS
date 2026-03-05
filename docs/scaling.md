# Scaling

SentinelRS is designed for horizontal scaling at the worker layer and vertical scaling
at the agent/server layer. NATS JetStream decouples ingestion from processing.

## Architecture Recap

```
Agents ──gRPC──→ Server ──NATS JetStream──→ Workers ──→ TimescaleDB
                    │                          ↑
                    │          (consumer group) │
                    └──────────────────────────┘
```

The server writes batches to NATS. Workers consume from NATS and write to the database.
Multiple workers share a consumer group — NATS distributes messages across them automatically.

---

## Worker Scaling

### Docker Compose

Scale workers with a single flag:

```bash
docker compose -f deploy/docker-compose.yml up -d --scale workers=4
```

Each worker joins the same NATS consumer group. NATS distributes batches round-robin
across available workers.

### Binary

Launch multiple worker processes on different ports:

```bash
WORKER_API_ADDR=0.0.0.0:9091 sentinel-workers &
WORKER_API_ADDR=0.0.0.0:9092 sentinel-workers &
WORKER_API_ADDR=0.0.0.0:9093 sentinel-workers &
```

All workers connect to the same NATS and TimescaleDB instances.

### Environment

| Variable          | Default            | Description               |
| ----------------- | ------------------ | ------------------------- |
| `NATS_URL`        | `nats://nats:4222` | NATS connection           |
| `DATABASE_URL`    | —                  | TimescaleDB connection    |
| `BATCH_SIZE`      | `100`              | Rows per DB insert batch  |
| `WORKER_API_ADDR` | `0.0.0.0:9090`     | Worker health API address |

### Connection Pool

Both the server and workers use configurable connection pools:

| Variable                  | Default | Description                      |
| ------------------------- | ------- | -------------------------------- |
| `MAX_DB_CONNECTIONS`      | `10`    | Maximum pool connections         |
| `MIN_DB_CONNECTIONS`      | `1`     | Minimum idle connections         |
| `DB_IDLE_TIMEOUT_SECS`    | `300`   | Close idle connections after (s) |
| `DB_ACQUIRE_TIMEOUT_SECS` | `5`     | Timeout waiting for a connection |
| `DB_MAX_LIFETIME_SECS`    | `1800`  | Max connection lifetime (s)      |

**Pool sizing formula:**

```
total_connections = (num_workers × MAX_DB_CONNECTIONS) + (num_servers × MAX_DB_CONNECTIONS) + overhead
```

Keep total below PostgreSQL's `max_connections` with ~10% headroom.

### Database Boot Resilience

TimescaleDB can take 30–90s on first Docker boot. The server uses a retry loop:

| Variable                    | Default | Description                     |
| --------------------------- | ------- | ------------------------------- |
| `DB_WAIT_TIMEOUT_SECS`      | `120`   | Total deadline for DB readiness |
| `DB_WAIT_RETRY_INTERVAL_MS` | `1000`  | Delay between retries           |
| `DB_WAIT_MAX_RETRIES`       | `60`    | Maximum retry attempts          |

### Worker Identity & Peer Discovery

Each worker generates a unique ID at startup (`<hostname>-<uuid8>`). When `REGISTRY_ENABLED=true`, workers register in a NATS KV bucket and emit heartbeats. The `/status` endpoint lists known peers.

### Scaling Guidelines

| Agents   | Recommended Workers | DB Connections (total) |
| -------- | ------------------- | ---------------------- |
| 1–50     | 1–2                 | 10–20                  |
| 50–500   | 2–4                 | 20–40                  |
| 500–5000 | 4–8                 | 40–80                  |
| 5000+    | 8+                  | 80+                    |

---

## NATS JetStream Tuning

### Stream Configuration

The `metrics` stream is created by `deploy/nats-setup.sh`:

```bash
nats stream add metrics \
  --subjects "metrics.>" \
  --storage file \
  --retention limits \
  --max-bytes 1073741824 \
  --max-age 72h \
  --replicas 1 \
  --discard old
```

Key parameters:

| Parameter     | Recommendation              | Description                 |
| ------------- | --------------------------- | --------------------------- |
| `--max-bytes` | 1 GB minimum                | Maximum stream size on disk |
| `--max-age`   | 72h (3 days)                | Retention window            |
| `--replicas`  | 1 (standalone), 3 (cluster) | Replication factor          |
| `--storage`   | `file`                      | Persistent storage          |

### Consumer Configuration

Workers use a durable pull consumer:

```bash
nats consumer add metrics sentinel-workers \
  --durable sentinel-workers \
  --deliver all \
  --ack explicit \
  --max-deliver 5 \
  --filter "" \
  --pull
```

Key settings:

| Setting         | Value    | Description                           |
| --------------- | -------- | ------------------------------------- |
| `--ack`         | explicit | Workers ACK after successful DB write |
| `--max-deliver` | 5        | Max redelivery attempts before DLQ    |
| `--pull`        | —        | Pull-based for back-pressure support  |

---

## TimescaleDB Tuning

### Chunk Interval

Default: 1 day. For high-throughput (>1M rows/day), reduce:

```sql
SELECT set_chunk_time_interval('metrics_time', INTERVAL '6 hours');
SELECT set_chunk_time_interval('metrics_raw', INTERVAL '6 hours');
```

### Retention Policies

Default retention (from migrations):

| Table          | Retention |
| -------------- | --------- |
| `metrics_raw`  | 7 days    |
| `metrics_time` | 90 days   |
| `alerts`       | 365 days  |

Adjust for your storage budget:

```sql
SELECT remove_retention_policy('metrics_raw');
SELECT add_retention_policy('metrics_raw', INTERVAL '30 days');
```

### Continuous Aggregates

Pre-computed 5-minute and 1-hour rollups reduce query time for dashboards:

| Aggregate        | Interval | Source        |
| ---------------- | -------- | ------------- |
| `metrics_5m`     | 5 min    | `metrics_raw` |
| `metrics_hourly` | 1 hour   | `metrics_raw` |

Refresh policies run automatically. Force a manual refresh:

```sql
CALL refresh_continuous_aggregate('metrics_5m', NOW() - INTERVAL '1 hour', NOW());
```

### Connection Pooling

For >5 workers, add PgBouncer in front of TimescaleDB:

```yaml
# docker-compose.override.yml
services:
    pgbouncer:
        image: edoburu/pgbouncer
        environment:
            DATABASE_URL: postgres://sentinel:sentinel@timescaledb:5432/sentinel
            POOL_MODE: transaction
            MAX_CLIENT_CONN: 200
            DEFAULT_POOL_SIZE: 20
        ports:
            - "6432:6432"
```

Point workers to `postgres://sentinel:sentinel@pgbouncer:6432/sentinel`.

---

## Server Scaling

The server is a single process handling gRPC + REST. Scale vertically:

- **CPU**: gRPC stream handling, HMAC verification, batch signing
- **Memory**: Connected agent sessions, DashMap state
- **Connections**: Each agent holds one persistent gRPC stream

For very large fleets (>500 agents), consider running multiple server instances
behind a load balancer with sticky sessions (required for bidirectional gRPC streams).

### Rate Limiting

The server has built-in REST rate limiting:

```bash
RATE_LIMIT_RPS=200 sentinel-server
```

---

## Agent Scaling

Each agent is a lightweight process. Scaling considerations:

| Setting            | Description                              |
| ------------------ | ---------------------------------------- |
| `interval_seconds` | Collection interval (default: 10s)       |
| `plugins`          | Number of WASM plugins                   |
| WAL                | Size depends on buffer during disconnect |

For high-frequency collection (1s intervals), ensure the gRPC link can handle throughput.

---

## Monitoring the Stack

### Worker Health

Each worker exposes a health API:

```bash
curl http://worker-host:9090/healthz
```

### NATS Monitoring

```bash
# Stream info
nats stream info metrics

# Consumer info
nats consumer info metrics sentinel-workers

# Pending messages (lag)
nats consumer info metrics sentinel-workers | grep "Unprocessed"
```

### Database Size

```sql
SELECT hypertable_name, pg_size_pretty(hypertable_size(format('%I', hypertable_name)::regclass))
FROM timescaledb_information.hypertables;
```

---

## Capacity Planning

| Fleet Size  | Workers | NATS Storage | DB Storage (30d) |
| ----------- | ------- | ------------ | ---------------- |
| 10 agents   | 1       | 512 MB       | ~5 GB            |
| 50 agents   | 2       | 1 GB         | ~25 GB           |
| 200 agents  | 4       | 2 GB         | ~100 GB          |
| 500+ agents | 8+      | 4 GB+        | ~250 GB+         |

Estimates assume 10s collection interval with ~20 metrics per agent.
