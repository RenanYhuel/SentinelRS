#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERSION="${1:-0.1.0}"
DIST_DIR="$ROOT_DIR/dist/deb"

mkdir -p "$DIST_DIR"

if ! command -v cargo-deb &>/dev/null; then
    cargo install cargo-deb
fi

cargo build --release --target x86_64-unknown-linux-musl

cd "$ROOT_DIR"
cargo deb \
    --no-build \
    --target x86_64-unknown-linux-musl \
    --deb-version "$VERSION" \
    --output "$DIST_DIR/sentinel_${VERSION}_amd64.deb" \
    -- --manifest-path packaging/deb/cargo-deb.toml

echo "Debian package: $DIST_DIR/sentinel_${VERSION}_amd64.deb"
