#!/bin/bash

set -e

# --- Detect available dialog tool (whiptail or dialog) ---
if command -v whiptail &> /dev/null; then
    DIALOG=whiptail
elif command -v dialog &> /dev/null; then
    DIALOG=dialog
else
    echo "This installer requires 'dialog' (macOS does not ship with whiptail by default)."
    echo "Install it via Homebrew: brew install dialog"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"

INSTALL_DIR="$HOME/.xcx"
INSTALL_BIN_DIR="$INSTALL_DIR/bin"
INSTALL_LIB_DIR="$INSTALL_DIR/lib"
USER_BIN_DIR="$HOME/.xcx/bin"

if [ ! -f "$SCRIPT_DIR/xcx" ]; then
    $DIALOG --title "Error" --msgbox "Could not find xcx binary at $SCRIPT_DIR/xcx\n\nInstallation cannot proceed." 10 60
    exit 1
fi

$DIALOG --title "Setup - XCX Compiler Ecosystem" --msgbox "Welcome to the XCX Compiler Ecosystem Setup Wizard.\n\nThis will install XCX version 4.1 on your computer (macOS).\n\nIt is recommended that you close all other applications before continuing." 12 70

LICENSE_FILE="$SCRIPT_DIR/resources/LICENSE.txt"
if [ -f "$LICENSE_FILE" ]; then
    $DIALOG --title "License Agreement" --textbox "$LICENSE_FILE" 20 80 --scrolltext
    if ! $DIALOG --title "License Agreement" --yesno "Do you accept the terms of the License Agreement?" 10 50; then
        echo "Installation aborted by user (License rejected)."
        exit 0
    fi
fi

COMPONENTS=$($DIALOG --title "Select Components" --checklist \
"Select the components you want to install; clear the components you do not want to install.\nXCX Compiler Core is required and will always be installed." 15 70 3 \
"PAX" "PAX Package Manager" ON \
"MATH" "Math Standard Library" ON \
"DOC" "Offline Documentation" ON 3>&1 1>&2 2>&3)

if [ $? -ne 0 ]; then
    echo "Installation aborted by user."
    exit 0
fi

INSTALL_PAX=false
INSTALL_MATH=false
INSTALL_DOC=false

if echo "$COMPONENTS" | grep -q "PAX"; then INSTALL_PAX=true; fi
if echo "$COMPONENTS" | grep -q "MATH"; then INSTALL_MATH=true; fi
if echo "$COMPONENTS" | grep -q "DOC"; then INSTALL_DOC=true; fi

TASKS=$($DIALOG --title "Select Additional Tasks" --checklist \
"Which additional tasks should be performed?" 12 70 1 \
"SHELLRC" "Add $USER_BIN_DIR to PATH in your shell rc file" ON 3>&1 1>&2 2>&3)

if [ $? -ne 0 ]; then
    echo "Installation aborted by user."
    exit 0
fi

INSTALL_SHELLRC=false
if echo "$TASKS" | grep -q "SHELLRC"; then INSTALL_SHELLRC=true; fi

SUMMARY="Setup is now ready to begin installing XCX on your computer.\n\nDestination location:\n  $INSTALL_DIR\n\nSelected components:\n  XCX Compiler Core"
if [ "$INSTALL_PAX" = true ]; then SUMMARY="$SUMMARY\n  PAX Package Manager"; fi
if [ "$INSTALL_MATH" = true ]; then SUMMARY="$SUMMARY\n  Math Standard Library"; fi
if [ "$INSTALL_DOC" = true ]; then SUMMARY="$SUMMARY\n  Offline Documentation"; fi
SUMMARY="$SUMMARY\n\nAdditional tasks:"
if [ "$INSTALL_SHELLRC" = true ]; then
    SUMMARY="$SUMMARY\n  Add $USER_BIN_DIR to PATH"
else
    SUMMARY="$SUMMARY\n  (None)"
fi

if ! $DIALOG --title "Ready to Install" --yesno "$SUMMARY\n\nClick Yes to continue with the installation, or No to exit." 18 70; then
    echo "Installation aborted by user."
    exit 0
fi

{
    echo 5

    mkdir -p "$INSTALL_BIN_DIR"
    mkdir -p "$INSTALL_LIB_DIR"

    echo 15

    cp "$SCRIPT_DIR/xcx" "$INSTALL_BIN_DIR/"
    chmod +x "$INSTALL_BIN_DIR/xcx"

    # Remove Gatekeeper quarantine attribute (binary is not notarized)
    xattr -d com.apple.quarantine "$INSTALL_BIN_DIR/xcx" 2>/dev/null || true

    echo 35

    if [ "$INSTALL_PAX" = true ] && [ -d "$SCRIPT_DIR/lib/pax" ]; then
        cp -r "$SCRIPT_DIR/lib/pax" "$INSTALL_LIB_DIR/"
    fi
    if [ -f "$SCRIPT_DIR/lib/VERSION" ]; then
        cp "$SCRIPT_DIR/lib/VERSION" "$INSTALL_LIB_DIR/"
    fi
    echo 45

    if [ "$INSTALL_MATH" = true ] && [ -d "$SCRIPT_DIR/lib/mathlib" ]; then
        cp -r "$SCRIPT_DIR/lib/mathlib" "$INSTALL_LIB_DIR/"
    fi
    echo 60

    if [ "$INSTALL_DOC" = true ] && [ -d "$SCRIPT_DIR/lib/doc" ]; then
        cp -r "$SCRIPT_DIR/lib/doc" "$INSTALL_LIB_DIR/"
    fi
    echo 70

    if [ -d "$SCRIPT_DIR/resources" ]; then
        cp "$SCRIPT_DIR/resources/LICENSE.txt" "$INSTALL_DIR/" 2>/dev/null || true
        cp "$SCRIPT_DIR/resources/README.txt" "$INSTALL_DIR/" 2>/dev/null || true
    fi
    echo 80

    if [ "$INSTALL_SHELLRC" = true ]; then
        SHELL_RC=""
        case "$SHELL" in
            */zsh) SHELL_RC="$HOME/.zshrc" ;;
            */bash) SHELL_RC="$HOME/.bash_profile" ;;
            *) SHELL_RC="$HOME/.profile" ;;
        esac

        PATH_LINE="export PATH=\"$USER_BIN_DIR:\$PATH\""
        if [ -f "$SHELL_RC" ] && ! grep -qF "$PATH_LINE" "$SHELL_RC"; then
            echo "" >> "$SHELL_RC"
            echo "# Added by XCX installer" >> "$SHELL_RC"
            echo "$PATH_LINE" >> "$SHELL_RC"
        elif [ ! -f "$SHELL_RC" ]; then
            echo "# Added by XCX installer" > "$SHELL_RC"
            echo "$PATH_LINE" >> "$SHELL_RC"
        fi
    fi

    echo 90

    cat > "$INSTALL_DIR/uninstall.sh" <<EOF
#!/bin/bash
if ! $DIALOG --title "XCX Uninstaller" --yesno "Are you sure you want to completely remove XCX and all of its components?" 10 60; then
    exit 0
fi

echo "Uninstalling XCX..."
rm -rf "$INSTALL_DIR"

$DIALOG --title "Uninstall Complete" --msgbox "XCX Compiler Ecosystem was successfully removed from your computer.\n\nNote: the PATH entry in your shell rc file was not automatically removed - remove it manually if you wish." 10 70
EOF
    chmod +x "$INSTALL_DIR/uninstall.sh"

    echo 100
} | $DIALOG --title "Setup - XCX Compiler Ecosystem" --gauge "Installing components...\nPlease wait while Setup installs XCX on your computer." 8 60 0

$DIALOG --title "Setup - XCX Compiler Ecosystem" --msgbox "Setup has finished installing XCX on your computer.\n\nInstalled to:\n$INSTALL_DIR\n\nExecutable linked in:\n$USER_BIN_DIR\n\nRestart your terminal (or run 'source ~/.zshrc'), so that the 'xcx' command becomes available in your PATH." 14 70