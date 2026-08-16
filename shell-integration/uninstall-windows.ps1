[CmdletBinding()]
param([switch]$KeepFiles)
$ErrorActionPreference = 'Stop'
$MarkerStart = '# >>> AUTOMEXIA SHELL INTEGRATION >>>'
$MarkerEnd = '# <<< AUTOMEXIA SHELL INTEGRATION <<<'
$InstallRoot = Join-Path $env:LOCALAPPDATA 'Automexia\shell-integration'
$StatePath = Join-Path $InstallRoot 'install-state.json'

function Remove-MarkedBlock([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing to edit linked PowerShell profile: $Path"
    }
    if ($item.Length -gt 1MB) { throw "PowerShell profile exceeds the 1 MiB safety ceiling: $Path" }
    $text = [IO.File]::ReadAllText($Path)
    $starts = ([regex]::Matches($text, [regex]::Escape($MarkerStart))).Count
    $ends = ([regex]::Matches($text, [regex]::Escape($MarkerEnd))).Count
    if ($starts -eq 0 -and $ends -eq 0) { return }
    if ($starts -ne 1 -or $ends -ne 1) {
        throw "Malformed Automexia managed block; profile left unchanged: $Path"
    }
    $pattern = '(?ms)^# >>> AUTOMEXIA SHELL INTEGRATION >>>\r?\n.*?^# <<< AUTOMEXIA SHELL INTEGRATION <<<\r?\n?'
    $temporary = "$Path.automexia-$PID.tmp"
    [IO.File]::WriteAllText($temporary, [regex]::Replace($text, $pattern, ''), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

$profiles = New-Object System.Collections.Generic.List[string]
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

$wsl = Get-Command wsl.exe -ErrorAction SilentlyContinue
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
for f in "$HOME/.bashrc" "$HOME/.zshrc"; do
  remove_profile "$f"
done
cfg="${AUTOMEXIA_CONFIG_HOME:-${XDG_CONFIG_HOME:-$HOME/.config}/automexia}"
fish_cfg="${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d"
rm -f "$cfg/shell-integration.bash" "$cfg/shell-integration.zsh" \
  "$cfg/automexia-completion.bash" "$cfg/automexia-completion.zsh" \
  "$cfg/automexia-eza-filter.pl" "$fish_cfg/automexia.fish" \
  "$fish_cfg/automexia-completion.fish"
completion="$cfg/generated/completion"
for shell in powershell bash zsh fish cmd; do
  case "$shell" in powershell) ext=ps1;; bash) ext=bash;; zsh) ext=zsh;; fish) ext=fish;; cmd) ext=cmd;; esac
  dir="$completion/$shell"
  [ ! -L "$dir" ] || { printf 'linked completion directory refused: %s\n' "$dir" >&2; exit 1; }
  for command in git docker kubectl oc helm terraform tofu aws_completer az gcloud ssh; do
    file="$dir/$command.$ext"
    rm -f "$file" "$file.sha256" "$file.json" "$file.allow-override"
  done
  rmdir "$dir" 2>/dev/null || true
done
rm -f "$completion/.disabled"
rmdir "$completion" "$cfg/generated" 2>/dev/null || true
rmdir "$cfg" 2>/dev/null || true
'@
    $cleanupBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($cleanup))
    $decode = "printf '%s' '$cleanupBase64' | base64 -d | sh"
    $distributions = @(
        & $wsl.Source --list --quiet 2>$null |
            ForEach-Object { ([string]$_).Replace([string][char]0, '').Trim() } |
            Where-Object { $_ -and $_ -notmatch '^(?i:docker-desktop(?:-data)?)$' }
    )
    foreach ($distribution in $distributions) {
        & $wsl.Source --distribution $distribution --exec sh -c $decode
        if ($LASTEXITCODE -ne 0) { throw "WSL cleanup failed for $distribution" }
    }
}

$completionRoot = Join-Path $env:LOCALAPPDATA 'Automexia\Terminal\generated\completion'
if (Test-Path -LiteralPath $completionRoot) {
    $rootItem = Get-Item -LiteralPath $completionRoot -Force
    if ($rootItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing linked completion root: $completionRoot"
    }
    foreach ($shell in @('powershell', 'bash', 'zsh', 'fish', 'cmd')) {
        $extension = @{ powershell='ps1'; bash='bash'; zsh='zsh'; fish='fish'; cmd='cmd' }[$shell]
        foreach ($command in @('git','docker','kubectl','oc','helm','terraform','tofu','aws_completer','az','gcloud','ssh')) {
            $artifact = Join-Path $completionRoot "$shell\$command.$extension"
            foreach ($path in @($artifact, "$artifact.sha256", "$artifact.json", "$artifact.allow-override")) {
                Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
            }
        }
        Remove-Item -LiteralPath (Join-Path $completionRoot $shell) -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath (Join-Path $completionRoot '.disabled') -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $completionRoot -ErrorAction SilentlyContinue
}

if (-not $KeepFiles) {
    Remove-Item -LiteralPath $InstallRoot -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host 'Automexia shell integration removed. Restart your shells.' -ForegroundColor Green
