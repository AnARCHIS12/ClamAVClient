#!/bin/bash

# Script pré-suppression pour macOS
# Désactive et supprime le launchd

set -e

echo "Désinstallation ClamAVClient pour macOS..."

PLIST_FILE="$HOME/Library/LaunchAgents/com.openai.clamavclient.plist"

# Désactiver le launchd
if launchctl list | grep -q "com.openai.clamavclient"; then
    launchctl unload "$PLIST_FILE"
fi

# Supprimer le fichier plist
if [ -f "$PLIST_FILE" ]; then
    rm -f "$PLIST_FILE"
fi

# Supprimer le lien symbolique
if [ -L "/usr/local/bin/clamavclient" ]; then
    rm -f "/usr/local/bin/clamavclient"
fi

echo "Désinstallation terminée."
