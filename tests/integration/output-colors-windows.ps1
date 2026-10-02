# Loaded by the existing Windows native harness. Uses its isolated config,
# real shell, window/input driver, presented-frame snapshots and owned cleanup.
function Test-AutomexiaOutputColors {
    function Wait-OutputState([scriptblock]$Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(12)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 40
        } while ([DateTime]::UtcNow -lt $deadline)
        Write-Host ($state.settings | ConvertTo-Json -Depth 6 -Compress)
        Write-Host ("Last control: " + $state.last_control)
        Capture-OutputFrame 'failure'
        throw "Output color state did not settle: $script:testStage"
    }
    function Click-OutputControl($Bounds) {
        if ($null -eq $Bounds -or @($Bounds).Count -ne 4) { throw 'Missing output control geometry' }
        $x = [int][Math]::Round([double]$Bounds[0] + [double]$Bounds[2] * 0.5)
        $y = [int][Math]::Round([double]$Bounds[1] + [double]$Bounds[3] * 0.5)
        $scale = [double](Read-AutomexiaSnapshot).scale_factor
        $physicalX = [int][Math]::Round($x * $scale)
        $physicalY = [int][Math]::Round($y * $scale)
        if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
            -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $physicalX + 6, $physicalY)) { throw 'Pointer move failed' }
        # The physical move already emits WM_MOUSEMOVE. Posting another move
        # from this DPI-virtualized driver can scale its coordinates twice and
        # race the actual pointer. Settle a nearby position first so opening a
        # new sheet under an unchanged pointer still receives a real move.
        $null = Wait-OutputState { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - ($physicalX + 6) / $scale) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y) -le 1 }
        if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $physicalX, $physicalY)) { throw 'Pointer move failed' }
        $null = Wait-OutputState { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y) -le 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $physicalX, $physicalY)) {
            throw "Real pointer click lost foreground or physical client ownership during $script:testStage"
        }
    }
    function Open-OutputCategory([string]$Id) {
        $script:testStage = 'open color category ' + $Id
        Send-AutomexiaTestControl ('open-customizations:' + [guid]::NewGuid().ToString('N'))
        $rootState = Wait-OutputState { param($s) $s.settings.ready -and $null -eq $s.settings.active_category -and $s.settings.package_inventory_ready }
        # Inventory and font/layout can update immediately after opening the
        # sheet. A press across that refresh is intentionally canceled by the
        # view, so require the presented control to settle before the click.
        $stableUntil = [DateTime]::UtcNow.AddSeconds(12)
        $priorBounds = $null
        $settled = $false
        do {
            $control = @($rootState.settings.controls | Where-Object id -eq $Id)
            if ($rootState.settings.ready -and $rootState.settings.package_inventory_ready -and $control.Count -eq 1) {
                $bounds = @($control[0].bounds)
                if ($null -ne $priorBounds -and
                    [Math]::Abs([double]$bounds[0] - [double]$priorBounds[0]) -lt 0.25 -and
                    [Math]::Abs([double]$bounds[1] - [double]$priorBounds[1]) -lt 0.25 -and
                    [Math]::Abs([double]$bounds[2] - [double]$priorBounds[2]) -lt 0.25 -and
                    [Math]::Abs([double]$bounds[3] - [double]$priorBounds[3]) -lt 0.25) {
                    $settled = $true
                    break
                }
                $priorBounds = $bounds
            } else {
                $priorBounds = $null
            }
            Start-Sleep -Milliseconds 100
            $rootState = Read-AutomexiaSnapshot
        } while ([DateTime]::UtcNow -lt $stableUntil)
        if ($control.Count -ne 1) { throw "Output category is not reachable: $Id" }
        if (-not $settled) { throw "Output category geometry did not settle: $Id" }
        Click-OutputControl $control[0].bounds
        return Wait-OutputState { param($s) $s.settings.ready -and $s.settings.active_category -eq $Id }
    }
    function Close-OutputCategory {
        $state = Read-AutomexiaSnapshot
        if ($null -ne $state.settings.active_slot) {
            [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)
            $state = Wait-OutputState { param($s) $s.settings.ready -and $null -eq $s.settings.active_slot }
        }
        if ($state.settings.preview_edit_mode) {
            [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)
            $null = Wait-OutputState { param($s) $s.settings.ready -and -not $s.settings.preview_edit_mode }
        }
        for ($step = 0; $step -lt 2; $step++) {
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)) { throw 'Escape failed' }
            if ($step -eq 0) { $null = Wait-OutputState { param($s) $s.settings.ready -and $null -eq $s.settings.active_category } }
        }
        return Wait-OutputState { param($s) -not $s.settings.open }
    }
    function Capture-OutputFrame([string]$Name) {
        if (-not [string]::IsNullOrWhiteSpace($ResultCapture)) {
            $capture = [IO.Path]::ChangeExtension($ResultCapture, ($Name + '.png'))
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window, $capture)
        }
    }
    $coreId = 'terminal.command_output_highlighting'
    $kubeId = 'terminal.kubernetes_highlighting'
    [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1200, 780, $true)
    [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)
    try {
        $script:testStage = 'ordinary output backgrounds enabled'
        Send-AutomexiaTestControl "write-line:output-plain:Write-Output 'alpha.txt'; Write-Output 'beta.txt'; Write-Output 'gamma.txt'"
        $on = Wait-OutputState { param($s) @($s.command_result_backgrounds).Count -ge 3 }
        Capture-OutputFrame 'core-on'
        $wallpaperRgb = $null
        if ([string]$on.renderer_backend -eq 'wgpu') {
            # The WGPU fixture supplies an opaque warm wallpaper. Blank
            # lower-right pixels must expose it while command bands remain
            # visible over the wallpaper in their own output rows.
            $wallpaperX = [int][Math]::Floor([double]$on.window_width * 0.85)
            $wallpaperY = [int][Math]::Floor([double]$on.window_height * 0.75)
            $wallpaper = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
                $window, $wallpaperX, $wallpaperY, 32, 32)
            if ($wallpaper.SampleCount -lt 100 -or
                [Math]::Abs([int]$wallpaper.MeanRed - 123) -gt 20 -or
                [Math]::Abs([int]$wallpaper.MeanGreen - 77) -gt 20 -or
                [Math]::Abs([int]$wallpaper.MeanBlue - 49) -gt 20) {
                throw "Opaque fixture wallpaper is not visible in blank terminal pixels: samples=$($wallpaper.SampleCount), rgb=$($wallpaper.MeanRed)/$($wallpaper.MeanGreen)/$($wallpaper.MeanBlue)"
            }
            $wallpaperRgb = @($wallpaper.MeanRed,$wallpaper.MeanGreen,$wallpaper.MeanBlue)
        }
        $page = Open-OutputCategory $coreId
        if (-not $page.settings.command_output_enabled -or -not $page.settings.kubernetes_enabled) { throw 'Fresh color defaults are not independent and enabled' }
        if (@($page.settings.targets | Where-Object id -eq 'command_output.band.success').Count -ne 1) { throw 'Successful output preview is not editable' }
        Capture-OutputFrame 'core-customization'
        $script:testStage = 'activate command-output preview editing'
        Click-OutputControl $page.settings.edit_button
        $edit = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
        $script:testStage = 'select successful command-output preview row'
        Click-OutputControl ($edit.settings.targets | Where-Object id -eq 'command_output.band.success').bounds
        $detail = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.active_slot -eq 'command_output.band.success' }
        $script:testStage = 'open successful command background color editor'
        Click-OutputControl ($detail.settings.controls | Where-Object id -eq 'command_output.backgrounds.success').bounds
        $editor = Wait-OutputState { param($s) $s.settings.ready -and $null -ne $s.settings.color_editor -and $s.settings.color_editor.id -eq 'command_output.backgrounds.success' }
        $script:testStage = 'replace successful command background color text'
        Click-OutputControl $editor.settings.color_editor.input
        if (-not [AutomexiaResizeDriver]::ReplaceColorHex($window, '#2040E080')) { throw 'Real color selection and text input failed' }
        $editor = Read-AutomexiaSnapshot -AfterSequence ([int64]$editor.sequence)
        Capture-OutputFrame 'color-editor'
        $script:testStage = 'apply successful command background RGBA'
        Click-OutputControl $editor.settings.color_editor.apply
        $null = Wait-OutputState { param($s) $s.settings.ready -and $null -eq $s.settings.color_editor }
        # Real numeric input exercises the discoverable opacity control and its
        # retained-output repaint without a new command or resize.
        foreach ($percent in @(0, 100, 50)) {
            $script:testStage = "type background opacity $percent percent"
            $detail = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.active_slot -eq 'command_output.band.success' }
            Click-OutputControl ($detail.settings.controls | Where-Object id -eq 'command_output.opacity.success').bounds
            $null = Wait-OutputState { param($s) $null -ne $s.settings.numeric_editor -and $s.settings.numeric_editor.id -eq 'command_output.opacity.success' }
            [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x41, $true, $false, $false)
            foreach ($digit in ([string]$percent).ToCharArray()) {
                [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, (0x30 + [int]::Parse([string]$digit)), $false, $false, $false)
            }
            [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x0D, $false, $false, $false)
            $null = Wait-OutputState { param($s) $s.settings.ready -and $null -eq $s.settings.numeric_editor }
            Capture-OutputFrame ('opacity-' + $percent)
            $opacityState = Close-OutputCategory
            $alpha = [int][Math]::Round($percent * 255 / 100, [MidpointRounding]::AwayFromZero)
            $matching = @($opacityState.command_result_backgrounds | Where-Object {
                (@($_[1] | ForEach-Object { [int][Math]::Round($_ * 255) }) -join ',') -eq "32,64,224,$alpha"
            })
            if (($percent -eq 0 -and @($opacityState.command_result_backgrounds).Count -ne 0) -or
                ($percent -gt 0 -and $matching.Count -lt 3)) { throw "Opacity $percent did not repaint retained output" }
            $page = Open-OutputCategory $coreId
            Click-OutputControl $page.settings.edit_button
            $edit = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
            Click-OutputControl ($edit.settings.targets | Where-Object id -eq 'command_output.band.success').bounds
            $null = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.active_slot -eq 'command_output.band.success' }
        }
        $custom = Close-OutputCategory
        $expectedColor = @(32,64,224,128)
        $customBands = @($custom.command_result_backgrounds | Where-Object {
            $actual = @($_[1] | ForEach-Object { [int][Math]::Round($_ * 255) })
            ($actual -join ',') -eq ($expectedColor -join ',')
        })
        if ($customBands.Count -lt 3) {
            Write-Host ($custom.command_result_backgrounds | ConvertTo-Json -Depth 4 -Compress)
            throw 'Custom RGBA did not reach the retained command backgrounds'
        }
        Capture-OutputFrame 'core-custom-color'
        # Sample actual presented pixels, not just the renderer's band metadata.
        # The first completed row contains stable foreground text; the distant
        # right side of the same band is blank and proves that the custom fill
        # really reached the frame. Reuse these physical coordinates after Off.
        $firstBand = @($custom.command_result_backgrounds[0][0])
        $bandScale = [double]$custom.scale_factor
        $glyphX = [int][Math]::Floor(([double]$firstBand[0] + 4.0) * $bandScale)
        $glyphY = [int][Math]::Floor([double]$firstBand[1] * $bandScale)
        $glyphWidth = [int][Math]::Floor(
            [Math]::Min(110.0, [double]$firstBand[2] * 0.25) * $bandScale)
        $glyphHeight = [int][Math]::Ceiling([double]$firstBand[3] * $bandScale)
        $fillX = [int][Math]::Floor(
            ([double]$firstBand[0] + [double]$firstBand[2] * 0.60) * $bandScale)
        $fillY = [int][Math]::Floor(
            ([double]$firstBand[1] + [double]$firstBand[3] * 0.20) * $bandScale)
        $fillWidth = [int][Math]::Floor(
            [Math]::Min(80.0, [double]$firstBand[2] * 0.15) * $bandScale)
        $fillHeight = [int][Math]::Ceiling([double]$firstBand[3] * 0.60 * $bandScale)
        if ($bandScale -le 0 -or $glyphWidth -lt 64 -or $glyphHeight -lt 8 -or
            $fillWidth -lt 16 -or $fillHeight -lt 4) {
            throw 'Completed-command pixel sample geometry is too small'
        }
        $customGlyph = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
            $window, $glyphX, $glyphY, $glyphWidth, $glyphHeight)
        $customFill = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
            $window, $fillX, $fillY, $fillWidth, $fillHeight)
        $page = Open-OutputCategory $coreId
        $script:testStage = 'disable command backgrounds without terminal output or resize'
        Click-OutputControl ($page.settings.controls | Where-Object id -eq $coreId).bounds
        $null = Wait-OutputState { param($s) $s.settings.ready -and -not $s.settings.command_output_enabled -and $s.settings.kubernetes_enabled }
        $off = Close-OutputCategory
        if (@($off.command_result_backgrounds).Count -ne 0) { throw 'Core Off left painted command backgrounds' }
        if ($off.command_result_key -ne $on.command_result_key) { throw 'Changing colors altered command ownership' }
        Capture-OutputFrame 'core-off'
        $offGlyph = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
            $window, $glyphX, $glyphY, $glyphWidth, $glyphHeight)
        $offFill = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
            $window, $fillX, $fillY, $fillWidth, $fillHeight)
        if ($offGlyph.SampleCount -lt 100 -or
            $offGlyph.BrightForegroundSampleCount -lt 24 -or
            $offGlyph.MaximumLuminance -lt 160) {
            throw "Retained uncolored command glyph baseline is missing: samples=$($offGlyph.SampleCount), bright=$($offGlyph.BrightForegroundSampleCount), maximum=$($offGlyph.MaximumLuminance)"
        }
        $fillBlueDelta = [int]$customFill.MeanBlue - [int]$offFill.MeanBlue
        if ($fillBlueDelta -lt 40) {
            throw "Custom command background did not reach blank presented pixels: blue delta $fillBlueDelta"
        }
        $minimumBrightGlyphs = [Math]::Max(
            24, [int][Math]::Floor([int]$offGlyph.BrightForegroundSampleCount * 0.70))
        if ($customGlyph.BrightForegroundSampleCount -lt $minimumBrightGlyphs -or
            $customGlyph.MaximumLuminance -lt ($offGlyph.MaximumLuminance - 24)) {
            throw "Custom band obscured retained glyphs: bright=$($customGlyph.BrightForegroundSampleCount)/$($offGlyph.BrightForegroundSampleCount), maximum=$($customGlyph.MaximumLuminance)/$($offGlyph.MaximumLuminance), blank-blue-delta=$fillBlueDelta"
        }
        $bandHeight = [double]$firstBand[3]
        foreach ($paint in $custom.command_result_backgrounds) {
            if ([Math]::Abs([double]$paint[0][3] - $bandHeight) -gt 0.75) {
                throw 'A completed-command row has a truncated background band'
            }
        }
        $lastBand = @($custom.command_result_backgrounds[$custom.command_result_backgrounds.Count - 1][0])
        $surfaceBottom = [double]$custom.command_result_surface[1] + [double]$custom.command_result_surface[3]
        $lastBandBottom = [double]$lastBand[1] + [double]$lastBand[3]
        if ($surfaceBottom - $lastBandBottom -lt -1.0 -or
            $surfaceBottom - $lastBandBottom -gt 3.0) {
            throw 'The final completed-command row stops short of the output-cell boundary'
        }

        $script:testStage = 'disable Kubernetes independently'
        $page = Open-OutputCategory $kubeId
        Capture-OutputFrame 'kubernetes-customization'
        Click-OutputControl ($page.settings.controls | Where-Object id -eq $kubeId).bounds
        $null = Wait-OutputState { param($s) $s.settings.ready -and -not $s.settings.kubernetes_enabled -and -not $s.settings.command_output_enabled }
        $null = Close-OutputCategory
        $page = Open-OutputCategory $coreId
        Click-OutputControl ($page.settings.controls | Where-Object id -eq $coreId).bounds
        $null = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.command_output_enabled -and -not $s.settings.kubernetes_enabled }
        $null = Close-OutputCategory

        $script:testStage = 'PowerShell ANSI error keeps its completed-command background'
        $page = Open-OutputCategory $coreId
        if ($page.settings.log_output_enabled) {
            Click-OutputControl ($page.settings.controls | Where-Object id -eq 'terminal.output_highlighting').bounds
        }
        $null = Wait-OutputState { param($s) $s.settings.ready -and -not $s.settings.log_output_enabled }
        $null = Close-OutputCategory
        $beforeError = Read-AutomexiaSnapshot
        Send-AutomexiaTestControl "write-line:output-ansi-error:Write-Error 'Fictional failure for output color regression'"
        $errorOutput = Wait-OutputState { param($s) $s.command_result_key -ne $beforeError.command_result_key -and $s.command_result_exit_code -ne 0 -and $null -ne $s.command_result_surface }
        $errorSurface = $errorOutput.command_result_surface
        $errorBands = @($errorOutput.command_result_backgrounds | Where-Object {
            $_[0][1] -ge $errorSurface[1] -and $_[0][1] -lt ($errorSurface[1] + $errorSurface[3])
        })
        if ($errorBands.Count -lt 2) { throw 'Colored PowerShell diagnostics lost their failure backgrounds' }
        Capture-OutputFrame 'powershell-error-command-highlight'

        $script:testStage = 'ordinary table retains custom command tint'
        $beforePlainTable = Read-AutomexiaSnapshot
        Send-AutomexiaTestControl "write-line:output-ordinary-table:Write-Output 'NAME      VALUE'; Write-Output 'alpha     ordinary-text'; Write-Output 'beta      another-value'"
        $plainTable = Wait-OutputState { param($s) $s.command_result_key -ne $beforePlainTable.command_result_key -and $s.inline_table_count -ge 1 -and $null -ne $s.command_result_surface }
        $plainSurface = $plainTable.command_result_surface
        $plainScale = [double]$plainTable.scale_factor
        $plainX = [int][Math]::Floor(($plainSurface[0] + 4) * $plainScale)
        $plainY = [int][Math]::Floor($plainSurface[1] * $plainScale)
        $plainWidth = [int][Math]::Floor(220 * $plainScale)
        $plainHeight = [int][Math]::Floor($plainSurface[3] * $plainScale)
        $plainCellHeight = [double](Get-ActiveAutomexiaPanel $plainTable).cell_height
        if ([Math]::Abs($plainSurface[3] * $plainScale - 3 * $plainCellHeight) -gt 1) {
            throw 'Ordinary table fixture must contain one header and two single-line data rows'
        }
        $plainRowYs = @(1, 2 | ForEach-Object {
            [int][Math]::Round($plainSurface[1] * $plainScale + $_ * $plainCellHeight + 3)
        })
        $plainRowHeight = [int]$plainCellHeight - 6
        Capture-OutputFrame 'ordinary-table-command-on'
        $plainOnPixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $plainX, $plainY, $plainWidth, $plainHeight)
        $plainOnRows = @($plainRowYs | ForEach-Object {
            [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $plainX, $_, $plainWidth, $plainRowHeight)
        })
        $page = Open-OutputCategory $coreId
        Click-OutputControl ($page.settings.controls | Where-Object id -eq $coreId).bounds
        $null = Wait-OutputState { param($s) $s.settings.ready -and -not $s.settings.command_output_enabled }
        $plainOff = Close-OutputCategory
        if ($plainOff.command_result_key -ne $plainTable.command_result_key -or
            [Math]::Abs($plainOff.command_result_surface[1] - $plainSurface[1]) -gt 1) { throw 'Ordinary table moved during color edit' }
        Capture-OutputFrame 'ordinary-table-command-off'
        $plainOffPixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $plainX, $plainY, $plainWidth, $plainHeight)
        $plainBlueDelta = $plainOnPixels.MeanBlue - $plainOffPixels.MeanBlue
        if ($plainBlueDelta -lt 8) { throw "Ordinary table masked the core background: blue delta $plainBlueDelta" }
        $plainRowDeltas = @(for ($row = 0; $row -lt $plainRowYs.Count; $row++) {
            $off = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window, $plainX, $plainRowYs[$row], $plainWidth, $plainRowHeight)
            $delta = $plainOnRows[$row].MeanBlue - $off.MeanBlue
            if ($delta -lt 8) {
                Write-Host (@{ surface = $plainSurface; scale = $plainScale; cell_height = $plainCellHeight; grid_origin = (Get-ActiveAutomexiaPanel $plainTable).grid_origin } | ConvertTo-Json -Compress)
                throw "Ordinary table data row $row masked the core background: blue delta $delta"
            }
            $delta
        })
        $page = Open-OutputCategory $coreId
        Click-OutputControl ($page.settings.controls | Where-Object id -eq $coreId).bounds
        $null = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.command_output_enabled }
        $null = Close-OutputCategory

        $script:testStage = 'status table excludes core command backgrounds'
        $before = Read-AutomexiaSnapshot
        Send-AutomexiaTestControl "write-line:output-table:Write-Output 'NAME        READY   STATUS             RESTARTS   AGE'; Write-Output 'demo-api    0/1     Running            0          3m'; Write-Output 'demo-job    1/1     Running            0          3m'; Write-Output 'demo-task   0/1     CrashLoopBackOff   1          3m'"
        $table = Wait-OutputState { param($s) $s.command_result_key -ne $before.command_result_key -and $null -ne $s.command_result_surface -and $s.inline_table_count -ge 1 }
        $surface = $table.command_result_surface
        $overlap = @($table.command_result_backgrounds | Where-Object { $_[0][1] -ge $surface[1] -and $_[0][1] -lt ($surface[1] + $surface[3]) })
        if ($overlap.Count -ne 0) { throw 'Core bands tinted the Kubernetes table while its colors were off' }
        Capture-OutputFrame 'kubernetes-off'
        # Sample only the current command's table surface. The distinct
        # magenta success foreground below must appear here after a real
        # Kubernetes color edit, never in this disabled baseline.
        $tableScale = [double]$table.scale_factor
        $tablePixelX = [int][Math]::Floor(([double]$surface[0] + 4.0) * $tableScale)
        $tablePixelY = [int][Math]::Floor([double]$surface[1] * $tableScale)
        $tablePixelWidth = [int][Math]::Floor(
            [Math]::Min(560.0, [double]$surface[2] * 0.5) * $tableScale)
        $tablePixelHeight = [int][Math]::Ceiling([double]$surface[3] * $tableScale)
        if ($tablePixelWidth -lt 128 -or $tablePixelHeight -lt 24) {
            throw 'Kubernetes table pixel sample geometry is too small'
        }
        $tableOffPixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
            $window, $tablePixelX, $tablePixelY,
            $tablePixelWidth, $tablePixelHeight, 255, 0, 255, 24)
        if ($tableOffPixels.TargetColorSampleCount -gt 2) {
            throw 'Disabled Kubernetes table already contains the custom magenta foreground'
        }
        $page = Open-OutputCategory $kubeId
        Click-OutputControl ($page.settings.controls | Where-Object id -eq $kubeId).bounds
        $null = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.kubernetes_enabled -and $s.settings.command_output_enabled }
        $tableOn = Close-OutputCategory
        if ($tableOn.command_result_key -ne $table.command_result_key) { throw 'Kubernetes coloring changed command identity' }
        Capture-OutputFrame 'kubernetes-on'
        $script:testStage = 'edit Kubernetes success foreground through the preview'
        $page = Open-OutputCategory $kubeId
        Click-OutputControl $page.settings.edit_button
        $edit = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.preview_edit_mode }
        Click-OutputControl ($edit.settings.targets | Where-Object id -eq 'kubernetes.severity.success').bounds
        $detail = Wait-OutputState { param($s) $s.settings.ready -and $s.settings.active_slot -eq 'kubernetes.severity.success' }
        Click-OutputControl ($detail.settings.controls | Where-Object id -eq 'kubernetes.colors.success').bounds
        $editor = Wait-OutputState { param($s) $s.settings.ready -and $null -ne $s.settings.color_editor -and $s.settings.color_editor.id -eq 'kubernetes.colors.success' }
        Click-OutputControl $editor.settings.color_editor.input
        if (-not [AutomexiaResizeDriver]::ReplaceColorHex($window, '#FF00FF')) {
            throw 'Real Kubernetes color selection and text input failed'
        }
        $editor = Read-AutomexiaSnapshot -AfterSequence ([int64]$editor.sequence)
        Capture-OutputFrame 'kubernetes-color-editor'
        Click-OutputControl $editor.settings.color_editor.apply
        $null = Wait-OutputState { param($s) $s.settings.ready -and $null -eq $s.settings.color_editor }
        $tableCustom = Close-OutputCategory
        if ($tableCustom.command_result_key -ne $table.command_result_key -or
            $tableCustom.inline_table_count -lt 1 -or
            @($tableCustom.command_result_backgrounds | Where-Object {
                $_[0][1] -ge $surface[1] -and $_[0][1] -lt ($surface[1] + $surface[3])
            }).Count -ne 0) {
            throw 'Kubernetes color edit changed table or core-band ownership'
        }
        $customSurface = @($tableCustom.command_result_surface)
        if ($customSurface.Count -ne 4 -or
            [Math]::Abs([double]$customSurface[0] - [double]$surface[0]) -gt 1.0 -or
            [Math]::Abs([double]$customSurface[1] - [double]$surface[1]) -gt 1.0 -or
            [Math]::Abs([double]$customSurface[2] - [double]$surface[2]) -gt 1.0 -or
            [Math]::Abs([double]$customSurface[3] - [double]$surface[3]) -gt 1.0) {
            throw 'Kubernetes table surface moved before its retained pixel comparison'
        }
        Capture-OutputFrame 'kubernetes-custom-color'
        $tableCustomPixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats(
            $window, $tablePixelX, $tablePixelY,
            $tablePixelWidth, $tablePixelHeight, 255, 0, 255, 24)
        if ($tableCustomPixels.TargetColorSampleCount -lt 24) {
            throw "Custom Kubernetes success foreground did not reach table glyphs: magenta=$($tableCustomPixels.TargetColorSampleCount), disabled=$($tableOffPixels.TargetColorSampleCount)"
        }
        # The real retained table painter must honor the same custom Kubernetes
        # palette for non-pod resources, without a resize to discover the table.
        $nonPodSamples = @()
        foreach ($fixture in @(
            @{ Name = 'deployment'; Header = 'NAME  READY  UP-TO-DATE  AVAILABLE  AGE'; Row = 'api   2/2    2           2          1d' },
            @{ Name = 'statefulset'; Header = 'NAME  READY  AGE'; Row = 'db    2/2    1d' },
            @{ Name = 'volume'; Header = 'NAME  STATUS  VOLUME  CAPACITY  ACCESS MODES  STORAGECLASS  AGE'; Row = 'data  Bound   disk    1Gi       RWO           standard      1d' },
            @{ Name = 'namespace'; Header = 'NAME  STATUS  AGE'; Row = 'demo  Active  1d' }
        )) {
            $script:testStage = 'non-pod table palette: ' + $fixture.Name
            $previousTable = Read-AutomexiaSnapshot
            Send-AutomexiaTestControl ("write-line:output-resource-" + $fixture.Name + ":Write-Output '" + $fixture.Header + "'; Write-Output '" + $fixture.Row + "'")
            $tableCustom = Wait-OutputState { param($s) $s.command_result_key -ne $previousTable.command_result_key -and $null -ne $s.command_result_surface -and $s.inline_table_count -ge 1 }
            $resourceSurface = @($tableCustom.command_result_surface)
            $resourceScale = [double]$tableCustom.scale_factor
            $resourcePixels = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window,
                [int][Math]::Floor(([double]$resourceSurface[0] + 4) * $resourceScale),
                [int][Math]::Floor([double]$resourceSurface[1] * $resourceScale),
                [int][Math]::Floor([Math]::Min(560.0, [double]$resourceSurface[2] * 0.5) * $resourceScale),
                [int][Math]::Ceiling([double]$resourceSurface[3] * $resourceScale), 255, 0, 255, 24)
            if ($resourcePixels.TargetColorSampleCount -lt 24) { throw ('Non-pod Kubernetes table lost its configured palette: ' + $fixture.Name) }
            Capture-OutputFrame ('kubernetes-' + $fixture.Name)
            $nonPodSamples += @{ resource = $fixture.Name; colored_pixels = $resourcePixels.TargetColorSampleCount }
        }
        # Native resize exercises retained narrow table composition, not a new command.
        $script:testStage = 'retained status table after narrow resize'
        [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 640, 500, $true)
        $narrow = Wait-OutputState { param($s) $s.sequence -gt $tableCustom.sequence -and ($s.window_width / $s.scale_factor) -le 640 -and $s.inline_table_count -ge 1 }
        Capture-OutputFrame 'narrow'

        [void][AutomexiaResizeDriver]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        $null = Wait-OutputState { param($s) $s.confirm_quit_active }
        [void][AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x59, $false, $false, $false)
        if (-not $process.WaitForExit(15000)) { throw 'Output color native fixture did not shut down' }
        if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
            [IO.File]::WriteAllText([IO.Path]::GetFullPath($ResourceReport), (@{
                schema_version = 1; workflow = 'independent-output-colors'; passed = $true;
                renderer = [string]$on.renderer_backend;
                initial_window = @(1200,780); final_window = @(640,500);
                initial_grid = @($on.columns,$on.rows); final_grid = @($narrow.columns,$narrow.rows);
                initial_physical_window = @($on.window_width,$on.window_height);
                final_physical_window = @($narrow.window_width,$narrow.window_height);
                scale_factor = $narrow.scale_factor;
                switch_combinations = 4; native_shell = 'Windows PowerShell';
                core_painted_rows = @($on.command_result_backgrounds).Count
                opaque_wallpaper_rgb = $wallpaperRgb
                wallpaper_evidence = $(if ($null -eq $wallpaperRgb) { 'unsupported by CPU renderer' } else { 'visible on actual WGPU renderer' })
                custom_rgba_verified = $true
                numeric_opacity_percentages = @(0,100,50)
                powershell_ansi_failure_background_rows = $errorBands.Count
                ordinary_table_blue_delta = $plainBlueDelta
                ordinary_table_row_blue_deltas = $plainRowDeltas
                custom_fill_blue_delta = $fillBlueDelta
                glyph_bright_custom = $customGlyph.BrightForegroundSampleCount
                glyph_bright_uncolored = $offGlyph.BrightForegroundSampleCount
                kubernetes_custom_foreground_rgb = @(255,0,255)
                kubernetes_table_glyph_samples = $tableCustomPixels.TargetColorSampleCount
                kubernetes_disabled_glyph_samples = $tableOffPixels.TargetColorSampleCount
                kubernetes_non_pod_samples = $nonPodSamples
            } | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
        }
        Write-Host 'Independent output colors: native switches, preview, retained table and shutdown passed'
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
}
