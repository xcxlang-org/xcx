#!/bin/bash

set -e
if ! command -v whiptail &> /dev/null; then
    echo "This installer requires 'whiptail'. Please install it using your distribution's package manager (e.g., sudo apt install whiptail, or pacman -S libnewt)."
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"

INSTALL_DIR="$HOME/.local/share/xcx"
INSTALL_BIN_DIR="$INSTALL_DIR/bin"
INSTALL_LIB_DIR="$INSTALL_DIR/lib"
USER_BIN_DIR="$HOME/.local/bin"

if [ ! -f "$SCRIPT_DIR/xcx" ]; then
    whiptail --title "Error" --msgbox "Could not find xcx binary at $SCRIPT_DIR/xcx\n\nInstallation cannot proceed." 10 60
    exit 1
fi

whiptail --title "Setup - XCX Compiler Ecosystem" --msgbox "Welcome to the XCX Compiler Ecosystem Setup Wizard.\n\nThis will install XCX version 4.2 on your computer.\n\nIt is recommended that you close all other applications before continuing." 12 70

LICENSE_FILE="$SCRIPT_DIR/resources/LICENSE.txt"
if [ -f "$LICENSE_FILE" ]; then
    whiptail --title "License Agreement" --textbox "$LICENSE_FILE" 20 80 --scrolltext
    if ! whiptail --title "License Agreement" --yesno "Do you accept the terms of the License Agreement?" 10 50; then
        echo "Installation aborted by user (License rejected)."
        exit 0
    fi
fi

COMPONENTS=$(whiptail --title "Select Components" --checklist \
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

TASKS=$(whiptail --title "Select Additional Tasks" --checklist \
"Which additional tasks should be performed?" 12 70 2 \
"MIME" "Register file associations (.xcx, .pax) and icons" ON 3>&1 1>&2 2>&3)

if [ $? -ne 0 ]; then
    echo "Installation aborted by user."
    exit 0
fi

INSTALL_MIME=false
if echo "$TASKS" | grep -q "MIME"; then INSTALL_MIME=true; fi

SUMMARY="Setup is now ready to begin installing XCX on your computer.\n\nDestination location:\n  $INSTALL_DIR\n\nSelected components:\n  XCX Compiler Core"
if [ "$INSTALL_PAX" = true ]; then SUMMARY="$SUMMARY\n  PAX Package Manager"; fi
if [ "$INSTALL_MATH" = true ]; then SUMMARY="$SUMMARY\n  Math Standard Library"; fi
if [ "$INSTALL_DOC" = true ]; then SUMMARY="$SUMMARY\n  Offline Documentation"; fi
SUMMARY="$SUMMARY\n\nAdditional tasks:"
if [ "$INSTALL_MIME" = true ]; then
    SUMMARY="$SUMMARY\n  Register file associations and icons"
else
    SUMMARY="$SUMMARY\n  (None)"
fi

if ! whiptail --title "Ready to Install" --yesno "$SUMMARY\n\nClick Yes to continue with the installation, or No to exit." 18 70; then
    echo "Installation aborted by user."
    exit 0
fi

{
    echo 5
    
    mkdir -p "$INSTALL_BIN_DIR"
    mkdir -p "$INSTALL_LIB_DIR"
    mkdir -p "$USER_BIN_DIR"
    
    echo 15
    
    cp "$SCRIPT_DIR/xcx" "$INSTALL_BIN_DIR/"
    chmod +x "$INSTALL_BIN_DIR/xcx"
    
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
    echo 65
    
    if [ -d "$SCRIPT_DIR/resources" ]; then
        cp "$SCRIPT_DIR/resources/LICENSE.txt" "$INSTALL_DIR/" 2>/dev/null || true
        cp "$SCRIPT_DIR/resources/README.txt" "$INSTALL_DIR/" 2>/dev/null || true
        cp -r "$SCRIPT_DIR/resources/icons" "$INSTALL_DIR/" 2>/dev/null || true
    fi
    echo 65
    
    ln -sf "$INSTALL_BIN_DIR/xcx" "$USER_BIN_DIR/xcx"
    
    echo 75
    
    if [ "$INSTALL_MIME" = true ]; then
        MIME_DIR="$HOME/.local/share/mime/packages"
        mkdir -p "$MIME_DIR"
        
        cat > "$MIME_DIR/application-x-xcx.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-xcx">
    <comment>XCX Script File</comment>
    <glob pattern="*.xcx"/>
    <icon name="application-x-xcx"/>
  </mime-type>
</mime-info>
EOF

        cat > "$MIME_DIR/application-x-pax.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-pax">
    <comment>PAX Package File</comment>
    <glob pattern="*.pax"/>
    <icon name="application-x-pax"/>
  </mime-type>
</mime-info>
EOF

        if command -v update-mime-database &> /dev/null; then
            update-mime-database "$HOME/.local/share/mime"
        fi
        
        ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/mimetypes"
        mkdir -p "$ICON_DIR"
        
        if [ -f "$INSTALL_DIR/icons/xcx.ico" ]; then
            cp "$INSTALL_DIR/icons/xcx.ico" "$ICON_DIR/application-x-xcx.ico"
        fi
        if [ -f "$INSTALL_DIR/icons/pax.ico" ]; then
            cp "$INSTALL_DIR/icons/pax.ico" "$ICON_DIR/application-x-pax.ico"
        fi
        
        if command -v gtk-update-icon-cache &> /dev/null; then
            gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
        fi
    fi

    echo 90

    cat > "$INSTALL_DIR/uninstall.sh" <<EOF
#!/bin/bash
if ! whiptail --title "XCX Uninstaller" --yesno "Are you sure you want to completely remove XCX and all of its components?" 10 60; then
    exit 0
fi

echo "Uninstalling XCX..."
rm -rf "$INSTALL_DIR"
rm -f "$USER_BIN_DIR/xcx"

rm -f "$HOME/.local/share/mime/packages/application-x-xcx.xml"
rm -f "$HOME/.local/share/mime/packages/application-x-pax.xml"
if command -v update-mime-database &> /dev/null; then
    update-mime-database "$HOME/.local/share/mime"
fi

rm -f "$HOME/.local/share/icons/hicolor/256x256/mimetypes/application-x-xcx.ico"
rm -f "$HOME/.local/share/icons/hicolor/256x256/mimetypes/application-x-pax.ico"
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

whiptail --title "Uninstall Complete" --msgbox "XCX Compiler Ecosystem was successfully removed from your computer." 8 60
EOF
    chmod +x "$INSTALL_DIR/uninstall.sh"
    
    echo 100
} | whiptail --title "Setup - XCX Compiler Ecosystem" --gauge "Installing components...\nPlease wait while Setup installs XCX on your computer." 8 60 0

whiptail --title "Setup - XCX Compiler Ecosystem" --msgbox "Setup has finished installing XCX on your computer.\n\nInstalled to:\n$INSTALL_DIR\n\nExecutable linked in:\n$USER_BIN_DIR\n\nMake sure $USER_BIN_DIR is in your PATH." 14 70
