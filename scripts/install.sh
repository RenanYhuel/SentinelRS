#!/usr/bin/env bash
set -euo pipefail

REPO="sentinelrs/sentinelrs"
BINARY="sentinel_cli"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
INSTALL_NAME="sentinel"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[info]${NC}  $*"; }
ok()    { echo -e "${GREEN}[ok]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[warn]${NC}  $*"; }
error() { echo -e "${RED}[error]${NC} $*" >&2; exit 1; }

detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) error "Unsupported OS: $os" ;;
    esac
}

detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)  echo "amd64" ;;
        aarch64|arm64) echo "arm64" ;;
        *) error "Unsupported architecture: $arch" ;;
    esac
}

get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/'
    else
        error "curl or wget required"
    fi
}

download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -fsSL -o "$dest" "$url"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    fi
}

main() {
    local version="${VERSION:-}"
    local os arch asset_name download_url

    os="$(detect_os)"
    arch="$(detect_arch)"

    if [ -z "$version" ]; then
        info "Fetching latest version..."
        version="$(get_latest_version)"
    fi

    if [ -z "$version" ]; then
        error "Could not determine latest version"
    fi

    info "Installing SentinelRS CLI ${version} for ${os}/${arch}"

    case "$os" in
        linux)
            asset_name="sentinel-linux-${arch}.tar.gz"
            ;;
        macos)
            asset_name="sentinel-macos-universal.tar.gz"
            ;;
        windows)
            asset_name="sentinel-windows-amd64.zip"
            ;;
    esac

    download_url="https://github.com/${REPO}/releases/download/${version}/${asset_name}"

    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    info "Downloading ${download_url}..."
    download "$download_url" "${tmpdir}/${asset_name}"

    info "Downloading checksums..."
    download "https://github.com/${REPO}/releases/download/${version}/SHA256SUMS.txt" "${tmpdir}/SHA256SUMS.txt"

    info "Verifying checksum..."
    (cd "$tmpdir" && grep "$asset_name" SHA256SUMS.txt | sha256sum -c --quiet 2>/dev/null) || warn "Checksum verification skipped"

    info "Extracting..."
    case "$asset_name" in
        *.tar.gz)
            tar -xzf "${tmpdir}/${asset_name}" -C "$tmpdir"
            ;;
        *.zip)
            unzip -qo "${tmpdir}/${asset_name}" -d "$tmpdir"
            ;;
    esac

    if [ ! -w "$INSTALL_DIR" ]; then
        warn "${INSTALL_DIR} requires elevated privileges"
        sudo mkdir -p "$INSTALL_DIR"
        sudo cp "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${INSTALL_NAME}"
        sudo chmod +x "${INSTALL_DIR}/${INSTALL_NAME}"
    else
        mkdir -p "$INSTALL_DIR"
        cp "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${INSTALL_NAME}"
        chmod +x "${INSTALL_DIR}/${INSTALL_NAME}"
    fi

    ok "SentinelRS CLI installed to ${INSTALL_DIR}/${INSTALL_NAME}"

    if command -v "$INSTALL_NAME" &>/dev/null; then
        ok "Version: $("$INSTALL_NAME" --version 2>/dev/null || echo "$version")"
    else
        warn "Add ${INSTALL_DIR} to your PATH"
        echo ""
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
    fi

    echo ""
    info "Quick start:"
    echo "  sentinel init     — Initialize a new project"
    echo "  sentinel up       — Start the stack"
    echo "  sentinel status   — Check service status"
}

main "$@"
