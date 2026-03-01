# Contributing to SentinelRS

Thank you for your interest in contributing to SentinelRS.

## Getting Started

1. Fork the repository
2. Clone your fork
3. Set up the development environment (see [docs/development.md](docs/development.md))
4. Create a feature branch from `main`

## Development Workflow

```bash
# Create a branch
git checkout -b feat/my-feature

# Make changes, then verify
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Commit and push
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

1. Create a new file in `crates/cli/src/cmd/`
2. Define the args struct and `execute` function
3. Register the command in `crates/cli/src/cmd/mod.rs`
4. Add tests in `crates/cli/src/tests/`

## Adding a New REST Endpoint

1. Create a handler in the appropriate module under `crates/server/src/rest/`
2. Register the route in `crates/server/src/rest/router.rs`
3. Add integration tests in `crates/server/tests/`

## Adding a WASM Host Function

1. Add the function in `crates/agent/src/plugin/runtime.rs`
2. Document the function signature in `docs/security.md`
3. Update the `capabilities` list in the manifest schema

## Reporting Issues

- Use GitHub Issues
- Include: steps to reproduce, expected behavior, actual behavior
- For bugs: include OS, Rust version and relevant logs

## License

By contributing, you agree that your contributions will be licensed under the [Business Source License 1.1](LICENSE), which converts to Apache 2.0 on the Change Date.
