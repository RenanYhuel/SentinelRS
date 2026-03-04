# Security

## Transport Security

### TLS

All communication between agents and server can be encrypted with TLS 1.2+.

```bash
sentinel_server \
  --tls-cert /path/to/cert.pem \
  --tls-key  /path/to/key.pem
```

### Mutual TLS (mTLS)

Adding `--tls-ca` enables client certificate verification. Only agents presenting a certificate signed by the specified CA can connect.

```bash
sentinel_server \
  --tls-cert /path/to/cert.pem \
  --tls-key  /path/to/key.pem \
  --tls-ca   /path/to/ca.pem
```

## Batch Signing (HMAC-SHA256)

Every metric batch is signed to prevent tampering and ensure authenticity.

### Flow

```
Agent                          Server
  │                              │
  │  payload = serialize(batch)  │
  │  sig = HMAC-SHA256(key, payload + timestamp)
  │                              │
  │── { payload, sig, ts } ────▶ │
  │                              │  verify HMAC
  │                              │  check timestamp in window
  │◀── BatchAck ─────────────── │
```

### Replay Protection

The server rejects batches with timestamps outside the configured window:

```
REPLAY_WINDOW_MS=300000    # 5 minutes (default)
```

Any batch with `|now - batch_timestamp| > window` is rejected with `REJECTED` status.

### Key Grace Period

During key rotation, both old and new keys are valid for a configurable grace period:

```
KEY_GRACE_PERIOD_MS=86400000    # 24 hours (default)
```

## Key Management

### Agent Key Store

Agent keys are stored encrypted with AES-256-GCM. The store location is set in the agent config:

```yaml
security:
    key_store: "auto" # auto | file | /path/to/keyfile
```

- `auto`: Platform-specific secure storage (OS keychain when available, encrypted file fallback)
- `file`: Encrypted file in the agent's data directory
- Explicit path: Encrypted file at the given path

### Key Rotation

Rotate an agent's HMAC key:

```bash
# Via CLI
sentinel key rotate --agent-id my-server

# Via REST API
curl -X POST http://localhost:8080/v1/agents/my-server/rotate-key
```

The server generates a new key, pushes it via `ConfigUpdate` on the gRPC stream, and keeps the old key valid for the grace period.

### List and Delete Keys

```bash
sentinel key list --agent-id my-server
sentinel key delete --agent-id my-server --key-id old-key-uuid
```

## JWT Authentication

The REST API uses JWT (HS256) for protected endpoints.

### Token Structure

```json
{
    "sub": "sentinel-cli",
    "iat": 1709568000,
    "exp": 1709654400
}
```

### Configuration

Set the JWT signing secret on the server:

```bash
sentinel_server --jwt-secret "your-strong-secret-here"
```

The secret must be at least 32 characters for adequate security.

## Agent Provisioning

### Bootstrap Flow

```
CLI                    Server                Agent
 │                       │                     │
 │── generate-install ──▶│                     │
 │◀── token + command ── │                     │
 │                       │                     │
 │         (deploy agent with token)           │
 │                       │                     │
 │                       │◀── BootstrapReq ─── │
 │                       │   (token + hw_id)   │
 │                       │                     │
 │                       │── BootstrapResp ──▶ │
 │                       │   (agent_id, secret)│
 │                       │                     │
 │                       │  (token invalidated)│
```

1. CLI generates a one-time bootstrap token via `sentinel agents generate-install`
2. Token is embedded in the install command
3. Agent presents the token on first connection
4. Server provisions the agent (assigns ID, generates HMAC secret)
5. Token is immediately invalidated (single-use)

### Token Security

- Tokens are hashed before storage (never stored in plaintext)
- Tokens expire after a configurable TTL
- Each token can only be used once
- Status codes: `OK`, `INVALID_TOKEN`, `EXPIRED_TOKEN`

## WAL Integrity

The Write-Ahead Log uses CRC32 checksums on every segment to detect corruption. On replay, corrupted segments are skipped and logged.

## Compression

Metric batches above 1024 bytes are automatically gzip-compressed before transmission. The server detects compression from the message header and decompresses transparently.

## WASM Plugin Sandboxing

Plugins run in an isolated Wasmtime sandbox with:

- **No filesystem access**: Plugins cannot read or write host files
- **No network access**: Plugins cannot make network calls
- **Memory limit**: Configurable per-plugin memory cap
- **Execution timeout**: Plugins are killed after timeout
- **Host functions only**: `log(level, message)` and `emit_metric_json(json_string)`

### Plugin Signing

Production deployments can require WASM plugins to be signed. Unsigned plugins are rejected at load time.

## Webhook Signing

Outbound webhook notifications are signed with HMAC-SHA256 when a `secret` is configured:

```json
{
    "name": "my-webhook",
    "ntype": "webhook",
    "config": {
        "url": "https://example.com/hook",
        "secret": "webhook-signing-secret"
    }
}
```

The signature is sent in the `X-Sentinel-Signature` header for verification by the receiver.

## Security Checklist

| Item                                  | Status |
| ------------------------------------- | ------ |
| Set unique JWT_SECRET                 | [ ]    |
| Set unique agent secrets              | [ ]    |
| Enable TLS on gRPC                    | [ ]    |
| Enable TLS on REST API                | [ ]    |
| Restrict DB access                    | [ ]    |
| Configure replay window               | [ ]    |
| Store secrets in env vars (not files) | [ ]    |
| Enable mTLS for high-security envs    | [ ]    |
| Sign WASM plugins in production       | [ ]    |
| Set webhook signing secrets           | [ ]    |
