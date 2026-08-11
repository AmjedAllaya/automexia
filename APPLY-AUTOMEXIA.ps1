param([Parameter(Mandatory=$true)][string]$ProjectRoot)
$ErrorActionPreference = "Stop"
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)

function Invoke-AutomexiaPython {
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
        throw "Python 3 is required."
    }
    if ($LASTEXITCODE -ne 0) { throw "$ScriptName failed" }
}

# Direct apply remains intentionally stricter than bootstrap: it refuses a
# dirty checkout. BOOTSTRAP-WINDOWS.ps1 owns the safe resume/--allow-dirty path.
Invoke-AutomexiaPython -ScriptName "apply_automexia.py" -ScriptArguments @($ProjectRoot)

# The patcher is intentionally a pure source transformer. If the pinned Rust
# toolchain is available, normalize its generated output before verification so
# direct-apply users get the same source shape as bootstrap users.
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    & (Join-Path $PSScriptRoot "FORMAT-WINDOWS.ps1") -ProjectRoot $ProjectRoot
} else {
    Write-Warning "Cargo is unavailable; Rust source normalization was skipped."
}

# Never leave a direct caller with an unverified applied tree.
Invoke-AutomexiaPython -ScriptName "verify_warning_cleanup.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPython -ScriptName "verify_architecture.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPython -ScriptName "verify_devops_extension.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPython -ScriptName "verify_visual_system.py" -ScriptArguments @($ProjectRoot)
Invoke-AutomexiaPython -ScriptName "verify_source_quality.py" -ScriptArguments @($ProjectRoot)

Write-Host "Automexia source integration applied and verified: $ProjectRoot" -ForegroundColor Green
