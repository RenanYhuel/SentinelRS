# Cahier des charges technique — SentinelRS (V1)

Dernière mise à jour : 2026-02-28

Objectif : spécifier de manière exhaustive et technique le système de monitoring distribué "SentinelRS" écrit en Rust, composé d'un agent léger multi‑plateforme et d'une API/ingest/worker centrale avec dashboard (UI en React — non détaillé ici). Ce document couvre les choix d'architecture, contrats, formats, sécurité, stockage, opérabilité, packaging, tests et roadmap V1.

---

Table des matières

- Résumé exécutif
- Choix architecturaux et décisions V1
- Architecture globale et flux de données
- Spécification détaillée de l'agent
    - Modules internes
    - Mécanique de collecte des métriques
    - Plugins (WASM) : interface et sécurité
    - Buffer local (WAL) : format, compaction, garanties
    - Batching, compression, signatures HMAC
    - Exporter Prometheus / endpoints health
- Contrat agent ↔ serveur (Protobuf + gRPC)
    - Endpoints RPC principaux
    - Messages Protobuf conceptuels
    - Auth, headers, erreurs
- API Gateway / Ingest et Broker (NATS JetStream)
    - Publication & sujets
    - Workers: ingestion, stockage, alerting
- Stockage (TimescaleDB / Postgres) : schéma recommandé
- Moteur d'alerting & règles
- Notifiers (webhooks, Discord, Slack, Telegram, email)
- Dashboard (React) — périmètre fonctionnel
- Observabilité & monitoring interne
- CI/CD, tests et stratégie E2E
- Packaging & distribution multi‑plateforme
- Sécurité approfondie (provisioning, HMAC, rotation)
- Organisation du dépôt, modules Rust, conventions
- Roadmap V1 & critères d'acceptation
- Risques, mitigations et décisions ouvertes
- Annexes : exemples Protobuf, format WAL, checklists

---

## Résumé exécutif

SentinelRS fournit la collecte, centralisation et le traitement des métriques et états d'instances serveurs et services. V1 vise la fiabilité, sécurité et scalabilité opérationnelle en combinant :

- Agent Rust léger (Linux/Windows/macOS) — lecture métriques système + plugins WASM + WAL local durable + envoi via gRPC (Protobuf) au serveur.
- API centrale Rust (tonic/axum) qui valide et publie dans NATS JetStream.
- Workers Rust consommant JetStream, écrivant TimescaleDB (Postgres) et évaluant règles d'alerte.
- Dashboard React (UI) pour suivi, gestion config/plugins et règles — non implémenté dans ce ticket.

Choix principaux validés pour V1 : Protobuf + gRPC (tonic/prost), NATS JetStream, plugins en WASM (wasmtime), Append‑only WAL.

---

## 1. Choix architecturaux et décisions V1 (rappel)

- Transport agent ↔ serveur : Protobuf (v3) + gRPC (tonic/prost)
- Broker/queue : NATS JetStream
- Plugin model agent : WASM (wasmtime)
- Local buffer agent : Append‑only WAL (segmented files) avec compaction
- Authentification : HMAC‑SHA256 (symmetric) sur chaque batch + TLS obligatoire (rustls)
- Stockage TSDB : PostgreSQL + TimescaleDB pour V1

Ces décisions dictent choix de crates Rust, outils de packaging et tests décrits plus bas.

---

## 2. Architecture globale et flux de données

Diagramme logique (conceptuel)

Agent (Rust)
├─ Collector (system metrics + plugins)
├─ WAL local (append-only)
├─ Scheduler + Batch composer
├─ Signer HMAC + Compressor (gzip)
└─ Exporter gRPC (Push) + Prometheus exporter

API Gateway (Rust) — gRPC/HTTP
├─ TLS + Auth validation (HMAC)
├─ Schema validation (Protobuf) + dedup checks
└─ Publisher → NATS JetStream (subject per agent/cluster)

NATS JetStream (Broker)
└─ Durable stream `sentinel.metrics` (partitioning par agent_id possible)

Workers (Rust)
├─ Consumer JetStream → parse Batch
├─ Insert into TimescaleDB (metrics table)
├─ Compute aggregates/rolling stats
└─ Evaluate alert rules → trigger notifiers

TimescaleDB (Postgres)
├─ Hypertables pour metrics
├─ Raw batches archive (jsonb) pour debugging
└─ Alerts, rules, agents metadata

Dashboard (React)
└─ Read data from API (REST/gRPC-Web) — CRUD rules, view charts, manage agents

Flux résumé : Agent → gRPC → API → NATS → Workers → TimescaleDB → Dashboard/Notifiers

---

## 3. Spécification détaillée de l'agent

Objectifs : binaire Rust léger, faible empreinte, docker‑friendly, exécutable en tant que daemon/service, configurable via YAML, sécurisé, résilient aux partitions.

Contraintes multi‑plateformes

- Use `tokio` pour runtime async.
- Use crates cross‑platform : `sysinfo` ou `heim` pour métriques, `wasmtime` pour WASM, `serde_yaml` pour config.

Comportements obligatoires

- Démarrer en tant que démon/service.
- Charger configuration `config.yml` (YAML). Support reload à chaud (SIGHUP ou API).
- Collecter métriques système (périodicité configurable) et exécuter checks plugins.
- Stocker en WAL chaque enregistrement jusqu'à ack serveur.
- Composer batchs et envoyer via gRPC signés (HMAC) et compressés si volumineux.
- Exposer `/metrics` Prometheus et `/healthz` & `/ready` HTTP endpoints.
- Logs structurés JSON et métriques internes pour l'agent.

Modules internes (crates / fichiers suggérés)

- `collector` : lecture CPU (per-core), mémoire (total, used, cached, buffers, swap), disk IO, network IO, uptime, process count, open sockets.
    - Crates possibles : `sysinfo`, `heim` (attention aux plateformes), `pnet` ou `netstat2` pour sockets.
- `plugin_manager` : loader WASM (wasmtime), plugin lifecycle, sandboxing, resource limits.
- `scheduler` : job scheduler (intervals), backoff management for failing tasks.
- `buffer` : WAL manager (append-only segmented files), index small metadata file for head/tail.
- `exporter` : gRPC client (tonic) with TLS via rustls + optional HTTP fallback.
- `api` : local HTTP server for `/metrics`, `/healthz`, `/ready`, admin endpoints.
- `config` : typed config structs with serde + validation rules.
- `security` : HMAC signer/verifier, key store integration (OS keystore) and local secrets encryption.

Format de configuration (`config.yml`) — champs clefs (extrait)

```yaml
agent_id: null # null => registration flow to obtain one
server: https://sentinel.example.com:8443
tls:
    ca_cert: /etc/sentinel/ca.pem
    verify_hostname: true
collect:
    interval_seconds: 10
    metrics:
        cpu: true
        mem: true
        disk: true
plugins_dir: /var/lib/sentinel/plugins
buffer:
    wal_dir: /var/lib/sentinel/wal
    segment_size_mb: 16
    max_retention_days: 7
security:
    key_store: auto # os, file
    rotation_check_interval_hours: 24
```

Métriques collectées (noms exemples) — V1 minimal

- `cpu.core.<i>.usage_percent` (float)
- `mem.total_bytes`, `mem.used_bytes`, `mem.cached_bytes`, `mem.buffers_bytes`, `mem.swap_used_bytes`
- `disk.<device>.read_bytes_total`, `disk.<device>.write_bytes_total`, `disk.<device>.iops`
- `net.<iface>.bytes_sent`, `net.<iface>.bytes_recv`
- `uptime_seconds`
- `process.count_total`, `process.count_zombie`
- `sockets.tcp_open`, `sockets.udp_open`

Metric record model (internal)

- `Metric` {
    - `name: String`,
    - `labels: Map<String,String>`,
    - `value: Number` (float64 preferred),
    - `metric_type: ENUM` (GAUGE, COUNTER, HISTOGRAM),
    - `timestamp_ms: i64`
      }

Batch struct (Protobuf counterpart à définir) :

- `agent_id`, `batch_id` (uuid v4), `seq_start`, `seq_end`, `created_at_ms`, `metrics: []`, `meta`.

Batching & flush policy

- Flush on interval (ex: 10s) or when uncompressed size > 64KB.
- Compress with gzip when compressed_size > threshold.
- Each batch assigned `batch_id` UUID and sequential `seq` numbers for ordering.

Signatures HMAC

- Algorithm: HMAC‑SHA256
- Signature over canonical protobuf bytes + `batch_id` + `timestamp_ms` ordering to prevent manipulation.
- Metadata headers (gRPC metadata) : `x-agent-id`, `x-signature` (base64), `x-key-id` (kid).
- Server checks signature + timestamp ± window (configurable, default ±30s).

Buffer local (WAL) — format et garanties

- WAL segments: `wal-0000001.log`, `wal-0000002.log` …
- Record layout : [4B length][protobuf Envelope bytes][4B CRC32]
- On send succeed (server ack), mark record as acknowledged and allow compaction.
- Compaction: background job rewrites segments removing acknowledged records, produces new compacted segments.
- Guarantee : at‑least‑once delivery; idempotence guaranteed via `batch_id` server‑side dedup.

Retry policy & ordering

- Sequential sending per agent by default to preserve order; optional concurrent pipeline if ordering not required for some metrics.
- Exponential backoff with jitter on network errors.

Prometheus exporter

- Expose `/metrics` with standard Prometheus exposition using `prometheus` crate.
- Map internal metrics and plugin results to Prometheus format.

Health & readiness

- `/healthz` returns 200 when agent running and WAL writable.
- `/ready` returns 200 when agent has recent connectivity or WAL present (configurable PO policy).

Agent registration flow

- If `agent_id` absent, agent calls `Register` RPC with hardware fingerprint and capabilities.
- Server returns assigned `agent_id` and provisioning secret; agent stores secret encrypted.

Plugin model (WASM)

- Plugins packaged as WASM modules with small manifest JSON (`plugin.toml` equivalent): name, version, capabilities, entry function name, resource limits.
- WASM runtime: wasmtime — execute plugin in sandbox with limited memory/CPU time.
- Host functions to expose to plugin: timers, HTTP client helper (with timeouts), basic filesystem read (restricted), metrics builder API.
- Plugin trait (concept) : receives `Context` -> returns `Vec<Metric>`.
- Plugin lifecycle: download via server (signed), verify signature, store in plugin_dir, load with wasmtime.

Sécurité plugins

- Signature verification of WASM binaries (server signs, agent verifies using server public key).
- Resource caps (memory, CPU time) enforced, plugin runtime OOM or trap should not crash agent.

---

## 4. Contrat agent ↔ serveur (Protobuf + gRPC)

Raison : Protobuf fournit schema strict, compatibilité et compaction binaire (meilleur débit). gRPC (tonic) est le transport recommandé.

Services gRPC principaux (noms suggérés)

- `AgentService` {
    - `rpc Register(RegisterRequest) returns (RegisterResponse)`
    - `rpc PushMetrics(Batch) returns (PushResponse)`
    - `rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse)`
    - `rpc GetConfig(GetConfigRequest) returns (GetConfigResponse)`
      }

Exemples de messages (extraits conceptuels)

```protobuf
message Metric {
  string name = 1;
  map<string,string> labels = 2;
  oneof value {
    double value_double = 3;
    int64 value_int = 4;
  }
  int64 timestamp_ms = 5;
  MetricType type = 6;
}

message Batch {
  string agent_id = 1;
  string batch_id = 2; // uuid
  uint64 seq_start = 3;
  uint64 seq_end = 4;
  int64 created_at_ms = 5;
  repeated Metric metrics = 6;
}

message PushResponse {
  enum Status { OK = 0; REJECTED = 1; RETRY = 2; }
  Status status = 1;
  string message = 2;
}
```

Auth & headers

- Use TLS (gRPC with rustls). In addition, expect metadata headers: `x-agent-id`, `x-signature`, `x-key-id`.
- The server verifies HMAC using stored secret for agent_id. On mismatch => authentication error.

Errors

- Use gRPC status codes: `Unauthenticated` (401), `InvalidArgument` (400), `ResourceExhausted` (429), `Internal` (500). Return helpful error messages for debugging.

Idempotence

- Server uses `batch_id` as idempotency key and stores processed `batch_id`s with TTL (configurable) to prevent double processing.

---

## 5. API Gateway / Ingest & Broker (NATS JetStream)

Rôle de l'API : validation, auth, publish dans JetStream. L'API ne fait pas le traitement intensif : responsabilité limitée à ingestion/validation/forward.

Publication

- Subject naming: `sentinel.metrics` as the main stream; optionally `sentinel.metrics.<agent_id_prefix>` for sharding.
- Use JetStream durable streams with ack policy `AckExplicit` to ensure workers confirm processing.

Workers

- Worker subscribers (push/pull consumers) lisent messages, valident batch, persist raw batch à Postgres, transforment et insèrent les lignes timeseries.
- Worker concurrency: adjustable; ensure per‑agent ordering if required by using per‑agent consumer or stream partitioning.

Replay & reprocessing

- JetStream supports replay; workers must store processed offsets and dedup via `batch_id` pour éviter double writes.

Monitoring

- Track JetStream lag (consumer lag), failed messages, redeliveries.

---

## 6. Stockage (TimescaleDB / PostgreSQL) — schéma recommandé

Principes : Hypertable TimescaleDB pour séries temps, tables relationnelles pour metadata.

Tables principales

- `agents (agent_id PK, hw_id, registered_at, last_seen, version, meta jsonb)`
- `metrics_time (time timestamptz, agent_id text, metric_name text, labels jsonb, value double precision)` — hypertable with index on (metric_name, agent_id)
- `metrics_raw (id serial, agent_id text, batch_id text, payload jsonb, received_at timestamptz)`
- `alerts (id uuid, agent_id text, rule_id uuid, state enum, started_at timestamptz, resolved_at timestamptz, payload jsonb)`
- `rules (id uuid, name text, expr jsonb, severity text, enabled bool, created_by text)`

Indexation & partitioning

- Time index on `metrics_time(time)` via Timescale.
- Secondary indexes on (agent_id, metric_name). Labels stored as JSONB for flexible queries; create GIN index on labels if frequent label queries.

Retention

- Configurable per-metric retention; background job to downsample and purge raw metrics as per policy.

---

## 7. Moteur d'alerting & règles

Types de règles V1

- Threshold: e.g., `cpu.core.0.usage_percent > 90 for 2m`
- Absence: e.g., heartbeat missing > X interval
- Simple anomaly: deviation vs rolling mean > N sigma (basic)

Évaluation

- Workers compute évaluation à intervalles configurables (default 30s).
- Rules expressed en JSON DSL simple (PromQL deferred to V2).

State & dedup

- Alert fingerprinting pour dédup; table `alerts` contient l'état courant.
- Support silence windows et escalations.

---

## 8. Notifiers

V1 supports webhooks, Discord, Slack, Telegram, email (SMTP). Implement each notifier as separate Rust module with retry/backoff and templating.

Webhook contract

- POST JSON to configured URL with `application/json` contenant `alert` payload, `alert_id`, `agent_id`, `rule`, `severity`, `first_seen`, `value`, `links`.

Security

- Support secret signature on outgoing webhook (HMAC using webhook secret) to allow receiver verification.

---

## 9. Dashboard (React) — périmètre V1 (fonctionnel, pas d'implémentation)

Fonctionnalités minimales à fournir via API :

- Liste agents + statut online/offline
- Vue agent détaillée : métriques historiques + realtime chart (timescale queries), last heartbeat, config
- CRUD rules d'alerting
- Gestion des notifications (ajout channels, test webhook)
- Upload / gestion plugins WASM (server stores metadata)

Le dashboard sera une SPA React. Détails UI non développés ici.

---

## 10. Observabilité et monitoring interne

Agent

- Expose métriques d'agent (ingest success rate, queue length, WAL size) via `/metrics`.

Server

- Expose Prometheus metrics for ingestion rate, validation errors, JetStream lag, worker latency, DB write latency.

Tracing

- Optionnel : OpenTelemetry instrumentation (future), traces to Jaeger/Tempo.

Logs

- JSON structured logs with fields : `ts, level, target, message, agent_id?, batch_id?, trace_id?`.

---

## 11. Tests, CI/CD et stratégie E2E

Tests

- Unit tests (`cargo test`) pour chaque crate.
- Integration tests in `tests/` using `tokio::test` and mocks (`mockito`) for HTTP where needed.
- Plugin tests: WASM plugin unit tests et runtime integration.

E2E

- Docker Compose stack for E2E: NATS (with JetStream), Postgres+Timescale, server, agent(s) containerized.
- Scenarios: WAL replay, agent restart, alert generation, signature tampering rejection.

CI (GitHub Actions recommended)

- Matrix: Ubuntu-latest, Windows-latest (MSVC), macOS-latest.
- Steps: fmt (rustfmt), lint (clippy), unit tests, integration tests, build artifacts.

Release

- Build static linux musl binary for containers; MSVC builds for Windows; macOS x86_64/arm64 builds.

---

## 12. Packaging & distribution

Per OS

- Linux: static binary + systemd unit file + `.deb` via `cargo-deb`.
- Windows: MSVC build + windows service registration helper + installer via WiX.
- macOS: signed bundle + launchd plist (notarization documented).
- Containers: minimal image with static binary.

Upgrades

- Agents should support zero‑downtime upgrade: systemd restart or windows service update. Provide `--migrate` helper if needed.

---

## 13. Sécurité détaillée

Provisioning

- Server issues `agent_id` and symmetric `secret` on register; agent stores encrypted.
- Alternative future mode : asymmetric (agent generates keypair and server stores public key).

Signing

- HMAC-SHA256(secret, canonical_bytes) where `canonical_bytes` = protobuf `Batch` bytes + `created_at_ms` + `batch_id` ordering macro; include `kid` to handle rotation.
- Signature transmitted as base64 in `x-signature` metadata.

Key rotation

- Server tracks keys per agent avec `kid` et supporte previous keys during grace period.
- Agents poll `GetConfig` for rotation instructions.

Replay & anti‑tamper

- Server vérifie `created_at_ms` within accepted window et stocke recent `batch_id` pour dedup.

Secrets at rest

- Use OS keystore where possible: Windows DPAPI, macOS Keychain, Linux secret service or file encrypted avec AES‑GCM using TPM/key derivation.

Network security

- TLS required; recommend TLS 1.3 via rustls; optional mTLS.

---

## 14. Organisation du dépôt & crates Rust

Workspace layout (suggestion)

```
SentinelRS/
├─ Cargo.toml (workspace)
├─ crates/
│  ├─ common/ (proto generated + shared types)
│  ├─ agent/ (binary)
│  ├─ server/ (binary)
│  ├─ workers/ (binary or service)
│  ├─ plugins/ (wasm helpers + examples)
│  └─ cli/ (administration CLI)
├─ ui/ (React app)
└─ deploy/ (systemd, docker-compose, installers)
```

Conventions

- Use `clippy` strict linting; `rustfmt` enforced; GitHub Actions run on PRs.
- Protobuf definitions in `crates/common/proto/` with build.rs generating Rust types via `prost`.

---

## 15. Roadmap V1 (milestones détaillés)

Sprint 0 — initialisation (week 0)

- Create workspace, common proto stubs, CI skeleton.

Sprint 1 — Agent core & WAL (weeks 1–3)

- Implement agent collector pour CPU/mem/disk/net, WAL append, basic batch composer, gRPC client stub.
- Unit tests + integration local.

Sprint 2 — Server ingest + NATS (weeks 4–6)

- Implement server gRPC endpoints, validate HMAC, publish to NATS JetStream; store raw batch in Postgres.

Sprint 3 — Workers & storage (weeks 7–9)

- Worker to consume JetStream, write TimescaleDB rows, basic threshold alerting, notifier to webhook.

Sprint 4 — Plugins WASM + Prometheus scraping (weeks 10–12)

- WASM runtime integration, plugin examples (nginx stub_status, redis ping), Prometheus scraping support.

Sprint 5 — Packaging, CI, E2E tests (weeks 13–15)

- Build artifacts for Linux/Windows/macOS, docker images, E2E CI via docker-compose.

---

## 16. Critères d'acceptation V1

- Agent collects baseline metrics and sends batches to server successfully.
- WAL persists data across agent restart/network outage and server receives batches upon reconnection.
- Server publishes to NATS; worker persists metrics to TimescaleDB.
- Threshold rule triggers and a webhook notifier is received.
- HMAC signing rejects tampered batches and server returns Unauthenticated.
- Cross‑platform builds for Linux/Windows/macOS succeed in CI matrix.

---

## 17. Risques, mitigations et décisions ouvertes

Risques techniques notables

- Plugins dynamiques Rust natifs — fragile across toolchains ⇒ mitigation: WASM choisi.
- Cross compilation complexity ⇒ mitigation: CI matrix with `cross` ou docker containers.
- Exactly‑once delivery trop coûteux ⇒ design for at‑least‑once + idempotence.

Décisions ouvertes (à confirmer si besoin)

1. Politique de rétention par défaut (ex: 30 jours raw, 365 jours aggregates)
2. Taille/intervalle de batchs par défaut (ex: 10s / 64KB)
3. Politique de clé rotation automatique vs manuelle

---

## 18. Annexes

Annexe A — Exemple de `.proto` (extrait minimal)

```proto
syntax = "proto3";
package sentinel.common;

message Metric {
  string name = 1;
  map<string,string> labels = 2;
  oneof value {
    double d = 3;
    int64 i = 4;
  }
  int64 timestamp_ms = 5;
}

message Batch {
  string agent_id = 1;
  string batch_id = 2;
  uint64 seq_start = 3;
  uint64 seq_end = 4;
  int64 created_at_ms = 5;
  repeated Metric metrics = 6;
}

service AgentService {
  rpc Register(RegisterRequest) returns (RegisterResponse);
  rpc PushMetrics(Batch) returns (PushResponse);
}
```

Annexe B — Format WAL (exemple binaire concept)

- Segment file: sequence of records.
- Record: [u32 len][protobuf bytes][u32 crc32]
- On ack, mark record tombstoned; compaction removes tombstones.

Annexe C — Checklist de sécurité (agent)

- Secrets encrypted at rest
- TLS v1.3 enforced
- HMAC signature and timestamp window
- Recent `batch_id` dedup store

---

## Prochaine étape proposée

1. Valider ce cahier des charges.
2. Je génère alors les `.proto` complets, le skeleton Cargo workspace et le backlog sprint 1 (tâches techniques détaillées) — voulez‑vous que je le fasse maintenant ?

---

Fin du cahier des charges technique (fichier généré automatiquement).
