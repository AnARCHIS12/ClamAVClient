#!/bin/bash

# Wrapper Linux pour ClamAVClient - demande automatiquement les droits administratifs
# Ce script est inclus dans le build final

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
APP_BINARY="$APP_DIR/clamavclient"

if [ "$EUID" -ne 0 ]; then
    # Utiliser pkexec pour l'élévation graphique
    if command -v pkexec &> /dev/null; then
        pkexec env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" "$APP_BINARY" "$@"
        exit $?
    # Fallback vers sudo
    elif command -v sudo &> /dev/null; then
        sudo "$APP_BINARY" "$@"
        exit $?
    else
        echo "Erreur: Impossible d'obtenir les droits administratifs."
        echo "Installez pkexec ou sudo."
        exit 1
    fi
fi

# Si on est root, lancer l'application
exec "$APP_BINARY" "$@"
