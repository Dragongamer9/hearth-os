@echo off
rem Starts the built Nimbus in a normal window (pass no args to the exe for fullscreen).
start "" "%~dp0..\..\hosts\windows\Nimbus.Host\bin\x64\Debug\net8.0-windows10.0.19041.0\Nimbus.exe" --windowed
