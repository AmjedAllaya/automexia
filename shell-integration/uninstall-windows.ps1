[CmdletBinding()]
param(
    [switch]$KeepFiles,
    [switch]$SkipWsl,
    [string[]]$PowerShellProfilePathOverride
)
$ErrorActionPreference = 'Stop'
$MarkerStart = '# >>> AUTOMEXIA SHELL INTEGRATION >>>'
$MarkerEnd = '# <<< AUTOMEXIA SHELL INTEGRATION <<<'
$InstallRoot = Join-Path $env:LOCALAPPDATA 'Automexia\shell-integration'
$StatePath = Join-Path $InstallRoot 'install-state.json'
$PackageRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
. (Join-Path $PackageRoot 'windows-path-safety.ps1')
. (Join-Path $PackageRoot 'windows-wsl.ps1')

function Test-AutomexiaAbsoluteWindowsPath([string]$Path) {
    return -not [string]::IsNullOrWhiteSpace($Path) -and
        $Path -match '^(?:[A-Za-z]:[\\/]|\\\\[^\\/]+[\\/][^\\/]+)'
}

function Assert-AutomexiaRealDirectory([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    if (-not $item.PSIsContainer -or $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing linked or non-directory managed path: $Path"
    }
}

function Assert-AutomexiaOwnedFile([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.PSIsContainer -or $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing linked or non-regular managed file: $Path"
    }
}

function Remove-AutomexiaOwnedFile([string]$Path) {
    Assert-AutomexiaOwnedFile $Path
    Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
}

Assert-AutomexiaRealDirectory (Join-Path $env:LOCALAPPDATA 'Automexia')
Assert-AutomexiaRealDirectory $InstallRoot
Assert-AutomexiaOwnedFile $StatePath
foreach ($name in @(
    'automexia.ps1',
    'automexia.format.ps1xml',
    'automexia-completion.ps1',
    'automexia.cmd',
    'automexia-ls.cmd',
    'automexia-ls.ps1',
    'install-state.json'
)) {
    Assert-AutomexiaOwnedFile (Join-Path $InstallRoot $name)
}

$configRoot = if ($env:AUTOMEXIA_CONFIG_HOME) {
    $env:AUTOMEXIA_CONFIG_HOME
} else {
    Join-Path $env:LOCALAPPDATA 'Automexia\Terminal'
}
if (-not (Test-AutomexiaAbsoluteWindowsPath $configRoot)) {
    throw 'Automexia config root must be an absolute drive or UNC path.'
}
$generatedRoot = Join-Path $configRoot 'generated'
$completionRoot = Join-Path $generatedRoot 'completion'
Assert-AutomexiaRealDirectory $configRoot
Assert-AutomexiaRealDirectory $generatedRoot
Assert-AutomexiaRealDirectory $completionRoot
if (Test-Path -LiteralPath $completionRoot) {
    foreach ($shell in @('powershell', 'bash', 'zsh', 'fish', 'cmd')) {
        $shellRoot = Join-Path $completionRoot $shell
        Assert-AutomexiaRealDirectory $shellRoot
        $extension = @{ powershell='ps1'; bash='bash'; zsh='zsh'; fish='fish'; cmd='cmd' }[$shell]
        foreach ($command in @('git','docker','kubectl','oc','helm','terraform','tofu','aws_completer','az','gcloud','ssh')) {
            $artifact = Join-Path $completionRoot "$shell\$command.$extension"
            foreach ($path in @($artifact, "$artifact.sha256", "$artifact.json", "$artifact.allow-override")) {
                Assert-AutomexiaOwnedFile $path
            }
        }
    }
    Assert-AutomexiaOwnedFile (Join-Path $completionRoot '.disabled')
}

function Remove-MarkedBlock([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    Assert-AutomexiaSafeProfilePathChain $Path
    if ($item.Length -gt 1MB) { throw "PowerShell profile exceeds the 1 MiB safety ceiling: $Path" }
    $text = [IO.File]::ReadAllText($Path)
    $starts = ([regex]::Matches($text, [regex]::Escape($MarkerStart))).Count
    $ends = ([regex]::Matches($text, [regex]::Escape($MarkerEnd))).Count
    if ($starts -eq 0 -and $ends -eq 0) { return }
    if ($starts -ne 1 -or $ends -ne 1) {
        throw "Malformed Automexia managed block; profile left unchanged: $Path"
    }
    $pattern = '(?ms)^# >>> AUTOMEXIA SHELL INTEGRATION >>>\r?\n.*?^# <<< AUTOMEXIA SHELL INTEGRATION <<<\r?\n?'
    if ([regex]::Matches($text, $pattern).Count -ne 1) {
        throw "Malformed or reversed Automexia managed block; profile left unchanged: $Path"
    }
    $temporary = "$Path.automexia-$PID.tmp"
    [IO.File]::WriteAllText($temporary, [regex]::Replace($text, $pattern, ''), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

$profiles = New-Object System.Collections.Generic.List[string]
if ($PowerShellProfilePathOverride) {
    foreach ($profilePath in $PowerShellProfilePathOverride) {
        if (-not [string]::IsNullOrWhiteSpace($profilePath)) {
            $profiles.Add([IO.Path]::GetFullPath($profilePath))
        }
    }
} else {
    if ($PROFILE -and $PROFILE.CurrentUserCurrentHost) {
        $profiles.Add([string]$PROFILE.CurrentUserCurrentHost)
    } elseif ($PROFILE) {
        $profiles.Add([string]$PROFILE)
    }
    $pwsh = Get-Command pwsh.exe -ErrorAction SilentlyContinue
    if ($pwsh) {
        try {
            $pwshProfile = & $pwsh.Source -NoLogo -NoProfile -Command '$PROFILE.CurrentUserCurrentHost' 2>$null | Select-Object -First 1
            if ($pwshProfile) { $profiles.Add([string]$pwshProfile) }
        } catch {}
    }
}
if (Test-Path -LiteralPath $StatePath -PathType Leaf) {
    $stateItem = Get-Item -LiteralPath $StatePath -Force
    if (-not $stateItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) -and $stateItem.Length -le 256KB) {
        try {
            $state = [IO.File]::ReadAllText($StatePath) | ConvertFrom-Json
            foreach ($profile in @($state.Profiles)) {
                if ($profile) { $profiles.Add([string]$profile) }
            }
        } catch {}
    }
}
$profiles | Select-Object -Unique | ForEach-Object { Remove-MarkedBlock $_ }

$wsl = if (-not $SkipWsl) { Get-Command wsl.exe -ErrorAction SilentlyContinue }
if ($wsl) {
    $cleanup = @'
set -eu
umask 077
remove_profile() {
  f=$1
  [ -f "$f" ] || return 0
  [ ! -L "$f" ] || { printf 'linked shell profile refused: %s\n' "$f" >&2; exit 1; }
  [ "$(wc -c <"$f")" -le 1048576 ] || { printf 'shell profile exceeds 1 MiB: %s\n' "$f" >&2; exit 1; }
  starts=$(grep -Fxc '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$f" 2>/dev/null || true)
  ends=$(grep -Fxc '# <<< AUTOMEXIA SHELL INTEGRATION <<<' "$f" 2>/dev/null || true)
  if [ "$starts" -eq 0 ] && [ "$ends" -eq 0 ]; then return 0; fi
  [ "$starts" -eq 1 ] && [ "$ends" -eq 1 ] || { printf 'malformed Automexia markers; profile unchanged: %s\n' "$f" >&2; exit 1; }
  tmp="$f.automexia-$$.tmp"
  awk 'BEGIN{skip=0} /# >>> AUTOMEXIA SHELL INTEGRATION >>>/{skip=1;next} /# <<< AUTOMEXIA SHELL INTEGRATION <<</{skip=0;next} !skip{print}' "$f" > "$tmp"
  chmod --reference="$f" "$tmp" 2>/dev/null || true
  mv "$tmp" "$f"
}
cfg="${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}"
fish_cfg="${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d"
case "$cfg" in /*) ;; *) printf 'absolute Automexia config root required\n' >&2; exit 1;; esac
case "$fish_cfg" in /*) ;; *) printf 'absolute Fish config root required\n' >&2; exit 1;; esac
assert_dir() {
  [ ! -L "$1" ] || { printf 'linked managed directory refused: %s\n' "$1" >&2; exit 1; }
  if [ -e "$1" ] && [ ! -d "$1" ]; then printf 'non-directory managed path refused: %s\n' "$1" >&2; exit 1; fi
}
assert_file() {
  [ ! -L "$1" ] || { printf 'linked managed file refused: %s\n' "$1" >&2; exit 1; }
  if [ -e "$1" ] && [ ! -f "$1" ]; then printf 'non-file managed path refused: %s\n' "$1" >&2; exit 1; fi
}
remove_file() { assert_file "$1"; rm -f "$1"; }
completion="$cfg/generated/completion"
for dir in "$cfg" "$fish_cfg" "$cfg/generated" "$completion"; do assert_dir "$dir"; done
for file in "$cfg/shell-integration.bash" "$cfg/shell-integration.zsh" \
  "$cfg/automexia-completion.bash" "$cfg/automexia-completion.zsh" \
  "$cfg/automexia-eza-filter.pl" "$fish_cfg/automexia.fish" \
  "$fish_cfg/automexia-completion.fish"; do assert_file "$file"; done
completion="$cfg/generated/completion"
for shell in powershell bash zsh fish cmd; do
  case "$shell" in powershell) ext=ps1;; bash) ext=bash;; zsh) ext=zsh;; fish) ext=fish;; cmd) ext=cmd;; esac
  dir="$completion/$shell"
  assert_dir "$dir"
  for command in git docker kubectl oc helm terraform tofu aws_completer az gcloud ssh; do
    file="$dir/$command.$ext"
    for owned in "$file" "$file.sha256" "$file.json" "$file.allow-override"; do assert_file "$owned"; done
  done
done
assert_file "$completion/.disabled"
for f in "$HOME/.bashrc" "$HOME/.zshrc"; do remove_profile "$f"; done
for file in "$cfg/shell-integration.bash" "$cfg/shell-integration.zsh" \
  "$cfg/automexia-completion.bash" "$cfg/automexia-completion.zsh" \
  "$cfg/automexia-eza-filter.pl" "$fish_cfg/automexia.fish" \
  "$fish_cfg/automexia-completion.fish"; do remove_file "$file"; done
for shell in powershell bash zsh fish cmd; do
  case "$shell" in powershell) ext=ps1;; bash) ext=bash;; zsh) ext=zsh;; fish) ext=fish;; cmd) ext=cmd;; esac
  dir="$completion/$shell"
  for command in git docker kubectl oc helm terraform tofu aws_completer az gcloud ssh; do
    file="$dir/$command.$ext"
    for owned in "$file" "$file.sha256" "$file.json" "$file.allow-override"; do remove_file "$owned"; done
  done
  rmdir "$dir" 2>/dev/null || true
done
remove_file "$completion/.disabled"
rmdir "$completion" "$cfg/generated" 2>/dev/null || true
rmdir "$cfg" 2>/dev/null || true
'@
    $distributions = @(
        & $wsl.Source --list --quiet 2>$null |
            ForEach-Object { ([string]$_).Replace([string][char]0, '').Trim() } |
            Where-Object { $_ -and $_ -notmatch '^(?i:docker-desktop(?:-data)?)$' }
    )
    foreach ($distribution in $distributions) {
        $wslResult = Invoke-AutomexiaWslScript `
            $wsl.Source $distribution $cleanup
        if ($wslResult.ExitCode -ne 0) {
            $detail = @($wslResult.Stdout, $wslResult.Stderr) |
                Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
            throw "WSL cleanup failed for ${distribution}: $($detail -join [Environment]::NewLine)"
        }
    }
}

if (Test-Path -LiteralPath $completionRoot) {
    foreach ($shell in @('powershell', 'bash', 'zsh', 'fish', 'cmd')) {
        $extension = @{ powershell='ps1'; bash='bash'; zsh='zsh'; fish='fish'; cmd='cmd' }[$shell]
        foreach ($command in @('git','docker','kubectl','oc','helm','terraform','tofu','aws_completer','az','gcloud','ssh')) {
            $artifact = Join-Path $completionRoot "$shell\$command.$extension"
            foreach ($path in @($artifact, "$artifact.sha256", "$artifact.json", "$artifact.allow-override")) {
                Remove-AutomexiaOwnedFile $path
            }
        }
        Remove-Item -LiteralPath (Join-Path $completionRoot $shell) -ErrorAction SilentlyContinue
    }
    Remove-AutomexiaOwnedFile (Join-Path $completionRoot '.disabled')
    Remove-Item -LiteralPath $completionRoot -ErrorAction SilentlyContinue
}

if (-not $KeepFiles) {
    foreach ($name in @(
        'automexia.ps1',
        'automexia.format.ps1xml',
        'automexia-completion.ps1',
        'automexia.cmd',
        'automexia-ls.cmd',
        'automexia-ls.ps1',
        'install-state.json'
    )) {
        Remove-AutomexiaOwnedFile (Join-Path $InstallRoot $name)
    }
    Remove-Item -LiteralPath $InstallRoot -Force -ErrorAction SilentlyContinue
}
Write-Host 'Automexia shell integration removed. Restart your shells.' -ForegroundColor Green
