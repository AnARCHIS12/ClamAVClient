#!/bin/bash

# Script d'installation du démarrage automatique au boot
# Linux: systemd
# macOS: launchd
# Windows: entrée registre

set -e

case "$(uname -s)" in
  Linux*)
    echo "Installation du service systemd pour Linux..."
    SERVICE_FILE="/etc/systemd/system/clamavclient.service"
    
    if [ -f "src-tauri/com.openai.clamavclient.service" ]; then
      sudo cp "src-tauri/com.openai.clamavclient.service" "$SERVICE_FILE"
      sudo systemctl daemon-reload
      sudo systemctl enable clamavclient.service
      echo "Service systemd installé et activé"
      echo "Pour démarrer maintenant: sudo systemctl start clamavclient"
    else
      echo "Fichier service introuvable"
      exit 1
    fi
    ;;
  Darwin*)
    echo "Installation du launchd pour macOS..."
    PLIST_FILE="$HOME/Library/LaunchAgents/com.openai.clamavclient.plist"
    
    if [ -f "src-tauri/com.openai.clamavclient.plist" ]; then
      mkdir -p "$HOME/Library/LaunchAgents"
      cp "src-tauri/com.openai.clamavclient.plist" "$PLIST_FILE"
      launchctl load "$PLIST_FILE"
      echo "Launchd installé et activé"
      echo "Pour démarrer maintenant: launchctl start com.openai.clamavclient"
    else
      echo "Fichier plist introuvable"
      exit 1
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "Installation de l'entrée registre pour Windows..."
    # PowerShell script pour Windows
    powershell.exe -Command "
      \$regPath = 'HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run'
      \$regName = 'ClamAVClient'
      \$regValue = 'C:\\Program Files\\ClamAVClient\\clamavclient.exe'
      
      if (Test-Path \$regValue) {
        Set-ItemProperty -Path \$regPath -Name \$regName -Value \$regValue
        Write-Host 'Entrée registre ajoutée'
      } else {
        Write-Host 'Exécutable introuvable: \$regValue'
      }
    "
    ;;
  *)
    echo "Système non supporté"
    exit 1
    ;;
esac

echo "Installation terminée."
