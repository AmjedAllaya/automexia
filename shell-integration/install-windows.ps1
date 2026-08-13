[CmdletBinding()]
param(
    [switch]$SkipPowerShell,
    [switch]$SkipCmd,
    [switch]$SkipWsl,
    [switch]$Quiet,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$MarkerStart = '# >>> AUTOMEXIA SHELL INTEGRATION >>>'
$MarkerEnd = '# <<< AUTOMEXIA SHELL INTEGRATION <<<'
$PackageRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
    throw 'LOCALAPPDATA is unavailable; Automexia cannot install its Windows shell integration.'
}
$InstallRoot = Join-Path $env:LOCALAPPDATA 'Automexia\shell-integration'
$StampPath = Join-Path $InstallRoot 'install-state.json'
$script:DetectedWslExecutable = $null
$script:DetectedWslDistributions = @()
$script:DetectedPowerShellProfiles = @()

function Write-InstallMessage([string]$Message, [ConsoleColor]$Color = [ConsoleColor]::Cyan) {
    if (-not $Quiet) { Write-Host $Message -ForegroundColor $Color }
}

function Get-TextSha256([string]$Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Get-SourceFingerprint {
    $relativeSources = @(
        'install-windows.ps1',
        'powershell\automexia.ps1',
        'powershell\automexia.format.ps1xml',
        'cmd\automexia.cmd',
        'cmd\automexia-ls.cmd',
        'cmd\automexia-ls.ps1',
        'bash\automexia.bash',
        'zsh\automexia.zsh',
        'posix\automexia-eza-filter.pl'
    )
    $parts = New-Object System.Collections.Generic.List[string]
    $parts.Add('schema=2')
    $parts.Add("user=$([Environment]::UserName)")
    $parts.Add("comspec=$($env:ComSpec)")
    $parts.Add("skip-powershell=$([bool]$SkipPowerShell)")
    $parts.Add("skip-cmd=$([bool]$SkipCmd)")
    $parts.Add("skip-wsl=$([bool]$SkipWsl)")
    if (-not $SkipPowerShell) {
        $detectedProfiles = New-Object System.Collections.Generic.List[string]
        if ($PROFILE -and $PROFILE.CurrentUserCurrentHost) {
            $detectedProfiles.Add([string]$PROFILE.CurrentUserCurrentHost)
        } elseif ($PROFILE) {
            $detectedProfiles.Add([string]$PROFILE)
        }
        $pwsh = Get-Command pwsh.exe -ErrorAction SilentlyContinue
        $parts.Add("pwsh=$($pwsh.Source)")
        if ($pwsh) {
            try {
                $pwshProfile = & $pwsh.Source -NoLogo -NoProfile -Command '$PROFILE.CurrentUserCurrentHost' 2>$null | Select-Object -First 1
                if ($pwshProfile) { $detectedProfiles.Add([string]$pwshProfile) }
            } catch {}
        }
        $script:DetectedPowerShellProfiles = @($detectedProfiles | Select-Object -Unique)
        foreach ($profilePath in $script:DetectedPowerShellProfiles) {
            $parts.Add("profile=$profilePath")
        }
    }
    foreach ($relative in $relativeSources) {
        $source = Join-Path $PackageRoot $relative
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Required shell-integration source is missing: $source"
        }
        $parts.Add("$relative=$((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant())")
    }
    if (-not $SkipWsl) {
        $wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
        $parts.Add("wsl=$($wsl.Source)")
        if ($wsl) {
            $rawDistributions = @(& $wsl.Source --list --quiet 2>$null)
            if ($LASTEXITCODE -ne 0) {
                throw 'WSL is installed, but its distribution list could not be read.'
            }
            $script:DetectedWslExecutable = $wsl.Source
            $script:DetectedWslDistributions = @(
                $rawDistributions |
                    ForEach-Object { ([string]$_).Replace([string][char]0, '').Trim() } |
                    Where-Object {
                        $_ -and $_ -notmatch '^(?i:docker-desktop(?:-data)?)$'
                    }
            )
            $parts.Add("wsl-distributions=$($script:DetectedWslDistributions -join '|')")
        }
    }
    Get-TextSha256 ($parts -join "`n")
}

function Test-Marker([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }
    return (Get-Content -LiteralPath $Path -Raw).Contains($MarkerStart)
}

function Test-StampedInstall([string]$Fingerprint) {
    if (-not (Test-Path -LiteralPath $StampPath -PathType Leaf)) { return $false }
    try {
        $state = Get-Content -LiteralPath $StampPath -Raw | ConvertFrom-Json
        if ($state.Schema -ne 2 -or $state.Fingerprint -ne $Fingerprint) { return $false }
        foreach ($file in @($state.Files)) {
            if (-not (Test-Path -LiteralPath $file.Path -PathType Leaf)) { return $false }
            $actual = (Get-FileHash -LiteralPath $file.Path -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($actual -ne $file.Sha256) { return $false }
        }
        if (-not $SkipPowerShell) {
            foreach ($profile in @($state.Profiles)) {
                if (-not (Test-Marker ([string]$profile))) { return $false }
            }
        }
        return $true
    } catch {
        return $false
    }
}

function Copy-Atomically([string]$Source, [string]$Destination) {
    $temporary = "$Destination.automexia-$PID.tmp"
    Copy-Item -LiteralPath $Source -Destination $temporary -Force
    Move-Item -LiteralPath $temporary -Destination $Destination -Force
}

function Write-TextAtomically([string]$Destination, [string]$Text, [Text.Encoding]$Encoding) {
    $temporary = "$Destination.automexia-$PID.tmp"
    [IO.File]::WriteAllText($temporary, $Text, $Encoding)
    Move-Item -LiteralPath $temporary -Destination $Destination -Force
}

function Add-MarkedBlock([string]$Path, [string]$Body) {
    if ([string]::IsNullOrWhiteSpace($Path)) { return }
    $existing = if (Test-Path -LiteralPath $Path) { Get-Content -LiteralPath $Path -Raw } else { '' }
    if ($existing.Contains($MarkerStart)) { return }
    $directory = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    Add-Content -LiteralPath $Path -Value "`r`n$MarkerStart`r`n$Body`r`n$MarkerEnd`r`n" -Encoding UTF8
}

$fingerprint = Get-SourceFingerprint
if (-not $Force -and (Test-StampedInstall $fingerprint)) {
    Write-InstallMessage 'Automexia shell integration is already current.' Green
    return
}

New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
$installedFiles = New-Object System.Collections.Generic.List[string]
$profiles = New-Object System.Collections.Generic.List[string]

if (-not $SkipPowerShell) {
    $PowerShellIntegration = Join-Path $InstallRoot 'automexia.ps1'
    $PowerShellFormat = Join-Path $InstallRoot 'automexia.format.ps1xml'
    Copy-Atomically (Join-Path $PackageRoot 'powershell\automexia.ps1') $PowerShellIntegration
    Copy-Atomically (Join-Path $PackageRoot 'powershell\automexia.format.ps1xml') $PowerShellFormat
    $installedFiles.Add($PowerShellIntegration)
    $installedFiles.Add($PowerShellFormat)

    # Use each engine's real profile path. Documents may be redirected to
    # OneDrive, so hard-coding $HOME\Documents is not reliable. Automexia also
    # sources the LocalAppData script directly for its initial shell; profile
    # hooks cover nested PowerShell sessions created inside the terminal.
    foreach ($profilePath in $script:DetectedPowerShellProfiles) {
        $profiles.Add($profilePath)
    }
    $hookPath = $PowerShellIntegration.Replace("'", "''")
    $hook = "if (`$env:TERM_PROGRAM -eq 'Automexia' -or `$env:AUTOMEXIA_SHELL_INTEGRATION -eq '1') { . '$hookPath' }"
    foreach ($profilePath in ($profiles | Select-Object -Unique)) {
        Add-MarkedBlock $profilePath $hook
    }
    Write-InstallMessage "PowerShell integration installed: $PowerShellIntegration"
}

if (-not $SkipCmd) {
    $cmdSourceRoot = Join-Path $PackageRoot 'cmd'
    $cmdIntegration = Join-Path $InstallRoot 'automexia.cmd'
    $cmdSource = [IO.File]::ReadAllText((Join-Path $cmdSourceRoot 'automexia.cmd'), [Text.Encoding]::UTF8)
    $cmdUser = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([Environment]::UserName))
    $cmdExecutable = if ($env:ComSpec) { $env:ComSpec } else { Join-Path $env:SystemRoot 'System32\cmd.exe' }
    $cmdPath = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($cmdExecutable))
    $cmdSource = $cmdSource.Replace('__AUTOMEXIA_CMD_USER_BASE64__', $cmdUser)
    $cmdSource = $cmdSource.Replace('__AUTOMEXIA_CMD_PATH_BASE64__', $cmdPath)
    $cmdSource = [regex]::Replace($cmdSource, "\r?\n", "`r`n")
    Write-TextAtomically $cmdIntegration $cmdSource ([Text.UTF8Encoding]::new($true))
    $cmdListLauncher = [IO.File]::ReadAllText((Join-Path $cmdSourceRoot 'automexia-ls.cmd'), [Text.Encoding]::UTF8)
    $cmdListLauncher = [regex]::Replace($cmdListLauncher, "\r?\n", "`r`n")
    $cmdListPath = Join-Path $InstallRoot 'automexia-ls.cmd'
    $cmdListPowerShell = Join-Path $InstallRoot 'automexia-ls.ps1'
    Write-TextAtomically $cmdListPath $cmdListLauncher ([Text.Encoding]::ASCII)
    Copy-Atomically (Join-Path $cmdSourceRoot 'automexia-ls.ps1') $cmdListPowerShell
    $installedFiles.Add($cmdIntegration)
    $installedFiles.Add($cmdListPath)
    $installedFiles.Add($cmdListPowerShell)
    Write-InstallMessage "Command Prompt integration installed: $cmdIntegration"
}

if (-not $SkipWsl -and $script:DetectedWslExecutable -and $script:DetectedWslDistributions.Count -gt 0) {
    # Install Bash and Zsh hooks in one non-login WSL invocation. Source bytes
    # travel as base64 because Windows PowerShell 5.1 otherwise corrupts UTF-8.
    $bashPath = Join-Path $PackageRoot 'bash\automexia.bash'
    $zshPath = Join-Path $PackageRoot 'zsh\automexia.zsh'
    $ezaFilterPath = Join-Path $PackageRoot 'posix\automexia-eza-filter.pl'
    $bash = [IO.File]::ReadAllText($bashPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $zsh = [IO.File]::ReadAllText($zshPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $ezaFilter = [IO.File]::ReadAllText($ezaFilterPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $payload = @"
set -eu
cfg="`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia"
mkdir -p "`$cfg"
cat > "`$cfg/shell-integration.bash" <<'AUTOMEXIA_BASH_EOF'
$bash
AUTOMEXIA_BASH_EOF
cat > "`$cfg/shell-integration.zsh" <<'AUTOMEXIA_ZSH_EOF'
$zsh
AUTOMEXIA_ZSH_EOF
cat > "`$cfg/automexia-eza-filter.pl" <<'AUTOMEXIA_EZA_FILTER_EOF'
$ezaFilter
AUTOMEXIA_EZA_FILTER_EOF
chmod 0644 "`$cfg/automexia-eza-filter.pl"
append_block() {
  file=`$1
  source_line=`$2
  touch "`$file"
  if ! grep -Fq '$MarkerStart' "`$file" 2>/dev/null; then
    printf '\n$MarkerStart\n%s\n$MarkerEnd\n' "`$source_line" >> "`$file"
  fi
}
append_block "`$HOME/.bashrc" '[ -r "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.bash" ] && . "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.bash"'
append_block "`$HOME/.zshrc" '[ -r "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.zsh" ] && . "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.zsh"'
printf 'AUTOMEXIA_WSL_INTEGRATION_OK\n'
"@
    $payloadBytes = [Text.Encoding]::UTF8.GetBytes($payload)
    $payloadBase64 = [Convert]::ToBase64String($payloadBytes)
    $decodeCommand = "printf '%s' '$payloadBase64' | base64 -d | sh"
    foreach ($distribution in $script:DetectedWslDistributions) {
        $result = & $script:DetectedWslExecutable --distribution $distribution --exec sh -c $decodeCommand 2>&1
        if ($LASTEXITCODE -ne 0 -or ($result -notcontains 'AUTOMEXIA_WSL_INTEGRATION_OK')) {
            throw "WSL shell integration install failed for ${distribution}: $($result -join [Environment]::NewLine)"
        }
    }
    Write-InstallMessage "WSL Bash/Zsh integration installed for $($script:DetectedWslDistributions.Count) user distribution(s)."
}

$fileState = foreach ($path in $installedFiles) {
    [ordered]@{
        Path = $path
        Sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}
$state = [ordered]@{
    Schema = 2
    Fingerprint = $fingerprint
    InstalledAtUtc = [DateTime]::UtcNow.ToString('o')
    Files = @($fileState)
    Profiles = @($profiles | Select-Object -Unique)
}
$stateTemporary = "$StampPath.automexia-$PID.tmp"
[IO.File]::WriteAllText(
    $stateTemporary,
    ($state | ConvertTo-Json -Depth 4),
    [Text.UTF8Encoding]::new($false)
)
Move-Item -LiteralPath $stateTemporary -Destination $StampPath -Force
Write-InstallMessage 'Automexia shell integration is ready for the next launch.' Green
