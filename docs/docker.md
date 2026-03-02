# Docker Deployment

## Images

SentinelRS provides four Docker images, each built with a multi-stage Dockerfile using `rust:1.85-bookworm` for compilation and `debian:bookworm-slim` for runtime.

| Image               | Dockerfile                 | Default ports | Description                        |
| ------------------- | -------------------------- | ------------- | ---------------------------------- |
| `sentinelrs/server` | `docker/Dockerfile.server` | 50051, 8080   | gRPC ingestion + REST API          |
| `sentinelrs/worker` | `docker/Dockerfile.worker` | 9091          | NATS consumer, DB writer, alerting |
| `sentinelrs/agent`  | `docker/Dockerfile.agent`  | 9090          | Metrics collector                  |
| `sentinelrs/cli`    | `docker/Dockerfile.cli`    | —             | Administration CLI                 |

All images run as a non-root `sentinel` user with health checks configured.

## Building images

```bash
docker compose -f deploy/docker-compose.yml build
```

Or individually:

```bash
docker build -f docker/Dockerfile.server -t sentinelrs/server .
docker build -f docker/Dockerfile.worker -t sentinelrs/worker .
docker build -f docker/Dockerfile.agent  -t sentinelrs/agent  .
docker build -f docker/Dockerfile.cli    -t sentinelrs/cli    .
```

## Compose stack

The reference `deploy/docker-compose.yml` brings up the full stack:

```bash
docker compose -f deploy/docker-compose.yml up -d
```

### Services

| Service           | Image                                 | Role                               |
| ----------------- | ------------------------------------- | ---------------------------------- |
| `timescaledb`     | `timescale/timescaledb:latest-pg14`   | Metric storage                     |
| `nats`            | `nats:2.10` (JetStream enabled)       | Message broker                     |
| `sentinel-server` | `sentinelrs/server`                   | gRPC + REST gateway                |
| `sentinel-worker` | `sentinelrs/worker`                   | Processing pipeline                |
| `sentinel-agent`  | `sentinelrs/agent` (profile: `agent`) | Collector (opt-in via `--profile`) |

### Volumes

| Volume                  | Mounted in     | Purpose                        |
| ----------------------- | -------------- | ------------------------------ |
| `sentinel-db-data`      | timescaledb    | PostgreSQL data                |
| `sentinel-nats-data`    | nats           | JetStream storage              |
| `sentinel-agent-config` | sentinel-agent | Agent config (`/etc/sentinel`) |

### Network

All services share a `sentinel-net` bridge network. Internal DNS resolution uses service names (e.g. `nats://nats:4222`, `postgres://timescaledb:5432`).

## Environment variables

### Server

| Variable           | Default                                                         | Description            |
| ------------------ | --------------------------------------------------------------- | ---------------------- |
| `DATABASE_URL`     | `postgres://sentinel:sentinel_secret@timescaledb:5432/sentinel` | TimescaleDB connection |
| `NATS_URL`         | `nats://nats:4222`                                              | NATS server            |
| `SERVER_GRPC_PORT` | `50051`                                                         | gRPC listen port       |
| `SERVER_REST_PORT` | `8080`                                                          | REST listen port       |
| `RUST_LOG`         | `info`                                                          | Log level              |

### Worker

| Variable                | Default                                                         | Description            |
| ----------------------- | --------------------------------------------------------------- | ---------------------- |
| `DATABASE_URL`          | `postgres://sentinel:sentinel_secret@timescaledb:5432/sentinel` | TimescaleDB connection |
| `NATS_URL`              | `nats://nats:4222`                                              | NATS server            |
| `WORKER_CONSUMER_GROUP` | `sentinel-workers`                                              | NATS consumer group    |
| `RUST_LOG`              | `info`                                                          | Log level              |

### Agent

| Variable                 | Default                        | Description             |
| ------------------------ | ------------------------------ | ----------------------- |
| `SERVER_URL`             | `http://sentinel-server:50051` | gRPC server URL         |
| `AGENT_COLLECT_INTERVAL` | `10`                           | Collection interval (s) |
| `BOOTSTRAP_TOKEN`        | —                              | Zero-touch provisioning |
| `RUST_LOG`               | `info`                         | Log level               |

## Scaling workers

Increase the replica count to scale metric processing horizontally:

```bash
docker compose -f deploy/docker-compose.yml up -d --scale sentinel-worker=3
```

Workers use NATS consumer groups for automatic load balancing. Each replica joins the same `sentinel-workers` consumer group and receives a subset of batches.

## Running the agent via compose

The agent is behind the `agent` profile and not started by default:

```bash
docker compose -f deploy/docker-compose.yml --profile agent up -d
```

For remote hosts, run the agent image standalone:

```bash
docker run -d \
  --name sentinel-agent \
  -e SERVER_URL=https://your-server:50051 \
  -e BOOTSTRAP_TOKEN=<token> \
  -v sentinel-agent-config:/etc/sentinel \
  sentinelrs/agent:latest
```

The agent auto-bootstraps on first run if no config exists at `/etc/sentinel/config.yml`.

## Health checks

All images include Docker `HEALTHCHECK` directives:

| Service | Endpoint                        | Interval | Timeout | Retries |
| ------- | ------------------------------- | -------- | ------- | ------- |
| server  | `http://localhost:8080/healthz` | 10s      | 5s      | 5       |
| worker  | `http://localhost:9091/healthz` | 10s      | 5s      | 5       |
| agent   | `http://localhost:9090/healthz` | 15s      | 5s      | 3       |
| tsdb    | `pg_isready`                    | 5s       | 3s      | 10      |
| nats    | `http://localhost:8222/healthz` | 5s       | 3s      | 10      |

## Using the CLI via Docker

```bash
docker run --rm --network sentinel-net \
  sentinelrs/cli:latest agents list --server http://sentinel-server:8080
```

Or create a shell alias:

```bash
alias sentinel='docker run --rm --network sentinel-net sentinelrs/cli:latest'
sentinel cluster status --server http://sentinel-server:8080
```

## Production recommendations

- Pin image tags to a specific version (e.g. `sentinelrs/server:2.0.0`) instead of `latest`.
- Mount TLS certificates into the server container and set the appropriate environment variables.
- Use external managed PostgreSQL and NATS clusters instead of the compose-provided containers.
- Set `POSTGRES_PASSWORD` and other secrets via Docker secrets or a `.env` file excluded from version control.
- Monitor container health and resource usage with your existing infrastructure tooling.
