param(
    [string]$Binary,
    [string]$OutputDirectory,
    [ValidateRange(1048576, 268435456)]
    [int64]$MaximumVerifierLogBytes = 67108864
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
    & cargo build -p automexia-terminal --locked --features visual-test-hooks
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
$lowResourceXmlLog = Join-Path $OutputDirectory 'application-verifier-low-resource.xml'
$lowResourceReport = Join-Path $OutputDirectory 'resource-low-resource.json'

function Invoke-AppVerifier {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & $appverif.Source @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "appverif $($Arguments -join ' ') failed with exit $LASTEXITCODE"
    }
}

function Protect-AppVerifierLog {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Application Verifier did not export $([IO.Path]::GetFileName($Path))."
    }
    $logFile = Get-Item -LiteralPath $Path
    if ($logFile.Length -gt $MaximumVerifierLogBytes) {
        throw "Application Verifier log exceeded the $MaximumVerifierLogBytes-byte private-artifact ceiling: $($logFile.Length)"
    }

    # Logs remain private. Redact repository/home roots before any summary is
    # retained; never enumerate the process environment or terminal contents.
    $xml = [IO.File]::ReadAllText($Path)
    $xml = $xml.Replace($root, '<WORKSPACE>')
    if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
        $xml = $xml.Replace($env:USERPROFILE, '<HOME>')
    }
    [IO.File]::WriteAllText($Path, $xml, [Text.UTF8Encoding]::new($false))
    if ($xml -match '(?i)\bSeverity\s*=\s*"Error"' -or
        $xml -match '(?i)\bStopCode\s*=\s*"(?:0x)?[1-9a-f][0-9a-f]*"') {
        throw 'Application Verifier reported an enabled-layer failure; inspect the private redacted XML evidence.'
    }
}

function Disable-AppVerifierTarget {
    & $appverif.Source -disable '*' -for $target | Out-Null
    & $appverif.Source -delete settings -for $target | Out-Null
    $script:enabled = $false
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
    Invoke-AppVerifier '-stamp' 'log' '-for' $target '-with' 'Stamp=AUTOMEXIA_S1_BASICS_START'

    & (Join-Path $root 'tests\integration\resize-stress-windows.ps1') `
        -Binary $Binary `
        -ResourceReport $resourceReport `
        -MaximumPrivateBytesGrowth 1610612736 `
        -MaximumWorkingSetGrowth 1610612736
    if ($LASTEXITCODE -ne 0) {
        throw "Native resize/resource stress failed with exit $LASTEXITCODE"
    }

    Invoke-AppVerifier '-stamp' 'log' '-for' $target '-with' 'Stamp=AUTOMEXIA_S1_BASICS_END'
    Invoke-AppVerifier '-export' 'log' '-for' $target '-with' "To=$xmlLog" `
        'StampFrom=AUTOMEXIA_S1_BASICS_START' 'StampTo=AUTOMEXIA_S1_BASICS_END'
    Protect-AppVerifierLog -Path $xmlLog

    # Microsoft requires fault injection to run separately from ordinary
    # Basics coverage. Use a bounded 0.1% probability after a five-second
    # startup grace period, reduce repeated preview cycles, and retain a
    # distinct redacted log/resource report.
    Disable-AppVerifierTarget
    Invoke-AppVerifier '/verify' $target '/faults' '1000' '5000'
    $enabled = $true
    Invoke-AppVerifier '-stamp' 'log' '-for' $target '-with' 'Stamp=AUTOMEXIA_S1_LOW_RESOURCE_START'

    & (Join-Path $root 'tests\integration\resize-stress-windows.ps1') `
        -Binary $Binary `
        -ResourceReport $lowResourceReport `
        -ImagePreviewLifecycleCycles 2 `
        -MaximumPrivateBytesGrowth 1610612736 `
        -MaximumWorkingSetGrowth 1610612736
    if ($LASTEXITCODE -ne 0) {
        throw "Application Verifier low-resource scenario failed with exit $LASTEXITCODE"
    }

    Invoke-AppVerifier '-stamp' 'log' '-for' $target '-with' 'Stamp=AUTOMEXIA_S1_LOW_RESOURCE_END'
    Invoke-AppVerifier '-export' 'log' '-for' $target '-with' "To=$lowResourceXmlLog" `
        'StampFrom=AUTOMEXIA_S1_LOW_RESOURCE_START' 'StampTo=AUTOMEXIA_S1_LOW_RESOURCE_END'
    Protect-AppVerifierLog -Path $lowResourceXmlLog
    Write-Host "PASS: Application Verifier Basics and bounded low-resource phases completed; evidence: $OutputDirectory"
} finally {
    if ($enabled) {
        Disable-AppVerifierTarget
    }
    $remaining = (& $appverif.Source -query '*' -for $target 2>&1 | Out-String)
    if ($remaining -match '(?im)^\s*Test\s+\[[^]]+\]\s+enabled') {
        throw "Application Verifier cleanup failed: settings remain enabled for $target"
    }
}