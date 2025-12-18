# Starts an elevated PowerShell that runs the provided command in the repo root.
param(
    [string]$Command = 'npm run tauri-dev',
    [string]$WorkingDirectory = (Join-Path (Split-Path -Parent (Resolve-Path $MyInvocation.MyCommand.Definition)) '..')
)

 $escapedCommand = "cd '$WorkingDirectory'; $Command"
Start-Process -FilePath powershell.exe -ArgumentList '-NoProfile', '-Command', $escapedCommand -Verb RunAs
