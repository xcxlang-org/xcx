#!/bin/sh
# FreeBSD is experimental: this script must stay plain POSIX sh because the
# CI VM installs only the rust package (no bash) before running it.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$SCRIPT_DIR/dist/xcx-installer-freebsd"

# Single source of truth for the package version.
XCX_VERSION="$(tr -d '[:space:]' < "$PROJECT_ROOT/lib/VERSION")"

echo "Building XCX FreeBSD Distribution Package (v$XCX_VERSION)..."

rm -rf "$SCRIPT_DIR/dist"
mkdir -p "$DIST_DIR"

cp "$SCRIPT_DIR/install.sh" "$DIST_DIR/"
chmod +x "$DIST_DIR/install.sh"

if [ -f "$PROJECT_ROOT/target/release/xcx" ]; then
    cp "$PROJECT_ROOT/target/release/xcx" "$DIST_DIR/xcx"
else
    echo "Error: Could not find xcx binary! Run 'cargo build --release' first."
    exit 1
fi
chmod +x "$DIST_DIR/xcx"

mkdir -p "$DIST_DIR/lib"
cp -r "$PROJECT_ROOT/lib/mathlib" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/lib/pax" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/lib/doc" "$DIST_DIR/lib/" 2>/dev/null || true
cp "$PROJECT_ROOT/lib/VERSION" "$DIST_DIR/lib/" 2>/dev/null || true

mkdir -p "$DIST_DIR/resources"
cp "$PROJECT_ROOT/Windows/resources/LICENSE.txt" "$DIST_DIR/resources/" 2>/dev/null || true
cp "$PROJECT_ROOT/FreeBSD/README.txt" "$DIST_DIR/resources/" 2>/dev/null || true

echo "Creating tarball..."
cd "$SCRIPT_DIR/dist"
tar -czf "xcx-installer-freebsd-v$XCX_VERSION.tar.gz" "xcx-installer-freebsd"

echo "Package successfully built at: $SCRIPT_DIR/dist/xcx-installer-freebsd-v$XCX_VERSION.tar.gz"
