@echo off
rem Builds the test game used to try launching from Nimbus. Output: tools\TestGame\bin\NimbusTestGame.exe
set PATH=%ProgramFiles%\dotnet;%PATH%
cd /d "%~dp0tools\TestGame"
dotnet build -c Release -o bin -nologo > "%~dp0testgame.log" 2>&1
echo EXIT=%ERRORLEVEL% >> "%~dp0testgame.log"
