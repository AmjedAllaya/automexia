param([Parameter(Mandatory=$true)][string]$ProjectRoot)
$ErrorActionPreference = "Stop"
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)

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

Invoke-AutomexiaPythonCheck -ScriptName "verify_package.py"
Invoke-AutomexiaPythonCheck -ScriptName "verify_patcher.py"
Invoke-AutomexiaPythonCheck -ScriptName "verify_regressions.py"
Invoke-AutomexiaPythonCheck -ScriptName "verify_warning_cleanup.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_architecture.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_devops_extension.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_visual_system.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_source_quality.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPythonCheck -ScriptName "verify_shell_behavior.py"

Push-Location $ProjectRoot
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust/Cargo is required."
    }

    cargo metadata --no-deps --format-version 1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }

    Write-Host "Verifying rustfmt-normalized generated source..." -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot "FORMAT-WINDOWS.ps1") -ProjectRoot $ProjectRoot -CheckOnly

    cargo check -p rioterm --all-targets
    if ($LASTEXITCODE -ne 0) { throw "cargo check -p rioterm --all-targets failed" }

    cargo clippy -p rioterm -p rio-window -p sugarloaf --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy warning gate failed" }

    cargo test -p rioterm -p rio-vt -p rio-backend -p rio-window -p sugarloaf -p teletypewriter
    if ($LASTEXITCODE -ne 0) { throw "terminal-critical cargo tests failed" }

    Write-Host "Automexia source checks passed." -ForegroundColor Green
} finally {
    Pop-Location
}
