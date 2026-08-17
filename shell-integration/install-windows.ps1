[CmdletBinding()]
param(
    [switch]$SkipPowerShell,
    [switch]$SkipCmd,
    [switch]$SkipWsl,
    [switch]$Quiet,
    [switch]$Force,
    [string[]]$PowerShellProfilePathOverride
)

$ErrorActionPreference = 'Stop'
$MarkerStart = '# >>> AUTOMEXIA SHELL INTEGRATION >>>'
$MarkerEnd = '# <<< AUTOMEXIA SHELL INTEGRATION <<<'
$PackageRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
. (Join-Path $PackageRoot 'windows-path-safety.ps1')
. (Join-Path $PackageRoot 'windows-wsl.ps1')
if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
    throw 'LOCALAPPDATA is unavailable; Automexia cannot install its Windows shell integration.'
}
$InstallRoot = Join-Path $env:LOCALAPPDATA 'Automexia\shell-integration'
$StampPath = Join-Path $InstallRoot 'install-state.json'
$script:DetectedWslExecutable = $null
$script:DetectedWslDistributions = @()
$script:DetectedPowerShellProfiles = @()
$ProfileBytesLimit = 1MB
$StateBytesLimit = 256KB

function Assert-AutomexiaRealDirectory([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    if (-not $item.PSIsContainer -or $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing linked or non-directory installation path: $Path"
    }
}

Assert-AutomexiaRealDirectory (Join-Path $env:LOCALAPPDATA 'Automexia')
Assert-AutomexiaRealDirectory $InstallRoot

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
        'windows-path-safety.ps1',
        'windows-wsl.ps1',
        'powershell\automexia.ps1',
        'powershell\automexia.format.ps1xml',
        'cmd\automexia.cmd',
        'cmd\automexia-alias-loader.ps1',
        'cmd\automexia-ls.cmd',
        'cmd\automexia-ls.ps1',
        'bash\automexia.bash',
        'zsh\automexia.zsh',
        'fish\automexia.fish',
        'completion\powershell\automexia-completion.ps1',
        'completion\bash\automexia-completion.bash',
        'completion\zsh\automexia-completion.zsh',
        'completion\fish\automexia-completion.fish',
        'posix\automexia-eza-filter.pl'
    )
    $parts = New-Object System.Collections.Generic.List[string]
    $parts.Add('schema=3')
    $parts.Add("user=$([Environment]::UserName)")
    $parts.Add("comspec=$($env:ComSpec)")
    $parts.Add("skip-powershell=$([bool]$SkipPowerShell)")
    $parts.Add("skip-cmd=$([bool]$SkipCmd)")
    $parts.Add("skip-wsl=$([bool]$SkipWsl)")
    if (-not $SkipPowerShell) {
        $detectedProfiles = New-Object System.Collections.Generic.List[string]
        if ($PowerShellProfilePathOverride) {
            foreach ($profilePath in $PowerShellProfilePathOverride) {
                if (-not [string]::IsNullOrWhiteSpace($profilePath)) {
                    $detectedProfiles.Add([IO.Path]::GetFullPath($profilePath))
                }
            }
            $parts.Add('pwsh=profile-path-override')
        } else {
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
        $parts.Add("$relative=$(Get-AutomexiaFileSha256 $source)")
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

function Get-AutomexiaPowerShellHook {
    $hookPath = (Join-Path $InstallRoot 'automexia.ps1').Replace("'", "''")
    return "if (`$env:TERM_PROGRAM -eq 'Automexia' -or `$env:AUTOMEXIA_SHELL_INTEGRATION -eq '1') { . '$hookPath' }"
}

function Test-MarkedBlockBody([string]$Path, [string]$ExpectedBody) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }
    $item = Get-Item -LiteralPath $Path -Force
    try {
        Assert-AutomexiaSafeProfilePathChain $Path
    } catch {
        return $false
    }
    if ($item.Length -gt $ProfileBytesLimit) {
        return $false
    }
    $text = [IO.File]::ReadAllText($Path)
    $pattern = '(?ms)^# >>> AUTOMEXIA SHELL INTEGRATION >>>\r?\n(?<body>.*?)\r?\n# <<< AUTOMEXIA SHELL INTEGRATION <<<(?:\r?\n)?'
    $matches = [regex]::Matches($text, $pattern)
    return $matches.Count -eq 1 -and $matches[0].Groups['body'].Value -ceq $ExpectedBody
}

function Test-StampedInstall([string]$Fingerprint) {
    if (-not (Test-Path -LiteralPath $StampPath -PathType Leaf)) { return $false }
    try {
        $stateItem = Get-Item -LiteralPath $StampPath -Force
        if ($stateItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) -or
            $stateItem.Length -gt $StateBytesLimit) { return $false }
        $state = [IO.File]::ReadAllText($StampPath) | ConvertFrom-Json
        if ($state.Schema -ne 3 -or $state.Fingerprint -ne $Fingerprint) { return $false }
        foreach ($file in @($state.Files)) {
            if (-not (Test-Path -LiteralPath $file.Path -PathType Leaf)) { return $false }
            $actual = Get-AutomexiaFileSha256 $file.Path
            if ($actual -ne $file.Sha256) { return $false }
        }
        if (-not $SkipPowerShell) {
            $expectedHook = Get-AutomexiaPowerShellHook
            foreach ($profile in @($state.Profiles)) {
                if (-not (Test-MarkedBlockBody ([string]$profile) $expectedHook)) { return $false }
            }
        }
        return $true
    } catch {
        return $false
    }
}

function Copy-Atomically([string]$Source, [string]$Destination) {
    if (Test-Path -LiteralPath $Destination) {
        $destinationItem = Get-Item -LiteralPath $Destination -Force
        if ($destinationItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
            throw "Refusing linked shell-integration destination: $Destination"
        }
    }
    $temporary = "$Destination.automexia-$PID.tmp"
    try {
        Copy-Item -LiteralPath $Source -Destination $temporary -Force
        Move-Item -LiteralPath $temporary -Destination $Destination -Force
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
}

function Write-TextAtomically([string]$Destination, [string]$Text, [Text.Encoding]$Encoding) {
    if (Test-Path -LiteralPath $Destination) {
        $destinationItem = Get-Item -LiteralPath $Destination -Force
        if ($destinationItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
            throw "Refusing linked shell-integration destination: $Destination"
        }
    }
    $temporary = "$Destination.automexia-$PID.tmp"
    try {
        [IO.File]::WriteAllText($temporary, $Text, $Encoding)
        Move-Item -LiteralPath $temporary -Destination $Destination -Force
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
}

function Add-MarkedBlock([string]$Path, [string]$Body) {
    if ([string]::IsNullOrWhiteSpace($Path)) { return }
    $directory = Split-Path -Parent $Path
    Assert-AutomexiaSafeProfilePathChain $directory
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    $directoryItem = Get-Item -LiteralPath $directory -Force
    if (-not $directoryItem.PSIsContainer) { throw "PowerShell profile parent is not a directory: $directory" }
    Assert-AutomexiaSafeProfilePathChain $directory
    $existing = ''
    $encoding = [Text.UTF8Encoding]::new($false)
    $acl = $null
    if (Test-Path -LiteralPath $Path) {
        $item = Get-Item -LiteralPath $Path -Force
        Assert-AutomexiaSafeProfilePathChain $Path
        if (-not $item.PSIsContainer -and $item.Length -gt $ProfileBytesLimit) {
            throw "PowerShell profile exceeds the 1 MiB safety ceiling: $Path"
        }
        if ($item.PSIsContainer) { throw "PowerShell profile is not a regular file: $Path" }
        $bytes = [IO.File]::ReadAllBytes($Path)
        if ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
            $encoding = [Text.Encoding]::Unicode
        } elseif ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFE -and $bytes[1] -eq 0xFF) {
            $encoding = [Text.Encoding]::BigEndianUnicode
        } elseif ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
            $encoding = [Text.UTF8Encoding]::new($true)
        }
        $existing = [IO.File]::ReadAllText($Path, $encoding)
        try { $acl = Get-Acl -LiteralPath $Path } catch {}
    }
    $starts = ([regex]::Matches($existing, [regex]::Escape($MarkerStart))).Count
    $ends = ([regex]::Matches($existing, [regex]::Escape($MarkerEnd))).Count
    if (($starts -ne 0 -or $ends -ne 0) -and ($starts -ne 1 -or $ends -ne 1)) {
        throw "Malformed Automexia managed block; profile left unchanged: $Path"
    }
    $newline = if ($existing.Contains("`r`n")) { "`r`n" } else { [Environment]::NewLine }
    if ($starts -eq 1) {
        $pattern = '(?ms)^# >>> AUTOMEXIA SHELL INTEGRATION >>>\r?\n.*?^# <<< AUTOMEXIA SHELL INTEGRATION <<<\r?\n?'
        if ([regex]::Matches($existing, $pattern).Count -ne 1) {
            throw "Malformed or reversed Automexia managed block; profile left unchanged: $Path"
        }
        $existing = [regex]::Replace($existing, $pattern, '')
    }
    $updated = $existing.TrimEnd([char[]]"`r`n") + $newline + $MarkerStart + $newline + $Body + $newline + $MarkerEnd + $newline
    $temporary = "$Path.automexia-$PID.tmp"
    try {
        [IO.File]::WriteAllText($temporary, $updated, $encoding)
        if ($null -ne $acl) { Set-Acl -LiteralPath $temporary -AclObject $acl }
        Move-Item -LiteralPath $temporary -Destination $Path -Force
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
}

$fingerprint = Get-SourceFingerprint
if (-not $Force -and (Test-StampedInstall $fingerprint)) {
    Write-InstallMessage 'Automexia shell integration is already current.' Green
    return
}

New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
Assert-AutomexiaRealDirectory $InstallRoot
$installedFiles = New-Object System.Collections.Generic.List[string]
$profiles = New-Object System.Collections.Generic.List[string]

if (-not $SkipPowerShell) {
    $PowerShellIntegration = Join-Path $InstallRoot 'automexia.ps1'
    $PowerShellFormat = Join-Path $InstallRoot 'automexia.format.ps1xml'
    $PowerShellCompletion = Join-Path $InstallRoot 'automexia-completion.ps1'
    Copy-Atomically (Join-Path $PackageRoot 'powershell\automexia.ps1') $PowerShellIntegration
    Copy-Atomically (Join-Path $PackageRoot 'powershell\automexia.format.ps1xml') $PowerShellFormat
    Copy-Atomically (Join-Path $PackageRoot 'completion\powershell\automexia-completion.ps1') $PowerShellCompletion
    $installedFiles.Add($PowerShellIntegration)
    $installedFiles.Add($PowerShellFormat)
    $installedFiles.Add($PowerShellCompletion)

    # Use each engine's real profile path. Documents may be redirected to
    # OneDrive, so hard-coding $HOME\Documents is not reliable. Automexia also
    # sources the LocalAppData script directly for its initial shell; profile
    # hooks cover nested PowerShell sessions created inside the terminal.
    foreach ($profilePath in $script:DetectedPowerShellProfiles) {
        $profiles.Add($profilePath)
    }
    $hook = Get-AutomexiaPowerShellHook
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
    Write-TextAtomically $cmdIntegration $cmdSource ([Text.Encoding]::ASCII)
    $cmdListLauncher = [IO.File]::ReadAllText((Join-Path $cmdSourceRoot 'automexia-ls.cmd'), [Text.Encoding]::UTF8)
    $cmdListLauncher = [regex]::Replace($cmdListLauncher, "\r?\n", "`r`n")
    $cmdListPath = Join-Path $InstallRoot 'automexia-ls.cmd'
    $cmdListPowerShell = Join-Path $InstallRoot 'automexia-ls.ps1'
    $cmdAliasLoader = Join-Path $InstallRoot 'automexia-alias-loader.ps1'
    Write-TextAtomically $cmdListPath $cmdListLauncher ([Text.Encoding]::ASCII)
    Copy-Atomically (Join-Path $cmdSourceRoot 'automexia-ls.ps1') $cmdListPowerShell
    Copy-Atomically (Join-Path $cmdSourceRoot 'automexia-alias-loader.ps1') $cmdAliasLoader
    $installedFiles.Add($cmdIntegration)
    $installedFiles.Add($cmdListPath)
    $installedFiles.Add($cmdListPowerShell)
    $installedFiles.Add($cmdAliasLoader)
    Write-InstallMessage "Command Prompt integration installed: $cmdIntegration"
}

if (-not $SkipWsl -and $script:DetectedWslExecutable -and $script:DetectedWslDistributions.Count -gt 0) {
    # Install Bash, Zsh, and Fish hooks in one non-login WSL invocation. Source bytes
    # travel as base64 because Windows PowerShell 5.1 otherwise corrupts UTF-8.
    $bashPath = Join-Path $PackageRoot 'bash\automexia.bash'
    $zshPath = Join-Path $PackageRoot 'zsh\automexia.zsh'
    $fishPath = Join-Path $PackageRoot 'fish\automexia.fish'
    $bashCompletionPath = Join-Path $PackageRoot 'completion\bash\automexia-completion.bash'
    $zshCompletionPath = Join-Path $PackageRoot 'completion\zsh\automexia-completion.zsh'
    $fishCompletionPath = Join-Path $PackageRoot 'completion\fish\automexia-completion.fish'
    $ezaFilterPath = Join-Path $PackageRoot 'posix\automexia-eza-filter.pl'
    $bash = [IO.File]::ReadAllText($bashPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $zsh = [IO.File]::ReadAllText($zshPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $fish = [IO.File]::ReadAllText($fishPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $bashCompletion = [IO.File]::ReadAllText($bashCompletionPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $zshCompletion = [IO.File]::ReadAllText($zshCompletionPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $fishCompletion = [IO.File]::ReadAllText($fishCompletionPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $ezaFilter = [IO.File]::ReadAllText($ezaFilterPath, [Text.Encoding]::UTF8).TrimEnd([char[]]"`r`n")
    $payload = @"
set -eu
umask 077
cfg="`${AUTOMEXIA_CONFIG_HOME:-`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia}"
fish_cfg="`${XDG_CONFIG_HOME:-`$HOME/.config}/fish/conf.d"
case "`$cfg" in /*) ;; *) printf 'absolute Automexia config root required\n' >&2; exit 1;; esac
case "`$fish_cfg" in /*) ;; *) printf 'absolute Fish config root required\n' >&2; exit 1;; esac
[ ! -L "`$cfg" ] || { printf 'linked Automexia config root refused\n' >&2; exit 1; }
[ ! -L "`$fish_cfg" ] || { printf 'linked Fish config root refused\n' >&2; exit 1; }
mkdir -p "`$cfg" "`$fish_cfg"
suffix=".automexia-`$$.tmp"
cleanup_install() {
  rm -f "`$cfg/shell-integration.bash`$suffix" "`$cfg/shell-integration.zsh`$suffix" \
    "`$cfg/automexia-completion.bash`$suffix" "`$cfg/automexia-completion.zsh`$suffix" \
    "`$cfg/automexia-eza-filter.pl`$suffix" "`$fish_cfg/automexia.fish`$suffix" \
    "`$fish_cfg/automexia-completion.fish`$suffix" \
    "`$HOME/.bashrc`$suffix" "`$HOME/.zshrc`$suffix"
}
trap cleanup_install EXIT HUP INT TERM
cat > "`$cfg/shell-integration.bash`$suffix" <<'AUTOMEXIA_BASH_EOF'
$bash
AUTOMEXIA_BASH_EOF
cat > "`$cfg/shell-integration.zsh`$suffix" <<'AUTOMEXIA_ZSH_EOF'
$zsh
AUTOMEXIA_ZSH_EOF
cat > "`$cfg/automexia-completion.bash`$suffix" <<'AUTOMEXIA_BASH_COMPLETION_EOF'
$bashCompletion
AUTOMEXIA_BASH_COMPLETION_EOF
cat > "`$cfg/automexia-completion.zsh`$suffix" <<'AUTOMEXIA_ZSH_COMPLETION_EOF'
$zshCompletion
AUTOMEXIA_ZSH_COMPLETION_EOF
cat > "`$fish_cfg/automexia.fish`$suffix" <<'AUTOMEXIA_FISH_EOF'
$fish
AUTOMEXIA_FISH_EOF
cat > "`$fish_cfg/automexia-completion.fish`$suffix" <<'AUTOMEXIA_FISH_COMPLETION_EOF'
$fishCompletion
AUTOMEXIA_FISH_COMPLETION_EOF
cat > "`$cfg/automexia-eza-filter.pl`$suffix" <<'AUTOMEXIA_EZA_FILTER_EOF'
$ezaFilter
AUTOMEXIA_EZA_FILTER_EOF
for file in shell-integration.bash shell-integration.zsh automexia-completion.bash automexia-completion.zsh automexia-eza-filter.pl; do
  chmod 0644 "`$cfg/`$file`$suffix"
  mv -f "`$cfg/`$file`$suffix" "`$cfg/`$file"
done
for file in automexia.fish automexia-completion.fish; do
  chmod 0644 "`$fish_cfg/`$file`$suffix"
  mv -f "`$fish_cfg/`$file`$suffix" "`$fish_cfg/`$file"
done
append_block() {
  file=`$1
  source_line=`$2
  [ ! -L "`$file" ] || { printf 'linked shell profile refused: %s\n' "`$file" >&2; exit 1; }
  [ ! -e "`$file" ] || [ -f "`$file" ] || { printf 'non-file shell profile refused: %s\n' "`$file" >&2; exit 1; }
  [ ! -f "`$file" ] || [ "`$(wc -c <"`$file")" -le 1048576 ] || { printf 'shell profile exceeds 1 MiB: %s\n' "`$file" >&2; exit 1; }
  starts=0
  ends=0
  if [ -f "`$file" ]; then
    starts=`$(grep -Fxc '$MarkerStart' "`$file" 2>/dev/null || true); starts=`${starts:-0}
    ends=`$(grep -Fxc '$MarkerEnd' "`$file" 2>/dev/null || true); ends=`${ends:-0}
  fi
  if [ "`$starts" -eq 1 ] && [ "`$ends" -eq 1 ]; then
    start_line=`$(grep -Fn '$MarkerStart' "`$file" | cut -d: -f1)
    end_line=`$(grep -Fn '$MarkerEnd' "`$file" | cut -d: -f1)
    [ "`$start_line" -lt "`$end_line" ] || { printf 'reversed Automexia profile markers: %s\n' "`$file" >&2; exit 1; }
  else
    [ "`$starts" -eq 0 ] && [ "`$ends" -eq 0 ] || { printf 'malformed Automexia profile markers: %s\n' "`$file" >&2; exit 1; }
  fi
  tmp="`$file`$suffix"
  if [ "`$starts" -eq 1 ]; then
    permissions=`$(stat -c '%a' "`$file" 2>/dev/null || stat -f '%Lp' "`$file")
    awk -v start='$MarkerStart' -v end='$MarkerEnd' '`$0 == start {skip=1;next} `$0 == end {skip=0;next} !skip {print}' "`$file" >"`$tmp"
    chmod "`$permissions" "`$tmp"
  elif [ -f "`$file" ]; then cp -p "`$file" "`$tmp"; else : >"`$tmp"; fi
  printf '\n$MarkerStart\n%s\n$MarkerEnd\n' "`$source_line" >> "`$tmp"
  mv -f "`$tmp" "`$file"
}
append_block "`$HOME/.bashrc" '[ -r "`${AUTOMEXIA_CONFIG_HOME:-`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia}/shell-integration.bash" ] && . "`${AUTOMEXIA_CONFIG_HOME:-`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia}/shell-integration.bash"'
append_block "`$HOME/.zshrc" '[ -r "`${AUTOMEXIA_CONFIG_HOME:-`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia}/shell-integration.zsh" ] && . "`${AUTOMEXIA_CONFIG_HOME:-`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia}/shell-integration.zsh"'
printf 'AUTOMEXIA_WSL_INTEGRATION_OK\n'
"@
    foreach ($distribution in $script:DetectedWslDistributions) {
        $wslResult = Invoke-AutomexiaWslScript `
            $script:DetectedWslExecutable $distribution $payload
        $stdoutLines = @($wslResult.Stdout -split '\r?\n' | Where-Object { $_ })
        if ($wslResult.ExitCode -ne 0 -or
            $stdoutLines -notcontains 'AUTOMEXIA_WSL_INTEGRATION_OK') {
            $detail = @($wslResult.Stdout, $wslResult.Stderr) |
                Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
            throw "WSL shell integration install failed for ${distribution}: $($detail -join [Environment]::NewLine)"
        }
    }
    Write-InstallMessage "WSL Bash/Zsh/Fish integration and completion adapters installed for $($script:DetectedWslDistributions.Count) user distribution(s)."
}

$fileState = foreach ($path in $installedFiles) {
    [ordered]@{
        Path = $path
        Sha256 = Get-AutomexiaFileSha256 $path
    }
}
$state = [ordered]@{
    Schema = 3
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
