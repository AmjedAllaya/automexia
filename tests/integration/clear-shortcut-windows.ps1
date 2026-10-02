# Uses the existing owned native window and real foreground keyboard route.
function Test-AutomexiaClearShortcut([string]$Name) {
    function Get-ClearEditor($Panel) {
        $rows = ([string]$Panel.visible_text).Split("`n")
        $first = -1
        for ($i = 0; $i -le [int]$Panel.cursor_row -and $i -lt $rows.Count; $i++) {
            if ($rows[$i].StartsWith([string][char]0x03bb + ' ') -or $rows[$i] -eq [string][char]0x03bb) { $first = $i }
        }
        $value = if ($first -ge 0) { ($rows[$first..([int]$Panel.cursor_row)] -join '').TrimEnd() } else { '' }
        return @{ First = $first; Text = $value }
    }
    Submit-FixtureCommand 'echo CLEAR_OLD_OUTPUT_812' ('clear-seed-' + $Name)
    $null = Wait-InputState { param($s)
        $p = Get-ActiveAutomexiaPanel $s
        $p.shell_prompt_active -and ([string]$p.raw_cursor_line_text).Trim() -eq [string][char]0x03bb -and
            ([string]$p.visible_text).Contains('CLEAR_OLD_OUTPUT_812')
    }
    $columns = [int](Read-AutomexiaSnapshot).columns
    $wrapped = 'echo CLEAR_WRAPPED_914_' + ('x' * ($columns + 12))
    foreach ($draft in @('', 'echo CLEAR_DRAFT_913', $wrapped)) {
        $kind = if (-not $draft) { 'empty' } elseif ($draft -eq $wrapped) { 'wrapped' } else { 'partial' }
        $expectedInput = ([string][char]0x03bb + ' ' + $draft).TrimEnd()
        if ($draft) {
            Send-AutomexiaTestControl ('write-text:clear-draft-' + $Name + ':' + $draft)
            $null = Wait-InputState { param($s) (Get-ClearEditor (Get-ActiveAutomexiaPanel $s)).Text -eq $expectedInput }
        }
        foreach ($repeat in 1..2) {
            $script:testStage = "Ctrl+L $Name $kind repeat=$repeat"
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                [void][IO.Directory]::CreateDirectory([IO.Path]::GetFullPath($ModalCaptureDirectory))
                $before = Read-AutomexiaSnapshot
                [IO.File]::WriteAllText((Join-Path $ModalCaptureDirectory ("before-$Name-$kind-$repeat.json")), ($before | ConvertTo-Json -Depth 12))
            }
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x4c, $false, $true, $false)) {
                throw 'Cannot deliver Ctrl+L to the owned native window'
            }
            # A clear need not emit bytes (CMD). Read the resulting stable frame.
            Start-Sleep -Milliseconds 650
            $state = Read-AutomexiaSnapshot
            $panel = Get-ActiveAutomexiaPanel $state
            $text = [string]$panel.visible_text
            $problems = @()
            if ($text.Contains('CLEAR_OLD_OUTPUT_812')) { $problems += 'old output remains' }
            if (($text.ToCharArray() | Where-Object { $_ -eq [char]0x03bb } | Measure-Object).Count -ne 1) { $problems += 'not exactly one input prompt' }
            $editor = Get-ClearEditor $panel
            if ($editor.Text -ne $expectedInput) { $problems += 'input changed' }
            if ($draft) {
                $visual = if ($editor.First -ge 0) { $panel.source_row_visual_origins[$editor.First] } else { $null }
                $rgb = if ($Name -like '*fish*') { @(180, 210, 180) } else { @(181, 140, 255) }
                if ($null -eq $visual) { $problems += 'draft has no visible row' }
                else {
                    $x = [int][Math]::Floor([double]$panel.grid_origin[0] + 2 * [double]$panel.cell_width)
                    $y = [int][Math]::Floor([double]$panel.grid_origin[1] + [int]$visual * [double]$panel.cell_height)
                    $pixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $x, $y,
                        [int](4 * [double]$panel.cell_width), [int]$panel.cell_height, $rgb[0], $rgb[1], $rgb[2], 24)
                    if ($pixels.TargetColorSampleCount -lt 4) { $problems += 'preserved command is not painted' }
                }
            }
            # Integrated prompt: row zero is the information-bar spacer, then
            # the complete directory and one editor row. Native cursor stays owned.
            $rawLines = $text.Split("`n")
            $lines = @($rawLines)
            $top = @($panel.source_row_visual_origins)
            $first = 0
            while ($first -lt $top.Count -and ($null -eq $top[$first] -or [int]$top[$first] -ne 0)) { $first++ }
            if ($first -lt $rawLines.Count) { $lines = @($rawLines[$first..($rawLines.Count - 1)]) }
            if ($Name -like '*fish*') {
                if ($lines.Count -lt 2 -or $lines[0].Trim() -or -not $lines[1].Contains([char]0x03bb)) { $problems += 'Fish information bar missing at top' }
            } elseif ($lines.Count -lt 3 -or -not $lines[1].Trim() -or $lines[1].Contains([char]0x03bb)) { $problems += 'context missing at top' }
            $topTags = @($state.prompt_context_paints | Where-Object {
                $y = [double]$_[2][1] * [double]$state.scale_factor
                $y -ge [double]$panel.grid_origin[1] - 1 -and
                    $y -lt [double]$panel.grid_origin[1] + [double]$panel.cell_height
            })
            if ($topTags.Count -eq 0) { $problems += 'information tags are not painted at top' }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                [void][IO.Directory]::CreateDirectory([IO.Path]::GetFullPath($ModalCaptureDirectory))
                $stem = Join-Path $ModalCaptureDirectory ("clear-$Name-$kind-$repeat")
                [IO.File]::WriteAllText(($stem + '.json'), ($state | ConvertTo-Json -Depth 12))
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, ($stem + '.png'))
            }
            if ($problems.Count) { $script:clearFailures += "$script:testStage : $($problems -join ', ')" }
            Write-Host "$script:testStage : $($problems -join ', ')"
        }
        if ($draft) {
            # Verify the real editor retained the buffer without accepting it.
            Send-AutomexiaTestControl ('write-line:clear-accept-' + $Name + '-' + $kind + ':')
            $null = Wait-InputState { param($s)
                $p = Get-ActiveAutomexiaPanel $s
                $expectedOutput = $draft.Substring(5)
                $joined = ([string]$p.visible_text).Replace("`n", '')
                $p.shell_prompt_active -and ([string]$p.raw_cursor_line_text).Trim() -eq [string][char]0x03bb -and
                    [regex]::Matches($joined, [regex]::Escape($expectedOutput)).Count -eq 2
            }
        }
    }
}
