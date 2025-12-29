# Starts an elevated PowerShell that runs the provided command in the repo root and keeps the window open.
param(
        [string]$Command = 'npm run tauri-dev',
        [string]$WorkingDirectory = (Join-Path (Split-Path -Parent (Resolve-Path $MyInvocation.MyCommand.Definition)) '..')
)

$scriptBlock = @"
Set-Location -LiteralPath '$WorkingDirectory'
$Command
if (`$LASTEXITCODE -ne 0) {
    Write-Host "Command exited with code `$LASTEXITCODE" -ForegroundColor Red
}
"@

Start-Process -FilePath powershell.exe -Verb RunAs -WorkingDirectory $WorkingDirectory -ArgumentList '-NoProfile', '-NoLogo', '-NoExit', '-Command', $scriptBlock
