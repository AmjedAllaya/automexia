param([Parameter(Mandatory=$true)][string]$ProjectRoot)
$ErrorActionPreference = "Stop"
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)
$binary = Join-Path $ProjectRoot "dist\AutomexiaTerminal.exe"
if (-not (Test-Path $binary)) { throw "Binary not found. Run BUILD-WINDOWS.ps1 first." }
$ShellIntegration = Join-Path $env:LOCALAPPDATA "Automexia\shell-integration\automexia.ps1"
if (-not (Test-Path -LiteralPath $ShellIntegration)) {
    Write-Warning "Automexia shell integration is not installed. Run .\INSTALL-SHELL-INTEGRATION-WINDOWS.ps1 for the two-row prompt and PowerShell/WSL editor colors."
}
Start-Process -FilePath $binary
