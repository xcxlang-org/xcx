#!/bin/sh
# XCX Compiler Ecosystem installer for FreeBSD (experimental).
# Plain POSIX sh: FreeBSD does not ship whiptail or bash in the base system.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

INSTALL_DIR="$HOME/.local/share/xcx"
INSTALL_BIN_DIR="$INSTALL_DIR/bin"
INSTALL_LIB_DIR="$INSTALL_DIR/lib"
USER_BIN_DIR="$HOME/.local/bin"

XCX_VERSION="$(tr -d '[:space:]' < "$SCRIPT_DIR/lib/VERSION" 2>/dev/null || echo unknown)"

echo "XCX Compiler Ecosystem - FreeBSD installer (experimental)"
echo "Version: $XCX_VERSION"
echo

if [ ! -f "$SCRIPT_DIR/xcx" ]; then
    echo "Error: Could not find xcx binary at $SCRIPT_DIR/xcx"
    echo "Installation cannot proceed."
    exit 1
fi

echo "This will install XCX version $XCX_VERSION on your computer."
printf "Continue? [y/N] "
read -r answer
case "$answer" in
    y|Y|yes|YES) ;;
    *) echo "Installation aborted."; exit 0 ;;
esac

LICENSE_FILE="$SCRIPT_DIR/resources/LICENSE.txt"
if [ -f "$LICENSE_FILE" ]; then
    echo "--- LICENSE --------------------------------------------------------------------"
    cat "$LICENSE_FILE"
    echo "--------------------------------------------------------------------------------"
    printf "Do you accept the terms of the License Agreement? [y/N] "
    read -r answer
    case "$answer" in
        y|Y|yes|YES) ;;
        *) echo "Installation aborted by user (License rejected)."; exit 0 ;;
    esac
fi

echo "Installing to: $INSTALL_DIR"

mkdir -p "$INSTALL_BIN_DIR"
mkdir -p "$INSTALL_LIB_DIR"
mkdir -p "$USER_BIN_DIR"

cp "$SCRIPT_DIR/xcx" "$INSTALL_BIN_DIR/"
chmod +x "$INSTALL_BIN_DIR/xcx"

if [ -d "$SCRIPT_DIR/lib/pax" ]; then
    cp -r "$SCRIPT_DIR/lib/pax" "$INSTALL_LIB_DIR/"
fi
if [ -d "$SCRIPT_DIR/lib/mathlib" ]; then
    cp -r "$SCRIPT_DIR/lib/mathlib" "$INSTALL_LIB_DIR/"
fi
if [ -d "$SCRIPT_DIR/lib/doc" ]; then
    cp -r "$SCRIPT_DIR/lib/doc" "$INSTALL_LIB_DIR/"
fi
if [ -f "$SCRIPT_DIR/lib/VERSION" ]; then
    cp "$SCRIPT_DIR/lib/VERSION" "$INSTALL_LIB_DIR/"
fi

if [ -d "$SCRIPT_DIR/resources" ]; then
    cp "$SCRIPT_DIR/resources/LICENSE.txt" "$INSTALL_DIR/" 2>/dev/null || true
    cp "$SCRIPT_DIR/resources/README.txt" "$INSTALL_DIR/" 2>/dev/null || true
fi

ln -sf "$INSTALL_BIN_DIR/xcx" "$USER_BIN_DIR/xcx"

cat > "$INSTALL_DIR/uninstall.sh" <<EOF
#!/bin/sh
printf "Completely remove XCX and all of its components? [y/N] "
read -r answer
case "\$answer" in
    y|Y|yes|YES) ;;
    *) echo "Uninstall aborted."; exit 0 ;;
esac

echo "Uninstalling XCX..."
rm -rf "$INSTALL_DIR"
rm -f "$USER_BIN_DIR/xcx"
echo "XCX Compiler Ecosystem was removed from your computer."
EOF
chmod +x "$INSTALL_DIR/uninstall.sh"

echo
echo "Installation complete."
echo "  Installed to:        $INSTALL_DIR"
echo "  Executable linked in: $USER_BIN_DIR"
echo "Make sure $USER_BIN_DIR is in your PATH."
