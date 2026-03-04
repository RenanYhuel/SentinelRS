# Quickstart

Get SentinelRS running in under 5 minutes with Docker.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) and Docker Compose v2+
- (Optional) The `sentinel` CLI — see [Installation](installation.md)

## 1. Clone the repository

```bash
git clone https://github.com/RenanYhuel/SentinelRS.git
cd SentinelRS
```

## 2. Start the stack

```bash
docker compose -f deploy/docker-compose.yml up -d
```

This starts 4 services:

| Service         | Port  | Role                               |
| --------------- | ----- | ---------------------------------- |
| TimescaleDB     | 5432  | Time-series database               |
| NATS            | 4222  | Message bus (JetStream)            |
| sentinel-server | 50051 | gRPC ingestion + REST API on 8080  |
| sentinel-worker | —     | Background processing (alerts, DB) |

Check everything is healthy:

```bash
docker compose -f deploy/docker-compose.yml ps
```

Or with the CLI:

```bash
sentinel health
```

Expected output:

```
  ╭──────────────╮
  │   Health      │
  ╰──────────────╯

    Status           ● Healthy
    Server           http://localhost:8080
```

## 3. Configure the CLI

```bash
sentinel init
```

The interactive wizard creates `~/.config/sentinel/config.toml` with your server address. Accept defaults for a local Docker setup.

Verify:

```bash
sentinel config show
```

## 4. Add your first agent

```bash
sentinel agents add my-server
```

This provisions the agent in the database and returns a bootstrap token. To deploy the agent on a remote host:

```bash
sentinel agents generate-install --agent-id my-server --server grpc://your-server:50051
```

Copy the one-liner to the target machine and run it.

For local testing, start an agent container:

```bash
docker compose -f deploy/docker-compose.yml --profile agent up -d sentinel-agent
```

## 5. Verify metrics are flowing

Wait ~10 seconds for the first collection cycle, then:

```bash
sentinel metrics agent my-server
```

```
  ╭───────────────────────────╮
  │   Agent Metrics           │
  ╰───────────────────────────╯

    cpu.usage_percent     23.5
    memory.used_bytes     8589934592
    disk.usage_percent    45.2
```

## 6. Create an alert rule

```bash
sentinel rules create \
  --name "High CPU" \
  --metric cpu.usage_percent \
  --condition gt \
  --threshold 90 \
  --severity critical \
  --agent-pattern "*"
```

## 7. Set up notifications

```bash
sentinel notifiers create \
  --name alerts-discord \
  --type discord \
  --config '{"webhook_url": "https://discord.com/api/webhooks/..."}'
```

Link it to the rule:

```bash
sentinel notifiers link --rule "High CPU" --notifier alerts-discord
```

Test it:

```bash
sentinel notifiers test --notifier alerts-discord
```

## Next steps

| Topic                                         | Guide                             |
| --------------------------------------------- | --------------------------------- |
| Install the CLI binary                        | [Installation](installation.md)   |
| All CLI commands                              | [CLI Reference](cli-reference.md) |
| REST API endpoints                            | [API Reference](api-reference.md) |
| Configure agents, server, workers             | [Configuration](configuration.md) |
| Production deployment and TLS                 | [Deployment](deployment.md)       |
| Set up Discord, Slack, Email, PagerDuty, etc. | [Notifications](notifications.md) |
| Scale workers and partition load              | [Scaling](scaling.md)             |
| Understand the architecture                   | [Architecture](architecture.md)   |
