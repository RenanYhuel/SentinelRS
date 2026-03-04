# Contributing to SentinelRS

Thank you for your interest in contributing to SentinelRS.

## Getting Started

1. Fork the repository: https://github.com/RenanYhuel/SentinelRS
2. Clone your fork
3. Set up the development environment (see [docs/development.md](docs/development.md))
4. Create a feature branch from `main`

## Development Workflow

```bash
git checkout -b feat/my-feature

cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

git commit -m "feat: description of the change"
git push origin feat/my-feature
```

## Pull Request Process

1. Ensure CI passes (format, clippy, tests, build)
2. Write a clear PR title and description
3. Link related issues if applicable
4. Keep PRs focused — one feature or fix per PR

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add disk I/O collector
fix: handle WAL segment rotation on Windows
docs: update CLI reference for key rotation
test: add alert evaluator edge cases
refactor: extract HMAC verification into middleware
```

## Code Style

- Run `cargo fmt --all` before committing
- All clippy warnings must be resolved (`-D warnings`)
- Keep files small and focused — prefer many small modules over large files
- Minimal comments — write self-documenting code with clear naming
- Do not import or copy legacy code from the `old/` directory

## Adding a New CLI Command

See [docs/development.md](docs/development.md#how-to-add-a-new-cli-command).

## Adding a New REST Endpoint

See [docs/development.md](docs/development.md#how-to-add-a-new-rest-endpoint).

## Adding a WASM Host Function

See [docs/development.md](docs/development.md#how-to-add-a-new-wasm-host-function) and [docs/plugin-development.md](docs/plugin-development.md).

## Documentation

When adding features, update the relevant doc:

| Feature          | Doc to update                                            |
| ---------------- | -------------------------------------------------------- |
| CLI command      | [docs/cli-reference.md](docs/cli-reference.md)           |
| REST endpoint    | [docs/api-reference.md](docs/api-reference.md)           |
| Config option    | [docs/configuration.md](docs/configuration.md)           |
| Notification     | [docs/notifications.md](docs/notifications.md)           |
| Protocol change  | [docs/streaming.md](docs/streaming.md)                   |
| Security feature | [docs/security.md](docs/security.md)                     |
| WASM plugin API  | [docs/plugin-development.md](docs/plugin-development.md) |

## Reporting Issues

- Use [GitHub Issues](https://github.com/RenanYhuel/SentinelRS/issues)
- Include: steps to reproduce, expected behavior, actual behavior
- For bugs: include OS, Rust version and relevant logs

## License

By contributing, you agree that your contributions will be licensed under the [Business Source License 1.1](LICENSE), which converts to Apache 2.0 on the Change Date.
