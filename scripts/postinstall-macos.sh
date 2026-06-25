#!/bin/bash

# Script post-installation pour macOS
# Installe automatiquement le launchd

set -e

INSTALL_DIR="/Applications/ClamAvClient.app/Contents"
BIN_DIR="/usr/local/bin"
PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST_FILE="$PLIST_DIR/com.openai.clamavclient.plist"

echo "Installation post-installation ClamAVClient pour macOS..."

# Créer le lien symbolique vers l'exécutable
if [ -f "$INSTALL_DIR/MacOS/clamavclient" ]; then
    mkdir -p "$BIN_DIR"
    ln -sf "$INSTALL_DIR/MacOS/clamavclient" "$BIN_DIR/clamavclient"
    chmod +x "$BIN_DIR/clamavclient"
    echo "Lien symbolique créé: $BIN_DIR/clamavclient"
fi

# Installer le launchd
if [ -f "$INSTALL_DIR/Resources/com.openai.clamavclient.plist" ]; then
    mkdir -p "$PLIST_DIR"
    cp "$INSTALL_DIR/Resources/com.openai.clamavclient.plist" "$PLIST_FILE"
    launchctl load "$PLIST_FILE"
    echo "Launchd installé et activé"
fi

echo "Installation post-installation terminée."
