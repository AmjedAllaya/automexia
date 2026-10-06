# Runs inside the existing isolated native-window/input/snapshot/cleanup owner.
function Test-AutomexiaCommandInputColors {
    $wordFailures = [Collections.Generic.List[string]]::new()
    if ($ClearShortcutOnly) {
        . (Join-Path $PSScriptRoot 'clear-shortcut-windows.ps1')
        $script:clearFailures = @()
    }
    function Wait-InputState([scriptblock]$Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 40
        } while ([DateTime]::UtcNow -lt $deadline)
        $panel = Get-ActiveAutomexiaPanel $state
        if (($ClearShortcutOnly -or $PlainOutputColorsOnly) -and -not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            [void][IO.Directory]::CreateDirectory([IO.Path]::GetFullPath($ModalCaptureDirectory))
            [IO.File]::WriteAllText((Join-Path $ModalCaptureDirectory 'failed-state.json'), ($state | ConvertTo-Json -Depth 12))
        }
        Write-Host (@{ shell = $panel.shell_name; prompt = $panel.shell_prompt_active; last_control = $state.last_control; line = $panel.cursor_line_text } | ConvertTo-Json -Compress)
        throw "Command input did not settle: $script:testStage"
    }
    function Clear-InputDraft([string]$Draft, [string]$EmptyLine) {
        # Use the same PTY fixture owner as typing. This color test does not
        # depend on whether another desktop window owns keyboard focus.
        $shell = (Get-ActiveAutomexiaPanel (Read-AutomexiaSnapshot)).shell_name
        $erase = if ($shell -in @('bash', 'zsh', 'fish')) { [char]0x7f } else { [char]0x08 }
        Send-AutomexiaTestControl ('write-text:input-color-erase:' + ([string]$erase * $Draft.Length))
        $null = Wait-InputState { param($s) ([string](Get-ActiveAutomexiaPanel $s).raw_cursor_line_text) -eq $EmptyLine }
    }
    function Submit-FixtureCommand([string]$Command, [string]$Id) {
        Send-AutomexiaTestControl ('write-text:' + $Id + '-type:' + $Command)
        $null = Wait-InputState { param($s) $s.last_control -eq ($Id + '-type') -or $s.last_control -eq ('write-text:' + $Id + '-type:' + $Command) }
        Send-AutomexiaTestControl ('write-line:' + $Id + '-submit:')
    }
    function Assert-InputColors([string]$Name, [string]$Draft = 'docker ps -a', [int[]]$CommandRgb = @(181, 140, 255), [int[]]$OptionRgb = @(181, 140, 255)) {
        if ($WordDeletionOnly) { Test-WordDeletion $Name; return }
        if ($ClearShortcutOnly) { Test-AutomexiaClearShortcut $Name; return }
        if ($PlainOutputColorsOnly) { Test-PlainOutputColors $Name; return }
        # A shell identity receipt can precede its new prompt after nested CMD
        # exits. Capture the erase oracle only from the completed empty prompt.
        $script:testStage = "empty prompt before typed command colors: $Name"
        $baseline = Wait-InputState { param($s)
            $panel = Get-ActiveAutomexiaPanel $s
            $panel.shell_prompt_active -and
                ([string]$panel.raw_cursor_line_text).Trim() -eq [string][char]0x03bb
        }
        $empty = [string](Get-ActiveAutomexiaPanel $baseline).raw_cursor_line_text
        $script:testStage = "typed command colors: $Name"
        $control = 'write-text:input-color-' + $Name + ':' + $Draft
        Send-AutomexiaTestControl $control
        $state = Wait-InputState { param($s) $s.last_control -eq $control -and ([string](Get-ActiveAutomexiaPanel $s).cursor_line_text).EndsWith($Draft) }
        $panel = Get-ActiveAutomexiaPanel $state
        Write-Host ("{0}: viewport {1}x{2}, window {3}x{4}, scale {5}" -f $Name, $state.columns, $state.rows, $state.window_width, $state.window_height, $state.scale_factor)
        if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            $directory = [IO.Path]::GetFullPath($ModalCaptureDirectory)
            [void][IO.Directory]::CreateDirectory($directory)
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $directory ($Name + '-input.png')))
        }
        $line = [string]$panel.cursor_line_text
        foreach ($token in @($Draft.Split(' ')[0], $Draft.Split(' ')[-1])) {
            $rgb = if ($token.StartsWith('-')) { $OptionRgb } else { $CommandRgb }
            $column = $line.IndexOf($token, [StringComparison]::Ordinal)
            $x = [int][Math]::Floor([double]$panel.grid_origin[0] + $column * [double]$panel.cell_width)
            $visualRow = $panel.source_row_visual_origins[[int]$panel.cursor_row]
            if ($null -eq $visualRow) { throw 'Typed input has no visible projected row' }
            $y = [int][Math]::Floor([double]$panel.grid_origin[1] + [int]$visualRow * [double]$panel.cell_height)
            $pixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $x, $y,
                [int]($token.Length * [double]$panel.cell_width), [int]$panel.cell_height, $rgb[0], $rgb[1], $rgb[2], 24)
            if ($pixels.TargetColorSampleCount -lt 4) {
                Write-Host (@{ token = $token; expected = $rgb; region = @($x, $y, [int]($token.Length * [double]$panel.cell_width), [int]$panel.cell_height); source_row = $panel.cursor_row; visual_row = $visualRow } | ConvertTo-Json -Compress)
                throw "$Name did not color typed token $token (accent pixels: $($pixels.TargetColorSampleCount))"
            }
        }
        Write-Host "${Name}: typed command and option colors passed"
        Clear-InputDraft $Draft $empty
    }
    function Test-WordDeletion([string]$Name) {
        $script:testStage = "word deletion empty prompt: $Name"
        $baseline = Wait-InputState { param($s)
            $p = Get-ActiveAutomexiaPanel $s
            $p.shell_prompt_active -and ([string]$p.raw_cursor_line_text).Trim() -eq [string][char]0x03bb
        }
        $draft = 'echo alpha beta gamma'
        foreach ($case in @(
            @{ Name = 'backward'; Key = 0x08; Offset = -1; Expected = 'echo alpha beta ' },
            @{ Name = 'backward-start'; Key = 0x08; Offset = 0; Expected = $draft },
            @{ Name = 'backward-repeat'; Key = 0x08; Offset = -1; Count = 2; Expected = 'echo alpha ' },
            @{ Name = 'forward'; Key = 0x2e; Offset = 5; Expected = 'echo  beta gamma' },
            @{ Name = 'forward-middle'; Key = 0x2e; Offset = 7; Expected = 'echo al beta gamma' },
            @{ Name = 'forward-repeat'; Key = 0x2e; Offset = 5; Count = 2; Expected = 'echo  gamma' },
            @{ Name = 'forward-end'; Key = 0x2e; Offset = -1; Expected = $draft }
        )) {
            $script:testStage = "$Name word deletion $($case.Name)"
            Send-AutomexiaTestControl ('write-text:word-' + $Name + '-' + $case.Name + ':' + $draft)
            $null = Wait-InputState { param($s)
                ([string](Get-ActiveAutomexiaPanel $s).raw_cursor_line_text).TrimEnd() -eq ([string][char]0x03bb + ' ' + $draft)
            }
            if ($case.Offset -ge 0) {
                for ($i = $draft.Length; $i -gt $case.Offset; $i--) {
                    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x25, $true, $false, $false)) { throw 'Cannot move owned input cursor' }
                }
                $null = Wait-InputState { param($s) $s.cursor_column -eq (2 + $case.Offset) }
            }
            $count = if ($case.ContainsKey('Count')) { $case.Count } else { 1 }
            for ($keyIndex = 0; $keyIndex -lt $count; $keyIndex++) {
                if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $case.Key, ($case.Key -eq 0x2e), $true, $false)) { throw 'Cannot send word deletion key to owned window' }
                Start-Sleep -Milliseconds 80
            }
            Start-Sleep -Milliseconds 500
            $p = Get-ActiveAutomexiaPanel (Read-AutomexiaSnapshot)
            $actual = ([string]$p.raw_cursor_line_text).TrimEnd()
            $expected = ([string][char]0x03bb + ' ' + $case.Expected).TrimEnd()
            if ($actual -ne $expected) {
                $wordFailures.Add("$script:testStage expected <$expected>, actual <$actual>")
            }
            Write-Host "$script:testStage : <$actual>"
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x43, $false, $true, $false)) { throw 'Cannot cancel disposable input through owned window' }
            $null = Wait-InputState { param($s)
                $p = Get-ActiveAutomexiaPanel $s
                $p.shell_prompt_active -and ([string]$p.raw_cursor_line_text).Trim() -eq [string][char]0x03bb
            }
        }
    }
    function Test-PlainOutputColors([string]$Name) {
        $script:testStage = "plain output palette: $Name"
        $baseline = Wait-InputState { param($s)
            $p = Get-ActiveAutomexiaPanel $s
            $p.shell_prompt_active -and ([string]$p.raw_cursor_line_text).Trim() -eq [string][char]0x03bb
        }
        $cases = @(
            @{ Text = '/workspace/project'; Rgb = @(90, 210, 230) },
            @{ Text = 'No resources found in demo namespace.'; Rgb = @(90, 210, 230) },
            @{ Text = '[ERROR] fixture failed'; Rgb = @(230, 100, 110) },
            @{ Text = '[WARN] fixture retrying'; Rgb = @(230, 190, 100) },
            @{ Text = 'Build succeeded'; Rgb = @(100, 220, 150) },
            @{ Text = 'level=debug fixture'; Rgb = @(140, 160, 220) },
            @{ Text = 'logout'; Rgb = @(90, 210, 230) }
        )
        $shell = (Get-ActiveAutomexiaPanel $baseline).shell_name
        # Completion is owned by this output fixture. A nested shell's latest
        # displayed command-result key may still refer to its parent shell.
        $sentinel = 'PLAIN_OUTPUT_DONE_' + $Name
        $texts = @($cases | ForEach-Object { $_.Text }) + @($sentinel)
        $literals = @($texts | ForEach-Object { "'$_'" }) -join ' '
        if ($shell -eq 'CMD') {
            $command = @($texts | ForEach-Object { 'echo ' + $_ }) -join '&'
        } elseif ($shell -in @('bash', 'zsh', 'fish')) {
            $command = "printf '%s\n' $literals"
        } else {
            $command = 'Write-Output ' + (@($texts | ForEach-Object { "'$_'" }) -join ',')
        }
        Submit-FixtureCommand $command ('plain-output-' + $Name)
        $state = Wait-InputState { param($s)
            $p = Get-ActiveAutomexiaPanel $s
            $p.shell_prompt_active -and
                ([string]$p.raw_cursor_line_text).Trim() -eq [string][char]0x03bb -and
                @(([string]$p.visible_text).Split("`n") | Where-Object { $_.TrimEnd() -eq $sentinel }).Count -eq 1
        }
        $panel = Get-ActiveAutomexiaPanel $state
        $rows = ([string]$panel.visible_text).Split("`n")
        foreach ($case in $cases) {
            $sourceRow = -1
            for ($i = 0; $i -lt $rows.Count; $i++) {
                if ($rows[$i].TrimEnd() -eq $case.Text) { $sourceRow = $i }
            }
            if ($sourceRow -lt 0) { throw "$Name lost a plain output fixture row" }
            $visualRow = $panel.source_row_visual_origins[$sourceRow]
            if ($null -eq $visualRow) { throw "$Name output row is not visible" }
            $x = [int][Math]::Floor([double]$panel.grid_origin[0])
            $y = [int][Math]::Floor([double]$panel.grid_origin[1] + [int]$visualRow * [double]$panel.cell_height)
            $pixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $x, $y,
                [int]($case.Text.Length * [double]$panel.cell_width), [int]$panel.cell_height,
                $case.Rgb[0], $case.Rgb[1], $case.Rgb[2], 24)
            if ($pixels.TargetColorSampleCount -lt 8) {
                if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                    $directory = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                    [void][IO.Directory]::CreateDirectory($directory)
                    [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $directory ($Name + '-plain-output-failure.png')))
                    $facts = @{ fixture = $case.Text; rgb = $case.Rgb; region = @($x, $y, [int]($case.Text.Length * [double]$panel.cell_width), [int]$panel.cell_height); source_row = $sourceRow; visual_row = $visualRow; cell_size = @($panel.cell_width, $panel.cell_height); grid_origin = $panel.grid_origin; matched_pixels = $pixels.TargetColorSampleCount }
                    $facts.row_render_facts = @($panel.row_render_facts | Select-Object -Skip ([Math]::Max(0, $sourceRow - 2)) -First 4)
                    [IO.File]::WriteAllText((Join-Path $directory ($Name + '-plain-output-failure.json')), ($facts | ConvertTo-Json -Depth 4))
                }
                throw "$Name did not apply customized output colour to <$($case.Text)> ($($pixels.TargetColorSampleCount) pixels)"
            }
        }
        if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            $directory = [IO.Path]::GetFullPath($ModalCaptureDirectory)
            [void][IO.Directory]::CreateDirectory($directory)
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $directory ($Name + '-plain-output.png')))
        }
        Write-Host "${Name}: seven plain output rows use customized Information/Error/Warning/Success/Debug colours"
    }
    [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1200, 780, $true)
    [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)
    try {
        Assert-InputColors 'PowerShell'
        Send-AutomexiaTestControl 'write-text:input-color-cmd-type:cmd'
        # PSReadLine may display an inline prediction after the typed buffer.
        $null = Wait-InputState { param($s) $s.last_control -eq 'write-text:input-color-cmd-type:cmd' -and ([string](Get-ActiveAutomexiaPanel $s).cursor_line_text).Contains('cmd') }
        Send-AutomexiaTestControl 'write-line:input-color-cmd-submit:'
        $script:testStage = 'enter CMD'
        $null = Wait-InputState { param($s) (Get-ActiveAutomexiaPanel $s).shell_name -eq 'CMD' -and ([string](Get-ActiveAutomexiaPanel $s).cursor_line_text).Contains([char]0x03BB) }
        Assert-InputColors 'CMD'
        Submit-FixtureCommand 'exit' 'input-color-cmd-exit'
        $null = Wait-InputState { param($s) (Get-ActiveAutomexiaPanel $s).shell_name -eq 'PowerShell' }
        Assert-InputColors 'PowerShell-return'
        if (-not [string]::IsNullOrWhiteSpace($CommandInputWslDistro)) {
            $wslRoot = (& wsl.exe --distribution $CommandInputWslDistro --exec wslpath -a $root).Trim()
            $wslConfig = (& wsl.exe --distribution $CommandInputWslDistro --exec wslpath -a $configRoot).Trim()
            if ($wslRoot.Contains("'") -or $wslConfig.Contains("'") -or $CommandInputWslDistro.Contains("'")) { throw 'Fixture paths must not contain single quotes' }
            # Isolated startup resources; never modify the user's shell profiles.
            [IO.File]::WriteAllText((Join-Path $configRoot 'input.bashrc'), "source '$wslRoot/shell-integration/bash/automexia.bash'`n", [Text.UTF8Encoding]::new($false))
            [IO.File]::WriteAllText((Join-Path $configRoot '.zshrc'), "source '$wslRoot/shell-integration/zsh/automexia.zsh'`n", [Text.UTF8Encoding]::new($false))
            foreach ($shell in @('bash', 'zsh', 'fish')) {
                $script:testStage = "enter WSL $shell"
                $launch = "wsl.exe --distribution '$CommandInputWslDistro' --exec env TERM_PROGRAM=Automexia COLORTERM=truecolor AUTOMEXIA_SHELL_INTEGRATION=1 HISTFILE=/dev/null "
                if ($shell -eq 'bash') { $launch += "bash --noprofile --rcfile '$wslConfig/input.bashrc' -i" }
                elseif ($shell -eq 'zsh') { $launch += "ZDOTDIR='$wslConfig' zsh -d -i" }
                else { $launch += "fish --no-config --init-command `"set -g fish_history ''; source '$wslRoot/shell-integration/fish/automexia.fish'; function fish_prompt; printf '\u03bb '; end; set -g fish_color_command yellow; set -g fish_color_param cyan`" -i" }
                Submit-FixtureCommand $launch ('input-color-' + $shell)
                $null = Wait-InputState { param($s) (Get-ActiveAutomexiaPanel $s).shell_name -eq $shell -and (Get-ActiveAutomexiaPanel $s).shell_prompt_active -and ([string](Get-ActiveAutomexiaPanel $s).cursor_line_text).Trim() -eq [string][char]0x03bb }
                if ($shell -eq 'fish') { Assert-InputColors 'WSL-fish-native' 'echo --help' @(180, 210, 180) @(180, 180, 210) }
                else { Assert-InputColors ('WSL-' + $shell) }
                Submit-FixtureCommand 'exit' ('input-color-' + $shell + '-exit')
                $null = Wait-InputState { param($s) (Get-ActiveAutomexiaPanel $s).shell_name -eq 'PowerShell' -and (Get-ActiveAutomexiaPanel $s).shell_prompt_active }
            }
            Assert-InputColors 'PowerShell-after-WSL'
        } else { Write-Host 'EXTERNAL: WSL Bash/Zsh/Fish not requested; pass -CommandInputWslDistro to exercise them.' }
        if ($ClearShortcutOnly -and $script:clearFailures.Count -gt 0) {
            throw ('Ctrl+L regressions: ' + ($script:clearFailures -join '; '))
        }
        if ($WordDeletionOnly -and $wordFailures.Count -gt 0) {
            throw ('Word-deletion regressions: ' + ($wordFailures -join '; '))
        }
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
}
