#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERSION="${1:-0.1.0}"
APP_NAME="SentinelRS"
BUNDLE_DIR="$ROOT_DIR/dist/macos/$APP_NAME.app"
BINARIES=(sentinel_agent sentinel_server sentinel_workers sentinel_cli)

rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

cat > "$BUNDLE_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.sentinelrs.app</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleExecutable</key>
    <string>sentinel_cli</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
PLIST

UNIVERSAL_DIR="$ROOT_DIR/target/universal-apple-darwin/release"

if [ -d "$UNIVERSAL_DIR" ]; then
    for bin in "${BINARIES[@]}"; do
        cp "$UNIVERSAL_DIR/$bin" "$BUNDLE_DIR/Contents/MacOS/"
    done
else
    X86_DIR="$ROOT_DIR/target/x86_64-apple-darwin/release"
    ARM_DIR="$ROOT_DIR/target/aarch64-apple-darwin/release"

    if [ -d "$X86_DIR" ] && [ -d "$ARM_DIR" ]; then
        for bin in "${BINARIES[@]}"; do
            lipo -create "$X86_DIR/$bin" "$ARM_DIR/$bin" \
                -output "$BUNDLE_DIR/Contents/MacOS/$bin"
        done
    elif [ -d "$ARM_DIR" ]; then
        for bin in "${BINARIES[@]}"; do
            cp "$ARM_DIR/$bin" "$BUNDLE_DIR/Contents/MacOS/"
        done
    elif [ -d "$X86_DIR" ]; then
        for bin in "${BINARIES[@]}"; do
            cp "$X86_DIR/$bin" "$BUNDLE_DIR/Contents/MacOS/"
        done
    else
        echo "No macOS release binaries found. Build first."
        exit 1
    fi
fi

cp "$ROOT_DIR/config.example.yml" "$BUNDLE_DIR/Contents/Resources/"

TARBALL="$ROOT_DIR/dist/macos/sentinel-macos-${VERSION}.tar.gz"
cd "$ROOT_DIR/dist/macos"
tar czf "$TARBALL" "$APP_NAME.app"

echo "macOS bundle: $BUNDLE_DIR"
echo "Archive: $TARBALL"
