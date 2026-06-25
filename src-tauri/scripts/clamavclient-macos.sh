#!/bin/bash

# Wrapper macOS pour ClamAVClient - demande automatiquement les droits administratifs
# Ce script est inclus dans le build final

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
APP_BINARY="$APP_DIR/ClamAvClient.app/Contents/MacOS/clamavclient"

if [ "$EUID" -ne 0 ]; then
    # Utiliser osascript pour demander les droits via GUI
    osascript -e 'do shell script "'"$APP_BINARY"' $*" with administrator privileges' "$@"
    exit $?
fi

# Si on est root, lancer l'application
exec "$APP_BINARY" "$@"
