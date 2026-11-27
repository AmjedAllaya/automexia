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
    function Click-ChoiceBounds($Bounds, $Scale, [double]$Fraction = 0.5) {
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * $Fraction) * $Scale)
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
    Send-AutomexiaTestControl 'open-terminal-appearance:window-control-choices'
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

    function Open-ChoicePage([string]$Phase) {
        $script:testStage = "open window controls for $Phase"
        Send-AutomexiaTestControl "open-terminal-appearance:window-control-geometry-$Phase"
        $menu = Wait-Choice { param($s) $s.settings.ready -and @($s.settings.controls | Where-Object id -eq 'window-controls.style').Count -eq 1 }
        Click-ChoiceBounds (@($menu.settings.controls | Where-Object id -eq 'window-controls.style')[0].bounds) $menu.scale_factor
        Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $s.settings.active_category -eq 'window-controls.style' -and @($s.settings.targets).Count -eq 4 }
    }

    function Set-ChoiceGeometry([string]$Id, [double]$Fraction) {
        $script:testStage = "change $Id"
        $before = Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.save_pending }
        # At higher DPI the setting row may need keyboard focus to scroll into view.
        for ($attempt = 0; $attempt -lt 12 -and $Id -notin @($before.settings.controls.id); $attempt++) {
            $previousFocus = $before.settings.focused_control
            $previousListFocus = $before.settings.list_focused
            Choice-Key 0x28
            $before = Wait-Choice { param($s) $s.settings.ready -and ($s.settings.focused_control -ne $previousFocus -or $s.settings.list_focused -ne $previousListFocus) }
        }
        $row = @($before.settings.controls | Where-Object id -eq $Id)
        if ($row.Count -ne 1) { throw "Expected one visible geometry control: $Id" }
        $revision = $before.settings.catalog_revision
        Click-ChoiceBounds $row[0].bounds $before.scale_factor $Fraction
        Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $s.settings.catalog_revision -ne $revision -and $s.settings.window_controls_style -eq 'circles' }
    }

    function Read-ChoiceFill($Bitmap, [double]$Scale, [double]$X, [double]$Y,
                             [double]$SlotWidth, [double]$Height) {
        $starts = @(0..3 | ForEach-Object { [int][Math]::Round(($X + $_ * $SlotWidth) * $Scale) })
        $top = [int][Math]::Ceiling($Y * $Scale)
        $bottom = [int][Math]::Floor(($Y + $Height) * $Scale)
        if ($starts[0] -lt 0 -or $starts[3] -gt $Bitmap.Width -or $top -lt 0 -or $bottom -gt $Bitmap.Height -or $SlotWidth -le 0 -or $top -ge $bottom) {
            throw "Circle sample is outside the owned client frame: $script:testStage"
        }
        $left = @($Bitmap.Width, $Bitmap.Width, $Bitmap.Width)
        $right = @(-1, -1, -1)
        $rows = 0
        for ($yPixel = $top; $yPixel -lt $bottom; $yPixel++) {
            $rowLeft = @(-1, -1, -1)
            $rowRight = @(-1, -1, -1)
            for ($slot = 0; $slot -lt 3; $slot++) {
                for ($xPixel = $starts[$slot]; $xPixel -lt $starts[$slot + 1]; $xPixel++) {
                    $color = $Bitmap.GetPixel($xPixel, $yPixel)
                    if ([Math]::Abs([int]$color.R - 38) -le 28 -and
                        [Math]::Abs([int]$color.G - 201) -le 28 -and
                        [Math]::Abs([int]$color.B - 90) -le 28) {
                        if ($rowLeft[$slot] -lt 0) { $rowLeft[$slot] = $xPixel }
                        $rowRight[$slot] = $xPixel
                    }
                }
            }
            if (-1 -in $rowLeft) {
                if ($rows -gt 0) { break }
                continue
            }
            # Use the first aligned band across all three slots. Taking its outer
            # edges across rows prevents foreground glyphs from splitting a circle.
            $rows++
            for ($slot = 0; $slot -lt 3; $slot++) {
                $left[$slot] = [Math]::Min($left[$slot], $rowLeft[$slot])
                $right[$slot] = [Math]::Max($right[$slot], $rowRight[$slot])
            }
        }
        if ($rows -lt 4 -or @(0..2 | Where-Object { $right[$_] - $left[$_] -lt 3 }).Count -gt 0) {
            throw "Expected three distinct green circle fills: $script:testStage"
        }
        [pscustomobject]@{
            Width = @(0..2 | ForEach-Object { $right[$_] - $left[$_] + 1 })
            Center = @(0..2 | ForEach-Object { ($right[$_] + $left[$_]) * 0.5 })
            Gap = @(0..1 | ForEach-Object { $left[$_ + 1] - $right[$_] - 1 })
        }
    }

    function Capture-ChoiceFill([string]$Name, [double]$Scale, [double]$X, [double]$Y,
                                [double]$SlotWidth, [double]$Height) {
        $directory = $configRoot
        if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
            $directory = $ModalCaptureDirectory
            $null = New-Item -ItemType Directory -Force -Path $directory
        }
        $path = Join-Path $directory ("window-controls-$Name.png")
        [void][AutomexiaResizeDriver]::CaptureClientFrame($window, $path)
        $bitmap = [Drawing.Bitmap]::new($path)
        try { Read-ChoiceFill $bitmap $Scale $X $Y $SlotWidth $Height }
        finally { $bitmap.Dispose() }
    }

    function Measure-ChoiceGeometry($Page, [string]$Phase) {
        $script:testStage = "$Phase live preview"
        $bounds = $Page.settings.preview_bounds
        # The style cards span the inset preview content width. Use their
        # bounds for the origin and exclude their own sample from the scan.
        $cards = @($Page.settings.targets | Where-Object id -like 'window-controls.choose.*')
        if ($cards.Count -ne 4) { throw 'The live preview is missing its style card bounds.' }
        $previewLeft = ($cards | ForEach-Object { $_.bounds[0] } | Measure-Object -Minimum).Minimum
        $previewRight = ($cards | ForEach-Object { $_.bounds[0] + $_.bounds[2] } | Measure-Object -Maximum).Maximum
        $previewTop = ($cards | ForEach-Object { $_.bounds[1] + $_.bounds[3] } | Measure-Object -Maximum).Maximum
        $slotWidth = [Math]::Min(46.0, ($previewRight - $previewLeft) / 3.0)
        $preview = Capture-ChoiceFill "$Phase-preview" $Page.scale_factor $previewLeft $previewTop $slotWidth ($bounds[1] + $bounds[3] - $previewTop)

        $script:testStage = "$Phase title bar"
        # C is the Settings close mnemonic. Close the sheet so it cannot cover
        # even one caption slot while the actual window controls are sampled.
        Choice-Key 0x43
        $chrome = Wait-Choice { param($s) -not $s.settings.open -and $s.chrome.custom_controls -and -not $s.chrome.maximized }
        if ($chrome.scale_factor -ne $Page.scale_factor -or $chrome.window_width -ne $Page.window_width -or $chrome.window_height -ne $Page.window_height) {
            throw 'The owned fixture changed viewport during the pixel comparison.'
        }
        $caption = Capture-ChoiceFill "$Phase-caption" $chrome.scale_factor $chrome.chrome.controls_x 0 $chrome.chrome.button_width $chrome.chrome.header_height
        [pscustomobject]@{
            Preview = $preview
            Caption = $caption
            Viewport = @($chrome.window_width, $chrome.window_height, $chrome.scale_factor) -join '/'
        }
    }

    $page = Open-ChoicePage 'baseline'
    $card = @($page.settings.targets | Where-Object id -eq 'window-controls.choose.circles')
    if ($card.Count -ne 1) { throw 'Circles is not available for the geometry test.' }
    Click-ChoiceBounds $card[0].bounds $page.scale_factor
    $page = Wait-Choice { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $s.settings.window_controls_style -eq 'circles' }
    $baseline = Measure-ChoiceGeometry $page 'standard-balanced'

    $null = Open-ChoicePage 'size'
    $page = Set-ChoiceGeometry 'window-controls.circles.size' 0.25
    $compact = Measure-ChoiceGeometry $page 'compact-balanced'

    $null = Open-ChoicePage 'spacing'
    $page = Set-ChoiceGeometry 'window-controls.circles.spacing' 0.75
    $airy = Measure-ChoiceGeometry $page 'compact-airy'
    if ($baseline.Viewport -ne $compact.Viewport -or $compact.Viewport -ne $airy.Viewport) {
        throw 'Viewport and scale must stay fixed across geometry edits.'
    }
    foreach ($surface in @('Preview', 'Caption')) {
        for ($slot = 0; $slot -lt 3; $slot++) {
            if ($baseline.$surface.Width[$slot] - $compact.$surface.Width[$slot] -lt 3 -or
                [Math]::Abs($baseline.$surface.Center[$slot] - $compact.$surface.Center[$slot]) -gt 2) {
                throw "${surface}: size must shrink each circle without moving its center."
            }
            if ([Math]::Abs($compact.$surface.Width[$slot] - $airy.$surface.Width[$slot]) -gt 2) {
                throw "${surface}: spacing must preserve each circle diameter."
            }
        }
        for ($gap = 0; $gap -lt 2; $gap++) {
            if ($airy.$surface.Gap[$gap] - $compact.$surface.Gap[$gap] -lt 3) {
                throw "${surface}: Airy spacing must widen both gaps."
            }
        }
    }

    $script:testStage = 'restart with saved circle appearance'
    Restart-AutomexiaWindowControlsFixture
    if (-not [AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1200, 740, $true)) {
        throw 'Could not restore the viewport of the restarted window control fixture.'
    }
    $null = Wait-Choice {
        param($s)
        (@($s.window_width, $s.window_height, $s.scale_factor) -join '/') -eq $airy.Viewport -and
            $null -ne $s.latest_prompt_id -and [int]$s.latest_prompt_start_count -eq 1
    }
    $page = Open-ChoicePage 'restart'
    if ($page.settings.window_controls_style -ne 'circles') { throw 'Restart lost the saved Circles style.' }
    $restored = Measure-ChoiceGeometry $page 'restored-compact-airy'
    if ($restored.Viewport -ne $airy.Viewport) { throw 'Restart changed the comparison viewport.' }
    foreach ($surface in @('Preview', 'Caption')) {
        for ($slot = 0; $slot -lt 3; $slot++) {
            if ([Math]::Abs($restored.$surface.Width[$slot] - $airy.$surface.Width[$slot]) -gt 2) {
                throw "${surface}: restart lost the saved circle size."
            }
        }
        for ($gap = 0; $gap -lt 2; $gap++) {
            if ([Math]::Abs($restored.$surface.Gap[$gap] - $airy.$surface.Gap[$gap]) -gt 2) {
                throw "${surface}: restart lost the saved circle spacing."
            }
        }
    }
    Write-Output 'PASS: independent circle size and spacing in preview and title bar, restored after process restart'
}
