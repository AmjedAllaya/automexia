# Runs inside the existing isolated native-window/input/snapshot/cleanup owner.
function Test-AutomexiaCommandInputColors {
    $wordFailures = [Collections.Generic.List[string]]::new()
    if ($ClearShortcutOnly) {
        . (Join-Path $PSScriptRoot 'clear-shortcut-windows.ps1')
        $script:clearFailures = @()
    }
    function Wait-InputState([scriptblock]$Predicate, [int]$TimeoutSeconds = 15) {
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
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
        $diagnostic = @{ shell = $panel.shell_name; prompt = $panel.shell_prompt_active }
        if ($CommandInputWslSetupOnly) {
            $diagnostic.owned_route = $null -ne $panel.route_id
            $diagnostic.popup_visible = $null -ne $state.wsl_setup
            $diagnostic.popup_simulation = $state.wsl_setup.simulation -ceq $true
            $diagnostic.popup_actions = @($state.wsl_setup.actions | Where-Object { $null -ne $_ }).Count
            $diagnostic.tab_count = $state.window_tab_count
        } elseif ($CommandInputWslInstalledBash) {
            $diagnostic.integrated = [bool]$panel.shell_integration
            $diagnostic.distro_matches = $panel.shell_distro -ceq $CommandInputWslDistro
        } else {
            $diagnostic.last_control = $state.last_control
            $diagnostic.line = $panel.cursor_line_text
        }
        Write-Host ($diagnostic | ConvertTo-Json -Compress)
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
    if ($CommandInputWslSetupOnly) {
        $wsl = Join-Path ([Environment]::SystemDirectory) 'wsl.exe'
        $registered = @(& $wsl --list --quiet | ForEach-Object {
            $_.Replace(([char]0).ToString(), '').Trim()
        } | Where-Object { $_ })
        if ($LASTEXITCODE -ne 0 -or $registered.Count -ne 1 -or $registered[0] -ine $CommandInputWslDistro) {
            throw 'Native WSL setup requires the explicitly selected fixture as the sole registered distro'
        }
        $guestPreflight = 'test "${SHELL##*/}" = bash && ! command -v eza >/dev/null 2>&1 && test -x /usr/bin/sudo && test -x /usr/bin/apt-get'
        & $wsl --distribution $CommandInputWslDistro --exec /bin/sh -c $guestPreflight
        if ($LASTEXITCODE -ne 0) { throw 'Native WSL setup requires Bash, sudo and APT, with eza absent from the fixture PATH' }

        function Assert-WslSetupUsesTerminalWindow {
            $visibleWindows = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
            if ($visibleWindows.Count -ne 1 -or $visibleWindows[0] -ne $window) {
                throw 'WSL setup did not stay in the owned terminal window'
            }
        }
        function Enter-WslSetupPopup([string]$Command = 'wsl') {
            # Type and submit in one native batch. Waiting for a grid repaint
            # before Enter would bypass the fast-input regression.
            if (-not [AutomexiaResizeDriver]::SendActionText($window, $Command, $true)) {
                throw 'Could not submit bare wsl in the owned PowerShell window'
            }
            $offer = Wait-InputState { param($s)
                $null -ne $s.wsl_setup -and
                    @($s.wsl_setup.actions | Where-Object { $_.label -ceq 'Install eza' }).Count -eq 1
            } 22
            Assert-WslSetupUsesTerminalWindow
            if ($offer.wsl_setup.simulation -isnot [bool] -or -not $offer.wsl_setup.simulation) {
                throw 'The native fixture requires the debug WSL simulation flag'
            }
            return $offer
        }
        function Invoke-WslSetupDefault($Snapshot, [string]$Label) {
            if ($Snapshot.wsl_setup.focused_action -cne $Label -or
                -not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x0D, $false, $false, $false)) {
                throw "WSL setup did not accept the expected default action $Label"
            }
            $null = Wait-InputState { param($s) $null -eq $s.wsl_setup } 5
        }
        function Invoke-WslSetupSimulation($Snapshot) {
            $action = @($Snapshot.wsl_setup.actions | Where-Object { $_.label -ceq 'Install eza' })
            if ($action.Count -ne 1 -or @($action[0].bounds).Count -ne 4 -or
                $action[0].bounds[2] -le 0 -or $action[0].bounds[3] -le 0) {
                throw 'The simulation action has no clickable client bounds'
            }
            if ($Snapshot.wsl_setup.simulation -isnot [bool] -or -not $Snapshot.wsl_setup.simulation) {
                throw 'Install eza requires the debug fixture simulation flag'
            }
            $bounds = $action[0].bounds
            $x = [int][Math]::Round($bounds[0] + $bounds[2] * 0.5)
            $y = [int][Math]::Round($bounds[1] + $bounds[3] * 0.5)
            $hoverX = [int][Math]::Round($bounds[0] + $bounds[2] * 0.4)
            if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
                -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $hoverX, $y)) {
                throw 'Could not position the native pointer on Install eza'
            }
            $null = Wait-InputState { param($s)
                $s.wsl_setup.simulation -is [bool] -and $s.wsl_setup.simulation -and
                    $s.wsl_setup.hovered_action -ceq 'Install eza'
            } 5
            if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y) -or
                -not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) {
                throw 'Native Install eza click lost terminal window ownership'
            }
        }
        function Assert-WslSessionRetained([string]$Marker) {
            $pattern = '(?m)^' + [regex]::Escape($Marker) + ':Linux[ \t\r]*$'
            $probe = 'HISTFILE=/dev/null; set +o history; if ! command -v eza >/dev/null; then printf ''{0}:%s\n'' "$(uname -s)"; fi' -f $Marker
            Submit-FixtureCommand $probe $Marker
            $retained = Wait-InputState { param($s)
                ([string](Get-ActiveAutomexiaPanel $s).visible_text) -cmatch $pattern
            }
            $target = Get-ActiveAutomexiaPanel $retained
            if ($target.route_id -ne $source.route_id -or $target.shell_pid -ne $source.shell_pid -or
                $retained.window_tab_count -ne $baseline.window_tab_count -or
                $null -ne $retained.wsl_setup) {
                throw "Original WSL route or tab count changed during $script:testStage"
            }
        }

        $script:testStage = 'PowerShell prompt before bare WSL launch'
        $baseline = Wait-InputState { param($s)
            $panel = Get-ActiveAutomexiaPanel $s
            $panel.shell_name -eq 'PowerShell' -and $panel.shell_prompt_active -and
                ([string]$panel.raw_cursor_line_text).Trim() -eq [string][char]0x03bb
        }
        $source = Get-ActiveAutomexiaPanel $baseline
        if ($null -ne $baseline.wsl_setup) { throw 'WSL setup appeared before a WSL launch' }
        $script:testStage = 'WSL inventory command returns without a setup popup'
        if (-not [AutomexiaResizeDriver]::SendActionText($window, 'wsl --list --quiet', $true)) {
            throw 'Could not submit the read-only WSL inventory command'
        }
        $null = Wait-InputState { param($s)
            if ($null -ne $s.wsl_setup) { throw 'WSL inventory opened the setup popup' }
            $panel = Get-ActiveAutomexiaPanel $s
            $s.latest_prompt_id -ne $baseline.latest_prompt_id -and $panel.route_id -eq $source.route_id -and
                $panel.shell_name -eq 'PowerShell' -and $panel.shell_prompt_active
        }
        $script:testStage = 'bare WSL startup opens the in-app setup offer'
        $offer = Enter-WslSetupPopup
        if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
            [void][IO.Directory]::CreateDirectory($captureRoot)
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot 'wsl-setup.png'))
        }
        Invoke-WslSetupDefault $offer 'Cancel'

        $script:testStage = 'Cancel keeps the original route in WSL with eza absent'
        Assert-WslSessionRetained 'AMX_WSL_SETUP_RETAINED'
        Submit-FixtureCommand 'exit' 'wsl-setup-exit'
        $null = Wait-InputState { param($s)
            $panel = Get-ActiveAutomexiaPanel $s
            $s.latest_prompt_id -ne $baseline.latest_prompt_id -and $panel.route_id -eq $source.route_id -and
                $panel.shell_name -eq 'PowerShell' -and $panel.shell_prompt_active
        }
        Write-Host 'PASS: the in-app WSL offer defaulted to Cancel, kept eza absent, and exit restored PowerShell.'

        $script:testStage = 'second bare WSL launch exposes the in-app simulation action'
        $offer = Enter-WslSetupPopup 'WSL.EXE'
        Invoke-WslSetupSimulation $offer

        $script:testStage = 'in-app Install eza completes the APT simulation'
        $completed = Wait-InputState { param($s)
            $null -ne $s.wsl_setup -and ([string]$s.wsl_setup.summary).Contains(
                'APT simulation completed successfully. No packages were installed.')
        } 22
        Assert-WslSetupUsesTerminalWindow
        Invoke-WslSetupDefault $completed 'Close'
        $script:testStage = 'simulation keeps the original route in WSL with eza absent'
        Assert-WslSessionRetained 'AMX_WSL_SETUP_SIMULATED'
        Write-Host 'PASS: the in-app Install eza click completed the APT simulation and returned to the original WSL route with eza still absent.'
        return
    }
    $powerShellFields = @(
        @{ Text = 'docker'; Rgb = @(181, 140, 255) },
        @{ Text = '-a'; Rgb = @(181, 140, 255) }
    )
    $commandRgb = @(180, 180, 210)
    $argumentRgb = @(180, 210, 180)
    $plainFields = @(
        @{ Text = 'docker'; Rgb = $commandRgb },
        @{ Text = 'ps'; Rgb = $argumentRgb },
        @{ Text = '-a'; Rgb = @(206, 122, 255) }
    )
    function Assert-InputColors([string]$Name, [string]$Draft = 'docker ps -a', [hashtable[]]$Fields = $powerShellFields) {
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
        $draftStart = $line.Length - $Draft.Length
        $visualRow = $panel.source_row_visual_origins[[int]$panel.cursor_row]
        if ($null -eq $visualRow) { throw 'Typed input has no visible projected row' }
        $y = [int][Math]::Floor([double]$panel.grid_origin[1] + [int]$visualRow * [double]$panel.cell_height)
        foreach ($field in $Fields) {
            $token = $field.Text
            $rgb = $field.Rgb
            $offset = $Draft.IndexOf($token, [StringComparison]::Ordinal)
            if ($offset -lt 0) { throw "Unknown typed input fixture field <$token> in <$Draft>" }
            $column = $draftStart + $offset
            $x = [int][Math]::Floor([double]$panel.grid_origin[0] + $column * [double]$panel.cell_width)
            $pixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $x, $y,
                [int]($token.Length * [double]$panel.cell_width), [int]$panel.cell_height, $rgb[0], $rgb[1], $rgb[2], 24)
            if ($pixels.TargetColorSampleCount -lt 4) {
                Write-Host (@{ token = $token; expected = $rgb; region = @($x, $y, [int]($token.Length * [double]$panel.cell_width), [int]$panel.cell_height); source_row = $panel.cursor_row; visual_row = $visualRow } | ConvertTo-Json -Compress)
                throw "$Name did not color typed token $token (accent pixels: $($pixels.TargetColorSampleCount))"
            }
        }
        Write-Host "${Name}: typed input colors passed"
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
        $infoRgb = @(90, 210, 230)
        $errorRgb = @(230, 100, 110)
        $successRgb = @(100, 220, 150)
        $debugRgb = @(140, 160, 220)
        $subjectRgb = @(238, 164, 205)
        $versionRgb = @(198, 149, 240)
        $detailRgb = @(180, 220, 243)
        $cases = @(
            @{ Text = '/workspace/project'; Rgb = $infoRgb },
            @{ Text = 'No resources found in demo namespace.'; Rgb = $infoRgb },
            @{ Text = '[ERROR] fixture failed'; Rgb = $errorRgb },
            @{ Text = '[WARN] fixture retrying'; Rgb = @(230, 190, 100) },
            @{ Text = 'Build succeeded'; Rgb = $successRgb },
            @{ Text = 'level=debug fixture'; Rgb = $debugRgb },
            @{ Text = 'logout'; Rgb = $infoRgb },
            @{
                Text = 'Get:1 http://m.invalid s/m amd64 demo amd64 1.2.3 [24 kB]'
                Fields = @(
                    @{ Text = 'Get:1'; Rgb = $infoRgb },
                    @{ Text = 'http://m.invalid'; Rgb = $detailRgb },
                    @{ Text = 'demo'; Rgb = $subjectRgb },
                    @{ Text = '1.2.3'; Rgb = $versionRgb },
                    @{ Text = '[24 kB]'; Rgb = $debugRgb }
                )
            },
            @{
                Text = 'Fetched 25.3 MB in 3s (8565 kB/s)'
                Fields = @(
                    @{ Text = 'Fetched'; Rgb = $successRgb },
                    @{ Text = '25.3 MB'; Rgb = $debugRgb },
                    @{ Text = '8565 kB/s'; Rgb = $debugRgb }
                )
            },
            @{
                Text = 'Preparing to unpack .../00-demo_1.2.3_amd64.deb ...'
                Fields = @(
                    @{ Text = 'Preparing to unpack'; Rgb = $infoRgb },
                    @{ Text = '00-demo_1.2.3_amd64.deb'; Rgb = $subjectRgb }
                )
            },
            @{
                Text = 'Unpacking demo (1.2.3) over (1.2.2) ...'
                Fields = @(
                    @{ Text = 'Unpacking'; Rgb = $infoRgb },
                    @{ Text = 'demo'; Rgb = $subjectRgb },
                    @{ Text = '1.2.3'; Rgb = $versionRgb },
                    @{ Text = '1.2.2'; Rgb = $detailRgb }
                )
            },
            @{
                Text = 'Universal Time is now: Mon Jan 5 08:00:00 UTC 2026.'
                Fields = @(
                    @{ Text = 'Universal Time is now:'; Rgb = $infoRgb },
                    @{ Text = 'Mon Jan 5 08:00:00 UTC 2026'; Rgb = $debugRgb }
                )
            },
            @{
                Text = 'Err:2 https://m.invalid stable InRelease'
                Fields = @(@{ Text = 'Err:2'; Rgb = $errorRgb })
            }
        )
        $shell = (Get-ActiveAutomexiaPanel $baseline).shell_name
        # Completion is owned by this output fixture. A nested shell's latest
        # displayed command-result key may still refer to its parent shell.
        $sentinel = 'PLAIN_OUTPUT_DONE_' + $Name
        $texts = @($cases | ForEach-Object { $_.Text }) + @($sentinel)
        $literals = @($texts | ForEach-Object { "'$_'" }) -join ' '
        if ($shell -eq 'CMD') {
            $command = @($texts | ForEach-Object { 'echo ' + $_.Replace('(', '^(').Replace(')', '^)') }) -join '&'
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
                if ($rows[$i].TrimEnd() -ceq $case.Text) { $sourceRow = $i }
            }
            if ($sourceRow -lt 0) { throw "$Name lost a plain output fixture row" }
            $visualRow = $panel.source_row_visual_origins[$sourceRow]
            if ($null -eq $visualRow) { throw "$Name output row is not visible" }
            $y = [int][Math]::Floor([double]$panel.grid_origin[1] + [int]$visualRow * [double]$panel.cell_height)
            $fields = if ($case.ContainsKey('Fields')) { $case.Fields } else { $case }
            foreach ($field in $fields) {
                # The fixture uses ASCII, so these offsets are terminal columns.
                $column = $case.Text.IndexOf($field.Text, [StringComparison]::Ordinal)
                if ($column -lt 0) { throw "Unknown fixture field <$($field.Text)> in <$($case.Text)>" }
                $x = [int][Math]::Floor([double]$panel.grid_origin[0] + $column * [double]$panel.cell_width)
                $width = [int]($field.Text.Length * [double]$panel.cell_width)
                $height = [int]$panel.cell_height
                $pixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $x, $y, $width, $height,
                    $field.Rgb[0], $field.Rgb[1], $field.Rgb[2], 24)
                if ($pixels.TargetColorSampleCount -lt 8) {
                    if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                        $directory = [IO.Path]::GetFullPath($ModalCaptureDirectory)
                        [void][IO.Directory]::CreateDirectory($directory)
                        [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $directory ($Name + '-plain-output-failure.png')))
                        $facts = @{ fixture = $case.Text; field = $field.Text; rgb = $field.Rgb; region = @($x, $y, $width, $height); source_row = $sourceRow; visual_row = $visualRow; cell_size = @($panel.cell_width, $panel.cell_height); grid_origin = $panel.grid_origin; matched_pixels = $pixels.TargetColorSampleCount }
                        $facts.row_render_facts = @($panel.row_render_facts | Select-Object -Skip ([Math]::Max(0, $sourceRow - 2)) -First 4)
                        [IO.File]::WriteAllText((Join-Path $directory ($Name + '-plain-output-failure.json')), ($facts | ConvertTo-Json -Depth 4))
                    }
                    throw "$Name did not apply customized output colour to <$($field.Text)> in <$($case.Text)> ($($pixels.TargetColorSampleCount) pixels)"
                }
            }
        }
        if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            $directory = [IO.Path]::GetFullPath($ModalCaptureDirectory)
            [void][IO.Directory]::CreateDirectory($directory)
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $directory ($Name + '-plain-output.png')))
        }
        Write-Host "${Name}: plain output rows and package-manager fields use customized colours"
    }
    function Test-InstalledWslBashListing {
        $script:testStage = 'installed WSL Bash icon listing'
        $listingRoot = Join-Path $configRoot 'installed-wsl-icon-fixture'
        [void](New-Item -ItemType Directory -Path $listingRoot)
        [IO.File]::WriteAllText((Join-Path $listingRoot 'fixture-icon.rs'), '')
        $guestPathOutput = & wsl.exe --distribution $CommandInputWslDistro --exec wslpath -a $listingRoot
        if ($LASTEXITCODE -ne 0 -or @($guestPathOutput).Count -ne 1) {
            throw 'Could not translate the installed WSL Bash listing fixture path'
        }
        $guestPath = $guestPathOutput.Trim()
        if (-not $guestPath.StartsWith('/') -or $guestPath.Contains("'") -or $guestPath -match '\p{Cc}') {
            throw 'Invalid installed WSL Bash listing fixture path'
        }

        # Disable persistent history in this test shell before its next prompt.
        # Split the receipts so a command echo cannot satisfy their assertions.
        $command = "unset HISTFILE; set +o history; if type -P eza >/dev/null; then ls -1 -- '$guestPath'; else builtin printf '%s\n' AMX_WSL_EZA_''MISSING; fi; builtin printf '%s\n' AMX_WSL_LS_''DONE"
        Submit-FixtureCommand $command 'input-color-installed-wsl-listing'
        $state = Wait-InputState { param($s)
            $panel = Get-ActiveAutomexiaPanel $s
            $panel.shell_name -eq 'bash' -and $panel.shell_prompt_active -and
                ([string]$panel.raw_cursor_line_text).Trim() -eq [string][char]0x03bb -and
                ([string]$panel.visible_text) -cmatch '(?m)^AMX_WSL_LS_DONE[ \t\r]*$'
        }
        $output = [string](Get-ActiveAutomexiaPanel $state).visible_text
        if ($output -cmatch '(?m)^AMX_WSL_EZA_MISSING[ \t\r]*$') {
            throw 'Installed WSL Bash icon check requires eza on the Bash PATH'
        }
        # The retained grid can use a NUL cell beside a wide glyph.
        if ($output -cnotmatch '(?m)^[\x00 \t]*(?:\p{Co}|[\uDB80-\uDBFF][\uDC00-\uDFFF])[\x00 \t]+fixture-icon\.rs[ \t\r]*$') {
            throw 'Installed WSL Bash ls did not display a Nerd Font icon beside the fixture filename'
        }
        Write-Host 'Installed WSL Bash: interactive ls displays a Nerd Font icon.'
    }
    function Test-ContextTagValue($Segments, [string]$Value) {
        if ([string]::IsNullOrWhiteSpace($Value)) { return $false }
        foreach ($segment in $Segments) {
            if ($segment -ceq $Value) { return $true }
            if ($segment.Length -gt 1 -and $segment.EndsWith([string][char]0x2026) -and
                $Value.StartsWith($segment.Substring(0, $segment.Length - 1), [StringComparison]::Ordinal)) {
                return $true
            }
        }
        return $false
    }
    [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1200, 780, $true)
    [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)
    try {
        Assert-InputColors 'PowerShell'
        if (-not $CommandInputSkipCmd) {
            Send-AutomexiaTestControl 'write-text:input-color-cmd-type:cmd'
            # PSReadLine may display an inline prediction after the typed buffer.
            $null = Wait-InputState { param($s) $s.last_control -eq 'write-text:input-color-cmd-type:cmd' -and ([string](Get-ActiveAutomexiaPanel $s).cursor_line_text).Contains('cmd') }
            Send-AutomexiaTestControl 'write-line:input-color-cmd-submit:'
            $script:testStage = 'enter CMD'
            $null = Wait-InputState { param($s) (Get-ActiveAutomexiaPanel $s).shell_name -eq 'CMD' -and ([string](Get-ActiveAutomexiaPanel $s).cursor_line_text).Contains([char]0x03BB) }
            Assert-InputColors 'CMD' -Fields $plainFields
            Submit-FixtureCommand 'exit' 'input-color-cmd-exit'
            $null = Wait-InputState { param($s) (Get-ActiveAutomexiaPanel $s).shell_name -eq 'PowerShell' }
            Assert-InputColors 'PowerShell-return'
        }
        if (-not [string]::IsNullOrWhiteSpace($CommandInputWslDistro)) {
            if ($CommandInputWslDistro.Contains("'") -or $CommandInputWslDistro -match '\p{Cc}') {
                throw 'Fixture distro name must not contain single quotes or control characters'
            }
            if (-not $CommandInputWslInstalledBash) {
                $wslRoot = (& wsl.exe --distribution $CommandInputWslDistro --exec wslpath -a $root).Trim()
                $wslConfig = (& wsl.exe --distribution $CommandInputWslDistro --exec wslpath -a $configRoot).Trim()
                if ($wslRoot.Contains("'") -or $wslConfig.Contains("'")) { throw 'Fixture paths must not contain single quotes' }
                # Isolated startup resources; never modify the user's shell profiles.
                [IO.File]::WriteAllText((Join-Path $configRoot 'input.bashrc'), "source '$wslRoot/shell-integration/bash/automexia.bash'`n", [Text.UTF8Encoding]::new($false))
                [IO.File]::WriteAllText((Join-Path $configRoot '.zshrc'), "source '$wslRoot/shell-integration/zsh/automexia.zsh'`n", [Text.UTF8Encoding]::new($false))
            }
            foreach ($shell in $CommandInputWslShells) {
                $script:testStage = "enter WSL $shell"
                $launch = "wsl.exe --distribution '$CommandInputWslDistro'"
                if ($CommandInputWslInstalledBash) {
                    $script:testStage = 'enter WSL through installed Bash startup'
                } else {
                    $launch += ' --exec env TERM_PROGRAM=Automexia COLORTERM=truecolor AUTOMEXIA_SHELL_INTEGRATION=1 HISTFILE=/dev/null '
                    if ($shell -eq 'bash') { $launch += "bash --noprofile --rcfile '$wslConfig/input.bashrc' -i" }
                    elseif ($shell -eq 'zsh') { $launch += "ZDOTDIR='$wslConfig' zsh -d -i" }
                    else { $launch += "fish --no-config --init-command `"set -g fish_history ''; source '$wslRoot/shell-integration/fish/automexia.fish'; function fish_prompt; printf '\u03bb '; end; set -g fish_color_command yellow; set -g fish_color_param cyan`" -i" }
                }
                Submit-FixtureCommand $launch ('input-color-' + $shell)
                $null = Wait-InputState { param($s)
                    $panel = Get-ActiveAutomexiaPanel $s
                    $editorReady = $panel.shell_name -eq $shell -and $panel.shell_prompt_active -and
                        ([string]$panel.cursor_line_text).Trim() -eq [string][char]0x03bb
                    if (-not $editorReady -or -not $CommandInputWslInstalledBash) { return $editorReady }
                    if (-not $panel.shell_integration -or $panel.shell_distro -cne $CommandInputWslDistro -or
                        $null -eq $s.latest_prompt_id -or [int]$s.latest_prompt_start_count -ne 1) { return $false }
                    $ownedEditor = @($s.semantic_rows | Where-Object {
                        $_.input_ready -and $_.row -eq $panel.cursor_row -and $_.generation -eq $s.latest_prompt_id
                    }).Count -eq 1
                    $paintedContext = @($s.prompt_context_paints | Where-Object {
                        $null -ne $_[0] -and $_[0] -eq $s.latest_prompt_id
                    }).Count -gt 0
                    $ownedEditor -and $paintedContext -and $panel.context_session_id -eq $panel.route_id -and
                        (Test-ContextTagValue $panel.context_segments $panel.shell_distro) -and
                        (Test-ContextTagValue $panel.context_segments $panel.shell_user)
                }
                if ($CommandInputWslInstalledBash) {
                    Write-Host 'Installed WSL Bash: requested distro, integration marker, owned prompt, and distribution and user tags verified.'
                }
                if ($shell -eq 'fish') {
                    Assert-InputColors 'WSL-fish-native' -Draft 'echo --help' -Fields @(
                        @{ Text = 'echo'; Rgb = @(180, 210, 180) },
                        @{ Text = '--help'; Rgb = @(180, 180, 210) }
                    )
                } else {
                    Assert-InputColors ('WSL-' + $shell) -Fields $plainFields
                }
                if ($shell -eq 'bash' -and -not ($PlainOutputColorsOnly -or $WordDeletionOnly -or $ClearShortcutOnly)) {
                    # Type and erase this draft through the input path without submitting it.
                    Assert-InputColors 'WSL-bash-package-draft' -Draft 'sudo apt install nodejs' -Fields @(
                        @{ Text = 'sudo'; Rgb = $commandRgb },
                        @{ Text = 'apt'; Rgb = $argumentRgb },
                        @{ Text = 'install'; Rgb = $argumentRgb },
                        @{ Text = 'nodejs'; Rgb = $argumentRgb }
                    )
                }
                if ($CommandInputWslInstalledBash) { Test-InstalledWslBashListing }
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
