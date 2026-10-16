@echo off
rem Builds Nimbus and writes everything to build.log next to this script.
set LOG=%~dp0build.log
cd /d "%~dp0hosts\windows\Nimbus.Host"
set PATH=%ProgramFiles%\dotnet;%PATH%
echo === %DATE% %TIME% === > "%LOG%"
where dotnet >> "%LOG%" 2>&1
if errorlevel 1 (
  echo DOTNET_MISSING >> "%LOG%"
  echo .NET SDK not found. See build.log.
  timeout /t 5 >nul
  exit /b 1
)
echo --- SDKs --- >> "%LOG%"
dotnet --list-sdks >> "%LOG%" 2>&1
echo --- BUILD --- >> "%LOG%"
rem Close a running Nimbus so the build can replace Nimbus.exe
taskkill /im Nimbus.exe /f >nul 2>&1
dotnet build -c Debug -p:Platform=x64 -nologo -clp:NoSummary >> "%LOG%" 2>&1
echo BUILD_EXIT=%ERRORLEVEL% >> "%LOG%"
echo Done. See build.log.
timeout /t 3 >nul
