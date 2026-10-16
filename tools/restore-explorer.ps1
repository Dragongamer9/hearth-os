# Restores explorer.exe as the shell for the CURRENT user and starts it.
# Run from Task Manager (Ctrl+Shift+Esc > Run new task > powershell) if Nimbus ever leaves you on a black screen:
#   powershell -ExecutionPolicy Bypass -File C:\path\to\restore-explorer.ps1

$key = 'HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Winlogon'
if (Get-ItemProperty -Path $key -Name Shell -ErrorAction SilentlyContinue) {
    Remove-ItemProperty -Path $key -Name Shell
    Write-Host 'Removed per-user Shell override. Explorer is the shell again at next sign-in.'
} else {
    Write-Host 'No per-user Shell override found.'
}
Start-Process "$env:WINDIR\explorer.exe"
