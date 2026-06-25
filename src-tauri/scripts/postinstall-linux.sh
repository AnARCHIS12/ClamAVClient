#!/bin/bash
# Post-installation ClamAVClient — Linux (deb/rpm)
# Exécuté automatiquement par le gestionnaire de paquets après installation

set -e

APP_BIN="/usr/bin/clamavclient"
POLICY_FILE="/usr/share/polkit-1/actions/com.openai.clamavclient.policy"
SERVICE_FILE="/etc/systemd/system/clamavclient-watcher.service"

# 1. Donner les droits d'exécution au binaire principal
if [ -f "$APP_BIN" ]; then
    chmod 755 "$APP_BIN"
fi

# 2. Installer la politique Polkit si elle n'est pas déjà en place
POLICY_SRC="/usr/lib/clamavclient/com.openai.clamavclient.policy"
if [ -f "$POLICY_SRC" ] && [ ! -f "$POLICY_FILE" ]; then
    cp "$POLICY_SRC" "$POLICY_FILE"
fi

# 3. Recharger Polkit
if command -v pkexec &>/dev/null && systemctl is-active --quiet polkit 2>/dev/null; then
    systemctl reload polkit 2>/dev/null || true
fi

# 4. Activer et démarrer le service systemd si disponible
if [ -f "$SERVICE_FILE" ] && command -v systemctl &>/dev/null; then
    systemctl daemon-reload 2>/dev/null || true
    systemctl enable clamavclient-watcher.service 2>/dev/null || true
fi

echo "[ClamAVClient] Installation terminée."
