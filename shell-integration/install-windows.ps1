[CmdletBinding()]
param(
    [switch]$SkipPowerShell,
    [switch]$SkipWsl
)
$ErrorActionPreference = 'Stop'
$PackageRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$InstallRoot = Join-Path $env:LOCALAPPDATA 'Automexia\shell-integration'
New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
$PowerShellIntegration = Join-Path $InstallRoot 'automexia.ps1'
Copy-Item (Join-Path $PackageRoot 'powershell\automexia.ps1') $PowerShellIntegration -Force

function Add-MarkedBlock([string]$Path, [string]$Body) {
    if ([string]::IsNullOrWhiteSpace($Path)) { return }
    $start = '# >>> AUTOMEXIA SHELL INTEGRATION >>>'
    $end = '# <<< AUTOMEXIA SHELL INTEGRATION <<<'
    $existing = if (Test-Path -LiteralPath $Path) { Get-Content -LiteralPath $Path -Raw } else { '' }
    if ($existing -match [regex]::Escape($start)) { return }
    $dir = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    Add-Content -LiteralPath $Path -Value "`r`n$start`r`n$Body`r`n$end`r`n" -Encoding UTF8
}

if (-not $SkipPowerShell) {
    # Use the engine's real profile path. Documents may be redirected to
    # OneDrive or another known folder, so hard-coding $HOME\Documents is not
    # reliable. The Automexia PTY launcher also sources the LocalAppData script
    # directly for the default shell; the profile hook covers nested pwsh /
    # powershell sessions opened from Automexia.
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

    $hookPath = $PowerShellIntegration.Replace("'", "''")
    $hook = "if (`$env:TERM_PROGRAM -eq 'Automexia' -or `$env:AUTOMEXIA_SHELL_INTEGRATION -eq '1') { . '$hookPath' }"
    foreach ($profilePath in ($profiles | Select-Object -Unique)) {
        Add-MarkedBlock $profilePath $hook
    }
    Write-Host "PowerShell integration installed: $PowerShellIntegration" -ForegroundColor Cyan
}

if (-not $SkipWsl -and (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
    # Install both Bash and Zsh hooks in ONE non-login WSL invocation. v0.3.7
    # launched WSL repeatedly and used `sh -lc`, needlessly paying startup/profile
    # cost several times. This payload does no distro enumeration and does not
    # source the user's rc files during installation.
    $bashPath = Join-Path $PackageRoot 'bash\automexia.bash'
    $zshPath = Join-Path $PackageRoot 'zsh\automexia.zsh'
    $bash = [System.IO.File]::ReadAllText($bashPath, [System.Text.Encoding]::UTF8)
    $zsh = [System.IO.File]::ReadAllText($zshPath, [System.Text.Encoding]::UTF8)
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
append_block() {
  file=`$1
  source_line=`$2
  touch "`$file"
  if ! grep -Fq '# >>> AUTOMEXIA SHELL INTEGRATION >>>' "`$file" 2>/dev/null; then
    printf '\n# >>> AUTOMEXIA SHELL INTEGRATION >>>\n%s\n# <<< AUTOMEXIA SHELL INTEGRATION <<<\n' "`$source_line" >> "`$file"
  fi
}
append_block "`$HOME/.bashrc" '[ -r "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.bash" ] && . "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.bash"'
append_block "`$HOME/.zshrc" '[ -r "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.zsh" ] && . "`${XDG_CONFIG_HOME:-`$HOME/.config}/automexia/shell-integration.zsh"'
printf 'AUTOMEXIA_WSL_INTEGRATION_OK\n'
"@
    # Windows PowerShell 5.1 encodes text piped to native commands using
    # a legacy console/OEM code page. That corrupted U+03BB into literal '??'
    # in the installed WSL prompt. Transfer the complete UTF-8 payload as
    # ASCII base64 instead; the WSL side decodes bytes before sh parses them.
    $payloadBytes = [System.Text.Encoding]::UTF8.GetBytes($payload)
    $payloadBase64 = [Convert]::ToBase64String($payloadBytes)
    $decodeCommand = "printf '%s' '$payloadBase64' | base64 -d | sh"
    $result = & wsl.exe --exec sh -c $decodeCommand 2>&1
    if ($LASTEXITCODE -ne 0 -or ($result -notcontains 'AUTOMEXIA_WSL_INTEGRATION_OK')) {
        throw "WSL shell integration install failed: $($result -join [Environment]::NewLine)"
    }
    Write-Host 'WSL Bash/Zsh integration installed in one fast pass.' -ForegroundColor Cyan
}

Write-Host 'Restart all Automexia windows after installing shell integration.' -ForegroundColor Green
