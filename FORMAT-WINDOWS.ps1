param(
    [Parameter(Mandatory=$true)][string]$ProjectRoot,
    [switch]$CheckOnly
)

$ErrorActionPreference = "Stop"
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust/Cargo is required to normalize Automexia Rust source."
}

Push-Location $ProjectRoot
try {
    if (-not $CheckOnly) {
        Write-Host "Normalizing generated Rust source with the pinned rustfmt toolchain..." -ForegroundColor Cyan
        cargo fmt --all
        if ($LASTEXITCODE -ne 0) {
            throw "cargo fmt --all failed"
        }
    }

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        if ($CheckOnly) {
            throw "cargo fmt --check failed; rerun BOOTSTRAP-WINDOWS.ps1 or FORMAT-WINDOWS.ps1 to normalize generated source"
        }
        throw "cargo fmt verification failed after normalization"
    }
} finally {
    Pop-Location
}
