````md
# SentinelRS

SentinelRS is a lightweight, modular distributed monitoring system written in Rust.

## Goal

Collect, sign and stream metrics from edge agents to a central ingestion service, with a focus on reliability (append-only WAL), safe extensibility (WASM plugins), and scalable ingestion (NATS JetStream + workers).

## Repository layout

- `crates/common` — shared types, protobufs and helpers
- `crates/agent` — agent binary (collector, WAL, exporter)
- `crates/server` — ingestion API (gRPC/REST), validation, NATS publisher
- `crates/workers` — consumers, transformers and DB writers
- `crates/cli` — admin CLI for operational tasks

## Quickstart (developer)

1. Install Rust toolchain (stable) and `cargo`.
2. Build the workspace:

```bash
cargo build --workspace --release
```
````

3. Run unit tests:

```bash
cargo test --workspace
```

## Notes

- Design documents (`CAHIER_DES_CHARGES.md`, `ROADMAP.md`) are intentionally kept out of the repository and ignored by Git for privacy — they remain in your local workspace.
- CI skeleton is provided in `.github/workflows/ci.yml`.

## Contributing

Please open issues or PRs against `main`. For major changes, draft a design note in a local document and discuss on an issue first.

## License

MIT (see `LICENSE` if added)

```

```
