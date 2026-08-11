[CmdletBinding()]
param([switch]$SkipWsl)
$ErrorActionPreference = 'Stop'

$integration = Join-Path $env:LOCALAPPDATA 'Automexia\shell-integration\automexia.ps1'
if (Test-Path -LiteralPath $integration) {
    $integrationText = Get-Content -LiteralPath $integration -Raw
    if ($integrationText -match 'automexia_prompt_active=MQ==' -and $integrationText -match '\$pathPrompt') {
        Write-Host "PASS PowerShell integration file + live prompt lifecycle/path: $integration" -ForegroundColor Green
    } else {
        throw "PowerShell integration is stale; reinstall v0.3.13 shell integration: $integration"
    }
} else {
    throw "PowerShell integration file missing: $integration"
}

$currentProfile = if ($PROFILE -and $PROFILE.CurrentUserCurrentHost) { [string]$PROFILE.CurrentUserCurrentHost } else { [string]$PROFILE }
Write-Host "PowerShell profile: $currentProfile" -ForegroundColor Cyan
if ($currentProfile -and (Test-Path -LiteralPath $currentProfile)) {
    $profileText = Get-Content -LiteralPath $currentProfile -Raw
    if ($profileText -match '# >>> AUTOMEXIA SHELL INTEGRATION >>>') {
        Write-Host 'PASS PowerShell profile hook present.' -ForegroundColor Green
    } else {
        Write-Host 'WARN PowerShell profile hook missing. The default Automexia shell still loads LocalAppData integration directly.' -ForegroundColor Yellow
    }
}

if (-not $SkipWsl -and (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
    $probe = @'
set -eu
cfg="${XDG_CONFIG_HOME:-$HOME/.config}/automexia"
[ -r "$cfg/shell-integration.bash" ] || { echo FAIL_BASH_FILE; exit 2; }
[ -f "$HOME/.bashrc" ] && grep -Fq '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "$HOME/.bashrc" || { echo FAIL_BASH_HOOK; exit 3; }
grep -Fq '\xCE\xBB' "$cfg/shell-integration.bash" || { echo FAIL_BASH_UTF8_GLYPH; exit 4; }
grep -Fq 'automexia_prompt_active=MQ==' "$cfg/shell-integration.bash" || { echo FAIL_BASH_PROMPT_STATE; exit 5; }
grep -Fq '\w' "$cfg/shell-integration.bash" || { echo FAIL_BASH_PATH_PROMPT; exit 6; }
echo PASS_WSL
'@
    $result = $probe | & wsl.exe --exec sh 2>&1
    if ($LASTEXITCODE -eq 0 -and ($result -contains 'PASS_WSL')) {
        Write-Host 'PASS WSL Bash integration file and hook present.' -ForegroundColor Green
    } else {
        throw "WSL integration check failed: $($result -join ' ')"
    }
}
