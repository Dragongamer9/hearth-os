@echo off
rem Makes Explorer your Windows shell again and starts it now. Safe to run any time.
rem From a black screen: Ctrl+Shift+Esc, File, Run new task, %USERPROFILE%\nimbus-restore.cmd
reg delete "HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon" /v Shell /f >nul 2>&1
taskkill /im Nimbus.exe /f >nul 2>&1
start "" "%WINDIR%\explorer.exe"
echo Explorer is your shell again.
timeout /t 5 >nul
