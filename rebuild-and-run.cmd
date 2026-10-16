@echo off
rem Rebuilds Nimbus, then starts it in a window if the build succeeded.
call "%~dp0build.cmd"
findstr /c:"BUILD_EXIT=0" "%~dp0build.log" >nul
if errorlevel 1 (
  echo Build failed. See build.log.
  pause
  exit /b 1
)
start "" "%~dp0hosts\windows\Nimbus.Host\bin\x64\Debug\net8.0-windows10.0.19041.0\Nimbus.exe" --windowed
