#!/bin/bash

# Script pré-suppression pour Linux (DEB)
# Désactive et supprime le service systemd

set -e

echo "Désinstallation ClamAVClient..."

# Désactiver et arrêter le service
if systemctl is-active --quiet clamavclient.service; then
    systemctl stop clamavclient.service
fi

if systemctl is-enabled --quiet clamavclient.service; then
    systemctl disable clamavclient.service
fi

# Supprimer le service systemd
if [ -f "/etc/systemd/system/clamavclient.service" ]; then
    rm -f "/etc/systemd/system/clamavclient.service"
    systemctl daemon-reload
fi

# Supprimer le lien symbolique
if [ -L "/usr/local/bin/clamavclient" ]; then
    rm -f "/usr/local/bin/clamavclient"
fi

# Supprimer la règle PolicyKit
if [ -f "/usr/share/polkit-1/actions/com.openai.clamavclient.policy" ]; then
    rm -f "/usr/share/polkit-1/actions/com.openai.clamavclient.policy"
fi

echo "Désinstallation terminée."
