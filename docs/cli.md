# CLI Reference

The `sentinel` CLI provides full administrative control over the SentinelRS platform: cluster monitoring, agent provisioning, alert rules, WAL inspection, key management and more.

## Global options

| Flag              | Description                                     |
| ----------------- | ----------------------------------------------- |
| `--json`          | Output as JSON instead of human-readable tables |
| `--server <URL>`  | Server base URL (overrides config file)         |
| `--config <PATH>` | Path to agent config file                       |

## Commands

### init

Interactive setup wizard. Configures the CLI's server URL and stores it in `~/.config/sentinel/config.json`.

```bash
sentinel init
```

---

### doctor

Run system diagnostics: checks server reachability, NATS connectivity and database health.

```bash
sentinel doctor
```

---

### completions

Generate shell completions.

```bash
sentinel completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

---

### agents

Manage registered agents.

#### agents list

```bash
sentinel agents list
```

Displays: Agent ID, HW ID, Version, Last Seen.

#### agents get

```bash
sentinel agents get <AGENT_ID>
```

Detailed view of a single agent: ID, hardware ID, version, registration time, last seen, connection status.

#### agents live

Watch an agent's metrics in real-time via SSE streaming.

```bash
sentinel agents live <AGENT_ID> [--interval <SECONDS>]
```

| Argument     | Default | Description                 |
| ------------ | ------- | --------------------------- |
| `--interval` | 2       | Refresh interval in seconds |

Displays a live-updating dashboard with CPU, memory, disk and network metrics.

#### agents delete

```bash
sentinel agents delete <AGENT_ID> [--yes]
```

Remove an agent from the registry. Prompts for confirmation unless `--yes` is passed.

#### agents generate-install

Generate a one-liner install command with a bootstrap token for zero-touch provisioning.

```bash
sentinel agents generate-install [--server <URL>]
```

Returns a command you can copy-paste on a target host to automatically provision and start an agent. See [provisioning.md](provisioning.md) for details.

---

### cluster

Cluster status and monitoring.

#### cluster status

```bash
sentinel cluster status
```

Shows cluster overview: connected agents count, total registered agents, server uptime, NATS stream stats.

#### cluster agents

```bash
sentinel cluster agents
```

Lists currently connected agents in the cluster with their session info, latency and uptime.

#### cluster watch

```bash
sentinel cluster watch
```

Streams real-time cluster events via SSE: agent connections, disconnections, presence changes. Press `Ctrl+C` to stop.

---

### config

Manage CLI and agent configuration.

#### config show

```bash
sentinel config show
```

Display the full resolved agent configuration.

#### config validate

```bash
sentinel config validate
```

Validate a YAML config file. Returns `{valid: true}` or a validation error.

#### config path

```bash
sentinel config path
```

Print the resolved config file path.

---

### rules

Alert rule management via the REST API.

#### rules list

```bash
sentinel rules list
```

Table: ID, Name, Metric, Condition, Threshold.

#### rules get

```bash
sentinel rules get <RULE_ID>
```

#### rules create

```bash
sentinel rules create --data <JSON>
```

Example:

```json
{
    "name": "High CPU",
    "metric_name": "cpu_usage",
    "condition": "GreaterThan",
    "threshold": 90.0,
    "severity": "Critical",
    "for_duration_ms": 60000,
    "agent_pattern": "*"
}
```

#### rules update

```bash
sentinel rules update <RULE_ID> --data <JSON>
```

#### rules delete

```bash
sentinel rules delete <RULE_ID> [--yes]
```

---

### notifiers

Test notification channels.

#### notifiers test

```bash
sentinel notifiers test --type <TYPE> --target <URL> [--secret <TOKEN>]
```

| Type      | Target format                          |
| --------- | -------------------------------------- |
| `webhook` | Any HTTP/HTTPS URL                     |
| `slack`   | `https://hooks.slack.com/...`          |
| `discord` | `https://discord.com/api/webhooks/...` |
| `smtp`    | JSON with `host`, `from`, `to` fields  |

In human mode, `--type` shows an interactive fuzzy-select picker.

---

### key

Agent encryption key management.

#### key rotate

```bash
sentinel key rotate --key-id <ID> --secret <BASE64_SECRET>
```

Requires `SENTINEL_MASTER_KEY` environment variable for AES-256-GCM encryption.

#### key list

```bash
sentinel key list
```

#### key delete

```bash
sentinel key delete <KEY_ID> [--yes]
```

---

### wal

WAL inspection and maintenance (reads from local agent data).

#### wal stats

```bash
sentinel wal stats
```

Summary: total size, segment count, unacked entries.

#### wal inspect

```bash
sentinel wal inspect [--limit <N>]
```

List unacked WAL entries with record ID, size, batch ID and metrics count.

#### wal compact

```bash
sentinel wal compact [--force] [--yes]
```

| Argument  | Description                         |
| --------- | ----------------------------------- |
| `--force` | Skip the 64 MB size threshold check |
| `--yes`   | Skip confirmation prompt            |

Rewrites segment files into a single compacted segment, removing all acked records.

#### wal meta

```bash
sentinel wal meta
```

Show head/tail sequence numbers, last segment index, acked count.

---

### metrics

Server metrics visualization.

```bash
sentinel metrics <SUBCOMMAND>
```

---

### health

```bash
sentinel health
```

Query `/healthz` and `/ready` on the server.

---

### status

```bash
sentinel status
```

Combined dashboard: server reachability, agent ID, WAL stats.

---

### register

Register a new agent via gRPC.

```bash
sentinel register --hw-id <HARDWARE_ID> [--agent-version <VERSION>] [--save]
```

Returns `agent_id` and `secret`. Use `--save` to persist credentials to `~/.config/sentinel/credentials.json`.

For zero-touch provisioning, prefer `agents generate-install`.

---

### force-send

Manually push unacked WAL entries to the server.

```bash
sentinel force-send [--limit <N>] [--yes]
```

| Argument  | Default | Description              |
| --------- | ------- | ------------------------ |
| `--limit` | 0 (all) | Maximum batches to send  |
| `--yes`   | —       | Skip confirmation prompt |

Reads unacked WAL entries, pushes each via gRPC, and acks on success.

---

### tail-logs

Tail agent log files with colorized output.

```bash
sentinel tail-logs [--file <PATH>] [--lines <N>] [--follow]
```

| Argument   | Default          | Description                   |
| ---------- | ---------------- | ----------------------------- |
| `--file`   | Platform default | Log file path                 |
| `--lines`  | 20               | Number of lines to display    |
| `--follow` | —                | Continuously follow new lines |

Default log paths:

- Linux: `/var/log/sentinel/agent.log`
- Windows: `%APPDATA%/sentinel/agent.log`

Color coding: red = error, yellow = warn, cyan = info, dim = debug.

---

### version

```bash
sentinel version
```

Displays binary name, version, architecture and OS.

## Output modes

All commands support two output modes:

- **Human** (default) — colored tables and formatted text
- **JSON** (`--json`) — machine-readable output for automation and piping to `jq`
