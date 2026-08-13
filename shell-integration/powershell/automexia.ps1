# Automexia shell integration — metadata + unified prompt/editor colors only.
# Native filesystem icons are added through PowerShell's formatting layer.
# Interactive `cmd`/`cmd.exe` launches are kept inside the current Automexia PTY
# and initialized by the dedicated CMD integration. Explicit cmd arguments are
# passed to the native executable unchanged.
if (($env:TERM_PROGRAM -eq 'Automexia' -or $env:AUTOMEXIA_SHELL_INTEGRATION -eq '1') -and -not $global:AutomexiaShellIntegrationLoaded) {
    $global:AutomexiaShellIntegrationLoaded = $true
    $script:AutomexiaEsc = [char]27
    $script:AutomexiaBel = [char]7
    [uint64]$script:AutomexiaPromptGeneration = 0
    $script:AutomexiaCachedPromptPath = $null
    $script:AutomexiaCachedStyledPromptPath = ''

    $env:COLORTERM = 'truecolor'
    $env:TERM_PROGRAM = 'Automexia'
    $env:AUTOMEXIA_SHELL_INTEGRATION = '1'

    $script:AutomexiaCmdExecutable = if ($env:ComSpec) {
        $env:ComSpec
    } else {
        Join-Path $env:SystemRoot 'System32\cmd.exe'
    }
    $script:AutomexiaCmdIntegration = Join-Path $PSScriptRoot 'automexia.cmd'
    if (-not (Test-Path -LiteralPath $script:AutomexiaCmdIntegration)) {
        # Repository-source layout; installation flattens these files.
        $script:AutomexiaCmdIntegration = Join-Path $PSScriptRoot '..\cmd\automexia.cmd'
    }
    function global:Invoke-AutomexiaCmd {
        [CmdletBinding(PositionalBinding = $false)]
        param(
            [Parameter(ValueFromRemainingArguments = $true, Position = 0)]
            [object[]]$ArgumentList
        )

        $nativeArguments = @($ArgumentList | ForEach-Object { [string]$_ })
        if ($nativeArguments.Count -gt 0 -or
            $env:AUTOMEXIA_PLAIN_CMD -eq '1' -or
            -not (Test-Path -LiteralPath $script:AutomexiaCmdIntegration)) {
            & $script:AutomexiaCmdExecutable @nativeArguments
            return
        }

        # Invoke the executable directly: the child inherits this exact ConPTY
        # and returns to the existing PowerShell session when the user types exit.
        $startup = 'call "{0}"' -f $script:AutomexiaCmdIntegration.Replace('"', '""')
        & $script:AutomexiaCmdExecutable /D /K $startup
    }
    Set-Alias -Name cmd -Value Invoke-AutomexiaCmd -Scope Global -Force
    Set-Alias -Name cmd.exe -Value Invoke-AutomexiaCmd -Scope Global -Force

    # Keep PowerShell's native `ls` -> Get-ChildItem alias and real filesystem
    # objects. Parsing a format file synchronously adds about 60 ms to every new
    # pane, so schedule that presentation-only work just after the first prompt
    # becomes visible. The event still runs automatically without user input.
    $formatPath = $null
    if ($env:AUTOMEXIA_PLAIN_LS -ne '1') {
        $candidateFormatPath = Join-Path $PSScriptRoot 'automexia.format.ps1xml'
        if (Test-Path -LiteralPath $candidateFormatPath) {
            $formatPath = $candidateFormatPath
        }
    }

    # Publish identity first, then the integration-ready marker. This lets a
    # newly cloned pane replace its equivalent seed in one atomic VT batch.
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_shell_name=UG93ZXJTaGVsbA==$script:AutomexiaBel")
    $automexiaShellUser = [Convert]::ToBase64String(
        [Text.Encoding]::UTF8.GetBytes([Environment]::UserName)
    )
    $automexiaShellExecutable = try {
        (Get-Process -Id $PID -ErrorAction Stop).Path
    } catch {
        if ($PSVersionTable.PSEdition -eq 'Core') {
            Join-Path $PSHOME 'pwsh.exe'
        } else {
            Join-Path $PSHOME 'powershell.exe'
        }
    }
    $automexiaShellPath = [Convert]::ToBase64String(
        [Text.Encoding]::UTF8.GetBytes($automexiaShellExecutable)
    )
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_shell_user=$automexiaShellUser$script:AutomexiaBel")
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_shell_path=$automexiaShellPath$script:AutomexiaBel")
    # Clear WSL-only metadata that may remain after a nested wsl.exe session
    # exits back into this native PowerShell terminal. Empty base64 payloads
    # are valid OSC 1337 user-variable values.
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_distro=$script:AutomexiaBel")
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_os_version=$script:AutomexiaBel")
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_shell=MQ==$script:AutomexiaBel")

    # ConsoleHost normally imports PSReadLine before the first prompt. Use the
    # already-loaded module when available; otherwise one direct import is much
    # cheaper than scanning every PSModulePath entry and then importing it.
    $psReadLineModule = Get-Module -Name PSReadLine -ErrorAction SilentlyContinue
    if ($null -eq $psReadLineModule) {
        $psReadLineModule = Import-Module PSReadLine -PassThru -ErrorAction SilentlyContinue
    }
    $configureEditorColors = $null -ne $psReadLineModule
    if ($null -ne $psReadLineModule) {
        # Mark editable-prompt state false immediately before PowerShell executes
        # an accepted line, while preserving any user history handler.
        try {
            $script:AutomexiaPreviousHistoryHandler = (Get-PSReadLineOption).AddToHistoryHandler
            Set-PSReadLineOption -AddToHistoryHandler {
                param($line)
                [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_prompt_active=MA==$script:AutomexiaBel")
                [Console]::Write("$script:AutomexiaEsc]133;C$script:AutomexiaBel")
                if ($script:AutomexiaPreviousHistoryHandler -is [scriptblock]) {
                    return (& $script:AutomexiaPreviousHistoryHandler $line)
                }
                if ($script:AutomexiaPreviousHistoryHandler -is [System.Delegate]) {
                    # PSReadLine 2.0 exposes its default predicate as a
                    # Func<string, object> which returns AddToHistoryOption.
                    # The call operator and Boolean coercion lose that contract
                    # and can leave AcceptLine waiting indefinitely.
                    return $script:AutomexiaPreviousHistoryHandler.DynamicInvoke(
                        [object[]]@($line)
                    )
                }
                return $true
            } -ErrorAction SilentlyContinue
        } catch {}

    }


    function script:Get-AutomexiaPromptPath {
        return (Get-Location).Path
    }

    function script:Format-AutomexiaPromptPath {
        param([Parameter(Mandatory)][string]$Path)

        if ($Path -ceq $script:AutomexiaCachedPromptPath) {
            return $script:AutomexiaCachedStyledPromptPath
        }

        # A restrained semantic hierarchy makes every component of a long path
        # independently scannable. The three parent hues repeat predictably;
        # the root and active leaf remain unique anchors. ANSI changes only
        # presentation, so copied text and VT path matching stay exact.
        $rootColor = "$script:AutomexiaEsc[38;2;98;176;255m"
        $parentColors = @(
            "$script:AutomexiaEsc[38;2;80;213;255m",
            "$script:AutomexiaEsc[38;2;167;139;250m",
            "$script:AutomexiaEsc[38;2;72;167;255m"
        )
        $leafColor = "$script:AutomexiaEsc[38;2;184;243;107m"
        $separatorColor = "$script:AutomexiaEsc[38;2;88;113;141m"
        $resetColor = "$script:AutomexiaEsc[0m"
        $tokens = [regex]::Split($Path, '([\\/]+)')
        $componentCount = 0
        foreach ($token in $tokens) {
            if ($token.Length -gt 0 -and $token[0] -ne [char]92 -and $token[0] -ne [char]47) {
                $componentCount++
            }
        }

        $componentIndex = 0
        $styled = [Text.StringBuilder]::new($Path.Length + 96)
        foreach ($token in $tokens) {
            if ($token.Length -eq 0) { continue }
            if ($token[0] -eq [char]92 -or $token[0] -eq [char]47) {
                [void]$styled.Append($separatorColor).Append($token)
                continue
            }

            $componentIndex++
            $color = if ($componentIndex -eq $componentCount) {
                $leafColor
            } elseif ($componentIndex -eq 1) {
                $rootColor
            } else {
                $parentColors[($componentIndex - 2) % $parentColors.Count]
            }
            [void]$styled.Append($color).Append($token)
        }
        [void]$styled.Append($resetColor)

        $script:AutomexiaCachedPromptPath = $Path
        $script:AutomexiaCachedStyledPromptPath = $styled.ToString()
        return $script:AutomexiaCachedStyledPromptPath
    }

    function global:prompt {
        $succeeded = $?
        $exitCode = if ($succeeded) { 0 } elseif ($null -ne $global:LASTEXITCODE) { $global:LASTEXITCODE } else { 1 }
        $script:AutomexiaPromptGeneration++
        $promptPath = Get-AutomexiaPromptPath
        $path = $promptPath.Replace('\', '/')
        $osc7 = "$script:AutomexiaEsc]7;file://localhost/$path$script:AutomexiaBel"
        $titleText = "PowerShell - {0}" -f $path
        $title = "$script:AutomexiaEsc]2;$titleText$script:AutomexiaBel"
        $done = "$script:AutomexiaEsc]133;D;$exitCode$script:AutomexiaBel"
        $ready = "$script:AutomexiaEsc]1337;SetUserVar=automexia_prompt_active=MQ==$script:AutomexiaBel"
        $start = "$script:AutomexiaEsc]133;A;aid=$script:AutomexiaPromptGeneration$script:AutomexiaBel"
        $continuation = "$script:AutomexiaEsc]133;P;k=c;aid=$script:AutomexiaPromptGeneration$script:AutomexiaBel"
        $input = "$script:AutomexiaEsc]133;B$script:AutomexiaBel"
        $pathPrompt = Format-AutomexiaPromptPath -Path $promptPath
        $lambdaColor = if ($exitCode -eq 0) { '38;2;124;255;178' } else { '38;2;255;111;145' }
        $lambdaGlyph = [char]0x03BB
        $lambda = $script:AutomexiaEsc + "[" + $lambdaColor + "m" + $lambdaGlyph + $script:AutomexiaEsc + "[0m "
        # Automexia owns the stable context spacer and complete path rows.
        # PSReadLine owns only the lambda, editable command, and cursor row.
        # This prevents PSReadLine's delayed SIGWINCH repaint from erasing or
        # duplicating a path which the terminal has already reflowed.
        [Console]::Write(
            $done + $osc7 + $title + $ready + $start + " `r`n" +
            $continuation + $pathPrompt + "`r`n" + $continuation
        )
        return $lambda + $input
    }

    # Formatting and editor colors mutate state owned by this PowerShell
    # runspace. Apply them synchronously while the integration is being sourced,
    # before PSReadLine owns the first editable prompt. Deferring either change
    # through Timer or PowerShell.OnIdle callbacks can race an accepted command
    # and leave ConPTY waiting inside the callback.
    try {
        if ($null -ne $formatPath) {
            Update-FormatData -PrependPath $formatPath -ErrorAction Stop
        }
        if ($configureEditorColors) {
            Set-PSReadLineOption -Colors @{
                Default   = '#EEF7F2'
                Command   = '#B58CFF'
                Keyword   = '#FF6F91'
                String    = '#FFD166'
                Operator  = '#89AFA0'
                Parameter = '#B58CFF'
                Variable  = '#48A7FF'
                Number    = '#FFD166'
                Type      = '#A4FFD0'
                Member    = '#82C2FF'
                Comment   = '#5D7A70'
            } -ErrorAction SilentlyContinue
            foreach ($extra in @(
                @{ Error = '#FF6F91' },
                @{ InlinePrediction = '#456B5D' },
                @{ Selection = '#90AEBE' }
            )) {
                try { Set-PSReadLineOption -Colors $extra -ErrorAction SilentlyContinue } catch {}
            }
        }
    } catch {
        # Presentation is optional. Unusual constrained hosts retain the
        # functional prompt and native shell behavior without custom styling.
        Write-Warning "Automexia shell styling could not be loaded: $($_.Exception.Message)"
    }
    Remove-Variable psReadLineModule, configureEditorColors, candidateFormatPath, formatPath, automexiaShellUser, automexiaShellExecutable, automexiaShellPath -ErrorAction SilentlyContinue
}
