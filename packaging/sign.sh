#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
ARTIFACTS_GLOB="${1:-$DIST_DIR/**/*.{tar.gz,zip,deb,msi}}"

sign_gpg() {
    local file="$1"
    if [ -z "${GPG_KEY_ID:-}" ]; then
        echo "GPG_KEY_ID not set, skipping GPG signing for $file"
        return 0
    fi
    gpg --batch --yes --detach-sign --armor \
        --local-user "$GPG_KEY_ID" \
        "$file"
    echo "GPG signed: ${file}.asc"
}

generate_checksums() {
    local dir="$1"
    cd "$dir"
    find . -type f \( -name '*.tar.gz' -o -name '*.zip' -o -name '*.deb' -o -name '*.msi' \) \
        -exec sha256sum {} \; | sort > SHA256SUMS.txt
    echo "Checksums: $dir/SHA256SUMS.txt"
    sign_gpg "SHA256SUMS.txt"
}

sign_macos() {
    local app_path="$1"
    if ! command -v codesign &>/dev/null; then
        echo "codesign not available, skipping macOS signing"
        return 0
    fi
    if [ -z "${APPLE_IDENTITY:-}" ]; then
        echo "APPLE_IDENTITY not set, skipping macOS code signing"
        return 0
    fi
    codesign --force --deep --sign "$APPLE_IDENTITY" "$app_path"
    echo "macOS signed: $app_path"
}

sign_windows() {
    local exe_path="$1"
    if ! command -v signtool &>/dev/null; then
        echo "signtool not available, skipping Windows signing"
        return 0
    fi
    if [ -z "${WIN_CERT_PATH:-}" ]; then
        echo "WIN_CERT_PATH not set, skipping Windows code signing"
        return 0
    fi
    signtool sign /f "$WIN_CERT_PATH" /p "${WIN_CERT_PASSWORD:-}" /tr http://timestamp.digicert.com /td sha256 "$exe_path"
    echo "Windows signed: $exe_path"
}

if [ -d "$DIST_DIR/macos" ]; then
    for app in "$DIST_DIR"/macos/*.app; do
        [ -d "$app" ] && sign_macos "$app"
    done
fi

if [ -d "$DIST_DIR/wix" ]; then
    for exe in "$DIST_DIR"/wix/*.msi; do
        [ -f "$exe" ] && sign_windows "$exe"
    done
fi

generate_checksums "$DIST_DIR"

echo "Signing complete."
