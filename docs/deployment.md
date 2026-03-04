# Deployment

## Docker (Recommended)

### Full Stack

```bash
git clone https://github.com/RenanYhuel/SentinelRS.git
cd SentinelRS
docker compose -f deploy/docker-compose.yml up -d
```

Services started:

| Service         | Image                               | Ports       | Health Check    |
| --------------- | ----------------------------------- | ----------- | --------------- |
| timescaledb     | `timescale/timescaledb:latest-pg14` | 5432        | `pg_isready`    |
| nats            | `nats:2.10-alpine`                  | 4222, 8222  | HTTP `/healthz` |
| sentinel-server | Built from Dockerfile.server        | 50051, 8080 | `GET /healthz`  |
| sentinel-worker | Built from Dockerfile.worker        | —           | —               |

### Agent Deployment

Start an agent container (opt-in profile):

```bash
docker compose -f deploy/docker-compose.yml --profile agent up -d sentinel-agent
```

Or deploy as standalone on any host:

```bash
docker run -d \
  --name sentinel-agent \
  -e SERVER_URL=grpc://your-server:50051 \
  -e COLLECT_INTERVAL=10 \
  -e BOOTSTRAP_TOKEN=your-token \
  sentinel-agent:latest
```

### Scale Workers

```bash
docker compose -f deploy/docker-compose.yml up -d --scale sentinel-worker=3
```

Workers use NATS consumer groups for automatic load balancing. No config changes needed.

### Custom Environment

Override environment variables in a `.env` file or inline:

```bash
DATABASE_URL=postgres://user:pass@db-host:5432/sentinel \
NATS_URL=nats://nats-host:4222 \
docker compose -f deploy/docker-compose.yml up -d
```

### Volumes

| Volume                  | Path in container          | Purpose                    |
| ----------------------- | -------------------------- | -------------------------- |
| `sentinel-db-data`      | `/var/lib/postgresql/data` | TimescaleDB persistence    |
| `sentinel-nats-data`    | `/data`                    | NATS JetStream persistence |
| `sentinel-agent-config` | `/etc/sentinel`            | Agent config (provisioned) |

## Binary Deployment

Build release binaries:

```bash
cargo build --release
```

### Server

```bash
./target/release/sentinel_server \
  --grpc-addr 0.0.0.0:50051 \
  --rest-addr 0.0.0.0:8080 \
  --jwt-secret "$JWT_SECRET" \
  --nats-url nats://localhost:4222 \
  --database-url "postgres://sentinel:sentinel@localhost:5432/sentinel"
```

Database migrations run automatically on first startup.

### Workers

```bash
NATS_URL=nats://localhost:4222 \
DATABASE_URL="postgres://sentinel:sentinel@localhost:5432/sentinel" \
./target/release/sentinel_workers
```

### Agent

```bash
./target/release/sentinel_agent --config /etc/sentinel/agent.yml
```

Or via environment:

```bash
SERVER_URL=grpc://server:50051 \
SENTINEL_AGENT_SECRET=your-secret \
./target/release/sentinel_agent
```

## Systemd Services

Example unit files are in `packaging/deb/`:

```ini
# /etc/systemd/system/sentinel-server.service
[Unit]
Description=SentinelRS Server
After=network.target postgresql.service nats.service

[Service]
ExecStart=/usr/bin/sentinel_server
EnvironmentFile=/etc/sentinel/server.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now sentinel-server
sudo systemctl enable --now sentinel-workers
sudo systemctl enable --now sentinel-agent
```

## TLS

### Development Certificates

```bash
./scripts/gen-dev-certs.sh
```

Generates self-signed certs in `./certs/`.

### Production TLS

```bash
sentinel_server \
  --tls-cert /etc/sentinel/tls/server.crt \
  --tls-key  /etc/sentinel/tls/server.key
```

### Mutual TLS (mTLS)

Require client certificates:

```bash
sentinel_server \
  --tls-cert /etc/sentinel/tls/server.crt \
  --tls-key  /etc/sentinel/tls/server.key \
  --tls-ca   /etc/sentinel/tls/ca.crt
```

Agents must then present a valid certificate signed by the same CA.

## Packaging

### Debian (.deb)

```bash
./packaging/deb/build-deb.sh
sudo dpkg -i target/debian/sentinel_*.deb
```

Installs binaries and systemd unit files.

### macOS (.app)

```bash
./packaging/macos/bundle.sh
```

### Windows (.msi)

```cmd
packaging\wix\build-msi.bat
```

## Production Checklist

### Security

- [ ] Set unique `JWT_SECRET` (not the default)
- [ ] Set unique `SENTINEL_AGENT_SECRET` per agent or global `SENTINEL_MASTER_KEY`
- [ ] Enable TLS on gRPC (or place behind a TLS-terminating proxy)
- [ ] Enable TLS on REST API
- [ ] Restrict database access to server and worker hosts
- [ ] Set `REPLAY_WINDOW_MS` to limit replay attacks (default: 5 min)

### Reliability

- [ ] Configure TimescaleDB with adequate disk space for retention
- [ ] Set up daily PostgreSQL backups
- [ ] Run at least 2 workers for redundancy
- [ ] Monitor NATS JetStream consumer lag
- [ ] Set up alerts on SentinelRS itself (`sentinel health`)

### Monitoring

- [ ] Scrape `/metrics` endpoint with Prometheus
- [ ] Monitor `sentinel agents status` for fleet overview
- [ ] Set up a notification channel for critical alerts
- [ ] Check `/healthz` and `/ready` endpoints from your load balancer

### Network

- [ ] Open port 50051 (gRPC) between agents and server
- [ ] Open port 8080 (REST) for CLI and dashboard access
- [ ] Keep port 4222 (NATS) and 5432 (PostgreSQL) internal only
