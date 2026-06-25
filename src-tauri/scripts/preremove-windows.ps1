# Pré-désinstallation ClamAVClient — Windows
# Script PowerShell exécuté automatiquement avant la désinstallation

$ErrorActionPreference = "SilentlyContinue"

$AppDir = "$env:ProgramFiles\ClamAvClient"

# 1. Supprimer l'exception Windows Defender
try {
    Remove-MpPreference -ExclusionPath $AppDir -ErrorAction SilentlyContinue
    Write-Host "[ClamAVClient] Exception Windows Defender supprimée."
} catch {}

# 2. Supprimer la règle pare-feu
try {
    Remove-NetFirewallRule -DisplayName "ClamAVClient" -ErrorAction SilentlyContinue
    Write-Host "[ClamAVClient] Règle pare-feu supprimée."
} catch {}

# 3. Supprimer le raccourci Bureau
try {
    $DesktopPath = [Environment]::GetFolderPath("CommonDesktopDirectory")
    $ShortcutPath = "$DesktopPath\ClamAvClient.lnk"
    if (Test-Path $ShortcutPath) {
        Remove-Item $ShortcutPath -Force
        Write-Host "[ClamAVClient] Raccourci Bureau supprimé."
    }
} catch {}

Write-Host "[ClamAVClient] Préparation à la désinstallation Windows terminée."
