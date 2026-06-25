#!/bin/bash

# Script de lancement avec élévation de privilèges pour Linux/macOS
# Utilise pkexec (PolicyKit) pour demander les droits via interface graphique

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

if [ "$EUID" -ne 0 ]; then
    # Utiliser pkexec pour l'élévation graphique (Linux)
    if command -v pkexec &> /dev/null; then
        pkexec env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" "$0" "$@"
        exit $?
    # Fallback vers sudo si pkexec n'est pas disponible
   elif command -v sudo &> /dev/null; then
        echo "ClamAVClient nécessite des droits administratifs pour scanner tous les fichiers système."
        echo "Relance avec sudo..."
        sudo "$0" "$@"
        exit $?
    else
        echo "Erreur: Impossible d'obtenir les droits administratifs."
        echo "Installez pkexec ou sudo."
        exit 1
    fi
fi

# Si on est root, lancer l'application normalement
cd "$PROJECT_DIR"
npm run tauri dev "$@"
