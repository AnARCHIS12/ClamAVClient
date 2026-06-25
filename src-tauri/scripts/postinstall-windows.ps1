# Post-installation ClamAVClient — Windows
# Script PowerShell exécuté automatiquement après l'installation

$ErrorActionPreference = "SilentlyContinue"

$AppDir = "$env:ProgramFiles\ClamAvClient"
$AppBin = "$AppDir\ClamAvClient.exe"

# 1. Ajouter une exception Windows Defender pour le dossier de l'app
try {
    Add-MpPreference -ExclusionPath $AppDir -ErrorAction SilentlyContinue
    Write-Host "[ClamAVClient] Exception Windows Defender ajoutée pour $AppDir"
} catch {}

# 2. Ajouter au Pare-feu Windows (si nécessaire pour clamd)
try {
    $ruleName = "ClamAVClient"
    $existing = Get-NetFirewallRule -DisplayName $ruleName -ErrorAction SilentlyContinue
    if (-not $existing) {
        New-NetFirewallRule -DisplayName $ruleName `
            -Direction Inbound `
            -Action Allow `
            -Program $AppBin `
            -Protocol TCP `
            -ErrorAction SilentlyContinue | Out-Null
        Write-Host "[ClamAVClient] Règle pare-feu créée."
    }
} catch {}

# 3. Créer un raccourci Bureau
try {
    $DesktopPath = [Environment]::GetFolderPath("CommonDesktopDirectory")
    $ShortcutPath = "$DesktopPath\ClamAvClient.lnk"
    $WScriptShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WScriptShell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = $AppBin
    $Shortcut.WorkingDirectory = $AppDir
    $Shortcut.Description = "Antivirus local ClamAV"
    $Shortcut.Save()
    Write-Host "[ClamAVClient] Raccourci Bureau créé."
} catch {}

Write-Host "[ClamAVClient] Installation Windows terminée."
