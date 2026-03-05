# Notifications

SentinelRS supports 10 notification backends. Each notifier is linked to one or more alert rules
and fires when an alert triggers.

## Workflow

```
Alert Rule fires → Linked notifiers → Delivery → History log
                                         ↓
                                   Failed → DLQ (retry)
```

1. Create a notifier with backend-specific config
2. Create an alert rule
3. Link notifier → rule
4. Test delivery

---

## Backends

### Webhook

Generic HTTP POST with JSON payload and HMAC signature.

```bash
sentinel notifiers create \
  --name webhook-prod \
  --type webhook \
  --config '{"url": "https://api.example.com/hooks/sentinel"}'
```

| Field    | Required | Description                |
| -------- | -------- | -------------------------- |
| `url`    | yes      | Endpoint URL               |
| `secret` | no       | HMAC-SHA256 signing secret |

When `secret` is set, requests include an `X-Sentinel-Signature` header.

Payload structure:

```json
{
    "alert_id": "uuid",
    "rule_name": "High CPU",
    "agent_id": "prod-db",
    "metric": "cpu.usage_percent",
    "value": 95.2,
    "threshold": 90.0,
    "severity": "critical",
    "fired_at": "2025-01-15T10:30:00Z"
}
```

---

### Slack

Posts to a Slack channel via incoming webhook.

```bash
sentinel notifiers create \
  --name slack-alerts \
  --type slack \
  --config '{"webhook_url": "https://hooks.slack.com/services/T00/B00/xxx"}'
```

| Field         | Required | Description            |
| ------------- | -------- | ---------------------- |
| `webhook_url` | yes      | Slack incoming webhook |

Setup:

1. Go to **Slack App Management** → **Incoming Webhooks**
2. Create a new webhook for your channel
3. Copy the URL

---

### Discord

Posts to a Discord channel via webhook.

```bash
sentinel notifiers create \
  --name discord-alerts \
  --type discord \
  --config '{"webhook_url": "https://discord.com/api/webhooks/123/abc"}'
```

| Field         | Required | Description         |
| ------------- | -------- | ------------------- |
| `webhook_url` | yes      | Discord webhook URL |

Setup:

1. Channel Settings → **Integrations** → **Webhooks**
2. Create webhook, copy URL

---

### SMTP (Email)

Sends alert emails.

```bash
sentinel notifiers create \
  --name email-alerts \
  --type smtp \
  --config '{
    "host": "smtp.gmail.com",
    "port": 587,
    "from": "sentinel@example.com",
    "to": "oncall@example.com",
    "username": "sentinel@example.com",
    "password": "app-password"
  }'
```

| Field      | Required | Default | Description       |
| ---------- | -------- | ------- | ----------------- |
| `host`     | yes      |         | SMTP server       |
| `port`     | no       | `587`   | SMTP port         |
| `from`     | yes      |         | Sender address    |
| `to`       | yes      |         | Recipient address |
| `username` | yes      |         | SMTP username     |
| `password` | yes      |         | SMTP password     |

For Gmail: use an [App Password](https://support.google.com/accounts/answer/185833).

---

### Telegram

Sends messages to a Telegram chat via bot API.

```bash
sentinel notifiers create \
  --name telegram-alerts \
  --type telegram \
  --config '{"bot_token": "123456:ABC-DEF", "chat_id": "-1001234567890"}'
```

| Field       | Required | Description               |
| ----------- | -------- | ------------------------- |
| `bot_token` | yes      | Bot token from @BotFather |
| `chat_id`   | yes      | Chat/group/channel ID     |

Setup:

1. Message [@BotFather](https://t.me/BotFather) → `/newbot`
2. Copy the token
3. Add bot to your group
4. Get chat ID via `https://api.telegram.org/bot<token>/getUpdates`

---

### PagerDuty

Creates PagerDuty incidents via Events API v2.

```bash
sentinel notifiers create \
  --name pagerduty-critical \
  --type pagerduty \
  --config '{"routing_key": "your-integration-routing-key"}'
```

| Field         | Required | Description                   |
| ------------- | -------- | ----------------------------- |
| `routing_key` | yes      | Events API v2 integration key |

Setup:

1. PagerDuty → Service → **Integrations** → Add **Events API v2**
2. Copy the integration/routing key

---

### Microsoft Teams

Posts adaptive cards to Teams via incoming webhook.

```bash
sentinel notifiers create \
  --name teams-alerts \
  --type teams \
  --config '{"webhook_url": "https://outlook.office.com/webhook/..."}'
```

| Field         | Required | Description            |
| ------------- | -------- | ---------------------- |
| `webhook_url` | yes      | Teams incoming webhook |

Setup:

1. Teams channel → **...** → **Connectors** → **Incoming Webhook**
2. Name it, copy URL

---

### OpsGenie

Creates OpsGenie alerts via v2 API.

```bash
sentinel notifiers create \
  --name opsgenie-critical \
  --type opsgenie \
  --config '{"api_key": "your-opsgenie-api-key"}'
```

| Field     | Required | Description      |
| --------- | -------- | ---------------- |
| `api_key` | yes      | OpsGenie API key |

Setup:

1. OpsGenie → **Settings** → **Integration** → **API**
2. Copy the API key

---

### Gotify

Push notifications via Gotify server.

```bash
sentinel notifiers create \
  --name gotify-alerts \
  --type gotify \
  --config '{"url": "https://gotify.example.com", "token": "app-token"}'
```

| Field   | Required | Description       |
| ------- | -------- | ----------------- |
| `url`   | yes      | Gotify server URL |
| `token` | yes      | Application token |

Setup:

1. Gotify Web UI → **Apps** → Create application
2. Copy the token

---

### ntfy

Push notifications via [ntfy.sh](https://ntfy.sh) or self-hosted ntfy.

```bash
sentinel notifiers create \
  --name ntfy-alerts \
  --type ntfy \
  --config '{"url": "https://ntfy.sh", "topic": "sentinel-alerts"}'
```

| Field   | Required | Default           | Description     |
| ------- | -------- | ----------------- | --------------- |
| `url`   | no       | `https://ntfy.sh` | ntfy server URL |
| `topic` | yes      |                   | Topic name      |

---

## Linking to Alert Rules

A notifier must be linked to at least one rule to receive alerts.

```bash
# Create a rule
sentinel rules create \
  --name "High CPU" \
  --metric cpu.usage_percent \
  --condition gt \
  --threshold 90 \
  --severity critical

# Link notifier to rule
sentinel notifiers link --rule "High CPU" --notifier slack-alerts

# Link multiple notifiers
sentinel notifiers link --rule "High CPU" --notifier discord-alerts
sentinel notifiers link --rule "High CPU" --notifier pagerduty-critical
```

---

## Testing

Always test a notifier after creation:

```bash
sentinel notifiers test --notifier slack-alerts
```

This sends a test payload with dummy alert data.

---

## Delivery History

View notification delivery log:

```bash
# All history
sentinel notifiers history

# Filter by notifier
sentinel notifiers history --notifier slack-alerts
```

Via REST API:

```bash
curl http://localhost:8080/api/v1/notification-history
curl http://localhost:8080/api/v1/notification-history?notifier_config_id=uuid
```

---

## Dead Letter Queue

Failed notifications are stored in the `notifications_dlq` table and retried automatically.

Check DLQ via database:

```sql
SELECT * FROM notifications_dlq ORDER BY created_at DESC LIMIT 10;
```

---

## Multi-Notifier Strategy

Combine backends for redundancy and routing by severity:

```bash
# Info → Slack only
sentinel notifiers link --rule "Disk Warning" --notifier slack-alerts

# Critical → Slack + PagerDuty + Email
sentinel notifiers link --rule "High CPU" --notifier slack-alerts
sentinel notifiers link --rule "High CPU" --notifier pagerduty-critical
sentinel notifiers link --rule "High CPU" --notifier email-alerts
```
