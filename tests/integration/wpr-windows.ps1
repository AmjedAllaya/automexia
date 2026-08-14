param(
    [string]$Binary,
    [string]$OutputDirectory,
    [ValidateRange(67108864, 4294967296)]
    [int64]$MaximumTraceBytes = 1073741824,
    [switch]$DeleteTraceAfterManifest
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'WPR tracing is supported only on Windows.'
}
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrWhiteSpace($Binary)) {
    $Binary = Join-Path $root 'target\debug\automexia.exe'
}
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
    $OutputDirectory = Join-Path $root "target\qa\wpr-$stamp"
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'WPR native tracing must run from an elevated controlled test session.'
}
$wpr = Get-Command wpr.exe -ErrorAction Stop
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    & cargo build -p automexia-terminal --locked
    if ($LASTEXITCODE -ne 0) { throw "Could not build Automexia (exit $LASTEXITCODE)." }
}
$Binary = (Resolve-Path -LiteralPath $Binary).Path
if ([IO.Path]::GetFileName($Binary) -ne 'automexia.exe') {
    throw "Refusing to trace unexpected target $Binary"
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$trace = Join-Path $OutputDirectory 'automexia-phase0.etl'
$resourceReport = Join-Path $OutputDirectory 'resource-baseline.json'
$started = $false
$stopped = $false
try {
    & $wpr.Source -start GeneralProfile -filemode
    if ($LASTEXITCODE -ne 0) {
        throw 'WPR could not start GeneralProfile; ensure no unrelated recording is active.'
    }
    $started = $true
    & (Join-Path $root 'tests\integration\resize-stress-windows.ps1') `
        -Binary $Binary `
        -ResourceReport $resourceReport
    & $wpr.Source -stop $trace
    if ($LASTEXITCODE -ne 0) { throw "WPR could not stop and save $trace" }
    $stopped = $true

    $traceFile = Get-Item -LiteralPath $trace
    if ($traceFile.Length -gt $MaximumTraceBytes) {
        throw "WPR trace exceeded the $MaximumTraceBytes-byte private-artifact ceiling: $($traceFile.Length)"
    }
    $video = @(Get-CimInstance Win32_VideoController | ForEach-Object {
        [ordered]@{ name = [string]$_.Name; driver_version = [string]$_.DriverVersion }
    })
    $manifest = [ordered]@{
        schema_version = 1
        private_artifact = $true
        bundle_policy = 'ETL is excluded from the default QA ZIP; share only through restricted retention.'
        profile = 'GeneralProfile filemode'
        trace_file = $traceFile.Name
        trace_bytes = [int64]$traceFile.Length
        trace_sha256 = (Get-FileHash -LiteralPath $trace -Algorithm SHA256).Hash.ToLowerInvariant()
        os = [Environment]::OSVersion.VersionString
        architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        gpu = $video
    } | ConvertTo-Json -Depth 6
    [IO.File]::WriteAllText(
        (Join-Path $OutputDirectory 'manifest.json'),
        $manifest,
        [Text.UTF8Encoding]::new($false))
    if ($DeleteTraceAfterManifest) {
        Remove-Item -LiteralPath $trace -Force
        Write-Host "PASS: WPR trace verified and removed after retaining its manifest: $OutputDirectory"
    } else {
        Write-Host "PASS: WPR native trace captured; private evidence: $OutputDirectory"
    }
} finally {
    if ($started -and -not $stopped) {
        & $wpr.Source -cancel | Out-Null
    }
}