# Troubleshooting

Common problems and solutions when running SentinelRS.

---

## Server

### Server won't start

**Port already in use**

```
Error: Address already in use (os error 98)
```

Another process is using port 50051 (gRPC) or 8080 (REST).

```bash
# Find the process
lsof -i :50051
lsof -i :8080

# Kill it or change ports
GRPC_ADDR=0.0.0.0:50052 REST_ADDR=0.0.0.0:8081 sentinel-server
```

**Database connection failed**

```
Error: connection refused (os error 111)
```

Check that TimescaleDB is running and `DATABASE_URL` is correct:

```bash
# Test connectivity
psql "$DATABASE_URL" -c "SELECT 1"

# Docker
docker compose ps timescaledb
docker compose logs timescaledb
```

**NATS connection failed**

```
Error: NATS connect error
```

Verify NATS is running and the URL is correct:

```bash
# Test connectivity
nats server ping --server nats://localhost:4222

# Docker
docker compose ps nats
docker compose logs nats
```

---

## Agent

### Agent won't connect

**Connection refused**

```
WARN connect error: transport error
```

1. Verify the server is running and the gRPC port is reachable:
    ```bash
    curl -v telnet://server-host:50051
    ```
2. Check agent config `server_addr`:
    ```yaml
    server_addr: "http://server-host:50051"
    ```
3. If using TLS, ensure the certificate is valid and the CA is trusted.

**Authentication failed**

```
ERROR HMAC verification failed
```

The agent secret doesn't match. Re-provision:

```bash
# On server: generate install command with new token
sentinel agents generate-install --agent-id my-agent --server grpc://host:50051

# On agent: re-run the install command
```

**Key rotation mismatch**

```
WARN key not found, trying grace period
```

After key rotation, old keys are accepted during the grace period (`KEY_GRACE_PERIOD_MS`).
If the grace period has passed:

```bash
# Check current keys
sentinel key list --agent-id my-agent

# Rotate again
sentinel key rotate --agent-id my-agent
# Then restart the agent
```

### Agent shows as Stale

An agent is marked **Stale** when it misses heartbeats but hasn't been disconnected long enough
to be **Offline**.

1. Check agent process: `systemctl status sentinel-agent`
2. Check network: can the agent reach the server gRPC port?
3. Check logs: `journalctl -u sentinel-agent -f`
4. Verify with: `sentinel agents health <id>`

---

## Workers

### Workers not processing batches

**Consumer lag growing**

```bash
nats consumer info metrics sentinel-workers
```

If `Num Pending` keeps growing:

1. Check worker logs for errors
2. Verify database connectivity from workers
3. Scale workers: `docker compose up -d --scale workers=4`

**Database write errors**

```
ERROR batch insert failed
```

1. Check TimescaleDB disk space: `df -h`
2. Check connection count: `SELECT count(*) FROM pg_stat_activity;`
3. Check for locks: `SELECT * FROM pg_locks WHERE NOT granted;`

### Workers crash-looping

Check logs:

```bash
docker compose logs workers --tail 50
# or
journalctl -u sentinel-workers -f
```

Common causes:

- Invalid `DATABASE_URL`
- NATS stream doesn't exist (run `deploy/nats-setup.sh`)
- Schema not migrated (migrations run on server start)

---

## CLI

### CLI can't reach server

```
Error: Connection refused
```

1. Check URL: `sentinel config show`
2. Test connectivity: `curl http://localhost:8080/healthz`
3. Set correct URL: `sentinel config set server.url http://my-server:8080`

### Command hangs

If a command hangs, the server may be unresponsive:

```bash
# Quick health check with timeout
curl -m 5 http://localhost:8080/healthz
```

### JSON output is empty

Some commands return empty arrays when no data exists:

```bash
sentinel agents list --json
# [] means no agents registered yet
```

---

## Notifications

### Test notification not received

```bash
sentinel notifiers test --notifier my-notifier
```

If the test succeeds (200 response) but no notification arrives:

- **Webhook**: Check the endpoint logs
- **Slack**: Verify the webhook URL hasn't been revoked
- **Discord**: Check channel permissions
- **Email**: Check spam folder; verify SMTP credentials
- **Telegram**: Ensure bot is in the group and has send permissions
- **PagerDuty**: Check the routing key and service status

### Notifications in DLQ

```sql
SELECT id, notifier_config_id, error_message, created_at
FROM notifications_dlq
ORDER BY created_at DESC
LIMIT 10;
```

Common DLQ reasons:

- Endpoint unreachable (network issue)
- 401/403 (invalid credentials/token)
- Rate limited by external service
- Malformed config

Fix the notifier config and failed deliveries will be retried.

---

## Metrics

### No metrics appearing

Checklist:

1. Agent running? `sentinel agents list`
2. Agent connected? `sentinel agents health <id>`
3. Batches flowing? `nats consumer info metrics sentinel-workers`
4. Worker processing? Check worker logs
5. Data in DB? `SELECT count(*) FROM metrics_raw WHERE time > NOW() - INTERVAL '1 hour';`

### Metrics delayed

Expected latency: agent interval (10s) + gRPC transit + NATS + worker processing.

If delay exceeds 30s:

1. Check NATS consumer lag
2. Check worker count vs. agent count
3. Check TimescaleDB write latency

### Continuous aggregates stale

Force refresh:

```sql
CALL refresh_continuous_aggregate('metrics_5m', NOW() - INTERVAL '1 hour', NOW());
CALL refresh_continuous_aggregate('metrics_hourly', NOW() - INTERVAL '1 day', NOW());
```

---

## TLS / mTLS

### Certificate errors

```
ERROR TLS handshake failed
```

1. Verify certificate validity: `openssl x509 -in cert.pem -noout -dates`
2. Verify CA chain: `openssl verify -CAfile ca.pem cert.pem`
3. Check hostname matches certificate SAN
4. Ensure both client and server use the same CA for mTLS

### Generate dev certificates

```bash
./scripts/gen-dev-certs.sh
```

Creates `certs/` with `ca.pem`, `server.pem`, `server-key.pem`, `client.pem`, `client-key.pem`.

---

## Docker

### Container keeps restarting

```bash
docker compose logs <service> --tail 50
```

Common causes:

- Missing environment variables (check `deploy/docker-compose.yml`)
- Database not ready yet (server depends on TimescaleDB being up)
- NATS stream not initialized (run setup script)

### Reset everything

```bash
docker compose -f deploy/docker-compose.yml down -v
docker compose -f deploy/docker-compose.yml up -d
```

`-v` removes volumes (deletes all data). Use only for full resets.

---

## Logs

### Enable debug logging

```bash
# Server
RUST_LOG=sentinel_server=debug sentinel-server

# Agent
RUST_LOG=sentinel_agent=debug sentinel-agent

# Workers
RUST_LOG=sentinel_workers=debug sentinel-workers

# Everything
RUST_LOG=debug sentinel-server
```

### Trace-level logging

For deep debugging (very verbose):

```bash
RUST_LOG=sentinel_server=trace,sentinel_common=trace sentinel-server
```

---

## Getting Help

1. Check this guide
2. Run `sentinel doctor` for automated diagnostics
3. Enable debug logging and inspect output
4. Open an issue: https://github.com/RenanYhuel/SentinelRS/issues
