@echo off
rem Publishes Nimbus (Release) to %LOCALAPPDATA%\Nimbus\app and makes it YOUR Windows shell.
rem Only your account is affected. Undo any time with restore-explorer.cmd.
setlocal
set APP=%LOCALAPPDATA%\Nimbus\app
set LOG=%~dp0install-shell.log
set PATH=%ProgramFiles%\dotnet;%PATH%
echo === %DATE% %TIME% === > "%LOG%"
taskkill /im Nimbus.exe /f >nul 2>&1
cd /d "%~dp0hosts\windows\Nimbus.Host"
echo Building Nimbus...
dotnet publish -c Release -r win-x64 --self-contained false -p:Platform=x64 -o "%APP%" -nologo >> "%LOG%" 2>&1
if errorlevel 1 (
  echo PUBLISH_FAILED >> "%LOG%"
  echo Build failed. Nothing was changed. See install-shell.log
  timeout /t 15 >nul
  exit /b 1
)
if not exist "%APP%\Nimbus.exe" (
  echo PUBLISH_MISSING_EXE >> "%LOG%"
  echo Nimbus.exe is missing after the build. Nothing was changed.
  timeout /t 15 >nul
  exit /b 1
)
rem Safety net first: the restore script goes somewhere easy to type from Task Manager.
copy /y "%~dp0restore-explorer.cmd" "%USERPROFILE%\nimbus-restore.cmd" >nul
copy /y "%~dp0restore-explorer.cmd" "%LOCALAPPDATA%\Nimbus\restore-explorer.cmd" >nul
reg add "HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon" /v Shell /t REG_SZ /d "\"%APP%\Nimbus.exe\" --shell" /f >> "%LOG%" 2>&1
echo INSTALL_EXIT=%ERRORLEVEL% >> "%LOG%"
reg query "HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon" /v Shell >> "%LOG%" 2>&1
echo.
echo Nimbus is now your shell. Sign out and back in to start it.
echo To undo: Ctrl+Shift+Esc, File, Run new task, then: %%USERPROFILE%%\nimbus-restore.cmd
timeout /t 15 >nul
