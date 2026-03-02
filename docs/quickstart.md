# Quick Start

Get SentinelRS running in 5 minutes with Docker.

## Prerequisites

- Docker and Docker Compose v2
- The `sentinel` CLI binary (or build from source with `cargo build -p sentinel_cli --release`)

## 1. Start the stack

```bash
docker compose -f deploy/docker-compose.yml up -d
```

This starts TimescaleDB, NATS JetStream, the SentinelRS server and a worker. Migrations run automatically on first boot.

| Service         | Port  | Description              |
| --------------- | ----- | ------------------------ |
| sentinel-server | 50051 | gRPC (agent streaming)   |
| sentinel-server | 8080  | REST API                 |
| sentinel-worker | 9091  | Worker health endpoint   |
| timescaledb     | 5432  | PostgreSQL / TimescaleDB |
| nats            | 4222  | NATS client              |
| nats            | 8222  | NATS monitoring          |

## 2. Configure the CLI

```bash
sentinel init
```

The interactive wizard stores the server URL in `~/.config/sentinel/config.json`.

## 3. Generate an install command

```bash
sentinel agents generate-install --server https://localhost:8080
```

This creates a bootstrap token and prints a one-liner you can paste on any target host to provision an agent automatically.

## 4. Deploy an agent

**Option A — Docker (compose profile)**

```bash
docker compose -f deploy/docker-compose.yml --profile agent up -d
```

The containerized agent auto-bootstraps using `BOOTSTRAP_TOKEN` and `SERVER_URL` environment variables.

**Option B — Native binary**

Copy the one-liner from step 3 onto the target host. It downloads the agent, sets `BOOTSTRAP_TOKEN`, and starts collection.

```bash
BOOTSTRAP_TOKEN=<token> SERVER_URL=https://server:50051 ./sentinel_agent --config /etc/sentinel/config.yml
```

If no config file exists, the agent enters zero-touch provisioning mode: it exchanges the token for credentials and writes its own config.

## 5. Verify

```bash
sentinel agents list
```

You should see your agent with its ID, version and last-seen timestamp.

Watch metrics in real-time:

```bash
sentinel agents live <AGENT_ID>
```

Or check cluster status:

```bash
sentinel cluster status
```

## Next steps

- [Docker guide](docker.md) — full Docker deployment reference
- [Provisioning](provisioning.md) — bootstrap tokens and zero-touch provisioning
- [Streaming](streaming.md) — gRPC V2 bidirectional protocol
- [CLI reference](cli.md) — all commands and options
- [Configuration](configuration.md) — agent, server and worker settings
- [Architecture](architecture.md) — system design and data flow
