#!/bin/sh

set -e

# FreeBSD installer. Uses bsddialog (in the FreeBSD base system since 14.0)
# and falls back to whiptail (pkg install newt) when available.

SCRIPT_DIR="$(cd "$(dirname "$0")" &> /dev/null && pwd)"

if command -v bsddialog &> /dev/null; then
    DIALOG="bsddialog"
elif command -v whiptail &> /dev/null; then
    DIALOG="whiptail"
else
    echo "This installer requires 'bsddialog' (FreeBSD base system) or 'whiptail' (pkg install newt)."
    exit 1
fi

INSTALL_DIR="$HOME/.local/share/xcx"
INSTALL_BIN_DIR="$INSTALL_DIR/bin"
INSTALL_LIB_DIR="$INSTALL_DIR/lib"
USER_BIN_DIR="$HOME/.local/bin"

if [ ! -f "$SCRIPT_DIR/xcx" ]; then
    echo "Could not find xcx binary at $SCRIPT_DIR/xcx - installation cannot proceed."
    exit 1
fi

XCX_VERSION="$(tr -d '[:space:]' < "$SCRIPT_DIR/lib/VERSION" 2>/dev/null || echo "unknown")"

$DIALOG --title "Setup - XCX Compiler Ecosystem" --msgbox "Welcome to the XCX Compiler Ecosystem Setup Wizard.\n\nThis will install XCX version $XCX_VERSION on your computer (FreeBSD).\n\nIt is recommended that you close all other applications before continuing." 12 70

LICENSE_FILE="$SCRIPT_DIR/resources/LICENSE.txt"
if [ -f "$LICENSE_FILE" ] && [ "$DIALOG" = "bsddialog" ]; then
    $DIALOG --title "License Agreement" --textbox "$LICENSE_FILE" 20 80
    if ! $DIALOG --title "License Agreement" --yesno "Do you accept the terms of the License Agreement?" 10 50; then
        echo "Installation aborted by user (License rejected)."
        exit 0
    fi
fi

COMPONENTS=$($DIALOG --title "Select Components" --checklist \
"Select the components you want to install; clear the components you do not want to install.\nXCX Compiler Core is required and will always be installed." 15 70 3 \
"PAX" "PAX Package Manager" on \
"MATH" "Math Standard Library" on \
"DOC" "Offline Documentation" on 3>&1 1>&2 2>&3) || true

INSTALL_PAX=false
INSTALL_MATH=false
INSTALL_DOC=false
case "$COMPONENTS" in
    *PAX*)  INSTALL_PAX=true ;;
esac
case "$COMPONENTS" in
    *MATH*) INSTALL_MATH=true ;;
esac
case "$COMPONENTS" in
    *DOC*)  INSTALL_DOC=true ;;
esac

SUMMARY="Setup is now ready to begin installing XCX on your computer.\n\nDestination location:\n  $INSTALL_DIR\n\nSelected components:\n  XCX Compiler Core"
if [ "$INSTALL_PAX" = true ]; then SUMMARY="$SUMMARY\n  PAX Package Manager"; fi
if [ "$INSTALL_MATH" = true ]; then SUMMARY="$SUMMARY\n  Math Standard Library"; fi
if [ "$INSTALL_DOC" = true ]; then SUMMARY="$SUMMARY\n  Offline Documentation"; fi

if ! $DIALOG --title "Ready to Install" --yesno "$SUMMARY\n\nSelect Yes to continue with the installation, or No to exit." 16 70; then
    echo "Installation aborted by user."
    exit 0
fi

mkdir -p "$INSTALL_BIN_DIR"
mkdir -p "$INSTALL_LIB_DIR"
mkdir -p "$USER_BIN_DIR"

cp "$SCRIPT_DIR/xcx" "$INSTALL_BIN_DIR/"
chmod +x "$INSTALL_BIN_DIR/xcx"

if [ "$INSTALL_PAX" = true ] && [ -d "$SCRIPT_DIR/lib/pax" ]; then
    cp -r "$SCRIPT_DIR/lib/pax" "$INSTALL_LIB_DIR/"
fi
if [ -f "$SCRIPT_DIR/lib/VERSION" ]; then
    cp "$SCRIPT_DIR/lib/VERSION" "$INSTALL_LIB_DIR/"
fi
if [ "$INSTALL_MATH" = true ] && [ -d "$SCRIPT_DIR/lib/mathlib" ]; then
    cp -r "$SCRIPT_DIR/lib/mathlib" "$INSTALL_LIB_DIR/"
fi
if [ "$INSTALL_DOC" = true ] && [ -d "$SCRIPT_DIR/lib/doc" ]; then
    cp -r "$SCRIPT_DIR/lib/doc" "$INSTALL_LIB_DIR/"
fi
if [ -d "$SCRIPT_DIR/resources" ]; then
    cp "$SCRIPT_DIR/resources/LICENSE.txt" "$INSTALL_DIR/" 2>/dev/null || true
    cp "$SCRIPT_DIR/resources/README.txt" "$INSTALL_DIR/" 2>/dev/null || true
fi

ln -sf "$INSTALL_BIN_DIR/xcx" "$USER_BIN_DIR/xcx"

cat > "$INSTALL_DIR/uninstall.sh" <<EOF
#!/bin/sh
echo "Uninstalling XCX..."
rm -rf "$INSTALL_DIR"
rm -f "$USER_BIN_DIR/xcx"
echo "XCX Compiler Ecosystem was removed from your computer."
EOF
chmod +x "$INSTALL_DIR/uninstall.sh"

$DIALOG --title "Setup - XCX Compiler Ecosystem" --msgbox "Setup has finished installing XCX on your computer.\n\nInstalled to:\n$INSTALL_DIR\n\nExecutable linked in:\n$USER_BIN_DIR\n\nMake sure $USER_BIN_DIR is in your PATH." 14 70
