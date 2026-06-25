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

# 4. Configurer le démarrage automatique avec Windows pour l'utilisateur courant (Registry Run)
try {
    $RegistryPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    New-ItemProperty -Path $RegistryPath -Name "ClamAVClient" -Value "`"$AppBin`"" -PropertyType String -Force | Out-Null
    Write-Host "[ClamAVClient] Enregistrement au démarrage automatique Windows (Run Key) effectué."
} catch {}

# 5. Installer clamd.exe comme Service Windows (démarrage automatique au boot)
try {
    $ClamdExe = Get-ChildItem -Path $AppDir -Filter "clamd.exe" -Recurse -File | Select-Object -First 1
    if ($ClamdExe) {
        $ClamdPath = $ClamdExe.FullName
        $ClamdBinDir = Split-Path -Parent $ClamdPath
        
        # S'assurer que le fichier de config clamd.conf existe
        $ClamdConf = Join-Path $ClamdBinDir "clamd.conf"
        if (-not (Test-Path $ClamdConf)) {
            # Chercher clamd.conf.sample
            $SampleConf = Join-Path $ClamdBinDir "clamd.conf.sample"
            if (-not (Test-Path $SampleConf)) {
                $SampleConf = Join-Path $AppDir "conf_examples\clamd.conf.sample" -Resolve
            }
            if (Test-Path $SampleConf) {
                Copy-Item $SampleConf $ClamdConf -Force
                # Commenter "Example" et configurer la base de données relative
                (Get-Content $ClamdConf) -replace '^Example', '# Example' | Set-Content $ClamdConf
                Add-Content -Path $ClamdConf -Value "`nDatabaseDirectory ..\db`nLogFile ..\clamd.log`nLocalSocket clamd.socket"
            }
        }

        # Installer le service si non présent
        $service = Get-Service -Name "clamd" -ErrorAction SilentlyContinue
        if (-not $service) {
            Start-Process -FilePath $ClamdPath -ArgumentList "--install" -WorkingDirectory $ClamdBinDir -Verb RunAs -Wait
            # Configurer le démarrage automatique du service
            Set-Service -Name "clamd" -StartupType Automatic
            Start-Service -Name "clamd" -ErrorAction SilentlyContinue
            Write-Host "[ClamAVClient] Service Windows 'clamd' installé et démarré."
        }
    }
} catch {
    Write-Host "[ClamAVClient] Erreur lors de l'installation du service clamd: $_"
}

Write-Host "[ClamAVClient] Installation Windows terminée."
