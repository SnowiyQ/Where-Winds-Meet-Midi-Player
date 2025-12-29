# Elevate and run dependency installation in the repo root.
param(
    [string]$Command = 'bun install',
    [string]$WorkingDirectory = (Join-Path (Split-Path -Parent (Resolve-Path $MyInvocation.MyCommand.Definition)) '..')
)

$scriptBlock = @"
Set-Location -LiteralPath '$WorkingDirectory'
$Command
if (`$LASTEXITCODE -ne 0) {
  Write-Host "Command exited with code `$LASTEXITCODE" -ForegroundColor Red
}
Write-Host ''
Read-Host 'Press Enter to close'
"@

Start-Process -FilePath powershell.exe -Verb RunAs -WorkingDirectory $WorkingDirectory -ArgumentList '-NoProfile', '-NoLogo', '-NoExit', '-Command', $scriptBlock
