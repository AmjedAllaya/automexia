param(
    [Parameter(Mandatory=$true)][string]$ProjectRoot,
    [switch]$Run
)

$ErrorActionPreference = "Stop"
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)

$ShellIntegration = Join-Path $env:LOCALAPPDATA "Automexia\shell-integration\automexia.ps1"
if (-not (Test-Path -LiteralPath $ShellIntegration)) {
    Write-Warning "Automexia shell integration is not installed. Run .\INSTALL-SHELL-INTEGRATION-WINDOWS.ps1 for the two-row prompt and PowerShell/WSL editor colors."
}

function Invoke-AutomexiaPythonCheck {
    param(
        [Parameter(Mandatory=$true)][string]$ScriptName,
        [string[]]$ScriptArguments = @()
    )
    $script = Join-Path $PSScriptRoot $ScriptName
    if (Get-Command py -ErrorAction SilentlyContinue) {
        & py -3 $script @ScriptArguments
    } elseif (Get-Command python -ErrorAction SilentlyContinue) {
        & python $script @ScriptArguments
    } else {
        throw "Python 3 is required for Automexia integration verification."
    }
    if ($LASTEXITCODE -ne 0) { throw "$ScriptName failed" }
}

# Cheap architectural/source gates first; deliberately skip tests/release codegen
# during the normal edit-check-run loop.
Invoke-AutomexiaPythonCheck -ScriptName "verify_package.py"
Invoke-AutomexiaPythonCheck -ScriptName "verify_warning_cleanup.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_architecture.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_devops_extension.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_visual_system.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_source_quality.py" -ScriptArguments @($ProjectRoot)

Push-Location $ProjectRoot
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust/Cargo is required."
    }

    Write-Host "Verifying rustfmt-normalized generated source..." -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot "FORMAT-WINDOWS.ps1") -ProjectRoot $ProjectRoot -CheckOnly

    cargo check -p rioterm --all-targets
    if ($LASTEXITCODE -ne 0) { throw "cargo check -p rioterm --all-targets failed" }

    if ($Run) {
        cargo run -p rioterm --bin rio
        if ($LASTEXITCODE -ne 0) { throw "cargo run -p rioterm --bin rio failed" }
    } else {
        Write-Host "Fast development check passed. Use -Run to launch the dev terminal." -ForegroundColor Green
    }
} finally {
    Pop-Location
}
