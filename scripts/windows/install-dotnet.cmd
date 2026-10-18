@echo off
rem Installs the Microsoft .NET 8 SDK via winget and logs to install.log.
set LOG=%~dp0install.log
echo === %DATE% %TIME% === > "%LOG%"
where winget >> "%LOG%" 2>&1
if errorlevel 1 (
  echo WINGET_MISSING >> "%LOG%"
  exit /b 1
)
winget install --id Microsoft.DotNet.SDK.8 --source winget --exact --silent --accept-package-agreements --accept-source-agreements --disable-interactivity >> "%LOG%" 2>&1
echo INSTALL_EXIT=%ERRORLEVEL% >> "%LOG%"
echo Done. See install.log.
timeout /t 3 >nul
