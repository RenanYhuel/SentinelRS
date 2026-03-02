# Zero-Touch Provisioning

SentinelRS V2 supports automated agent provisioning through bootstrap tokens. New agents can self-register, receive credentials and download their configuration without manual intervention.

## Overview

```
Admin                           Server                         Agent (new)
  │                               │                               │
  │ sentinel agents               │                               │
  │   generate-install            │                               │
  │──────────────────────────────▶│                               │
  │   ← bootstrap token + cmd    │                               │
  │                               │                               │
  │  (copy one-liner to host)     │                               │
  │                               │         BOOTSTRAP_TOKEN       │
  │                               │◀──────────────────────────────│
  │                               │  BootstrapRequest (token,     │
  │                               │    hw_id, agent_version)      │
  │                               │                               │
  │                               │  validate token               │
  │                               │  create agent_id + secret     │
  │                               │  generate config YAML         │
  │                               │                               │
  │                               │  BootstrapResponse ──────────▶│
  │                               │  (agent_id, secret, key_id,   │
  │                               │   config_yaml)                │
  │                               │                               │
  │                               │                  write config │
  │                               │                  scrub token  │
  │                               │                  start normal │
  │                               │◀───── OpenStream (handshake) ─│
```

## Bootstrap tokens

A bootstrap token is a one-time-use secret that authorizes an agent to register itself. Tokens are generated server-side and have a configurable expiry.

### Generating a token

```bash
sentinel agents generate-install --server https://server:8080
```

The command returns:

- The token value
- A ready-to-paste install command for the target host

### Token lifecycle

| State   | Description                                          |
| ------- | ---------------------------------------------------- |
| Active  | Token is valid and waiting for an agent to use it    |
| Used    | An agent has consumed the token; it cannot be reused |
| Expired | The token's TTL has elapsed without being used       |

### Token storage

Tokens are stored in the server's `TokenStore` (backed by the `agents` table). Each token record contains:

- Token hash (the raw token is never persisted)
- Creation timestamp
- Expiry timestamp
- Associated agent ID (set after consumption)

## Agent bootstrap flow

When an agent starts without an existing config file, it enters bootstrap mode automatically.

### Detection

The `detector` module checks two conditions:

1. The config file at the specified path does not exist
2. The `BOOTSTRAP_TOKEN` environment variable is set

If both are true, bootstrap begins.

### Negotiation

The `negotiator` module opens a gRPC stream to the server and sends a `BootstrapRequest`:

```protobuf
message BootstrapRequest {
  string bootstrap_token = 1;
  string hw_id = 2;
  string agent_version = 3;
}
```

The server responds with a `BootstrapResponse`:

```protobuf
message BootstrapResponse {
  BootstrapStatus status = 1;
  string agent_id = 2;
  string secret = 3;
  string key_id = 4;
  bytes config_yaml = 5;
  string message = 6;
}
```

### Status codes

| Status                    | Meaning                       | Agent action        |
| ------------------------- | ----------------------------- | ------------------- |
| `BOOTSTRAP_OK`            | Provisioning succeeded        | Write config, start |
| `BOOTSTRAP_INVALID_TOKEN` | Token unknown or already used | Exit with error     |
| `BOOTSTRAP_EXPIRED_TOKEN` | Token TTL expired             | Exit with error     |

### Config writing

On success, the `config_writer` module writes the received YAML config to the agent's config directory (default: `/etc/sentinel/config.yml`). The config includes the assigned `agent_id`, server URL and collection settings.

### Token cleanup

After successful provisioning, the `cleanup` module removes `BOOTSTRAP_TOKEN` from the process environment to prevent accidental re-use or leaking to child processes.

## Environment variables

| Variable          | Required | Description                                 |
| ----------------- | -------- | ------------------------------------------- |
| `BOOTSTRAP_TOKEN` | Yes      | One-time token from `generate-install`      |
| `SERVER_URL`      | Yes      | gRPC endpoint (e.g. `https://server:50051`) |

## Docker provisioning

The Docker agent image supports zero-touch provisioning natively:

```bash
docker run -d \
  -e BOOTSTRAP_TOKEN=<token> \
  -e SERVER_URL=https://server:50051 \
  -v sentinel-config:/etc/sentinel \
  sentinelrs/agent:latest
```

On first start, the container bootstraps, writes its config to the mounted volume, and begins normal operation. Subsequent restarts reuse the persisted config.

## Security considerations

- Tokens are single-use: once consumed, they cannot provision another agent.
- Tokens expire after a configurable TTL (server-side).
- Only the token hash is stored; the raw token exists only in transit.
- The `BOOTSTRAP_TOKEN` env var is scrubbed from the process after use.
- All bootstrap traffic uses the same gRPC channel (TLS recommended).
- The server validates the token before creating any credentials.
