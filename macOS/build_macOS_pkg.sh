#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$SCRIPT_DIR/dist/xcx-installer-macos"

echo "Building XCX macOS Distribution Package..."

rm -rf "$SCRIPT_DIR/dist"
mkdir -p "$DIST_DIR"

cp "$SCRIPT_DIR/install.sh" "$DIST_DIR/install.sh"
chmod +x "$DIST_DIR/install.sh"

if [ -f "$PROJECT_ROOT/target/release/xcx" ]; then
    cp "$PROJECT_ROOT/target/release/xcx" "$DIST_DIR/xcx"
elif [ -f "$SCRIPT_DIR/xcx" ]; then
    cp "$SCRIPT_DIR/xcx" "$DIST_DIR/"
elif [ -f "$PROJECT_ROOT/macos_setup/xcx" ]; then
    cp "$PROJECT_ROOT/macos_setup/xcx" "$DIST_DIR/"
else
    echo "Error: Could not find xcx binary!"
    exit 1
fi
chmod +x "$DIST_DIR/xcx"

# Remove Gatekeeper quarantine attribute from the binary at package-build time
xattr -d com.apple.quarantine "$DIST_DIR/xcx" 2>/dev/null || true

mkdir -p "$DIST_DIR/lib"
cp -r "$PROJECT_ROOT/xcx-installer-pkg/lib/mathlib" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/xcx-installer-pkg/lib/pax" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/lib/doc" "$DIST_DIR/lib/" 2>/dev/null || true
cp "$PROJECT_ROOT/lib/VERSION" "$DIST_DIR/lib/" 2>/dev/null || cp "$PROJECT_ROOT/xcx-installer-pkg/lib/VERSION" "$DIST_DIR/lib/" 2>/dev/null || true

mkdir -p "$DIST_DIR/resources"
cp "$PROJECT_ROOT/xcx-installer-pkg/resources/LICENSE.txt" "$DIST_DIR/resources/" 2>/dev/null || true
cp "$PROJECT_ROOT/xcx-installer-pkg/resources/README.txt" "$DIST_DIR/resources/" 2>/dev/null || true
# Note: no icons folder - not used on macOS (no MIME/UTI registration)

echo "Creating tarball..."
cd "$SCRIPT_DIR/dist"
tar -czvf "xcx-installer-macos-v4.1.tar.gz" "xcx-installer-macos"

echo "Package successfully built at: $SCRIPT_DIR/dist/xcx-installer-macos-v4.1.tar.gz"