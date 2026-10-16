@echo off
rem Installs Steam via winget and logs to install-steam.log.
set LOG=%~dp0install-steam.log
echo === %DATE% %TIME% === > "%LOG%"
winget install --id Valve.Steam --source winget --exact --silent --accept-package-agreements --accept-source-agreements --disable-interactivity >> "%LOG%" 2>&1
echo INSTALL_EXIT=%ERRORLEVEL% >> "%LOG%"
