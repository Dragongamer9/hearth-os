@echo off
rem Installs the latest Nimbus release from GitHub and makes it your shell (your account only).
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\install-nimbus.ps1" %*
