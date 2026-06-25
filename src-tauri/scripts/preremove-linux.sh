#!/bin/bash
# Pré-désinstallation ClamAVClient — Linux (deb/rpm)
# Exécuté automatiquement par le gestionnaire de paquets avant désinstallation

set -e

# 1. Arrêter et désactiver le service systemd
if command -v systemctl &>/dev/null; then
    systemctl stop clamavclient-watcher.service 2>/dev/null || true
    systemctl disable clamavclient-watcher.service 2>/dev/null || true
    systemctl daemon-reload 2>/dev/null || true
fi

# 2. Supprimer la politique Polkit
POLICY_FILE="/usr/share/polkit-1/actions/com.openai.clamavclient.policy"
if [ -f "$POLICY_FILE" ]; then
    rm -f "$POLICY_FILE"
fi

# 3. Recharger Polkit
if systemctl is-active --quiet polkit 2>/dev/null; then
    systemctl reload polkit 2>/dev/null || true
fi

echo "[ClamAVClient] Préparation à la désinstallation terminée."
