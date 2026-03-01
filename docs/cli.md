# CLI Reference

The `sentinel` CLI provides administrative control over agents, alert rules, WAL inspection, key management and more.

## Global Options

| Flag              | Description                                     |
| ----------------- | ----------------------------------------------- |
| `--json`          | Output as JSON instead of human-readable tables |
| `--server <URL>`  | Server base URL (overrides config file)         |
| `--config <PATH>` | Path to agent config file                       |

## Commands

### register

Register a new agent with the server.

```bash
sentinel register --hw-id <HARDWARE_ID> [--agent-version <VERSION>] [--save]
```

| Argument          | Required | Description                                                  |
| ----------------- | -------- | ------------------------------------------------------------ |
| `--hw-id`         | Yes      | Unique hardware identifier for this host                     |
| `--agent-version` | No       | Agent version string (defaults to crate version)             |
| `--save`          | No       | Persist credentials to `~/.config/sentinel/credentials.json` |

Returns an `agent_id` and `secret` from the server via gRPC.

---

### config

Manage agent configuration.

#### config show

Display the full resolved agent configuration.

```bash
sentinel config show
```

#### config validate

Validate a YAML config file.

```bash
sentinel config validate
```

Returns `{valid: true}` or a validation error with details.

#### config path

Print the resolved config file path.

```bash
sentinel config path
```

---

### wal

Inspect and manage the agent's Write-Ahead Log.

#### wal stats

Show WAL summary: total size, segment count, unacked entries.

```bash
sentinel wal stats
```

#### wal inspect

List unacked WAL entries.

```bash
sentinel wal inspect [--limit <N>]
```

| Argument  | Default | Description                |
| --------- | ------- | -------------------------- |
| `--limit` | 20      | Maximum entries to display |

Each entry shows: record ID, size in bytes, batch ID, metrics count.

#### wal compact

Compact the WAL by removing acknowledged records.

```bash
sentinel wal compact [--force] [--yes]
```

| Argument  | Description                         |
| --------- | ----------------------------------- |
| `--force` | Skip the 64 MB size threshold check |
| `--yes`   | Skip confirmation prompt            |

Rewrites segment files into a single compacted segment, removing all acked records.

#### wal meta

Show WAL metadata: head/tail sequence numbers, last segment index, acked count.

```bash
sentinel wal meta
```

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

Reads unacked WAL entries, pushes each via gRPC, and acks on success. Displays sent/failed/total counts.

---

### agents

Query registered agents via the REST API.

#### agents list

```bash
sentinel agents list
```

Displays a table: Agent ID, HW ID, Version, Last Seen.

#### agents get

```bash
sentinel agents get <AGENT_ID>
```

---

### rules

Manage alert rules via the REST API.

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

`--data` accepts inline JSON or a file path. Example:

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

Manage agent encryption keys.

#### key rotate

Store a new key in the encrypted file key store.

```bash
sentinel key rotate --key-id <ID> --secret <BASE64_SECRET>
```

Requires the `SENTINEL_MASTER_KEY` environment variable for AES-256-GCM encryption.

#### key list

List all stored encrypted key files.

```bash
sentinel key list
```

#### key delete

```bash
sentinel key delete <KEY_ID> [--yes]
```

---

### health

Check server health.

```bash
sentinel health
```

Queries `/healthz` and `/ready` endpoints.

---

### status

Combined status dashboard.

```bash
sentinel status
```

Checks server reachability, displays agent ID, WAL stats (segments and unacked count).

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

Displays binary name, version, architecture and OS. Human mode renders a formatted banner.

## Output Modes

All commands support two output modes:

- **Human** (default) — Colored tables and formatted text via `comfy-table` and `colored`
- **JSON** (`--json`) — Machine-readable JSON, suitable for piping to `jq` or automation scripts
