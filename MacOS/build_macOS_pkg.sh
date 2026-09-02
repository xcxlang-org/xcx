#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$SCRIPT_DIR/dist/xcx-installer-macos"

# Single source of truth for the package version.
XCX_VERSION="$(tr -d '[:space:]' < "$PROJECT_ROOT/lib/VERSION")"

echo "Building XCX macOS Distribution Package (v$XCX_VERSION)..."

rm -rf "$SCRIPT_DIR/dist"
mkdir -p "$DIST_DIR"

cp "$SCRIPT_DIR/install.sh" "$DIST_DIR/install.sh"
chmod +x "$DIST_DIR/install.sh"

if [ -f "$PROJECT_ROOT/target/release/xcx" ]; then
    cp "$PROJECT_ROOT/target/release/xcx" "$DIST_DIR/xcx"
else
    echo "Error: Could not find xcx binary! Run 'cargo build --release' first."
    exit 1
fi
chmod +x "$DIST_DIR/xcx"

# Remove Gatekeeper quarantine attribute from the binary at package-build time
xattr -d com.apple.quarantine "$DIST_DIR/xcx" 2>/dev/null || true

mkdir -p "$DIST_DIR/lib"
cp -r "$PROJECT_ROOT/lib/mathlib" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/lib/pax" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/lib/doc" "$DIST_DIR/lib/" 2>/dev/null || true
cp "$PROJECT_ROOT/lib/VERSION" "$DIST_DIR/lib/" 2>/dev/null || true

mkdir -p "$DIST_DIR/resources"
cp "$PROJECT_ROOT/Windows/resources/LICENSE.txt" "$DIST_DIR/resources/" 2>/dev/null || true
cp "$PROJECT_ROOT/MacOS/README.txt" "$DIST_DIR/resources/" 2>/dev/null || true
# Note: no icons folder - not used on macOS (no MIME/UTI registration)

echo "Creating tarball..."
cd "$SCRIPT_DIR/dist"
tar -czf "xcx-installer-macos-v$XCX_VERSION.tar.gz" "xcx-installer-macos"

echo "Package successfully built at: $SCRIPT_DIR/dist/xcx-installer-macos-v$XCX_VERSION.tar.gz"
