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

# 4. Supprimer l'enregistrement au démarrage automatique Windows (Run Key)
try {
    $RegistryPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    Remove-ItemProperty -Path $RegistryPath -Name "ClamAVClient" -ErrorAction SilentlyContinue | Out-Null
    Write-Host "[ClamAVClient] Enregistrement au démarrage automatique Windows supprimé."
} catch {}

# 5. Arrêter et désinstaller le service clamd si présent
try {
    $service = Get-Service -Name "clamd" -ErrorAction SilentlyContinue
    if ($service) {
        # Arrêter le service
        Stop-Service -Name "clamd" -Force -ErrorAction SilentlyContinue
        
        # Trouver clamd.exe et exécuter --uninstall-service
        $ClamdExe = Get-ChildItem -Path $AppDir -Filter "clamd.exe" -Recurse -File | Select-Object -First 1
        if ($ClamdExe) {
            Start-Process -FilePath $ClamdExe.FullName -ArgumentList "--uninstall-service" -WorkingDirectory $ClamdExe.DirectoryName -Verb RunAs -Wait
        } else {
            # Si non trouvé, utiliser sc.exe
            sc.exe delete clamd | Out-Null
        }
        Write-Host "[ClamAVClient] Service Windows clamd arrêté et désinstallé."
    }
} catch {}

Write-Host "[ClamAVClient] Préparation à la désinstallation Windows terminée."
