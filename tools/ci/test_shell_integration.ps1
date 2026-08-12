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
if ($integrationSource -notmatch 'SetUserVar=automexia_shell_user=') { throw 'PowerShell does not publish its explicit user identity' }
if ($integrationSource -notmatch 'SetUserVar=automexia_shell_path=') { throw 'PowerShell does not publish its executable path' }
if ($integrationSource -notmatch '133;P;k=c;aid=') { throw 'PowerShell prompt has no terminal-owned semantic-row marker' }
if ($integrationSource -match 'Get-AutomexiaGitSegment|gitSegment') { throw 'PowerShell still duplicates Git context on the editable path row' }
$global:LASTEXITCODE = 73
$emittedFirst = & prompt
$emittedSecond = & prompt
if ($script:AutomexiaPromptGeneration -ne 2) { throw 'PowerShell prompt identity does not advance once per prompt' }
if ($emittedFirst -notmatch [char]0x03BB) { throw 'PowerShell editable row has no lambda prompt' }
if ($emittedFirst -notmatch '133;B') { throw 'PowerShell editable row has no OSC 133 input marker' }
if ($emittedFirst -match [regex]::Escape((Get-Location).Path)) { throw 'PowerShell path is still owned by the PSReadLine prompt' }
if ($integrationSource -notmatch '\$continuation\s*\+\s*\$pathPrompt\s*\+\s*"`r`n"\s*\+\s*\$continuation') { throw 'PowerShell does not emit a terminal-owned complete-path row' }
if ($integrationSource -notmatch 'return\s+\$lambda\s*\+\s*\$input' -or $integrationSource -match 'return\s+\$pathPrompt') { throw 'PowerShell does not limit PSReadLine ownership to the editable lambda row' }
if ($integrationSource -match '\.\.\.[\\/]') { throw 'PowerShell prompt still truncates the current path' }
if ($global:LASTEXITCODE -ne 73) { throw 'PowerShell prompt changed LASTEXITCODE' }

$formatPath = Join-Path $root 'shell-integration\powershell\automexia.format.ps1xml'
if (-not (Test-Path -LiteralPath $formatPath)) { throw 'PowerShell filesystem icon view is missing' }
$formatSource = Get-Content -LiteralPath $formatPath -Raw
if ($formatSource -notmatch 'ReparsePoint' -or $formatSource -notmatch '0xF481') {
    throw 'PowerShell filesystem view has no differentiated symlink icon'
}
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
$folderGlyph = [char]0xF07B
$rustGlyph = [char]0xE7A8
$dockerGlyph = [char]0xF308
$kubernetesGlyph = [char]::ConvertFromUtf32(0xF10FE)
$terraformGlyph = [char]::ConvertFromUtf32(0xF1062)
$imageGlyph = [char]0xF1C5
$archiveGlyph = [char]0xF410
if ($formattedFilesystem -match '(?m)^\s*Icon(?:\s|$)') { throw 'PowerShell listing still exposes a standalone Icon column' }
if ($formattedFilesystem -notmatch '(?m)^Mode\s+Last Modified\s+Size\s+Name\s*$') {
    throw 'PowerShell listing headers are not Mode, Last Modified, Size, and Name'
}
if ($formattedFilesystem -notmatch [regex]::Escape("$folderGlyph apps\")) {
    throw 'PowerShell folder icon is not immediately before apps\ in the Name column'
}
if ($formattedFilesystem -notmatch [regex]::Escape("$rustGlyph lib.rs")) {
    throw 'PowerShell Rust icon is not immediately before lib.rs in the Name column'
}

$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) ("automexia-listing-{0}" -f [Guid]::NewGuid().ToString('N'))
try {
    $null = New-Item -ItemType Directory -Path $fixtureRoot
    $unicodeFolderName = 'folder space-é'
    $unicodeRustName = 'résumé space.rs'
    $longRustName = 'this is a very long Unicode-é-Rust-source-file-name.rs'
    $specialRustName = 'literal [x] $value.rs'
    $devopsNames = @('docker-compose.yml', 'Chart.yaml', 'main.tf', 'diagram.png', 'bundle.zip')
    $null = New-Item -ItemType Directory -Path (Join-Path $fixtureRoot $unicodeFolderName)
    $null = New-Item -ItemType File -Path (Join-Path $fixtureRoot $unicodeRustName)
    $null = New-Item -ItemType File -Path (Join-Path $fixtureRoot $longRustName)
    [IO.File]::WriteAllText((Join-Path $fixtureRoot $specialRustName), '')
    foreach ($name in $devopsNames) {
        $null = New-Item -ItemType File -Path (Join-Path $fixtureRoot $name)
    }

    $pipelineObjects = @(Get-ChildItem -LiteralPath $fixtureRoot |
        Where-Object Name -in @($unicodeFolderName, $unicodeRustName) |
        Sort-Object Name)
    if ($pipelineObjects.Count -ne 2 -or
        $pipelineObjects[0] -isnot [System.IO.DirectoryInfo] -or
        $pipelineObjects[1] -isnot [System.IO.FileInfo]) {
        throw 'PowerShell sorting or filtering no longer emits native DirectoryInfo/FileInfo values'
    }

    $unicodeListing = $pipelineObjects | Format-Table | Out-String -Width 240
    if ($unicodeListing -notmatch [regex]::Escape("$folderGlyph $unicodeFolderName\")) {
        throw 'PowerShell listing damaged a folder name containing spaces or Unicode'
    }
    if ($unicodeListing -notmatch [regex]::Escape("$rustGlyph $unicodeRustName")) {
        throw 'PowerShell listing damaged a Rust filename containing spaces or Unicode'
    }

    $specialListing = Get-Item -LiteralPath (Join-Path $fixtureRoot $specialRustName) |
        Format-Table | Out-String -Width 180
    if ($specialListing -notmatch [regex]::Escape("$rustGlyph $specialRustName")) {
        throw 'PowerShell listing damaged brackets, dollar signs, or spaces in a filename'
    }

    $devopsListing = Get-ChildItem -LiteralPath $fixtureRoot |
        Where-Object Name -in $devopsNames | Sort-Object Name |
        Format-Table | Out-String -Width 240
    foreach ($expectation in @(
        @{ Glyph = $dockerGlyph; Name = 'docker-compose.yml' },
        @{ Glyph = $kubernetesGlyph; Name = 'Chart.yaml' },
        @{ Glyph = $terraformGlyph; Name = 'main.tf' },
        @{ Glyph = $imageGlyph; Name = 'diagram.png' },
        @{ Glyph = $archiveGlyph; Name = 'bundle.zip' }
    )) {
        if ($devopsListing -notmatch [regex]::Escape("$($expectation.Glyph) $($expectation.Name)")) {
            throw "PowerShell listing did not use the differentiated icon for $($expectation.Name)"
        }
    }

    $longRustFile = Get-Item -LiteralPath (Join-Path $fixtureRoot $longRustName)
    $narrowListing = $longRustFile | Format-Table | Out-String -Width 62
    if ($narrowListing -notmatch [regex]::Escape("$rustGlyph this")) {
        throw 'Narrow PowerShell formatting separated or hid the icon from the filename prefix'
    }
    if ($narrowListing -match [regex]::Escape($longRustName)) {
        throw 'Narrow PowerShell formatting did not constrain the filename presentation'
    }
} finally {
    if (Test-Path -LiteralPath $fixtureRoot) {
        Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
    }
}
if ($integrationSource -notmatch 'AUTOMEXIA_PLAIN_LS') { throw 'PowerShell icon listing has no explicit opt-out' }

$bashIntegration = Get-Content (Join-Path $root 'shell-integration\bash\automexia.bash') -Raw
$zshIntegration = Get-Content (Join-Path $root 'shell-integration\zsh\automexia.zsh') -Raw
if ($bashIntegration -notmatch '133;A;aid=%s.*\\n' -or $bashIntegration -notmatch '(?s)38;2;72;167;255m%s.*\$PWD' -or $bashIntegration -notmatch "PS1=.*xCE.*133;B") { throw 'Bash does not keep terminal-owned context/path rows plus a Readline-owned lambda row' }
if ($zshIntegration -notmatch '133;A;aid=%s.*\\n' -or $zshIntegration -notmatch '(?s)38;2;72;167;255m%s.*\$PWD' -or $zshIntegration -notmatch "PROMPT=.*xCE.*133;B") { throw 'Zsh does not keep terminal-owned context/path rows plus a ZLE-owned lambda row' }
if ($bashIntegration -notmatch '133;D;%s') { throw 'Bash does not publish command exit status' }
if ($zshIntegration -notmatch '133;D;%s') { throw 'Zsh does not publish command exit status' }
if ($bashIntegration -notmatch 'automexia_shell_name=YmFzaA==') { throw 'Bash does not publish its real shell name' }
if ($zshIntegration -notmatch 'automexia_shell_name=enNo') { throw 'Zsh does not publish its real shell name' }
if ($bashIntegration -notmatch 'automexia_shell_user' -or $bashIntegration -notmatch 'automexia_shell_path') { throw 'Bash does not publish clone-safe user and shell-path metadata' }
if ($zshIntegration -notmatch 'automexia_shell_user' -or $zshIntegration -notmatch 'automexia_shell_path') { throw 'Zsh does not publish clone-safe user and shell-path metadata' }
if ($bashIntegration -notmatch '"\$PWD"' -or $bashIntegration -match 'PROMPT_DIRTRIM') { throw 'Bash integration does not emit the complete current path' }
if ($zshIntegration -notmatch '"\$PWD"' -or $zshIntegration -match '%[0-9]*~') { throw 'Zsh integration does not emit the complete current path' }
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
