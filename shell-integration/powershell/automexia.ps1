# Automexia shell integration — metadata + unified prompt/editor colors only.
# Native filesystem icons are added through PowerShell's formatting layer.
# No custom CLI commands are installed or intercepted.
if (($env:TERM_PROGRAM -eq 'Automexia' -or $env:AUTOMEXIA_SHELL_INTEGRATION -eq '1') -and -not $global:AutomexiaShellIntegrationLoaded) {
    $global:AutomexiaShellIntegrationLoaded = $true
    $script:AutomexiaEsc = [char]27
    $script:AutomexiaBel = [char]7
    [uint64]$script:AutomexiaPromptGeneration = 0

    $env:COLORTERM = 'truecolor'
    $env:TERM_PROGRAM = 'Automexia'
    $env:AUTOMEXIA_SHELL_INTEGRATION = '1'

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

    # Announce the integration once (base64('1') per OSC 1337 SetUserVar).
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_shell=MQ==$script:AutomexiaBel")
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_shell_name=UG93ZXJTaGVsbA==$script:AutomexiaBel")
    # Clear WSL-only metadata that may remain after a nested wsl.exe session
    # exits back into this native PowerShell terminal. Empty base64 payloads
    # are valid OSC 1337 user-variable values.
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_distro=$script:AutomexiaBel")
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_os_version=$script:AutomexiaBel")

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
                if ($null -ne $script:AutomexiaPreviousHistoryHandler) {
                    return [bool](& $script:AutomexiaPreviousHistoryHandler $line)
                }
                return $true
            } -ErrorAction SilentlyContinue
        } catch {}

    }


    function script:Get-AutomexiaPromptPath {
        return (Get-Location).Path
    }

    function global:prompt {
        $succeeded = $?
        $exitCode = if ($succeeded) { 0 } elseif ($null -ne $global:LASTEXITCODE) { $global:LASTEXITCODE } else { 1 }
        $script:AutomexiaPromptGeneration++
        $path = (Get-Location).Path.Replace('\', '/')
        $osc7 = "$script:AutomexiaEsc]7;file://localhost/$path$script:AutomexiaBel"
        $titleText = "PowerShell - {0}" -f $path
        $title = "$script:AutomexiaEsc]2;$titleText$script:AutomexiaBel"
        $done = "$script:AutomexiaEsc]133;D;$exitCode$script:AutomexiaBel"
        $ready = "$script:AutomexiaEsc]1337;SetUserVar=automexia_prompt_active=MQ==$script:AutomexiaBel"
        $start = "$script:AutomexiaEsc]133;A;aid=$script:AutomexiaPromptGeneration$script:AutomexiaBel"
        $continuation = "$script:AutomexiaEsc]133;P;k=c;aid=$script:AutomexiaPromptGeneration$script:AutomexiaBel"
        $input = "$script:AutomexiaEsc]133;B$script:AutomexiaBel"
        $pathPrompt = $script:AutomexiaEsc + "[38;2;72;167;255m" + (Get-AutomexiaPromptPath) + $script:AutomexiaEsc + "[0m"
        $lambdaColor = if ($exitCode -eq 0) { '38;2;124;255;178' } else { '38;2;255;111;145' }
        $lambdaGlyph = [char]0x03BB
        $lambda = $script:AutomexiaEsc + "[" + $lambdaColor + "m" + $lambdaGlyph + $script:AutomexiaEsc + "[0m "
        # Keep only the renderer-owned context row outside PSReadLine. The
        # complete path and short command row are one multiline editor prompt,
        # so PSReadLine redraws both after SIGWINCH instead of leaving the head
        # of a path in scrollback after repeated narrow/wide reflow.
        [Console]::Write($done + $osc7 + $title + $ready + $start + " `r`n" + $continuation)
        return $pathPrompt + "`r`n" + $continuation + $lambda + $input
    }

    # Formatting and editor colors do not affect shell correctness. Apply them
    # after the first prompt is visible so parsing the icon view and modern
    # PSReadLine roles cannot delay a newly created tab or split.
    if ($null -ne $formatPath -or $configureEditorColors) {
        $deferredEnhancements = [pscustomobject]@{
            FormatPath = $formatPath
            ConfigureEditorColors = $configureEditorColors
        }
        try {
            $script:AutomexiaEnhancementTimer = [System.Timers.Timer]::new(250)
            $script:AutomexiaEnhancementTimer.AutoReset = $false
            $script:AutomexiaEnhancementSubscription = Register-ObjectEvent `
                -InputObject $script:AutomexiaEnhancementTimer `
                -EventName Elapsed `
                -MaxTriggerCount 1 `
                -MessageData $deferredEnhancements `
                -Action {
                    try {
                        if ($null -ne $event.MessageData.FormatPath) {
                            Update-FormatData -PrependPath $event.MessageData.FormatPath -ErrorAction Stop
                        }
                        if ($event.MessageData.ConfigureEditorColors) {
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
                        Write-Warning "Automexia deferred shell styling could not be loaded: $($_.Exception.Message)"
                    } finally {
                        $event.Sender.Dispose()
                    }
                }
            $script:AutomexiaEnhancementTimer.Start()
        } catch {
            # Event registration is optional. Unusual constrained hosts retain
            # the functional prompt and native shell behavior without styling.
            Write-Warning "Automexia deferred shell styling could not be scheduled: $($_.Exception.Message)"
        }
    }
    Remove-Variable psReadLineModule, configureEditorColors, candidateFormatPath, formatPath, deferredEnhancements -ErrorAction SilentlyContinue
}
