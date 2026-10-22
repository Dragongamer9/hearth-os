# Installs or reinstalls Nimbus from the latest GitHub release and makes it your Windows shell.
#   Double-click install-nimbus.cmd, or run in PowerShell:
#   irm https://raw.githubusercontent.com/Dragongamer9/hearth-os/main/tools/install-nimbus.ps1 | iex
# Only your Windows account is affected. Undo any time: %USERPROFILE%\nimbus-restore.cmd
param([switch]$NoShell)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'   # much faster downloads in Windows PowerShell
$repo = 'Dragongamer9/hearth-os'
$root = Join-Path $env:LOCALAPPDATA 'Nimbus'
$app  = Join-Path $root 'app'
$winlogon = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'

function Step($t) { Write-Host "-> $t" -ForegroundColor Cyan }

try {
    Write-Host "Ember installer" -ForegroundColor White

    # Nimbus needs the .NET 8 Desktop Runtime (the SDK includes it).
    $runtimes = ''
    try { $runtimes = (& dotnet --list-runtimes 2>$null) -join "`n" } catch {}
    if ($runtimes -notmatch 'Microsoft\.WindowsDesktop\.App 8\.') {
        Step "Installing the .NET 8 Desktop Runtime (Windows may ask for permission)"
        winget install --id Microsoft.DotNet.DesktopRuntime.8 --exact --silent --accept-package-agreements --accept-source-agreements --disable-interactivity
    }

    Step "Finding the latest release"
    # The newest full release with a Windows build. Releases from before the 0.0.x renumbering (v0.3 to v0.5) are skipped.
    $all = Invoke-RestMethod "https://api.github.com/repos/$repo/releases?per_page=30" -Headers @{ 'User-Agent' = 'Nimbus-Installer' }
    $rel = $all | Where-Object { -not $_.draft -and -not $_.prerelease -and $_.tag_name -notmatch '^v?0\.[3-5]\.' -and ($_.assets | Where-Object name -eq 'Nimbus-win-x64.zip') } |
        Sort-Object { [version](($_.tag_name -replace '^v', '') -replace '-.*$', '') } -Descending | Select-Object -First 1
    if (-not $rel) { throw "No Ember release with a Windows build was found." }
    $zip = $rel.assets | Where-Object name -eq 'Nimbus-win-x64.zip'
    $sha = $rel.assets | Where-Object name -eq 'Nimbus-win-x64.zip.sha256'
    if (-not $zip) { throw "The latest release ($($rel.tag_name)) has no Nimbus-win-x64.zip." }

    Step "Downloading Ember $($rel.tag_name)"
    $tmp = Join-Path $env:TEMP "Nimbus-$($rel.tag_name).zip"
    Invoke-WebRequest $zip.browser_download_url -OutFile $tmp -UseBasicParsing
    if ($sha) {
        $c = (Invoke-WebRequest $sha.browser_download_url -UseBasicParsing).Content
        if ($c -is [byte[]]) { $c = [Text.Encoding]::ASCII.GetString($c) }
        $expected = ($c.Trim() -split '\s+')[0].ToLower()
        $actual = (Get-FileHash $tmp -Algorithm SHA256).Hash.ToLower()
        if ($actual -ne $expected) { throw "The download was damaged (checksum mismatch). Run the installer again." }
    }

    Step "Installing to $app"
    Get-Process Nimbus -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 1
    New-Item -ItemType Directory -Force $root | Out-Null
    if (Test-Path "$app.new") { Remove-Item "$app.new" -Recurse -Force }
    Expand-Archive $tmp "$app.new"
    if (-not (Test-Path "$app.new\Nimbus.exe")) { throw "The package is incomplete." }
    if (Test-Path "$app.old") { Remove-Item "$app.old" -Recurse -Force }
    if (Test-Path $app) { Move-Item $app "$app.old" }
    Move-Item "$app.new" $app
    Remove-Item $tmp -Force

    # Hearth Terminal comes in the package: a Start menu entry for it (Ember keeps it up to date from then on).
    $term = Join-Path $app 'hearth-terminal.exe'
    if (Test-Path $term) {
        try {
            $lnk = Join-Path ([Environment]::GetFolderPath('Programs')) 'Hearth Terminal.lnk'
            $sc = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
            $sc.TargetPath = $term; $sc.WorkingDirectory = $env:USERPROFILE; $sc.IconLocation = "$term,0"
            $sc.Description = 'Hearth Terminal: PowerShell, Command Prompt and WSL'
            $sc.Save()
        } catch { Write-Host "No Start menu entry for Hearth Terminal ($($_.Exception.Message))." -ForegroundColor Yellow }
    }

    # Safety net first: an undo script that's easy to run from Task Manager.
    $restore = @"
@echo off
rem Makes Explorer your Windows shell again and starts it now. Safe to run any time.
reg delete "HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon" /v Shell /f >nul 2>&1
taskkill /im Nimbus.exe /f >nul 2>&1
start "" "%WINDIR%\explorer.exe"
echo Explorer is your shell again.
timeout /t 5 >nul
"@
    Set-Content -Path (Join-Path $env:USERPROFILE 'nimbus-restore.cmd') -Value $restore -Encoding ASCII
    Set-Content -Path (Join-Path $root 'restore-explorer.cmd') -Value $restore -Encoding ASCII

    if (-not $NoShell) {
        Step "Making Ember your shell (your account only)"
        Set-ItemProperty $winlogon -Name Shell -Value "`"$app\Nimbus.exe`" --shell"
    }

    # Beacon (Battle for Middle-earth and more) comes with Ember: its own installer, per user and silent. Ember's
    # Library › Stores › Get Beacon does the same later; a failure here doesn't stop Ember's install.
    if (-not (Test-Path (Join-Path $env:LOCALAPPDATA 'Beacon\beacon.exe'))) {
        try {
            Step "Installing Beacon"
            $bsetup = Join-Path $env:TEMP 'Beacon-x64-setup.exe'
            Invoke-WebRequest -UseBasicParsing 'https://github.com/Dragongamer9/beacon/releases/latest/download/Beacon-x64-setup.exe' -OutFile $bsetup
            Start-Process $bsetup -ArgumentList '/S' -Wait
            Remove-Item $bsetup -ErrorAction SilentlyContinue
        } catch { Write-Host "Beacon wasn't installed ($($_.Exception.Message)). Get it later from Library > Stores." -ForegroundColor Yellow }
    }

    Write-Host ""
    Write-Host "Ember $($rel.tag_name) is installed." -ForegroundColor Green
    if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) {
        # Running inside a Nimbus session (no Explorer): start the new version right away.
        Start-Process "$app\Nimbus.exe" -ArgumentList '--shell'
    } elseif (-not $NoShell) {
        Write-Host "Sign out and back in to start it."
    }
    Write-Host "Undo any time: Ctrl+Shift+Esc > Run new task > %USERPROFILE%\nimbus-restore.cmd"
}
catch {
    Write-Host ""
    Write-Host "Install failed: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Nothing about your Windows shell was changed unless it says so above."
    # If Nimbus was stopped and nothing replaced it in a Nimbus-only session, bring something back.
    if (-not (Get-Process explorer, Nimbus -ErrorAction SilentlyContinue)) {
        if (Test-Path "$app\Nimbus.exe") { Start-Process "$app\Nimbus.exe" -ArgumentList '--shell' } else { Start-Process "$env:WINDIR\explorer.exe" }
    }
}
Start-Sleep -Seconds 8
