# Security

SentinelRS implements defense-in-depth across transport, data integrity, key management and plugin isolation.

## Transport Security (TLS)

### Server-side TLS

The server supports TLS on both gRPC (default `:50051`) and REST (default `:8080`) listeners. Ports are configurable via `--grpc-port` / `--rest-port` flags or `GRPC_ADDR` / `REST_ADDR` environment variables. Configure TLS via:

```yaml
tls:
    cert_path: /etc/sentinel/tls/server-cert.pem
    key_path: /etc/sentinel/tls/server-key.pem
```

Certificates are loaded as PEM files. The gRPC listener uses `tonic::transport::ServerTlsConfig`.

### Mutual TLS (mTLS)

When `ca_path` is provided, the server requires client certificates:

```yaml
tls:
    cert_path: /etc/sentinel/tls/server-cert.pem
    key_path: /etc/sentinel/tls/server-key.pem
    ca_path: /etc/sentinel/tls/ca-cert.pem
```

The CA certificate is used to verify the agent's client certificate, providing mutual authentication.

### Development Certificates

Generate self-signed RSA-4096 certificates:

```bash
./scripts/gen-dev-certs.sh [output_dir]
```

Generates a CA + server certificate with SAN entries for `localhost`, `127.0.0.1` and `::1`.

## Batch Signing (HMAC-SHA256)

Every metric batch sent by an agent is signed with HMAC-SHA256.

### Flow

1. Agent serializes the `Batch` protobuf payload
2. Agent computes `HMAC-SHA256(secret, payload)` and encodes as base64
3. Signature is attached to the gRPC metadata
4. Server recomputes the HMAC and compares — rejects on mismatch

### Replay Protection

The server enforces a **replay window** (default: 5 minutes). Batches with timestamps outside this window are rejected. Combined with the idempotency store (which tracks batch IDs), this prevents both replay attacks and duplicate processing.

### Key Grace Period

After key rotation, the server accepts signatures from the old key for a configurable grace period (default: 24 hours) to handle in-flight messages from agents that haven't yet picked up the new key.

## Key Management

### Agent Key Store

The agent stores its secret key using an encrypted file-based key store.

- Storage format: `{agent_id}.key` files in the key store directory
- Encryption: **AES-256-GCM**
- File format: `[12-byte nonce][ciphertext + authentication tag]`
- Master key: first 32 bytes of `SENTINEL_MASTER_KEY` environment variable

### CLI Key Store

The CLI uses `EncryptedFileStore` for key rotation operations:

- Files: `{key_id}.enc` in the key store directory
- Same AES-256-GCM encryption scheme
- Master key from `SENTINEL_MASTER_KEY`

### Key Rotation

Rotate an agent's key:

```bash
# Via REST API
curl -X POST http://localhost:8080/v1/agents/<agent_id>/rotate-key

# Via CLI
sentinel key rotate --key-id <ID> --secret <BASE64_SECRET>
```

The server generates a new key pair and returns the new `key_id` and `secret`. The old key remains valid during the grace period.

### OS Key Store

An `OsKeyStore` implementation exists as an in-memory fallback (backed by `HashMap<String, Vec<u8>>` behind a `Mutex`). Used in testing and environments where file-based storage is unavailable.

## Data Integrity

### WAL CRC32

Every WAL record includes a CRC32 checksum:

```
[4 bytes: payload length (LE)]
[8 bytes: record ID (LE)]
[N bytes: payload]
[4 bytes: CRC32 checksum (LE)]
```

On read, the CRC is recomputed and verified. Corrupted records are detected and rejected.

### Compression

Batch payloads above 1024 bytes are compressed with gzip (`flate2`) before transmission. The `should_compress(data)` function controls the threshold.

## WASM Plugin Sandboxing

Plugins run in a sandboxed WebAssembly environment powered by `wasmtime`.

### Resource Limits

Each plugin declares resource limits in its manifest:

| Limit           | Default | Description                                         |
| --------------- | ------- | --------------------------------------------------- |
| `max_memory_mb` | 64      | Maximum linear memory                               |
| `timeout_ms`    | 5000    | Execution timeout (enforced via epoch interruption) |
| `max_metrics`   | 1000    | Maximum metrics a plugin can emit per run           |

### Execution Model

1. The engine compiles the `.wasm` module with epoch interruption enabled
2. A `Store` is created with memory limits from the manifest
3. The exported `entry_fn` is called (signature: `() -> i32`, returns 0 on success)
4. A background thread increments the epoch after `timeout_ms` — triggering a trap if the function hasn't returned

### Host Functions

Plugins interact with the host through two imported functions (module: `"sentinel"`):

| Function           | Signature           | Description                                                                   |
| ------------------ | ------------------- | ----------------------------------------------------------------------------- |
| `log`              | `(ptr, len)`        | Write a log message to the host                                               |
| `emit_metric_json` | `(ptr, len) -> i32` | Emit a JSON-encoded metric. Returns 0 on success, -1 if `max_metrics` reached |

Plugins **cannot** access the filesystem, network, or any host resources beyond these two functions.

### Plugin Signing

Plugins can be signed before installation:

- `sign_blob(blob, key)` — computes HMAC-SHA256 of the WASM binary
- `verify_blob(blob, signature, key)` — verifies the signature before loading
- `store_blob(dir, name, blob)` — writes `{name}.wasm`
- `store_manifest(dir, name, yaml)` — writes `{name}.manifest.yml`

## JWT Authentication (REST API)

The REST API uses hand-rolled JWT tokens with HMAC-SHA256.

### Token Structure

```
Header:  {"alg": "HS256", "typ": "JWT"}  (base64)
Payload: {"sub": "<subject>", "exp": <unix_timestamp>}  (base64)
Signature: HMAC-SHA256(secret, header.payload)  (base64)
```

### Validation

1. Token is split into three parts
2. HMAC signature is recomputed and compared (constant-time)
3. Expiration (`exp`) is checked against current time
4. On success, the `sub` claim is extracted as the authenticated identity

Errors: `Malformed`, `InvalidSignature`, `Expired`.

## Security Checklist

- [ ] Set a strong, unique `jwt_secret` (do not use the default)
- [ ] Set `SENTINEL_MASTER_KEY` (32+ random bytes, base64 or raw)
- [ ] Enable TLS on the gRPC listener
- [ ] Enable mTLS if agents are on untrusted networks
- [ ] Verify WASM plugin signatures before deployment
- [ ] Review plugin manifests for reasonable resource limits
- [ ] Restrict NATS and PostgreSQL to internal networks
- [ ] Use `RUST_LOG=warn` or `info` in production (avoid leaking sensitive data in debug logs)
