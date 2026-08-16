param(
    [Parameter(Mandatory = $true)][string]$MsiPath,
    [Parameter(Mandatory = $true)][string]$PortableZipPath,
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$ExpectedPublisher,
    [string]$PreviousMsiPath
)

$ErrorActionPreference = 'Stop'
$msi = (Resolve-Path -LiteralPath $MsiPath).Path
$portableZip = (Resolve-Path -LiteralPath $PortableZipPath).Path
$previousMsi = if ([string]::IsNullOrWhiteSpace($PreviousMsiPath)) {
    $null
} else {
    (Resolve-Path -LiteralPath $PreviousMsiPath).Path
}
$installRoot = Join-Path $env:ProgramFiles 'Automexia Terminal'
$binary = Join-Path $installRoot 'automexia.exe'
$rioRoot = Join-Path $env:LOCALAPPDATA 'rio'
$rioSentinel = Join-Path $rioRoot 'automexia-package-coexistence.test'
$createdRioRoot = -not (Test-Path -LiteralPath $rioRoot)
$automexiaDataRoot = Join-Path $env:LOCALAPPDATA 'Automexia\Terminal'
$dataSentinel = Join-Path $automexiaDataRoot 'package-upgrade-preservation.test'
$createdDataRoot = -not (Test-Path -LiteralPath $automexiaDataRoot)
$portableRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
    'automexia-portable-{0}' -f [guid]::NewGuid().ToString('N'))
$installed = $false

function Assert-SignedShellResources {
    param([Parameter(Mandatory = $true)][string]$Root)

    $required = @(
        'install-windows.ps1',
        'uninstall-windows.ps1',
        'windows-path-safety.ps1',
        'windows-wsl.ps1',
        'cmd\automexia-ls.ps1',
        'completion\powershell\automexia-completion.ps1',
        'powershell\automexia.format.ps1xml',
        'powershell\automexia.ps1'
    )
    foreach ($relative in $required) {
        $path = Join-Path $Root $relative
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "signed shell resource is missing: $path"
        }
        $signature = Get-AuthenticodeSignature -LiteralPath $path
        if ($signature.Status -ne 'Valid' -or
            $signature.SignerCertificate.Subject -cne $ExpectedPublisher -or
            $null -eq $signature.TimeStamperCertificate) {
            throw "shell resource does not have the expected timestamped publisher: $path"
        }
    }
}

New-Item -ItemType Directory -Force -Path $rioRoot | Out-Null
Set-Content -LiteralPath $rioSentinel -Value 'must survive Automexia install and uninstall'

try {
    $initialMsi = if ($null -ne $previousMsi) { $previousMsi } else { $msi }
    $install = Start-Process msiexec.exe -ArgumentList @('/i', $initialMsi, '/qn', '/norestart') -Wait -PassThru
    if ($install.ExitCode -ne 0) { throw "MSI install failed with $($install.ExitCode)" }
    $installed = $true
    if (-not (Test-Path -LiteralPath $binary)) { throw "installed executable is missing at $binary" }

    New-Item -ItemType Directory -Force -Path $automexiaDataRoot | Out-Null
    Set-Content -LiteralPath $dataSentinel -Value 'must survive Automexia upgrade and uninstall'
    $upgradeArgs = if ($null -ne $previousMsi) {
        @('/i', $msi, '/qn', '/norestart')
    } else {
        @('/i', $msi, '/qn', '/norestart', 'REINSTALL=ALL', 'REINSTALLMODE=vomus')
    }
    $upgrade = Start-Process msiexec.exe -ArgumentList $upgradeArgs -Wait -PassThru
    if ($upgrade.ExitCode -ne 0) { throw "MSI upgrade/repair failed with $($upgrade.ExitCode)" }
    if (-not (Test-Path -LiteralPath $dataSentinel)) { throw 'MSI upgrade removed Automexia user data' }

    $reported = & $binary --version | Out-String
    if ($reported -notmatch [regex]::Escape($Version)) { throw "installed executable reported unexpected version: $reported" }
    $versionInfo = (Get-Item -LiteralPath $binary).VersionInfo
    if ($versionInfo.ProductName -ne 'Automexia Terminal') { throw "installed executable has unexpected product metadata: $($versionInfo.ProductName)" }
    if ($versionInfo.FileDescription -ne 'Automexia Terminal') { throw "installed executable has unexpected description: $($versionInfo.FileDescription)" }
    Add-Type -AssemblyName System.Drawing
    $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($binary)
    if (-not $icon) { throw 'installed executable has no associated application icon' }
    $icon.Dispose()
    if (-not (Test-Path 'Registry::HKEY_LOCAL_MACHINE\Software\Classes\automexia')) { throw 'automexia:// registration is missing' }
    if (-not (Test-Path 'Registry::HKEY_LOCAL_MACHINE\Software\Classes\Directory\shell\AutomexiaTerminal')) { throw 'directory context menu is missing' }
    $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine') -split ';'
    if ($installRoot -notin $machinePath) { throw 'Automexia install directory is missing from the machine PATH' }
    $desktopShortcut = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Automexia Terminal.lnk'
    $startMenuShortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Automexia Terminal\Automexia Terminal.lnk'
    if (-not (Test-Path -LiteralPath $desktopShortcut)) { throw 'desktop shortcut is missing' }
    if (-not (Test-Path -LiteralPath $startMenuShortcut)) { throw 'Start menu shortcut is missing' }
    $signature = Get-AuthenticodeSignature -LiteralPath $binary
    if ($signature.Status -ne 'Valid') { throw "installed executable signature is $($signature.Status)" }
    if ($signature.SignerCertificate.Subject -cne $ExpectedPublisher) {
        throw "installed executable publisher is '$($signature.SignerCertificate.Subject)'"
    }
    if ($null -eq $signature.TimeStamperCertificate) {
        throw 'installed executable has no trusted timestamp'
    }
    Assert-SignedShellResources -Root (Join-Path $installRoot 'shell-integration')

    Expand-Archive -LiteralPath $portableZip -DestinationPath $portableRoot
    $portableBinary = Get-ChildItem -LiteralPath $portableRoot -Recurse -File -Filter 'automexia.exe' |
        Select-Object -First 1
    if (-not $portableBinary) { throw 'portable ZIP does not contain automexia.exe' }
    $portableReported = & $portableBinary.FullName --version | Out-String
    if ($portableReported -notmatch [regex]::Escape($Version)) {
        throw "portable executable reported unexpected version: $portableReported"
    }
    $portableSignature = Get-AuthenticodeSignature -LiteralPath $portableBinary.FullName
    if ($portableSignature.Status -ne 'Valid') {
        throw "portable executable signature is $($portableSignature.Status)"
    }
    if ($portableSignature.SignerCertificate.Subject -cne $ExpectedPublisher) {
        throw "portable executable publisher is '$($portableSignature.SignerCertificate.Subject)'"
    }
    if ($null -eq $portableSignature.TimeStamperCertificate) {
        throw 'portable executable has no trusted timestamp'
    }
    Assert-SignedShellResources -Root (Join-Path $portableRoot 'shell-integration')
}
finally {
    if ($installed) {
        $uninstall = Start-Process msiexec.exe -ArgumentList @('/x', $msi, '/qn', '/norestart') -Wait -PassThru
        if ($uninstall.ExitCode -ne 0) { throw "MSI uninstall failed with $($uninstall.ExitCode)" }
    }
    Remove-Item -LiteralPath $portableRoot -Recurse -Force -ErrorAction SilentlyContinue
}

if (Test-Path -LiteralPath $binary) { throw 'uninstall left the Automexia executable behind' }
if (Test-Path 'Registry::HKEY_LOCAL_MACHINE\Software\Classes\automexia') { throw 'uninstall left the automexia:// registration behind' }
if (Test-Path 'Registry::HKEY_LOCAL_MACHINE\Software\Classes\Directory\shell\AutomexiaTerminal') { throw 'uninstall left the directory context menu behind' }
if (-not (Test-Path -LiteralPath $rioSentinel)) { throw 'Automexia packaging modified Rio user state' }
if (-not (Test-Path -LiteralPath $dataSentinel)) { throw 'uninstall removed Automexia user data' }
Remove-Item -LiteralPath $rioSentinel -Force
Remove-Item -LiteralPath $dataSentinel -Force
if ($createdRioRoot -and -not (Get-ChildItem -LiteralPath $rioRoot -Force)) {
    Remove-Item -LiteralPath $rioRoot -Force
}
if ($createdDataRoot -and -not (Get-ChildItem -LiteralPath $automexiaDataRoot -Force)) {
    Remove-Item -LiteralPath $automexiaDataRoot -Force
}
Write-Host 'PASS: MSI silent install/upgrade/uninstall, timestamped publisher identity, portable identity/signature, PATH, shortcuts, user-data preservation, and Rio coexistence'
