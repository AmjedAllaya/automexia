[CmdletBinding()]
param([switch]$KeepFiles)
$ErrorActionPreference = 'Stop'

function Remove-MarkedBlock([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) { return }
    $text = Get-Content -LiteralPath $Path -Raw
    $pattern = '(?ms)^# >>> AUTOMEXIA SHELL INTEGRATION >>>\r?\n.*?^# <<< AUTOMEXIA SHELL INTEGRATION <<<\r?\n?'
    Set-Content -LiteralPath $Path -Value ([regex]::Replace($text, $pattern, '')) -Encoding UTF8
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
$profiles | Select-Object -Unique | ForEach-Object { Remove-MarkedBlock $_ }

if (Get-Command wsl.exe -ErrorAction SilentlyContinue) {
    $cleanup = @'
set -eu
for f in "$HOME/.bashrc" "$HOME/.zshrc"; do
  [ -f "$f" ] || continue
  awk 'BEGIN{skip=0} /# >>> AUTOMEXIA SHELL INTEGRATION >>>/{skip=1;next} /# <<< AUTOMEXIA SHELL INTEGRATION <<</{skip=0;next} !skip{print}' "$f" > "$f.automexia.tmp"
  mv "$f.automexia.tmp" "$f"
done
cfg="${XDG_CONFIG_HOME:-$HOME/.config}/automexia"
rm -f "$cfg/shell-integration.bash" "$cfg/shell-integration.zsh" \
  "$cfg/automexia-eza-filter.pl"
rmdir "$cfg" 2>/dev/null || true
'@
    $cleanup | & wsl.exe --exec sh
}

if (-not $KeepFiles) {
    Remove-Item (Join-Path $env:LOCALAPPDATA 'Automexia\shell-integration') -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host 'Automexia shell integration removed. Restart your shells.' -ForegroundColor Green
