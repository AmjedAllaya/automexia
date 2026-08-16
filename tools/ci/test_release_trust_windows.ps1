param(
    [Parameter(Mandatory = $true)][string]$ArtifactDirectory,
    [Parameter(Mandatory = $true)][string]$ExpectedPublisher,
    [Parameter(Mandatory = $true)][ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$')][string]$Version,
    [Parameter(Mandatory = $true)][string]$EvidencePath,
    [ValidateRange(60, 3600)][int]$ScanTimeoutSeconds = 900,
    [ValidateRange(1, 168)][int]$MaximumSignatureAgeHours = 48,
    [ValidateRange(1, 10000)][int]$MaximumArchiveEntries = 4096
)

$ErrorActionPreference = 'Stop'
$artifactRoot = (Resolve-Path -LiteralPath $ArtifactDirectory).Path
if ([string]::IsNullOrWhiteSpace($ExpectedPublisher)) {
    throw 'ExpectedPublisher must be the exact subject of the approved public code-signing certificate'
}

$packages = @(Get-ChildItem -LiteralPath $artifactRoot -File)
$msiPackages = @($packages | Where-Object Extension -eq '.msi')
$zipPackages = @($packages | Where-Object Extension -eq '.zip')
if ($packages.Count -ne 4 -or $msiPackages.Count -ne 2 -or $zipPackages.Count -ne 2) {
    throw "Windows trust gate expects exactly two MSI and two ZIP packages; found $($packages.Count) files"
}
$expectedPackageNames = @(
    "automexia-terminal-$Version-aarch64-pc-windows-msvc.msi",
    "automexia-terminal-$Version-aarch64-pc-windows-msvc.zip",
    "automexia-terminal-$Version-x86_64-pc-windows-msvc.msi",
    "automexia-terminal-$Version-x86_64-pc-windows-msvc.zip"
)
$observedPackageNames = (@($packages.Name | Sort-Object -CaseSensitive) -join '|')
$requiredPackageNames = (@($expectedPackageNames | Sort-Object -CaseSensitive) -join '|')
if ($observedPackageNames -cne $requiredPackageNames) {
    throw "Windows trust package names do not match release $Version`: $observedPackageNames"
}

$temporaryRoots = [System.Collections.Generic.List[string]]::new()
$signatures = [System.Collections.Generic.List[object]]::new()
$scanJob = $null
$scanRoot = Join-Path ([IO.Path]::GetTempPath()) (
    'automexia-release-scan-{0}' -f [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($scanRoot) | Out-Null
$temporaryRoots.Add($scanRoot)
foreach ($package in $packages) {
    Copy-Item -LiteralPath $package.FullName -Destination $scanRoot
}

function Assert-TrustedSignature {
    param([Parameter(Mandatory = $true)][string]$Path)

    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) {
        throw "Authenticode validation failed for $Path with status $($signature.Status)"
    }
    if ($signature.SignerCertificate.Subject -cne $ExpectedPublisher) {
        throw "Publisher mismatch for $Path`: expected '$ExpectedPublisher', found '$($signature.SignerCertificate.Subject)'"
    }
    if ($null -eq $signature.TimeStamperCertificate) {
        throw "RFC 3161 timestamp is missing for $Path"
    }
    $codeSigningEku = @($signature.SignerCertificate.EnhancedKeyUsageList) |
        Where-Object { $_.ObjectId.Value -eq '1.3.6.1.5.5.7.3.3' }
    if ($codeSigningEku.Count -eq 0) {
        throw "Signer certificate for $Path has no code-signing extended key usage"
    }
    $signatures.Add([ordered]@{
            file = [IO.Path]::GetFileName($Path)
            signer_subject = $signature.SignerCertificate.Subject
            signer_thumbprint = $signature.SignerCertificate.Thumbprint
            timestamp_subject = $signature.TimeStamperCertificate.Subject
        })
}

function Expand-TrustedPortableArchive {
    param([Parameter(Mandatory = $true)][string]$Path)

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $totalExpandedBytes = [int64]0
        $entryCount = 0
        $fileEntries = [System.Collections.Generic.List[string]]::new()
        foreach ($entry in $archive.Entries) {
            $entryCount++
            if ($entryCount -gt $MaximumArchiveEntries) {
                throw "portable ZIP contains more than $MaximumArchiveEntries entries"
            }
            $normalized = $entry.FullName.Replace('\', '/')
            $segments = @($normalized.Split('/', [StringSplitOptions]::RemoveEmptyEntries))
            if ([IO.Path]::IsPathRooted($normalized) -or $normalized.Contains(':') -or $segments -contains '..') {
                throw "portable ZIP contains an unsafe path: $($entry.FullName)"
            }
            $portableName = $normalized
            while ($portableName.StartsWith('./')) { $portableName = $portableName.Substring(2) }
            if ([string]::IsNullOrEmpty($entry.Name)) {
                if (-not [string]::IsNullOrEmpty($portableName)) {
                    throw "portable ZIP contains an unexpected directory: $($entry.FullName)"
                }
                continue
            }
            if ($portableName.Contains('/')) {
                throw "portable ZIP content must be flat: $($entry.FullName)"
            }
            $fileEntries.Add($portableName)
            $totalExpandedBytes += $entry.Length
            if ($totalExpandedBytes -gt 536870912) {
                throw 'portable ZIP expands beyond the 512 MiB release limit'
            }
            if ($entry.CompressedLength -gt 0 -and $entry.Length -gt ($entry.CompressedLength * 200)) {
                throw "portable ZIP entry exceeds the 200:1 expansion-ratio limit: $($entry.FullName)"
            }
        }
        $expectedFiles = @('automexia.exe', 'LICENSE', 'NOTICE.md', 'README.md', 'THIRD_PARTY_NOTICES.md')
        $observedFiles = (@($fileEntries | Sort-Object -CaseSensitive) -join '|')
        $requiredFiles = (@($expectedFiles | Sort-Object -CaseSensitive) -join '|')
        if ($fileEntries.Count -ne $expectedFiles.Count -or $observedFiles -cne $requiredFiles) {
            throw "portable ZIP content mismatch: expected $($expectedFiles -join ', '); found $($fileEntries -join ', ')"
        }
    }
    finally {
        $archive.Dispose()
    }

    $destination = Join-Path ([IO.Path]::GetTempPath()) (
        'automexia-release-trust-{0}' -f [guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory($destination) | Out-Null
    $temporaryRoots.Add($destination)
    [IO.Compression.ZipFile]::ExtractToDirectory($Path, $destination)
    $binary = @(Get-ChildItem -LiteralPath $destination -Recurse -File -Filter 'automexia.exe')
    if ($binary.Count -ne 1) {
        throw 'portable ZIP extraction did not yield exactly one automexia.exe'
    }
    $productVersion = $binary[0].VersionInfo.ProductVersion
    if ($productVersion -cne $Version) {
        throw "portable executable version mismatch: expected '$Version', found '$productVersion'"
    }
    return $binary[0].FullName
}

function Find-DefenderScanner {
    $command = Get-Command 'MpCmdRun.exe' -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $legacy = Join-Path $env:ProgramFiles 'Windows Defender\MpCmdRun.exe'
    if (Test-Path -LiteralPath $legacy) { return $legacy }
    $platformRoot = Join-Path $env:ProgramData 'Microsoft\Windows Defender\Platform'
    $latest = Get-ChildItem -LiteralPath $platformRoot -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName 'MpCmdRun.exe' } |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
    if ($latest) { return $latest }
    throw 'Microsoft Defender MpCmdRun.exe is unavailable on the controlled release runner'
}

try {
    foreach ($msi in $msiPackages) {
        Assert-TrustedSignature -Path $msi.FullName
    }
    $portableIndex = 0
    foreach ($zip in $zipPackages) {
        $portableBinary = Expand-TrustedPortableArchive -Path $zip.FullName
        Assert-TrustedSignature -Path $portableBinary
        $portableIndex++
        Copy-Item -LiteralPath $portableBinary -Destination (
            Join-Path $scanRoot "automexia-portable-$portableIndex.exe")
    }

    $subjects = @($signatures | ForEach-Object signer_subject | Sort-Object -Unique)
    if ($subjects.Count -ne 1) {
        throw "Windows packages were not signed by one publisher identity: $($subjects -join ', ')"
    }

    $defender = Get-MpComputerStatus
    if (-not $defender.AMServiceEnabled -or -not $defender.AntivirusEnabled) {
        throw 'Microsoft Defender antivirus is not active on the controlled release runner'
    }
    $signatureAge = (Get-Date).ToUniversalTime() - $defender.AntivirusSignatureLastUpdated.ToUniversalTime()
    if ($signatureAge.TotalHours -gt $MaximumSignatureAgeHours) {
        throw "Defender security intelligence is $([math]::Round($signatureAge.TotalHours, 1)) hours old"
    }

    $scanner = Find-DefenderScanner
    $startedAt = Get-Date
    $scanJob = Start-Job -ScriptBlock {
        param($Executable, $Target)
        & $Executable -Scan -ScanType 3 -File $Target -DisableRemediation
        [pscustomobject]@{ ExitCode = $LASTEXITCODE }
    } -ArgumentList $scanner, $scanRoot
    if (-not (Wait-Job -Job $scanJob -Timeout $ScanTimeoutSeconds)) {
        Stop-Job -Job $scanJob
        throw "Defender release scan exceeded $ScanTimeoutSeconds seconds"
    }
    $jobOutput = @(Receive-Job -Job $scanJob)
    $scanResult = $jobOutput | Where-Object { $_.PSObject.Properties.Name -contains 'ExitCode' } |
        Select-Object -Last 1
    Remove-Job -Job $scanJob -Force
    $scanJob = $null
    if ($null -eq $scanResult -or $scanResult.ExitCode -ne 0) {
        throw "Defender release scan failed or found an unremediated threat (exit $($scanResult.ExitCode))"
    }
    $scanMilliseconds = [int64]((Get-Date) - $startedAt).TotalMilliseconds

    $evidence = [ordered]@{
        schema = 1
        version = $Version
        scanner = 'Microsoft Defender Antivirus'
        scanner_version = $defender.AMEngineVersion
        security_intelligence_version = $defender.AntivirusSignatureVersion
        security_intelligence_updated_utc = $defender.AntivirusSignatureLastUpdated.ToUniversalTime().ToString('o')
        artifact_count = $packages.Count
        artifact_bytes = [int64](($packages | Measure-Object Length -Sum).Sum)
        artifacts = @($packages | Sort-Object Name | ForEach-Object {
                [ordered]@{
                    name = $_.Name
                    size = [int64]$_.Length
                    sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
                }
            })
        signature_count = $signatures.Count
        publisher = $ExpectedPublisher
        scan_milliseconds = $scanMilliseconds
        scan_timeout_seconds = $ScanTimeoutSeconds
        result = 'pass'
    }
    $evidenceParent = Split-Path -Parent $EvidencePath
    if ($evidenceParent) { New-Item -ItemType Directory -Force -Path $evidenceParent | Out-Null }
    $temporaryEvidence = "$EvidencePath.$PID.tmp"
    $evidence | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $temporaryEvidence -Encoding utf8
    Move-Item -LiteralPath $temporaryEvidence -Destination $EvidencePath -Force
    Write-Host "PASS: $($signatures.Count) timestamped signatures match the approved publisher; Defender scanned $($packages.Count) packages in $scanMilliseconds ms"
}
finally {
    if ($null -ne $scanJob) {
        Stop-Job -Job $scanJob -ErrorAction SilentlyContinue
        Remove-Job -Job $scanJob -Force -ErrorAction SilentlyContinue
    }
    foreach ($temporaryRoot in $temporaryRoots) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
