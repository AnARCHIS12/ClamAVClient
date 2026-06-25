#!/bin/bash

# Script d'installation des wrappers pour Linux/macOS
# Ce script est exécuté après l'installation du package

set -e

case "$(uname -s)" in
  Linux*)
    echo "Installation du wrapper Linux..."
    INSTALL_DIR="/usr/local/bin"
    APP_DIR="/opt/clamavclient"
    
    # Créer le lien symbolique vers le wrapper
    if [ -f "$APP_DIR/scripts/clamavclient-linux.sh" ]; then
      ln -sf "$APP_DIR/scripts/clamavclient-linux.sh" "$INSTALL_DIR/clamavclient"
      chmod +x "$INSTALL_DIR/clamavclient"
      echo "Wrapper installé dans $INSTALL_DIR/clamavclient"
      
      # Installer la règle PolicyKit
      if [ -f "$APP_DIR/com.openai.clamavclient.policy" ]; then
        cp "$APP_DIR/com.openai.clamavclient.policy" /usr/share/polkit-1/actions/
        echo "Règle PolicyKit installée"
      fi
    fi
    ;;
  Darwin*)
    echo "Installation du wrapper macOS..."
    INSTALL_DIR="/usr/local/bin"
    APP_DIR="/Applications/ClamAvClient.app/Contents"
    
    # Créer le lien symbolique vers le wrapper
    if [ -f "$APP_DIR/Resources/scripts/clamavclient-macos.sh" ]; then
      ln -sf "$APP_DIR/Resources/scripts/clamavclient-macos.sh" "$INSTALL_DIR/clamavclient"
      chmod +x "$INSTALL_DIR/clamavclient"
      echo "Wrapper installé dans $INSTALL_DIR/clamavclient"
    fi
    ;;
esac

echo "Installation terminée."
