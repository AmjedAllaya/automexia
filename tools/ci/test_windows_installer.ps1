param(
    [Parameter(Mandatory = $true)][string]$MsiPath,
    [Parameter(Mandatory = $true)][string]$Version
)

$ErrorActionPreference = 'Stop'
$msi = (Resolve-Path -LiteralPath $MsiPath).Path
$installRoot = Join-Path $env:ProgramFiles 'Automexia Terminal'
$binary = Join-Path $installRoot 'automexia.exe'
$rioRoot = Join-Path $env:LOCALAPPDATA 'rio'
$rioSentinel = Join-Path $rioRoot 'automexia-package-coexistence.test'
$createdRioRoot = -not (Test-Path -LiteralPath $rioRoot)

New-Item -ItemType Directory -Force -Path $rioRoot | Out-Null
Set-Content -LiteralPath $rioSentinel -Value 'must survive Automexia install and uninstall'

try {
    $install = Start-Process msiexec.exe -ArgumentList @('/i', $msi, '/qn', '/norestart') -Wait -PassThru
    if ($install.ExitCode -ne 0) { throw "MSI install failed with $($install.ExitCode)" }
    if (-not (Test-Path -LiteralPath $binary)) { throw "installed executable is missing at $binary" }
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
}
finally {
    $uninstall = Start-Process msiexec.exe -ArgumentList @('/x', $msi, '/qn', '/norestart') -Wait -PassThru
    if ($uninstall.ExitCode -ne 0) { throw "MSI uninstall failed with $($uninstall.ExitCode)" }
}

if (Test-Path -LiteralPath $binary) { throw 'uninstall left the Automexia executable behind' }
if (Test-Path 'Registry::HKEY_LOCAL_MACHINE\Software\Classes\automexia') { throw 'uninstall left the automexia:// registration behind' }
if (Test-Path 'Registry::HKEY_LOCAL_MACHINE\Software\Classes\Directory\shell\AutomexiaTerminal') { throw 'uninstall left the directory context menu behind' }
if (-not (Test-Path -LiteralPath $rioSentinel)) { throw 'Automexia packaging modified Rio user state' }
Remove-Item -LiteralPath $rioSentinel -Force
if ($createdRioRoot -and -not (Get-ChildItem -LiteralPath $rioRoot -Force)) {
    Remove-Item -LiteralPath $rioRoot -Force
}
Write-Host 'PASS: MSI install, executable identity/icon, PATH, shortcuts, signature, Rio coexistence, and uninstall'
