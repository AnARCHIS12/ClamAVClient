# Script post-installation pour Windows
# Installe automatiquement l'entrée de démarrage dans le registre

$ErrorActionPreference = "Stop"

Write-Host "Installation post-installation ClamAVClient pour Windows..."

$InstallDir = "C:\Program Files\ClamAVClient"
$Executable = "$InstallDir\clamavclient.exe"
$RegPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$RegName = "ClamAVClient"

# Vérifier si l'exécutable existe
if (Test-Path $Executable) {
    # Ajouter l'entrée de démarrage automatique
    Set-ItemProperty -Path $RegPath -Name $RegName -Value $Executable
    Write-Host "Entrée de registre ajoutée pour le démarrage automatique"
} else {
    Write-Host "Avertissement: Exécutable introuvable à $Executable"
}

Write-Host "Installation post-installation terminée."
