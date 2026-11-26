@echo off
cd /d "%~dp0"
if not exist "assets\neural\malecns-v1.0.bin" (
  echo The official MaleCNS runtime graph is missing.
  echo See README.md for the one-time official download and import commands.
  pause
  exit /b 1
)
if exist "bin\swarmstrike.exe" (
  start "Swarmstrike" "bin\swarmstrike.exe"
  exit /b
)
if exist "target\debug\swarmstrike.exe" (
  start "Swarmstrike" "target\debug\swarmstrike.exe"
  exit /b
)
cargo run --locked
if errorlevel 1 pause
