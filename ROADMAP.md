# Roadmap technique détaillée — SentinelRS (checklist)

But : fournir un découpage extrêmement précis et exploitable pour réaliser l'ensemble des fonctionnalités de la V1 décrites dans le cahier des charges. Le document liste tâches atomiques, dépendances, critères d'acceptation, et notes d'implémentation pour chaque zone fonctionnelle.

---

Règles générales de la roadmap

- Chaque item est indépendant en termes de définition, mais les dépendances sont listées explicitement.
- Pas d'estimation temporelle ici — prioriser par dépendances et par valeur minimale viable.
- Les tâches doivent produire : code compilable, tests unitaires élémentaires, documentations minimales (README ou docstring) et CI green.

---

Index (sections)

- [x]   1. Repo & workspace
- [x]   2. Protobuf & common types
- [x]   3. Agent — architecture interne (sous-tâches)
- [x]   4. Plugins WASM
- [x]   5. WAL (buffer local) et politiques de compaction
- [x]   6. Exporter gRPC & Prometheus
- [x]   7. Server (API gateway / ingestion)
- [x]   8. Broker integration (NATS JetStream)
- [ ]   9. Workers (consommation / transformation / stockage)
- [ ]   10. TimescaleDB schema & storage
- [ ]   11. Alert engine & notifiers
- [x]   12. Admin CLI
- [ ]   13. Tests & E2E
- [ ]   14. CI / Releases / Packaging
- [ ]   15. Sécurité et provisioning
- [ ]   16. Observabilité & monitoring interne
- [ ]   17. Backlog initial (tickets détaillés)

---

1. Repo & workspace

- Objectif : structure monorepo Rust propre, scripts utilitaires et fichiers examples.
- Livrables : top-level `Cargo.toml` (workspace), `.gitignore`, `README.md`, `config.example.yml`, dossier `deploy/` avec manifestes.
- Sous-tâches :
    - [x] 1.1. Vérifier et valider `Cargo.toml` workspace (member listing). Critère : `cargo metadata` passe.
    - [x] 1.2. Ajouter `Makefile` ou `xtasks` pour builds courants (`make build-all`, `make fmt`, `make proto`). Critère : commandes exécutables.
    - [x] 1.3. Créer `deploy/docker-compose.yml` minimal avec NATS+Postgres+Timescale pour dev E2E. Critère : `docker-compose up` lance les services.
- Dépendances : aucune.

---

2. Protobuf & common types (`crates/common`)

- Objectif : définir `.proto` canoniques, codegen Rust (prost), helpers communs.
- Sous-tâches :
    - [x] 2.1. Finaliser `proto/sentinel.proto` (metrics, batch, register, heartbeat, config messages). Critère : `protoc`/`prost_build` génère Rust sans warnings.
    - [x] 2.2. Ajouter `build.rs` dans `crates/common` pour générer prost bindings automatiquement à la compilation. Critère : `cargo build` génère `common::proto` module.
    - [x] 2.3. Implémenter helpers Rust dans `crates/common/src` : canonicalize function pour Batch (deterministic bytes), batch_id generator, seq helper, Metric <-> JSON helpers. Critère : unit tests de roundtrip.
    - [x] 2.4. Ajouter docs (README dans `crates/common`) expliquant versioning des proto (compat rules).
- Dépendances : 1.1, 1.2 (outil de build disponible).

Notes d'implémentation

- Prost avec `bytes = true`, `out_dir` dans `build.rs`.
- Canonicalization : trier explicitement les maps (labels) avant sérialisation pour signature.

---

3. Agent — architecture interne (décomposer)

But : avoir un agent fonctionnel minimal qui collecte métriques système, écrit WAL, compose batchs et pousse au server (stub acceptable au début).

3.A Collector core

- [x] 3.A.1. Implémenter module `collector::system` : fonctions pour lire CPU per-core, mem, disk, net, uptime, process count.
    - Inputs : `collect_interval`.
    - Output : Vec<Metric> normales.
    - Tests : unit test mocking values (où possible) et integration smoke on Linux.
- [x] 3.A.2. Normalisation des noms de métriques et labels (naming conventions). Documenter.
- [x] 3.A.3. Abstraction `Collector` trait : `fn collect(&self) -> Vec<Metric>` pour permettre plugins et tests.
- Dépendances : 2.1 (types Metric/Batches).

    3.B Scheduler

- [x] 3.B.1. Implémenter scheduler simple (tokio):: spawn recurring tasks using `tokio::time::interval`.
- [x] 3.B.2. Support per-check intervals and jitter config.
- [x] Tests : simulate interval firing and ensure Collector invoked.

    3.C Buffer (WAL) API (interface only)

- [x] 3.C.1. Exposer API `Wal::append(bytes) -> record_id`, `Wal::pop_ack(record_id)`, `Wal::iter_unacked()`.
- [x] 3.C.2. WAL must be crash-safe: append then fsync policy configurable.
- [x] Tests : append/read roundtrip using tempdir.
- Dépendances : 3.A, 3.B.

    3.D Batch composer

- [x] 3.D.1. Compose batch from collected metrics and WAL records ordering.
- [x] 3.D.2. Assign `batch_id`, `seq_start/seq_end` using monotonic counters persisted in metadata file.
- [x] Tests : batch generation reproducible.

    3.E Signer & Compression

- [x] 3.E.1. Implement HMAC signer util using `hmac + sha2`. API: `sign_batch(secret, bytes) -> signature`.
- [x] 3.E.2. Compression wrapper: gzip compress when threshold.
- [x] Tests : verify signatures and compressed flag.

    3.F Exporter gRPC client (tonic)

- [x] 3.F.1. Stub client to call `AgentService::PushMetrics` using prost generated types. Metadata injection for `x-agent-id`, `x-signature`.
- [x] 3.F.2. Implement send loop: take next batch, send, handle response codes (OK/RETRY/REJECTED).
- [x] Tests : use mock server to verify metadata present and backoff logic.
- Dépendances : 2.1, 3.C, 3.D, 3.E.

    3.G Local HTTP (Prometheus exporter + health)

- [x] 3.G.1. Run tiny HTTP server exposing `/metrics`, `/healthz`, `/ready` using `axum` or `hyper`.
- [x] 3.G.2. Map internal agent metrics (queue length, wal size, last_send) to Prometheus.
- [x] Tests : HTTP smoke test.

    3.H Config & secure store

- [x] 3.H.1. Load `config.yml` via `serde_yaml`, validate schema.
- [x] 3.H.2. Implement secure storage of `secret` via OS keystore abstraction with fallback AES-GCM file encryption.
- [x] Tests : config parsing + simple key store mock.

    3.I Integration test (agent minimal)

- [x] 3.I.1. Run agent against local stub server (server stub that accepts PushMetrics and returns ACK). Validate WAL→send→ack→compaction.
- [x] Acceptance : agent sends a batch and server receives valid batch with correct metadata.

---

4. Plugins WASM (wasmtime)

- [x] 4.1. Define plugin manifest format (YAML/JSON) fields: name, version, entry_fn, capabilities, resource_limits.
- [x] 4.2. Implement plugin installer: download signed WASM blob, verify signature using server public key, store in `plugins_dir`.
- [x] 4.3. Implement runtime loader using `wasmtime` API: instantiate module, provide host functions (http_get, read_file_limited, metric_builder API).
- [x] 4.4. Resource constraints: set memory limit, CPU timeouts (wall clock), catch traps and report failure without crashing agent.
- [x] 4.5. Define plugin API contract (Rust interface documented): call returns list of Metrics (serialized as JSON or via a shared memory buffer), include error handling.
- [x] Tests : run sample plugins (nginx stub_status parser) in unit/integration.
- [x] Dépendances : 3.A, 3.B, 3.G.

Notes : Prefer passer un contexte minimal au plugin pour réduire la surface d'attaque.

---

5. WAL (append-only) détaillé

- [x] 5.1. Implement append-only writer: write then fsync optionally.
- [x] 5.2. Implement index file `wal.meta.json` storing head offset, tail offsets, last_seq, unacked_count.
- [x] 5.3. Implement ack marking: append tombstone entry or update index; plan compaction to rewrite only unacked records.
- [x] 5.4. Implement compaction job: schedule when free space < threshold or on trigger; compaction must be atomic (write new files then fsync and swap).
- [x] 5.5. Expose WAL metrics (size, unacked count) to Prometheus.
- [x] Tests : crash simulation (kill process after append without ack) and recovery ensures no data loss and ordering preserved.

---

6. Exporter gRPC & Prometheus (Agent ↔ Server)

- [x] 6.1. Implement tonic client based on `crates/common` proto.
- [x] 6.2. Add metadata injection and interceptors for signature header.
- [x] 6.3. Implement retry logic with exponential backoff and jitter; config options for max attempts or unlimited with local WAL retention.
- [x] 6.4. Add HTTP fallback (POST /v1/agent/metrics) for environments where gRPC is blocked.
- [x] Tests : integration with server stub + NATS stub (or test double).

---

7. Server (API gateway / ingestion)

- [x] 7.1. Bootstrap server skeleton (tonic gRPC + axum for REST on same process or separate gateway). Include TLS via rustls.
- [x] 7.2. Implement `Register` RPC: validate hw_id, generate `agent_id` + secret, store in `agents` table. Return config snapshot optionally.
- [x] 7.3. Implement `PushMetrics` RPC handler (validation, idempotency, publish to NATS, respond OK/RETRY/REJECTED).
- [x] 7.4. Expose REST endpoints for dashboard: agents list, agent detail, metrics query proxy, alerts list, raw batch inspect.
- [x] 7.5. Add rate limiting middleware and auth for UI/API (API tokens / JWT admin key).
- [x] Tests : unit tests for validation logic, integration tests with NATS dev.
- [x] Dépendances : 2.1 (proto), 8 (NATS integration), 10 (DB for agent persistence).

Notes d'implémentation : utiliser Tower middleware, stocker idempotency cache en Redis/Postgres.

---

8. Broker integration (NATS JetStream)

- [x] 8.1. Define stream `sentinel.metrics` configuration: retention, max_bytes, subjects.
- [x] 8.2. Implement publisher in server: create JetStream context, publish with headers (agent_id,batch_id,received_at).
- [x] 8.3. Implement durable consumer groups for workers; plan subject partitioning by agent prefix if scaling needed.
- [x] Tests : local NATS dev compose, publish and check message stored and retrievable.

Operational notes : Start with single stream; expose scripts `deploy/nats-setup.sh` to create stream & consumers.

---

9. Workers (ingestion → storage → alerting)

9.A Consumer & message handling

- [x] 9.A.1. Implement JetStream consumer (pull or push) with explicit ack.
- [x] 9.A.2. On message receipt: parse envelope, verify signature again (defense-in-depth), deserialize Batch.
- [x] 9.A.3. Dedup check: consult `batch_id` cache/table; if processed -> ack and skip.
- [x] Tests : unit parse + dedup.

    9.B Transformer

- [x] 9.B.1. Transform each Metric into DB row(s) for Timescale table schema.
- [x] 9.B.2. Normalize labels (store as JSONB) and flatten histograms if present.
- [x] Tests : sample batch -> expected rows generation.

    9.C Storage writer

- [x] 9.C.1. Use `sqlx` (async) with connection pool to TimescaleDB.
- [x] 9.C.2. Implement batched inserts with COPY-like performance; fallback to INSERT with transaction.
- [x] 9.C.3. Implement retry logic on DB transient errors with backoff.
- [x] Tests : integration tests with Timescale dev container.

    9.D Aggregator & rollups

- [x] 9.D.1. Implement aggregator module to compute rolling aggregates used by alert engine (or compute via DB views if preferred).
- [x] 9.D.2. Provide API for rules engine to fetch aggregated values efficiently.

    9.E Alert engine

- [x] 9.E.1. Evaluate rules per agent/metric as batches are processed or as periodic job triggered by worker.
- [x] 9.E.2. Implement rule state machine (ok -> firing -> resolved), fingerprinting to dedup.
- [x] 9.E.3. Persist alert events in `alerts` table and push to notifier queue.
- [x] Tests : unit rules evaluation with synthetic metrics inputs.

    9.F Notifier executor

- [x] 9.F.1. Implement pluggable notifiers (webhook, slack, discord, smtp). Each notifier must accept an alert payload and handle retries.
- [x] 9.F.2. Sign webhook payload with HMAC secret for target verification.
- [x] Tests : mocked HTTP endpoints and smtp server.

    9.G Observability & metrics

- [x] 9.G.1. Worker exposes Prometheus metrics: processing latency, db latency, errors, ack rates.

---

10. TimescaleDB schema & storage

- [x] 10.1. Define hypertables for `metrics_time` and create necessary indexes.
- [x] 10.2. Create `metrics_raw` JSONB table for raw batches with retention policy.
- [x] 10.3. Implement migrations (diesel/sqlx migration scripts). Put migrations in `migrations/` folder.
- [x] 10.4. Add views/materialized views for dashboard queries (top metrics, recent values).
- [x] Tests : run migrations and execute sample inserts + query correctness.

Notes : Use TimescaleDB native functions for downsampling & continuous aggregates where appropriate.

---

11. Alert engine & notifiers (détails)

- [x] 11.1. Define rule storage format (JSON DSL) and CRUD via server REST.
- [x] 11.2. Implement test harness for rules (simulate metric streams and verify expected alerts).
- [x] 11.3. Implement notifier retry policy and dead-letter handling (store failed notifications to `notifications_dlq`).
- [x] 11.4. UI hooks: provide test/send endpoint for each notifier to validate credentials.

---

12. Admin CLI (`crates/cli`)

- [x] 12.1. Subcommands to register agent (call server Register), show config, rotate key, inspect WAL (list unacked batches), force send, tail logs (if local file).
- [x] 12.2. Implement JSON output flag for programmatic use.
- [x] Tests : integration call to local server stub.

---

13. Tests & E2E

- [x] 13.1. Unit tests per crate (run `cargo test`). Each crate must have >= minimal coverage on core logic.
- [x] 13.2. Integration tests in `tests/` which spin up lightweight NATS + Postgres containers (via docker compose) and run real flows: agent stub sends batch -> server -> nats -> worker consumes -> db insert -> alert triggered.
- [x] 13.3. E2E scenario scripts in `tests/e2e/` automating these flows and verifying acceptance criteria.
- [x] 13.4. Add flakiness detection (retry until success or fail) to reduce CI brittleness.

---

14. CI / Releases / Packaging

- [ ] 14.1. GitHub Actions pipeline skeleton: fmt -> clippy -> unit tests -> build artifacts for linux/windows/macos -> run integration smoke (docker-compose).
- [ ] 14.2. Add release job building static linux binary (musl), Windows MSVC, macOS universal; produce GitHub Release artifacts.
- [ ] 14.3. Packaging scripts: `cargo-deb` config, WiX templates, macOS bundle script.
- [ ] 14.4. Sign artifacts where possible (GPG, code signing placeholders).

---

15. Sécurité et provisioning

- [ ] 15.1. Implement server provisioning flow: Register RPC stores `agent_id` + `secret` in `agents` table with metadata (kid versioning support).
- [ ] 15.2. Key rotation endpoints and server logic to accept old keys during grace period.
- [ ] 15.3. Secrets at rest: integrate OS keystore on agent; fallback AES-GCM file.
- [ ] 15.4. TLS management: load certs via config; provide script to generate self-signed certs for dev.
- [ ] Security acceptance tests: tamper signature -> server rejects; replay old batch -> server rejects if outside window; rotated key validation.

---

16. Observabilité & monitoring interne

- [ ] 16.1. Export Prometheus metrics on all components (agent, server, workers). Define metric names and labels convention.
- [ ] 16.2. Implement health checks `/healthz` & `/ready` for each binary.
- [ ] 16.3. Add structured logs (JSON) and log correlation fields (`agent_id`, `batch_id`, `trace_id`).

---

17. Backlog initial (tickets atomiques et priorités logiques)

- [ ] 17.1. `common/proto: finalize sentinel.proto` (includes Metric, Batch, Register, Heartbeat, Config) — depends: none.
- [x] 17.2. `common/build.rs: prost codegen` — depends: 17.1. (implémenté)
- [ ] 17.3. `agent/collector: implement sys metrics (CPU,Mem,Disk,Net)` — depends: 17.1.
- [ ] 17.4. `agent/wal: append/read/ack API` — depends: 17.3.
- [ ] 17.5. `agent/batch: composer + signer` — depends: 17.4, 17.2.
- [ ] 17.6. `server/grpc: PushMetrics handler + validation` — depends: 17.2.
- [ ] 17.7. `server/nats: publish to JetStream` — depends: 17.6.
- [ ] 17.8. `workers/consumer: pull from JetStream and transform` — depends: 17.7.
- [ ] 17.9. `workers/db: insert into timescale` — depends: 17.8, 10.1.
- [ ] 17.10. `agent/prometheus: /metrics` — depends: 17.3.
- [ ] 17.11. `plugins/wasm: runtime and sample plugin` — depends: 17.3, 17.2.
- [ ] 17.12. `ci: github actions skeleton` — depends: repository structure 1.x.
- [x] 17.13. `e2e: docker-compose for NATS+Postgres` — depends: 1.3. (deploy/docker-compose ajouté)

---

Livrables finaux attendus

- [ ] Code compilable via `cargo build --workspace`.
- [ ] Protos générés et utilisés par agent/server/workers.
- [ ] Agent capable d'envoyer un batch vers server stub, server publie sur NATS, worker consomme et écrit en DB (end-to-end minimal).
- [ ] Tests automatisés pour les flows critiques et CI pipeline green.

---

Notes d'implémentation transverses

- Logging : utilitaire `common::logging` pour config (json format) partagé.
- Feature flags : `wasm-plugins` cargo feature default off for faster dev builds.
- Platform specifics : agent compilation doit supporter linux/windows/macos; mais développer sur Linux container first.
- Safety : design for at-least-once delivery et idempotence via `batch_id` server-side dedup.

---
