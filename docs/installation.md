# Installation

## CLI Binary

### One-liner (Linux / macOS)

```bash
curl -sSL https://raw.githubusercontent.com/RenanYhuel/SentinelRS/main/scripts/install-sentinel.sh | sh
```

The script detects your OS and architecture, downloads the latest release from GitHub, and places the `sentinel` binary in `/usr/local/bin`.

### Homebrew (macOS / Linux)

```bash
brew tap RenanYhuel/sentinel
brew install sentinel
```

### Cargo (from source)

```bash
cargo install --git https://github.com/RenanYhuel/SentinelRS.git sentinel_cli
```

Or from a local clone:

```bash
git clone https://github.com/RenanYhuel/SentinelRS.git
cd SentinelRS
cargo build --release -p sentinel_cli
# Binary at target/release/sentinel_cli
```

### Windows (MSI)

Download the `.msi` installer from the [Releases](https://github.com/RenanYhuel/SentinelRS/releases) page. The installer adds `sentinel` to your PATH.

### Debian / Ubuntu (.deb)

```bash
curl -LO https://github.com/RenanYhuel/SentinelRS/releases/latest/download/sentinel_amd64.deb
sudo dpkg -i sentinel_amd64.deb
```

### Verify installation

```bash
sentinel version
```

```
sentinel 0.1.0 (rustc 1.XX.X)
```

## Shell Completions

Generate completions for your shell:

```bash
# Bash
sentinel completions bash > ~/.local/share/bash-completion/completions/sentinel

# Zsh
sentinel completions zsh > ~/.zfunc/_sentinel

# Fish
sentinel completions fish > ~/.config/fish/completions/sentinel.fish

# PowerShell
sentinel completions powershell > $PROFILE.CurrentUserAllHosts
```

Restart your shell or source the file.

## Server, Worker, and Agent

These binaries are typically deployed via Docker (see [Deployment](deployment.md)). For bare-metal:

```bash
cargo build --release -p sentinel_server
cargo build --release -p sentinel_workers
cargo build --release -p sentinel_agent
```

Binaries are placed in `target/release/`:

| Binary             | Description                    |
| ------------------ | ------------------------------ |
| `sentinel_server`  | gRPC + REST API server         |
| `sentinel_workers` | Background worker (alerts, DB) |
| `sentinel_agent`   | Metrics collector agent        |
| `sentinel_cli`     | CLI tool                       |

## Docker Images

Build all images:

```bash
docker compose -f deploy/docker-compose.yml build
```

Or individual images:

```bash
docker build -f docker/Dockerfile.server -t sentinel-server .
docker build -f docker/Dockerfile.worker -t sentinel-worker .
docker build -f docker/Dockerfile.agent -t sentinel-agent .
docker build -f docker/Dockerfile.cli -t sentinel-cli .
```

Images use multi-stage builds with `cargo-chef` for layer caching. Final images are based on `debian:bookworm-slim`.

## System Requirements

| Component   | Minimum           | Recommended       |
| ----------- | ----------------- | ----------------- |
| Server      | 1 CPU, 512 MB RAM | 2 CPU, 1 GB RAM   |
| Worker      | 1 CPU, 256 MB RAM | 2 CPU, 512 MB RAM |
| Agent       | < 1% CPU, 20 MB   | —                 |
| TimescaleDB | 1 CPU, 1 GB RAM   | 4 CPU, 4 GB RAM   |
| NATS        | 1 CPU, 128 MB RAM | 2 CPU, 512 MB RAM |
