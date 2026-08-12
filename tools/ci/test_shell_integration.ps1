$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$integration = Join-Path $root 'shell-integration\powershell\automexia.ps1'
$integrationSource = Get-Content -LiteralPath $integration -Raw
$env:TERM_PROGRAM = 'Automexia'

. $integration
$firstPrompt = (Get-Command prompt).ScriptBlock.ToString()
. $integration
$secondPrompt = (Get-Command prompt).ScriptBlock.ToString()

if (-not $global:AutomexiaShellIntegrationLoaded) { throw 'integration guard was not set' }
if ($firstPrompt -ne $secondPrompt) { throw 'second source changed the prompt handler' }
if ($secondPrompt -notmatch '\[char\]0x03BB') { throw 'prompt does not generate lambda from U+03BB' }
if ($secondPrompt -match 'alias docker|alias kubectl|function ax|function kgp') { throw 'integration adds forbidden commands' }
if ($integrationSource -match 'function\s+(global:)?ls\b|Set-Alias\s+ls') { throw 'PowerShell integration replaces native ls semantics' }
if ($integrationSource -notmatch 'SetUserVar=automexia_distro=\$script:AutomexiaBel') { throw 'PowerShell does not clear stale WSL distro metadata' }
if ($integrationSource -notmatch 'SetUserVar=automexia_os_version=\$script:AutomexiaBel') { throw 'PowerShell does not clear stale WSL version metadata' }
if ($integrationSource -notmatch 'SetUserVar=automexia_shell_name=UG93ZXJTaGVsbA==') { throw 'PowerShell does not publish its real shell name' }
if ($integrationSource -notmatch '133;P;k=c;aid=') { throw 'PowerShell prompt has no renderer-owned context-row marker' }
if ($integrationSource -match 'Get-AutomexiaGitSegment|gitSegment') { throw 'PowerShell still duplicates Git context on the editable path row' }
$global:LASTEXITCODE = 73
$emittedFirst = & prompt
$emittedSecond = & prompt
if ($script:AutomexiaPromptGeneration -ne 2) { throw 'PowerShell prompt identity does not advance once per prompt' }
if ($emittedFirst -notmatch [char]0x03BB) { throw 'PowerShell editable row has no lambda prompt' }
if ($emittedFirst -notmatch '133;B') { throw 'PowerShell editable row has no OSC 133 input marker' }
if ($emittedFirst -notmatch [regex]::Escape((Get-Location).Path)) { throw 'PowerShell multiline editor prompt does not contain the complete current path' }
if ($integrationSource -notmatch '\[Console\]::Write\(.*\$start.*\$continuation\)' -or $integrationSource -match '\[Console\]::Write\(.*\$pathPrompt') { throw 'PowerShell renderer-owned context row is not isolated from PSReadLine content' }
if ($integrationSource -notmatch 'return\s+\$pathPrompt.*`r`n.*\$continuation.*\$lambda.*\$input') { throw 'PowerShell does not return a resize-owned full-path multiline editor prompt' }
if ($integrationSource -match '\.\.\.[\\/]') { throw 'PowerShell prompt still truncates the current path' }
if ($global:LASTEXITCODE -ne 73) { throw 'PowerShell prompt changed LASTEXITCODE' }

$formatPath = Join-Path $root 'shell-integration\powershell\automexia.format.ps1xml'
if (-not (Test-Path -LiteralPath $formatPath)) { throw 'PowerShell filesystem icon view is missing' }
$formatDeadline = [DateTime]::UtcNow.AddSeconds(2)
do {
    $formatView = (Get-FormatData -TypeName System.IO.FileInfo).FormatViewDefinition |
        Where-Object Name -eq 'AutomexiaFileSystem'
    if (-not $formatView) { Start-Sleep -Milliseconds 25 }
} while (-not $formatView -and [DateTime]::UtcNow -lt $formatDeadline)
if (-not $formatView) { throw 'PowerShell filesystem icon view was not loaded' }
if ($integrationSource -notmatch 'System\.Timers\.Timer.*250' -or $integrationSource -notmatch 'Register-ObjectEvent') {
    throw 'PowerShell icon formatting is not deferred beyond the first prompt'
}
$lsAlias = Get-Alias ls -ErrorAction SilentlyContinue
if (-not $lsAlias -or $lsAlias.Definition -ne 'Get-ChildItem') { throw 'PowerShell ls no longer resolves to Get-ChildItem' }
$folder = Get-Item -LiteralPath (Join-Path $root 'apps')
$rustFile = Get-Item -LiteralPath (Join-Path $root 'apps\automexia-terminal\src\lib.rs')
$filesystemObjects = @($folder, $rustFile)
if ($filesystemObjects[0] -isnot [System.IO.DirectoryInfo] -or $filesystemObjects[1] -isnot [System.IO.FileInfo]) {
    throw 'PowerShell icon view changed filesystem object types'
}
$formattedFilesystem = $filesystemObjects | Format-Table | Out-String -Width 180
if ($formattedFilesystem -notmatch [char]0xF07B) { throw 'PowerShell listing has no folder icon' }
if ($formattedFilesystem -notmatch [char]0xE7A8) { throw 'PowerShell listing has no Rust file icon' }
if ($integrationSource -notmatch 'AUTOMEXIA_PLAIN_LS') { throw 'PowerShell icon listing has no explicit opt-out' }

$bashIntegration = Get-Content (Join-Path $root 'shell-integration\bash\automexia.bash') -Raw
$zshIntegration = Get-Content (Join-Path $root 'shell-integration\zsh\automexia.zsh') -Raw
if ($bashIntegration -notmatch '133;A;aid=%s.*\\n' -or $bashIntegration -notmatch 'PS1=.*133;P;k=c;aid=.*\$\{PWD\}.*\\n.*133;P;k=c;aid=') { throw 'Bash does not keep a stable context row plus resize-owned full-path Readline prompt' }
if ($zshIntegration -notmatch '133;A;aid=%s.*\\n' -or $zshIntegration -notmatch 'printf -v PROMPT.*133;P;k=c;aid=%s.*%d.*\\n.*133;P;k=c;aid=%s') { throw 'Zsh does not keep a stable context row plus resize-owned full-path ZLE prompt' }
if ($bashIntegration -notmatch '133;D;%s') { throw 'Bash does not publish command exit status' }
if ($zshIntegration -notmatch '133;D;%s') { throw 'Zsh does not publish command exit status' }
if ($bashIntegration -notmatch 'automexia_shell_name=YmFzaA==') { throw 'Bash does not publish its real shell name' }
if ($zshIntegration -notmatch 'automexia_shell_name=enNo') { throw 'Zsh does not publish its real shell name' }
if ($bashIntegration -notmatch '\$\{PWD\}' -or $bashIntegration -match 'PROMPT_DIRTRIM') { throw 'Bash prompt does not preserve the complete current path' }
if ($zshIntegration -notmatch '%d' -or $zshIntegration -match '%[0-9]*~') { throw 'Zsh prompt does not preserve the complete current path' }
if ($bashIntegration -match '__automexia_git_segment' -or $zshIntegration -match '__automexia_git_segment') { throw 'POSIX prompts still duplicate Git context on the editable path row' }
foreach ($source in @($bashIntegration, $zshIntegration)) {
    if ($source -notmatch 'function ls \{ __automexia_eza') { throw 'POSIX integration does not enable icon-aware ls through eza' }
    if ($source -notmatch 'function ll \{ __automexia_eza -lah --git') { throw 'POSIX integration does not enable the detailed Git-aware listing' }
    if ($source -notmatch '--header --group --time-style=long-iso') { throw 'POSIX long listing does not provide separated labeled columns' }
    if ($source -notmatch 'AUTOMEXIA_PLAIN_LS') { throw 'POSIX icon listing has no explicit opt-out' }
    if ($source -notmatch 'EZA_COLORS') { throw 'POSIX icon listing does not define the mockup-aligned metadata palette' }
    if ($source -notmatch 'hd=1;38;5;117') { throw 'POSIX listing headers are not visually emphasized' }
    if ($source -notmatch 'ur=38;5;81' -or $source -notmatch 'uw=38;5;220' -or $source -notmatch 'ux=38;5;114') { throw 'POSIX permission roles are not color-separated' }
    if ($source -notmatch 'ex=38;5;252') { throw 'WSL executable metadata is not neutralized for DrvFs listings' }
}

$installerSource = Get-Content (Join-Path $root 'shell-integration\install-windows.ps1') -Raw
if ($installerSource -notmatch '\.TrimEnd\(\[char\[\]\]"`r`n"\)') { throw 'WSL installer does not normalize here-document terminators deterministically' }
if ($installerSource -notmatch 'automexia\.format\.ps1xml') { throw 'Windows installer does not deploy the PowerShell icon view' }

$uninstall = Get-Content (Join-Path $root 'shell-integration\uninstall-windows.ps1') -Raw
if ($uninstall -notmatch 'AUTOMEXIA SHELL INTEGRATION') { throw 'uninstall marker cleanup is missing' }
Write-Output 'PASS: shell integration is idempotent, UTF-8-safe, WSL-isolated, icon-aware on PowerShell/Bash/Zsh, pipeline-safe, three-row prompt-identified, full-path, resize-safe, command-neutral, and uninstallable'
