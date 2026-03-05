# CLI Reference

```
sentinel [--json] [--server <url>] [--config <path>] <command>
```

## Global Flags

| Flag              | Description                                   |
| ----------------- | --------------------------------------------- |
| `--json`          | Output as JSON instead of human-readable      |
| `--server <url>`  | Override server URL (e.g. `http://host:8080`) |
| `--config <path>` | Use alternate config file                     |

---

## Setup

### `sentinel init`

Interactive setup wizard. Creates `~/.config/sentinel/config.toml` with server URL and output preferences.

```bash
sentinel init
```

Prompts for:

- Server URL (default: `http://localhost:8080`)
- Output mode (human or json)

### `sentinel doctor`

Run diagnostic checks against the server.

```bash
sentinel doctor
```

Checks server connectivity, API version, database status, NATS status.

### `sentinel completions <shell>`

Generate shell completions.

```bash
sentinel completions bash
sentinel completions zsh
sentinel completions fish
sentinel completions powershell
sentinel completions elvish
```

Pipe to a file in your completions directory. See [Installation](installation.md#shell-completions).

### `sentinel version`

Print version and build information.

```bash
sentinel version
```

```
sentinel 0.1.0 (rustc 1.XX.X)
```

---

## Server Status

### `sentinel health`

Check server health.

```bash
sentinel health
```

```
  ╭──────────────╮
  │   Health      │
  ╰──────────────╯

    Status           ● Healthy
    Server           http://localhost:8080
```

### `sentinel status`

Combined dashboard showing server health, cluster stats, and recent alerts.

```bash
sentinel status
```

---

## Agents

### `sentinel agents list` (alias: `ls`)

List all registered agents with live session data.

```bash
sentinel agents list
sentinel agents ls
```

```
  ╭──────────────╮
  │   Agents      │
  ╰──────────────╯

  Status      Agent        Version  CPU     Memory  Latency
  ● Online    prod-db      0.1.0    23.4%   72.1%   12ms
  ● Online    staging-web  0.1.0    8.1%    45.3%   24ms
  ● Stale     dev-cache    0.1.0    —       —       —
  ● Offline   old-server   0.0.9    —       —       —

  8/10 online
```

Columns show `—` for offline/stale agents where live data is unavailable.

### `sentinel agents get [id]` (alias: `show`)

Detailed view for a single agent. Omit the ID for interactive selection.

```bash
# Direct
sentinel agents get prod-db
sentinel agents show prod-db

# Interactive (fuzzy search)
sentinel agents get
```

```
  ╭─────────────────────╮
  │   Agent Details      │
  ╰─────────────────────╯

    Status           ● Online
    Agent ID         prod-db
    HW ID            abc123def
    Version          0.1.0
    Last Seen        2 minutes ago

  ──────────────────────────────────────────

  ● Session
  ──────────────────────────────────────────

    Hostname         prod-db-01
    OS               Ubuntu 22.04
    Connected        2 days ago
    Quality          Excellent
    Latency          12 ms
    Uptime           10d 2h 15m
    Heartbeats       15234

    CPU     ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  23.4%
    Memory  ████████████████████████████░░░░░░░░░░░░░░  72.1%
    Disk    ██████████████████░░░░░░░░░░░░░░░░░░░░░░░░  45.0%
```

Interactive mode shows a fuzzy search with status icons:

```
? Select agent
  ● prod-db (prod-db-01)
  ● staging-web (staging-web-01)
  ○ old-server (abc123)
```

### `sentinel agents add <id>`

Provision a new agent.

```bash
sentinel agents add my-new-server
```

Returns the agent ID and bootstrap information.

### `sentinel agents delete <id>` (alias: `rm`)

Remove an agent from the registry.

```bash
sentinel agents delete old-server
sentinel agents rm old-server
```

### `sentinel agents live <id>`

Watch live session data in real-time. Refreshes every second.

```bash
sentinel agents live prod-db
```

Displays system stats, latency, connection quality. Press `Ctrl+C` to stop.

### `sentinel agents generate-install`

Generate a one-liner install command with bootstrap token.

```bash
sentinel agents generate-install --agent-id my-server --server grpc://prod:50051
```

```
Install command:

  curl -sSL https://... | sh -s -- --token abc123 --server grpc://prod:50051

Token expires in 24 hours. Single use only.
```

### `sentinel agents status` (alias: `st`)

Fleet overview with status distribution and resource averages.

```bash
sentinel agents status
sentinel agents st
```

```
  ╭─────────────────────╮
  │   Fleet Overview     │
  ╰─────────────────────╯

    Total            10
    Online           8
    Offline          1
    Stale            1
    Avg CPU          34.5%
    Avg Memory       61.2%
    Avg Latency      15 ms

  ──────────────────────────────────────────

  Status  Agent        CPU     Memory  Latency  Host
  ●       prod-db      23.4%   72.1%   12ms     prod-db-01
  ●       staging-web  8.1%    45.3%   24ms     staging-01
  ●       dev-cache    —       —       —        —
  ●       old-server   —       —       —        —
```

### `sentinel agents health [id]`

Detailed health diagnostics for a connected agent. Includes latency percentiles and full system info.

```bash
sentinel agents health prod-db
```

```
  ╭─────────────────────╮
  │   Agent Health       │
  ╰─────────────────────╯

    Agent            prod-db
    Status           ● Online
    Quality          Excellent
    Connected        2 days ago
    Heartbeats       15234

  ──────────────────────────────────────────

  ● Latency
  ──────────────────────────────────────────

    Avg              12.5 ms
    Min              8.0 ms
    Max              45.0 ms
    P50              11.0 ms
    P95              28.0 ms
    P99              42.0 ms
    Jitter           3.2 ms
    Samples          128

  ──────────────────────────────────────────

  ● System
  ──────────────────────────────────────────

    Hostname         prod-db-01
    OS               Ubuntu 22.04
    Uptime           10d 2h 15m
    Load 1m          1.23
    Processes        342

    CPU     ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  23.4%
    Memory  ████████████████████████████░░░░░░░░░░░░░░  72.1%
    Disk    ██████████████████░░░░░░░░░░░░░░░░░░░░░░░░  45.0%

  ──────────────────────────────────────────

  ● Capabilities
  ──────────────────────────────────────────

    • metrics
    • plugins
```

---

## Metrics

### `sentinel metrics agent <id>` (alias: `a`)

Latest metric values for an agent with bar chart visualization.

```bash
sentinel metrics agent prod-db
sentinel metrics a prod-db
```

### `sentinel metrics history <id> <metric>` (alias: `hist`)

Time-series history with ASCII sparkline.

```bash
sentinel metrics history prod-db cpu.usage_percent
sentinel metrics hist prod-db cpu.usage_percent --hours 6
```

| Flag      | Default | Description     |
| --------- | ------- | --------------- |
| `--hours` | `24`    | Lookback window |

### `sentinel metrics names <id>` (alias: `ls`)

List all metric names collected for an agent.

```bash
sentinel metrics names prod-db
sentinel metrics ls prod-db
```

### `sentinel metrics summary` (alias: `fleet`)

Fleet-wide metrics overview across all agents.

```bash
sentinel metrics summary
sentinel metrics fleet
```

### `sentinel metrics compare <ids> <metric>` (alias: `cmp`)

Compare a metric across multiple agents in a side-by-side table.

```bash
sentinel metrics compare prod-db,staging-web cpu.usage_percent
sentinel metrics cmp prod-db,staging-web cpu.usage_percent --hours 6
```

### `sentinel metrics top <id>`

Top metrics by sample count.

```bash
sentinel metrics top prod-db --limit 5 --hours 24
```

| Flag      | Default | Description       |
| --------- | ------- | ----------------- |
| `--limit` | `10`    | Number of metrics |
| `--hours` | `24`    | Lookback window   |

### `sentinel metrics percentiles <id> <metric>` (alias: `pct`)

Percentile distribution with visual plot.

```bash
sentinel metrics percentiles prod-db cpu.usage_percent
sentinel metrics pct prod-db cpu.usage_percent --hours 12
```

### `sentinel metrics export <id>` (alias: `ex`)

Export raw metrics to stdout or file.

```bash
# JSON to stdout
sentinel metrics export prod-db

# CSV to file
sentinel metrics export prod-db --format csv --output metrics.csv

# Filter by metric
sentinel metrics export prod-db --metric cpu.usage_percent --hours 6
```

| Flag       | Default | Description      |
| ---------- | ------- | ---------------- |
| `--format` | `json`  | `json` or `csv`  |
| `--output` | stdout  | Output file path |
| `--metric` | all     | Filter by metric |
| `--hours`  | `24`    | Lookback window  |

### `sentinel metrics show`

Server-internal Prometheus metrics snapshot.

```bash
sentinel metrics show
```

### `sentinel metrics live`

Watch server metrics in real-time. Refreshes periodically.

```bash
sentinel metrics live
```

Press `Ctrl+C` to stop.

---

## Alert Rules

### `sentinel rules list` (alias: `ls`)

List all alert rules.

```bash
sentinel rules list
sentinel rules ls
```

### `sentinel rules get <id>` (alias: `show`)

Get rule details.

```bash
sentinel rules get uuid
sentinel rules show uuid
```

### `sentinel rules create` (alias: `add`)

Create a new alert rule.

```bash
sentinel rules create \
  --name "High CPU" \
  --metric cpu.usage_percent \
  --condition gt \
  --threshold 90 \
  --severity critical \
  --agent-pattern "*" \
  --for-duration 60000
```

| Flag              | Required | Description                          |
| ----------------- | -------- | ------------------------------------ |
| `--name`          | yes      | Rule name                            |
| `--metric`        | yes      | Metric name to evaluate              |
| `--condition`     | yes      | `gt`, `gte`, `lt`, `lte`, `eq`, `ne` |
| `--threshold`     | yes      | Threshold value                      |
| `--severity`      | yes      | `info`, `warning`, `critical`        |
| `--agent-pattern` | no       | Glob pattern (default: `*`)          |
| `--for-duration`  | no       | Duration in ms before firing         |

### `sentinel rules update <id>`

Update an existing rule.

```bash
sentinel rules update uuid --threshold 95 --severity warning
```

### `sentinel rules delete <id>` (alias: `rm`)

Delete a rule.

```bash
sentinel rules delete uuid
sentinel rules rm uuid
```

---

## Notifiers

### `sentinel notifiers create` (alias: `add`)

Create a notification channel.

```bash
# Discord
sentinel notifiers create \
  --name alerts-discord \
  --type discord \
  --config '{"webhook_url": "https://discord.com/api/webhooks/..."}'

# Slack
sentinel notifiers create \
  --name alerts-slack \
  --type slack \
  --config '{"webhook_url": "https://hooks.slack.com/services/..."}'

# Email
sentinel notifiers create \
  --name alerts-email \
  --type smtp \
  --config '{"host": "smtp.gmail.com", "from": "alerts@example.com", "to": "team@example.com", "username": "...", "password": "..."}'

# Telegram
sentinel notifiers create \
  --name alerts-telegram \
  --type telegram \
  --config '{"bot_token": "123:ABC", "chat_id": "-100123"}'

# PagerDuty
sentinel notifiers create \
  --name alerts-pagerduty \
  --type pagerduty \
  --config '{"routing_key": "your-routing-key"}'
```

See [Notifications](notifications.md) for all 10 supported backends.

### `sentinel notifiers list` (alias: `ls`)

List all configured notifiers.

```bash
sentinel notifiers list
sentinel notifiers ls
```

### `sentinel notifiers update <id>` (alias: `edit`)

Update a notifier configuration.

```bash
sentinel notifiers update uuid --config '{"webhook_url": "https://new-url..."}'
```

### `sentinel notifiers delete <id>` (alias: `rm`)

Delete a notifier.

```bash
sentinel notifiers delete uuid
```

### `sentinel notifiers test`

Send a test notification.

```bash
sentinel notifiers test --notifier alerts-discord
```

### `sentinel notifiers enable <id>` (alias: `toggle`)

Toggle a notifier enabled/disabled.

```bash
sentinel notifiers enable uuid
sentinel notifiers toggle uuid
```

### `sentinel notifiers link`

Link notifiers to an alert rule.

```bash
sentinel notifiers link --rule "High CPU" --notifier alerts-discord
```

### `sentinel notifiers history` (alias: `log`)

View notification delivery history.

```bash
sentinel notifiers history
sentinel notifiers log --notifier alerts-discord
```

---

## Cluster

### `sentinel cluster status`

Cluster overview: connected agents count, total heartbeats, average latency.

```bash
sentinel cluster status
```

### `sentinel cluster agents`

List IDs of currently connected agents.

```bash
sentinel cluster agents
```

### `sentinel cluster watch`

Real-time SSE stream of agent connection/disconnection events.

```bash
sentinel cluster watch
```

Press `Ctrl+C` to stop.

---

## Alerts

### `sentinel alerts list` (alias: `ls`)

List recent fired alerts.

```bash
sentinel alerts list
sentinel alerts ls
```

### `sentinel alerts get <id>` (alias: `show`)

Get alert details.

```bash
sentinel alerts get alert-uuid
```

---

## Configuration

### `sentinel config show`

Display current CLI configuration.

```bash
sentinel config show
```

### `sentinel config set <key> <value>`

Set a configuration value.

```bash
sentinel config set server.url http://my-server:8080
sentinel config set output.mode json
```

### `sentinel config edit`

Open interactive config editor.

```bash
sentinel config edit
```

### `sentinel config path`

Print the config file path.

```bash
sentinel config path
# /home/user/.config/sentinel/config.toml
```

### `sentinel config reset`

Reset to default configuration.

```bash
sentinel config reset
```

---

## Keys

### `sentinel key rotate`

Rotate an agent's HMAC signing key.

```bash
sentinel key rotate --agent-id prod-db
```

### `sentinel key list`

List keys for an agent.

```bash
sentinel key list --agent-id prod-db
```

### `sentinel key delete`

Delete a deprecated key.

```bash
sentinel key delete --agent-id prod-db --key-id old-key-uuid
```

---

## WAL (Write-Ahead Log)

### `sentinel wal stats`

WAL statistics: segment count, total size, pending batches.

```bash
sentinel wal stats
```

### `sentinel wal inspect`

Inspect WAL segments and their contents.

```bash
sentinel wal inspect
```

### `sentinel wal compact`

Force WAL compaction (remove acknowledged segments).

```bash
sentinel wal compact
```

### `sentinel wal meta`

Display WAL metadata (sequence numbers, offsets).

```bash
sentinel wal meta
```

---

## Misc

### `sentinel register`

Register a new agent via gRPC (low-level).

```bash
sentinel register --agent-id my-server --server grpc://localhost:50051
```

### `sentinel force-send`

Force-send unacknowledged WAL batches.

```bash
sentinel force-send
```

---

## Output Modes

All commands support two output modes:

### Human (default)

Colored tables, bar charts, sparklines, status icons.

```bash
sentinel agents list
```

### JSON

Machine-readable JSON for scripting and piping.

```bash
sentinel agents list --json
sentinel agents list --json | jq '.[].agent_id'
```

Every command that produces output supports `--json`.
