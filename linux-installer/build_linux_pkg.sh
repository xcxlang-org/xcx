
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$SCRIPT_DIR/dist/xcx-installer-linux"

echo "Building XCX Linux Distribution Package..."

rm -rf "$SCRIPT_DIR/dist"
mkdir -p "$DIST_DIR"

cp "$SCRIPT_DIR/install.sh" "$DIST_DIR/"
chmod +x "$DIST_DIR/install.sh"

if [ -f "$PROJECT_ROOT/target/release/xcx-compiler" ]; then
    cp "$PROJECT_ROOT/target/release/xcx-compiler" "$DIST_DIR/xcx"
elif [ -f "$SCRIPT_DIR/xcx" ]; then
    cp "$SCRIPT_DIR/xcx" "$DIST_DIR/"
elif [ -f "$PROJECT_ROOT/linux_setup/xcx" ]; then
    cp "$PROJECT_ROOT/linux_setup/xcx" "$DIST_DIR/"
else
    echo "Error: Could not find xcx binary!"
    exit 1
fi
chmod +x "$DIST_DIR/xcx"

mkdir -p "$DIST_DIR/lib"
cp -r "$PROJECT_ROOT/xcx-installer-pkg/lib/mathlib" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/xcx-installer-pkg/lib/pax" "$DIST_DIR/lib/" 2>/dev/null || true
cp -r "$PROJECT_ROOT/lib/doc" "$DIST_DIR/lib/"
cp "$PROJECT_ROOT/lib/VERSION" "$DIST_DIR/lib/" 2>/dev/null || cp "$PROJECT_ROOT/xcx-installer-pkg/lib/VERSION" "$DIST_DIR/lib/" || true

mkdir -p "$DIST_DIR/resources"
cp -r "$PROJECT_ROOT/xcx-installer-pkg/resources/icons" "$DIST_DIR/resources/"
cp "$PROJECT_ROOT/xcx-installer-pkg/resources/LICENSE.txt" "$DIST_DIR/resources/"
cp "$PROJECT_ROOT/xcx-installer-pkg/resources/README.txt" "$DIST_DIR/resources/"

echo "Creating tarball..."
cd "$SCRIPT_DIR/dist"
tar -czvf "xcx-installer-linux-v4.1.tar.gz" "xcx-installer-linux"

echo "Package successfully built at: $SCRIPT_DIR/dist/xcx-installer-linux-v4.1.tar.gz"
