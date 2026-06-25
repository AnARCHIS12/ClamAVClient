#!/bin/bash

# Script post-installation pour Linux (DEB)
# Installe automatiquement le service systemd et les règles PolicyKit

set -e

INSTALL_DIR="/opt/clamavclient"
BIN_DIR="/usr/local/bin"
SERVICE_FILE="/etc/systemd/system/clamavclient.service"
POLICY_FILE="/usr/share/polkit-1/actions/com.openai.clamavclient.policy"

echo "Installation post-installation ClamAVClient..."

# Créer le lien symbolique vers l'exécutable
if [ -f "$INSTALL_DIR/clamavclient" ]; then
    ln -sf "$INSTALL_DIR/clamavclient" "$BIN_DIR/clamavclient"
    chmod +x "$BIN_DIR/clamavclient"
    echo "Lien symbolique créé: $BIN_DIR/clamavclient"
fi

# Installer le service systemd
if [ -f "$INSTALL_DIR/com.openai.clamavclient.service" ]; then
    cp "$INSTALL_DIR/com.openai.clamavclient.service" "$SERVICE_FILE"
    systemctl daemon-reload
    systemctl enable clamavclient.service
    echo "Service systemd installé et activé"
fi

# Installer la règle PolicyKit
if [ -f "$INSTALL_DIR/com.openai.clamavclient.policy" ]; then
    cp "$INSTALL_DIR/com.openai.clamavclient.policy" "$POLICY_FILE"
    echo "Règle PolicyKit installée"
fi

echo "Installation post-installation terminée."
