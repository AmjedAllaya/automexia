param(
    [string]$Binary,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'Application Verifier testing is supported only on Windows.'
}

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrWhiteSpace($Binary)) {
    $Binary = Join-Path $root 'target\debug\automexia.exe'
}
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
    $OutputDirectory = Join-Path $root "target\qa\appverifier-$stamp"
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Application Verifier changes persistent machine settings and must run from an elevated controlled test session.'
}

$appverif = Get-Command appverif.exe -ErrorAction Stop
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    & cargo build -p automexia-terminal --locked
    if ($LASTEXITCODE -ne 0) {
        throw "Could not build the Automexia test binary (exit $LASTEXITCODE)."
    }
}
$Binary = (Resolve-Path -LiteralPath $Binary).Path
if ([IO.Path]::GetFileName($Binary) -ne 'automexia.exe') {
    throw "Refusing to configure Application Verifier for unexpected target $Binary"
}
$target = 'automexia.exe'
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$xmlLog = Join-Path $OutputDirectory 'application-verifier.xml'
$resourceReport = Join-Path $OutputDirectory 'resource-baseline.json'

function Invoke-AppVerifier {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & $appverif.Source @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "appverif $($Arguments -join ' ') failed with exit $LASTEXITCODE"
    }
}

$existing = (& $appverif.Source -query '*' -for $target 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) {
    throw "Could not query existing Application Verifier state for $target"
}
if ($existing -match '(?im)^\s*Test\s+\[[^]]+\]\s+enabled') {
    throw "Application Verifier already has settings for $target; refusing to overwrite maintainer-owned state."
}

$enabled = $false
try {
    # Microsoft's /verify shortcut enables the complete Basics layer, including
    # heap, handle, lock, TLS, exception, and dangerous-API checks.
    Invoke-AppVerifier '/verify' $target
    $enabled = $true
    Invoke-AppVerifier '-stamp' 'log' '-for' $target '-with' 'Stamp=AUTOMEXIA_PHASE0_START'

    & (Join-Path $root 'tests\integration\resize-stress-windows.ps1') `
        -Binary $Binary `
        -ResourceReport $resourceReport `
        -MaximumPrivateBytesGrowth 1610612736 `
        -MaximumWorkingSetGrowth 1610612736
    if ($LASTEXITCODE -ne 0) {
        throw "Native resize/resource stress failed with exit $LASTEXITCODE"
    }

    Invoke-AppVerifier '-stamp' 'log' '-for' $target '-with' 'Stamp=AUTOMEXIA_PHASE0_END'
    Invoke-AppVerifier '-export' 'log' '-for' $target '-with' "To=$xmlLog" `
        'StampFrom=AUTOMEXIA_PHASE0_START' 'StampTo=AUTOMEXIA_PHASE0_END'
    if (-not (Test-Path -LiteralPath $xmlLog -PathType Leaf)) {
        throw 'Application Verifier did not export its XML evidence.'
    }

    # The exported log is private QA evidence. Remove machine/user roots before
    # it can enter an evidence bundle; do not enumerate or capture environment.
    $xml = [IO.File]::ReadAllText($xmlLog)
    $xml = $xml.Replace($root, '<WORKSPACE>')
    if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
        $xml = $xml.Replace($env:USERPROFILE, '<HOME>')
    }
    [IO.File]::WriteAllText($xmlLog, $xml, [Text.UTF8Encoding]::new($false))
    Write-Host "PASS: Application Verifier Basics and native resource stress completed; evidence: $OutputDirectory"
} finally {
    if ($enabled) {
        & $appverif.Source -disable '*' -for $target | Out-Null
        & $appverif.Source -delete settings -for $target | Out-Null
    }
    $remaining = (& $appverif.Source -query '*' -for $target 2>&1 | Out-String)
    if ($remaining -match '(?im)^\s*Test\s+\[[^]]+\]\s+enabled') {
        throw "Application Verifier cleanup failed: settings remain enabled for $target"
    }
}