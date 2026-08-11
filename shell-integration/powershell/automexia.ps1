# Automexia shell integration — metadata + unified prompt/editor colors only.
# No custom CLI commands are installed or intercepted.
if (($env:TERM_PROGRAM -eq 'Automexia' -or $env:AUTOMEXIA_SHELL_INTEGRATION -eq '1') -and -not $global:AutomexiaShellIntegrationLoaded) {
    $global:AutomexiaShellIntegrationLoaded = $true
    $script:AutomexiaEsc = [char]27
    $script:AutomexiaBel = [char]7

    $env:COLORTERM = 'truecolor'
    $env:TERM_PROGRAM = 'Automexia'
    $env:AUTOMEXIA_SHELL_INTEGRATION = '1'

    # Announce the integration once (base64('1') per OSC 1337 SetUserVar).
    [Console]::Write("$script:AutomexiaEsc]1337;SetUserVar=automexia_shell=MQ==$script:AutomexiaBel")

    if (Get-Module -ListAvailable -Name PSReadLine) {
        Import-Module PSReadLine -ErrorAction SilentlyContinue
        # Keep the base table compatible with Windows PowerShell's older
        # PSReadLine while layering newer roles independently. One unsupported
        # modern key must never disable the entire Automexia editor palette.
        try {
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
        } catch {}
        foreach ($extra in @(
            @{ Error = '#FF6F91' },
            @{ InlinePrediction = '#456B5D' },
            @{ Selection = '#90AEBE' }
        )) {
            try { Set-PSReadLineOption -Colors $extra -ErrorAction SilentlyContinue } catch {}
        }

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
        $raw = (Get-Location).Path
        if ($raw.Length -le 64) { return $raw }
        $parts = @($raw -split '[\/]') | Where-Object { $_ -ne '' }
        if ($parts.Count -le 3) { return $raw }
        $tail = ($parts[($parts.Count - 3)..($parts.Count - 1)] -join '\')
        if ($raw -match '^[A-Za-z]:') { return $raw.Substring(0, 2) + '\...\' + $tail }
        return '...\' + $tail
    }

    function global:prompt {
        $succeeded = $?
        $exitCode = if ($succeeded) { 0 } elseif ($null -ne $global:LASTEXITCODE) { $global:LASTEXITCODE } else { 1 }
        $path = (Get-Location).Path.Replace('\', '/')
        $osc7 = "$script:AutomexiaEsc]7;file://localhost/$path$script:AutomexiaBel"
        $titleText = "{0}@{1}: {2}" -f $env:USERNAME, $env:COMPUTERNAME, $path
        $title = "$script:AutomexiaEsc]2;$titleText$script:AutomexiaBel"
        $done = "$script:AutomexiaEsc]133;D;$exitCode$script:AutomexiaBel"
        $ready = "$script:AutomexiaEsc]1337;SetUserVar=automexia_prompt_active=MQ==$script:AutomexiaBel"
        $start = "$script:AutomexiaEsc]133;A$script:AutomexiaBel"
        $continuation = "$script:AutomexiaEsc]133;P;k=c$script:AutomexiaBel"
        $input = "$script:AutomexiaEsc]133;B$script:AutomexiaBel"
        $pathPrompt = $script:AutomexiaEsc + "[38;2;72;167;255m" + (Get-AutomexiaPromptPath) + $script:AutomexiaEsc + "[0m "
        $lambdaColor = if ($exitCode -eq 0) { '38;2;124;255;178' } else { '38;2;255;111;145' }
        $lambdaGlyph = [char]0x03BB
        $lambda = $script:AutomexiaEsc + "[" + $lambdaColor + "m" + $lambdaGlyph + $script:AutomexiaEsc + "[0m "
        # DevOps owns the blank row above. The normal shell location stays on
        # the command line where users expect it, followed by lambda + input.
        return $done + $osc7 + $title + $ready + $start + "`n" + $continuation + $pathPrompt + $lambda + $input
    }
}
