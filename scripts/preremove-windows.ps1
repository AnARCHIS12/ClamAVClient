# Script pré-suppression pour Windows
# Supprime l'entrée de démarrage automatique du registre

$ErrorActionPreference = "Stop"

Write-Host "Désinstallation ClamAVClient pour Windows..."

$RegPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$RegName = "ClamAVClient"

# Supprimer l'entrée de démarrage automatique
if (Get-ItemProperty -Path $RegPath -Name $RegName -ErrorAction SilentlyContinue) {
    Remove-ItemProperty -Path $RegPath -Name $RegName
    Write-Host "Entrée de registre supprimée"
}

Write-Host "Désinstallation terminée."
