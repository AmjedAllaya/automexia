# Uses the isolated configuration, physical input and owned window of resize-stress.
function Test-AutomexiaWindowControlChoices {
    function Wait-Choice($Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(12)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 25
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Window style choices did not settle: $script:testStage"
    }
    function Click-ChoiceBounds($Bounds, $Scale) {
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * 0.5) * $Scale)
        $y = [int][Math]::Round(($Bounds[1] + $Bounds[3] * 0.5) * $Scale)
        if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
            -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Could not focus window style choice' }
        $null = Wait-Choice { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x / $Scale) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y / $Scale) -le 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) { throw 'Window style click lost focus' }
    }
    function Choice-Key([int]$Key) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $false, $false)) { throw 'Window style key lost focus' }
    }
    $script:testStage = 'window control choices'
    [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1200, 740, $true)
    Send-AutomexiaTestControl 'open-customizations:window-control-choices'
    $state = Wait-Choice { param($s) $s.settings.ready -and @($s.settings.controls | Where-Object id -eq 'window-controls.style').Count -eq 1 }
    Click-ChoiceBounds (@($state.settings.controls | Where-Object id -eq 'window-controls.style')[0].bounds) $state.scale_factor
    foreach ($cycle in 0..2) {
        foreach ($style in @('soft','glass','outline','circles')) {
            $state = Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.targets).Count -eq 4 }
            $card = @($state.settings.targets | Where-Object id -eq ('window-controls.choose.' + $style))
            if ($card.Count -ne 1) { throw 'Expected exactly one card per style' }
            Click-ChoiceBounds $card[0].bounds $state.scale_factor
            $state = Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $s.settings.window_controls_style -eq $style }
            if ($cycle -eq 0 -and -not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $null = New-Item -ItemType Directory -Force -Path $ModalCaptureDirectory
                $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory ($style + '.png')))
            }
        }
    }
    Choice-Key 0x1B
    $null = Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode }
    Choice-Key 0x45
    $null = Wait-Choice { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
    Choice-Key 0x24
    Choice-Key 0x0D
    $state = Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $s.settings.window_controls_style -eq 'soft' }
    $bottom = ($state.settings.targets | ForEach-Object { $_.bounds[1] + $_.bounds[3] } | Measure-Object -Maximum).Maximum
    $left = ($state.settings.targets | ForEach-Object { $_.bounds[0] } | Measure-Object -Minimum).Minimum
    # State samples below the cards are visual examples, including their close glyph.
    Click-ChoiceBounds @(($left + 115), ($bottom + 95), 2, 2) $state.scale_factor
    $null = Wait-Choice { param($s) $s.settings.ready -and $s.settings.window_controls_style -eq 'soft' -and @($s.settings.targets).Count -eq 4 }
    Choice-Key 0x1B
    $null = Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode }
    Choice-Key 0x1B
    $null = Wait-Choice { param($s) $s.settings.ready -and $null -eq $s.settings.active_category }
    Choice-Key 0x1B
    $null = Wait-Choice { param($s) -not $s.settings.open }
    Write-Output 'PASS: four selectable window styles, repeated persistence, keyboard choice and nonexecuting live state examples'
}
