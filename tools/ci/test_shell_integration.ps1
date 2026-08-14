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
if ($integrationSource -notmatch 'function global:Invoke-AutomexiaCmd' -or
    $integrationSource -notmatch 'Set-Alias -Name cmd' -or
    $integrationSource -match '(?im)^\s*Start-Process\b') {
    throw 'PowerShell does not keep an interactive CMD launch inside the existing Automexia PTY'
}
if ($integrationSource -notmatch 'SetUserVar=automexia_distro=\$script:AutomexiaBel') { throw 'PowerShell does not clear stale WSL distro metadata' }
if ($integrationSource -notmatch 'SetUserVar=automexia_os_version=\$script:AutomexiaBel') { throw 'PowerShell does not clear stale WSL version metadata' }
if ($integrationSource -notmatch 'SetUserVar=automexia_shell_name=UG93ZXJTaGVsbA==') { throw 'PowerShell does not publish its real shell name' }
if ($integrationSource -notmatch 'SetUserVar=automexia_shell_user=') { throw 'PowerShell does not publish its explicit user identity' }
if ($integrationSource -notmatch 'SetUserVar=automexia_shell_path=') { throw 'PowerShell does not publish its executable path' }
if ($integrationSource -notmatch 'function script:Publish-AutomexiaPowerShellIdentity' -or
    $firstPrompt -notmatch 'Publish-AutomexiaPowerShellIdentity') {
    throw 'PowerShell does not restore parent identity on every prompt after a nested shell exits'
}
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

$cmdAlias = Get-Command cmd -ErrorAction Stop
$cmdExeAlias = Get-Command cmd.exe -ErrorAction Stop
if ($cmdAlias.CommandType -ne 'Alias' -or $cmdAlias.Definition -ne 'Invoke-AutomexiaCmd' -or
    $cmdExeAlias.CommandType -ne 'Alias' -or $cmdExeAlias.Definition -ne 'Invoke-AutomexiaCmd') {
    throw 'PowerShell cmd/cmd.exe entry points do not resolve to the in-pane launcher'
}

# Replace only the executable with a harmless command and prove a truly bare
# invocation selects /D /K integration. This catches PowerShell's subtle null
# pipeline behavior without opening an interactive CMD child.
$previousCmdExecutable = $script:AutomexiaCmdExecutable
$previousCmdIntegration = $script:AutomexiaCmdIntegration
try {
    $script:AutomexiaCmdExecutable = 'Write-Output'
    $script:AutomexiaCmdIntegration = $integration
    $bareCmdArguments = @(& Invoke-AutomexiaCmd)
    if ($bareCmdArguments.Count -ne 3 -or
        $bareCmdArguments[0] -ne '/D' -or
        $bareCmdArguments[1] -ne '/K' -or
        $bareCmdArguments[2] -notmatch '^chcp 65001>nul & set "AUTOMEXIA_CMD_PROMPT_GLYPH=.+?" & call "') {
        throw 'Bare cmd was mistaken for an explicit empty argument and skipped integration'
    }
} finally {
    $script:AutomexiaCmdExecutable = $previousCmdExecutable
    $script:AutomexiaCmdIntegration = $previousCmdIntegration
}
$nativeCmdResult = (& cmd /D /C 'echo AUTOMEXIA_CMD_NATIVE_OK' | Out-String).Trim()
if ($nativeCmdResult -ne 'AUTOMEXIA_CMD_NATIVE_OK') {
    throw 'Explicit cmd /c behavior was changed by Automexia integration'
}

$samplePath = "D:\cloud project\$([char]0x00E9)\automexia-terminal"
$styledPath = Format-AutomexiaPromptPath -Path $samplePath
$escape = [char]27
$plainStyledPath = [regex]::Replace($styledPath, "$escape\[[0-9;]*m", '')
if ($plainStyledPath -cne $samplePath) {
    throw 'PowerShell semantic path coloring changed the literal copied path'
}
$expectedStyledPath =
    "$escape[38;2;98;176;255mD:" +
    "$escape[38;2;88;113;141m\" +
    "$escape[38;2;80;213;255mcloud project" +
    "$escape[38;2;88;113;141m\" +
    "$escape[38;2;167;139;250m$([char]0x00E9)" +
    "$escape[38;2;88;113;141m\" +
    "$escape[38;2;184;243;107mautomexia-terminal" +
    "$escape[0m"
if ($styledPath -cne $expectedStyledPath) {
    throw 'PowerShell path roles are not root/parent/separator/current-directory ordered'
}
if ((Format-AutomexiaPromptPath -Path $samplePath) -cne $styledPath) {
    throw 'PowerShell path-color cache is not stable for an unchanged directory'
}

$formatPath = Join-Path $root 'shell-integration\powershell\automexia.format.ps1xml'
if (-not (Test-Path -LiteralPath $formatPath)) { throw 'PowerShell filesystem icon view is missing' }
$formatSource = Get-Content -LiteralPath $formatPath -Raw
if ($formatSource -notmatch 'ReparsePoint' -or $formatSource -notmatch '0xF481') {
    throw 'PowerShell filesystem view has no differentiated symlink icon'
}
foreach ($categoryContract in @(
    @{ Pattern = 'secret\|secrets\|private'; Glyph = '0xF0250'; Color = '255;92;122' },
    @{ Pattern = 'config\|configs\|configuration'; Glyph = '0xF107F'; Color = '255;176;32' },
    @{ Pattern = 'logs\?\|logfiles'; Glyph = '0xF0C82'; Color = '242;201;76' },
    @{ Pattern = 'apps\?\|src\|source'; Glyph = '0xF19F6'; Color = '80;213;255' },
    @{ Pattern = 'docs\?\|documentation'; Glyph = '0xF10B7'; Color = '96;211;148' },
    @{ Pattern = 'tests\?\|specs'; Glyph = '0xF197E'; Color = '220;120;255' },
    @{ Pattern = 'target\|build\|dist'; Glyph = '0xF0D0B'; Color = '244;111;97' },
    @{ Pattern = 'data\|db\|database'; Glyph = '0xF12E3'; Color = '129;140;248' }
)) {
    if ($formatSource -notmatch $categoryContract.Pattern -or
        $formatSource -notmatch $categoryContract.Glyph -or
        $formatSource -notmatch [regex]::Escape($categoryContract.Color)) {
        throw "PowerShell filesystem taxonomy is missing $($categoryContract.Pattern)"
    }
}
if ($formatSource -notmatch 'PSVersionTable\.PSVersion\.Major -ge 7') {
    throw 'PowerShell category colors do not protect Windows PowerShell 5 table width'
}
if ($formatSource -notmatch '\[Console\]::IsOutputRedirected' -or
    $formatSource -notmatch 'WindowSize\.Width -ge 96' -or
    $formatSource -notmatch '38;5;\$\{legacyColor\}') {
    throw 'Windows PowerShell 5 does not use the width-safe direct-VT category palette'
}
$formatDeadline = [DateTime]::UtcNow.AddSeconds(2)
do {
    $formatView = (Get-FormatData -TypeName System.IO.FileInfo).FormatViewDefinition |
        Where-Object Name -eq 'AutomexiaFileSystem'
    if (-not $formatView) { Start-Sleep -Milliseconds 25 }
} while (-not $formatView -and [DateTime]::UtcNow -lt $formatDeadline)
if (-not $formatView) { throw 'PowerShell filesystem icon view was not loaded' }
if ($integrationSource -match '(?m)^\s*Register-EngineEvent\b' -or
    $integrationSource -match '\[System\.Timers\.Timer\]') {
    throw 'PowerShell presentation setup must not run from an asynchronous event callback'
}
if ($integrationSource -notmatch 'AutomexiaPreviousHistoryHandler -is \[scriptblock\]' -or
    $integrationSource -notmatch 'AutomexiaPreviousHistoryHandler -is \[System\.Delegate\]' -or
    $integrationSource -notmatch 'DynamicInvoke') {
    throw 'PowerShell history chaining does not preserve script-block and .NET delegate handlers'
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
$sourceFolderGlyph = [char]::ConvertFromUtf32(0xF19F6)
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
if ($formattedFilesystem -notmatch [regex]::Escape("$sourceFolderGlyph apps\")) {
    throw 'PowerShell source-folder icon is not immediately before apps\ in the Name column'
}
if ($formattedFilesystem -notmatch [regex]::Escape("$rustGlyph lib.rs")) {
    throw 'PowerShell Rust icon is not immediately before lib.rs in the Name column'
}
if ($PSVersionTable.PSVersion.Major -lt 7 -and
    [Console]::IsOutputRedirected -and
    $formattedFilesystem.Contains([string][char]27)) {
    throw 'Redirected Windows PowerShell output unexpectedly contains category ANSI bytes'
}

$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) ("automexia-listing-{0}" -f [Guid]::NewGuid().ToString('N'))
try {
    $null = New-Item -ItemType Directory -Path $fixtureRoot
    $unicodeFolderName = 'folder space-é'
    $unicodeRustName = 'résumé space.rs'
    $longRustName = 'this is a very long Unicode-é-Rust-source-file-name.rs'
    $specialRustName = 'literal [x] $value.rs'
    $devopsNames = @('docker-compose.yml', 'Chart.yaml', 'main.tf', 'diagram.png', 'bundle.zip')
    $categoryFileNames = @('.env.production', 'service.log', 'state.db', 'settings.json', 'README.md', 'deploy.ps1', 'main.py')
    $categoryFolders = @('secret', 'config', 'logs', 'src', 'docs', 'tests', 'target', 'assets', 'packages', 'tools', 'data', 'cache', 'infra', 'packaging')
    $null = New-Item -ItemType Directory -Path (Join-Path $fixtureRoot $unicodeFolderName)
    foreach ($name in $categoryFolders) {
        $null = New-Item -ItemType Directory -Path (Join-Path $fixtureRoot $name)
    }
    $null = New-Item -ItemType File -Path (Join-Path $fixtureRoot $unicodeRustName)
    $null = New-Item -ItemType File -Path (Join-Path $fixtureRoot $longRustName)
    [IO.File]::WriteAllText((Join-Path $fixtureRoot $specialRustName), '')
    foreach ($name in $devopsNames) {
        $null = New-Item -ItemType File -Path (Join-Path $fixtureRoot $name)
    }
    foreach ($name in $categoryFileNames) {
        [IO.File]::WriteAllText((Join-Path $fixtureRoot $name), 'classification must use names only')
    }

    $cmdRoot = Join-Path $root 'shell-integration\cmd'
    $cmdIntegrationPath = Join-Path $cmdRoot 'automexia.cmd'
    $cmdListingPath = Join-Path $cmdRoot 'automexia-ls.ps1'
    $cmdLauncherPath = Join-Path $cmdRoot 'automexia-ls.cmd'
    foreach ($requiredCmdFile in @($cmdIntegrationPath, $cmdListingPath, $cmdLauncherPath)) {
        if (-not (Test-Path -LiteralPath $requiredCmdFile)) {
            throw "CMD integration file is missing: $requiredCmdFile"
        }
    }
    $cmdSource = [IO.File]::ReadAllText($cmdIntegrationPath, [Text.Encoding]::UTF8)
    if ($cmdSource.ToCharArray() | Where-Object { [int]$_ -gt 127 } | Select-Object -First 1) {
        throw 'CMD integration source is not ASCII-safe and can be corrupted by the active console code page'
    }
    foreach ($contract in @(
        'SetUserVar=automexia_shell_name=Q01E',
        'SetUserVar=automexia_shell_user=__AUTOMEXIA_CMD_USER_BASE64__',
        'SetUserVar=automexia_shell_path=__AUTOMEXIA_CMD_PATH_BASE64__',
        'SetUserVar=automexia_distro=',
        'set "AUTOMEXIA_CMD_IDENTITY=',
        'PROMPT=%AUTOMEXIA_CMD_IDENTITY%',
        'AUTOMEXIA_CMD_PROMPT_GLYPH',
        ']7;file:///$P',
        ']133;A',
        ']133;P;k=c',
        ']133;B',
        '38;2;98;176;255m$P',
        'doskey ls=call',
        'doskey ll=call'
    )) {
        if ($cmdSource -notmatch [regex]::Escape($contract)) {
            throw "CMD integration is missing contract: $contract"
        }
    }
    if ($cmdSource -match 'doskey dir=') {
        throw 'CMD integration replaces the native DIR command'
    }

    $cmdListing = & $cmdListingPath -la $fixtureRoot | Out-String -Width 240
    if ($cmdListing -notmatch [regex]::Escape("$([char]::ConvertFromUtf32(0xF0250)) secret\") -or
        $cmdListing -notmatch [regex]::Escape("$rustGlyph $unicodeRustName")) {
        throw 'CMD ls helper does not retain Automexia category/file icons beside names'
    }

    # Generate the same account-specific batch installed by install-windows.ps1
    # and execute it inside a child CMD. This verifies prompt/identity state
    # without opening a window or depending on the caller's persistent profile.
    $cmdProbeRoot = Join-Path $fixtureRoot 'cmd-probe'
    $null = New-Item -ItemType Directory -Path $cmdProbeRoot
    $generatedCmd = Join-Path $cmdProbeRoot 'automexia.cmd'
    $generatedSource = $cmdSource.Replace(
        '__AUTOMEXIA_CMD_USER_BASE64__',
        [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([Environment]::UserName))
    )
    $generatedSource = $generatedSource.Replace(
        '__AUTOMEXIA_CMD_PATH_BASE64__',
        [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($env:ComSpec))
    )
    $generatedSource = [regex]::Replace($generatedSource, "\r?\n", "`r`n")
    [IO.File]::WriteAllText($generatedCmd, $generatedSource, [Text.Encoding]::ASCII)
    $probePath = Join-Path $cmdProbeRoot 'probe.cmd'
    $probeSource = "@echo off`r`ncall `"$generatedCmd`"`r`necho AUTOMEXIA_CMD_LOADED=%AUTOMEXIA_CMD_INTEGRATION_LOADED%`r`necho AUTOMEXIA_CMD_PROMPT=%PROMPT%`r`n"
    [IO.File]::WriteAllText($probePath, $probeSource, [Text.Encoding]::ASCII)
    $previousCmdPromptGlyph = $env:AUTOMEXIA_CMD_PROMPT_GLYPH
    try {
        $env:AUTOMEXIA_CMD_PROMPT_GLYPH = [char]0x03BB
        $cmdProbe = & $env:ComSpec /D /C $probePath | Out-String
    } finally {
        if ($null -eq $previousCmdPromptGlyph) {
            Remove-Item Env:AUTOMEXIA_CMD_PROMPT_GLYPH -ErrorAction SilentlyContinue
        } else {
            $env:AUTOMEXIA_CMD_PROMPT_GLYPH = $previousCmdPromptGlyph
        }
    }
    if ($cmdProbe -notmatch 'AUTOMEXIA_CMD_LOADED=1' -or
        $cmdProbe -notmatch [regex]::Escape('SetUserVar=automexia_shell_name=Q01E') -or
        $cmdProbe -notmatch [regex]::Escape(
            'SetUserVar=automexia_shell_user=' +
            [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([Environment]::UserName))) -or
        $cmdProbe -notmatch [regex]::Escape(
            'SetUserVar=automexia_shell_path=' +
            [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($env:ComSpec))) -or
        $cmdProbe -notmatch [regex]::Escape(']7;file:///$P') -or
        $cmdProbe -notmatch [regex]::Escape('SetUserVar=automexia_prompt_active=MQ==') -or
        $cmdProbe -notmatch [regex]::Escape([char]0x03BB)) {
        throw 'Native CMD startup did not install repeatable identity, prompt, and UTF-8 metadata in-process'
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

    $categoryListing = Get-ChildItem -LiteralPath $fixtureRoot |
        Where-Object Name -in $categoryFolders | Sort-Object Name |
        Format-Table | Out-String -Width 240
    foreach ($expectation in @(
        @{ Glyph = [char]::ConvertFromUtf32(0xF0250); Name = 'secret' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF107F); Name = 'config' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF0C82); Name = 'logs' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF19F6); Name = 'src' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF10B7); Name = 'docs' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF197E); Name = 'tests' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF0D0B); Name = 'target' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF024F); Name = 'assets' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF0253); Name = 'packages' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF19FC); Name = 'tools' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF12E3); Name = 'data' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF0ABA); Name = 'cache' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF0870); Name = 'infra' },
        @{ Glyph = [char]::ConvertFromUtf32(0xF06EB); Name = 'packaging' }
    )) {
        if ($categoryListing -notmatch [regex]::Escape("$($expectation.Glyph) $($expectation.Name)\")) {
            throw "PowerShell listing did not categorize the $($expectation.Name) folder"
        }
    }

    $categoryFileListing = Get-ChildItem -LiteralPath $fixtureRoot -Force |
        Where-Object Name -in $categoryFileNames | Sort-Object Name |
        Format-Table | Out-String -Width 240
    foreach ($expectation in @(
        @{ Glyph = [char]0xF023; Name = '.env.production' },
        @{ Glyph = [char]0xF15C; Name = 'service.log' },
        @{ Glyph = [char]0xF1C0; Name = 'state.db' },
        @{ Glyph = [char]0xE60B; Name = 'settings.json' },
        @{ Glyph = [char]0xE73E; Name = 'README.md' },
        @{ Glyph = [char]0xE86C; Name = 'deploy.ps1' },
        @{ Glyph = [char]0xF121; Name = 'main.py' }
    )) {
        if ($categoryFileListing -notmatch [regex]::Escape("$($expectation.Glyph) $($expectation.Name)")) {
            throw "PowerShell listing did not categorize the $($expectation.Name) file"
        }
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
if ($bashIntegration -notmatch '133;A;aid=%s.*\\n' -or $bashIntegration -notmatch '__automexia_print_colored_path "\$PWD"' -or $bashIntegration -notmatch "PS1=.*xCE.*133;B") { throw 'Bash does not keep a colored terminal-owned complete-path row plus a Readline-owned lambda row' }
if ($zshIntegration -notmatch '133;A;aid=%s.*\\n' -or $zshIntegration -notmatch '__automexia_print_colored_path "\$PWD"' -or $zshIntegration -notmatch "PROMPT=.*xCE.*133;B") { throw 'Zsh does not keep a colored terminal-owned complete-path row plus a ZLE-owned lambda row' }
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
    foreach ($pathColor in @(
        '98;176;255', '80;213;255', '167;139;250', '72;167;255',
        '184;243;107', '88;113;141'
    )) {
        if ($source -notmatch [regex]::Escape($pathColor)) { throw "POSIX prompt path is missing semantic color $pathColor" }
    }
    if ($source -notmatch 'function ls \{ __automexia_eza') { throw 'POSIX integration does not enable icon-aware ls through eza' }
    if ($source -notmatch 'function ll \{ __automexia_eza -lah --git') { throw 'POSIX integration does not enable the detailed Git-aware listing' }
    if ($source -notmatch '--header --group --time-style=long-iso') { throw 'POSIX long listing does not provide separated labeled columns' }
    if ($source -notmatch 'AUTOMEXIA_PLAIN_LS') { throw 'POSIX icon listing has no explicit opt-out' }
    if ($source -notmatch 'EZA_COLORS') { throw 'POSIX icon listing does not define the mockup-aligned metadata palette' }
    if ($source -notmatch '__automexia_run_eza' -or
        $source -notmatch 'automexia-eza-filter\.pl' -or
        $source -notmatch '-t 1' -or
        $source -notmatch '--width="\$\{COLUMNS:-80\}"') {
        throw 'POSIX icon listing does not apply category folder badges only on interactive output'
    }
    if ($source -notmatch 'hd=1;38;5;117') { throw 'POSIX listing headers are not visually emphasized' }
    if ($source -notmatch 'ur=38;5;81' -or $source -notmatch 'uw=38;5;220' -or $source -notmatch 'ux=38;5;114') { throw 'POSIX permission roles are not color-separated' }
    if ($source -notmatch 'ex=38;5;252') { throw 'WSL executable metadata is not neutralized for DrvFs listings' }
    foreach ($categoryColor in @(
        '*secret=1;38;5;203', '*config=38;5;214', '*logs=38;5;220',
        '*src=38;5;81', '*docs=38;5;114', '*tests=38;5;177',
        '*target=38;5;209', '*assets=38;5;211', '*data=38;5;105',
        '*cache=38;5;245', '*infra=38;5;39', '*packaging=38;5;214'
    )) {
        if ($source -notmatch [regex]::Escape($categoryColor)) {
            throw "POSIX icon listing is missing category color $categoryColor"
        }
    }
}
$bashEzaPalette = [regex]::Match($bashIntegration, "export EZA_COLORS='([^']+)'").Groups[1].Value
$zshEzaPalette = [regex]::Match($zshIntegration, "typeset -gx EZA_COLORS='([^']+)'").Groups[1].Value
if (-not $bashEzaPalette -or $bashEzaPalette -cne $zshEzaPalette) {
    throw 'Bash and Zsh folder/file category palettes have drifted apart'
}

$ezaFilterPath = Join-Path $root 'shell-integration\posix\automexia-eza-filter.pl'
if (-not (Test-Path -LiteralPath $ezaFilterPath)) {
    throw 'POSIX eza category-folder compatibility filter is missing'
}
$ezaFilter = Get-Content -LiteralPath $ezaFilterPath -Raw
foreach ($folderBadge in @(
    '0xF0250', '0xF107F', '0xF0C82', '0xF19F6', '0xF10B7',
    '0xF197E', '0xF0D0B', '0xF024F', '0xF0253', '0xE5FB',
    '0xF19FC', '0xF12E3', '0xF0ABA', '0xF0870', '0xF06EB'
)) {
    if ($ezaFilter -notmatch [regex]::Escape($folderBadge)) {
        throw "POSIX eza filter is missing composite folder badge $folderBadge"
    }
}
if ($ezaFilter -notmatch 'generic_folder' -or $ezaFilter -notmatch 'Filenames and non-interactive output') {
    throw 'POSIX eza filter does not constrain rewriting to generic interactive folder presentation'
}

$installerSource = Get-Content (Join-Path $root 'shell-integration\install-windows.ps1') -Raw
if ($installerSource -notmatch '\.TrimEnd\(\[char\[\]\]"`r`n"\)') { throw 'WSL installer does not normalize here-document terminators deterministically' }
if ($installerSource -notmatch 'automexia\.format\.ps1xml') { throw 'Windows installer does not deploy the PowerShell icon view' }
if ($installerSource -notmatch 'automexia-eza-filter\.pl' -or
    $installerSource -notmatch 'AUTOMEXIA_EZA_FILTER_EOF') {
    throw 'Windows installer does not deploy the POSIX composite-folder filter'
}
if ($installerSource -notmatch '--distribution \$distribution' -or
    $installerSource -notmatch 'docker-desktop') {
    throw 'Windows installer does not provision every detected user WSL distribution safely'
}
if ($installerSource -notmatch 'automexia\.cmd' -or
    $installerSource -notmatch '__AUTOMEXIA_CMD_USER_BASE64__' -or
    $installerSource -notmatch '__AUTOMEXIA_CMD_PATH_BASE64__') {
    throw 'Windows installer does not deploy account-specific CMD integration metadata'
}
if ($installerSource -notmatch '\[switch\]\$Quiet' -or
    $installerSource -notmatch '\[switch\]\$Force' -or
    $installerSource -notmatch 'install-state\.json' -or
    $installerSource -notmatch 'Get-SourceFingerprint' -or
    $installerSource -notmatch 'Test-StampedInstall') {
    throw 'Windows launch-time installer is not source-aware, quiet, forceable, and idempotent'
}
if ($installerSource -notmatch 'Move-Item -LiteralPath .* -Destination .* -Force') {
    throw 'Windows launch-time installer does not publish staged files atomically'
}

$installerFixture = Join-Path ([IO.Path]::GetTempPath()) ("automexia-installer-{0}" -f [Guid]::NewGuid().ToString('N'))
$previousLocalAppData = $env:LOCALAPPDATA
try {
    $env:LOCALAPPDATA = $installerFixture
    $installerPath = Join-Path $root 'shell-integration\install-windows.ps1'
    & $installerPath -SkipPowerShell -SkipWsl -Quiet
    $installedRoot = Join-Path $installerFixture 'Automexia\shell-integration'
    $installedCmd = Join-Path $installedRoot 'automexia.cmd'
    $installState = Join-Path $installedRoot 'install-state.json'
    if (-not (Test-Path -LiteralPath $installedCmd) -or -not (Test-Path -LiteralPath $installState)) {
        throw 'Windows automatic installer did not publish CMD integration and its state stamp'
    }
    $installedCmdBytes = [IO.File]::ReadAllBytes($installedCmd)
    if (($installedCmdBytes | Where-Object { $_ -gt 127 } | Select-Object -First 1) -or
        ($installedCmdBytes.Length -ge 3 -and
         $installedCmdBytes[0] -eq 0xEF -and
         $installedCmdBytes[1] -eq 0xBB -and
         $installedCmdBytes[2] -eq 0xBF)) {
        throw 'Windows installer did not deploy CMD integration as BOM-free ASCII'
    }
    $firstState = Get-Content -LiteralPath $installState -Raw
    & $installerPath -SkipPowerShell -SkipWsl -Quiet
    if ((Get-Content -LiteralPath $installState -Raw) -cne $firstState) {
        throw 'Windows automatic installer rewrote a current installation'
    }
    Add-Content -LiteralPath $installedCmd -Value 'locally altered'
    & $installerPath -SkipPowerShell -SkipWsl -Quiet
    if ((Get-Content -LiteralPath $installedCmd -Raw) -match 'locally altered') {
        throw 'Windows automatic installer did not repair an altered installed integration'
    }
} finally {
    $env:LOCALAPPDATA = $previousLocalAppData
    if (Test-Path -LiteralPath $installerFixture) {
        Remove-Item -LiteralPath $installerFixture -Recurse -Force
    }
}

$uninstall = Get-Content (Join-Path $root 'shell-integration\uninstall-windows.ps1') -Raw
if ($uninstall -notmatch 'AUTOMEXIA SHELL INTEGRATION') { throw 'uninstall marker cleanup is missing' }
Write-Output 'PASS: shell integration is idempotent, UTF-8-safe, WSL-isolated, composite-folder-aware on PowerShell/CMD/Bash/Zsh, pipeline-safe, semantically path-colored, three-row prompt-identified, full-path, resize-safe, script-safe, and uninstallable'
