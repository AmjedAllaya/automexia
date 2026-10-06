# Dot-sourced by the isolated native-window harness. Never edits host settings.
# Verify the controlled dark-theme fixture actually rendered, not just its input state.
function Assert-AutomexiaFontPickerPixels($State, [string]$Path) {
    $bitmap = [System.Drawing.Bitmap]::FromFile([IO.Path]::GetFullPath($Path))
    try {
        $scale = [double]$State.scale_factor
        foreach ($id in @('Search', 'Row(0)', 'Apply', 'Refresh', 'Back')) {
            $target = @($State.settings.font_picker.targets | Where-Object id -eq $id)[0]
            $bounds = $target.bounds
            $left = [int][Math]::Ceiling(($bounds[0] + 5) * $scale)
            $right = [int][Math]::Floor(($bounds[0] + $bounds[2] * 0.45) * $scale)
            $top = [int][Math]::Ceiling(($bounds[1] + 3) * $scale)
            $bottom = [int][Math]::Floor(($bounds[1] + $bounds[3] - 3) * $scale)
            $ink = 0
            for ($y = $top; $y -lt $bottom; $y++) {
                for ($x = $left; $x -lt $right; $x++) {
                    $pixel = $bitmap.GetPixel($x, $y)
                    if ([int]$pixel.R + [int]$pixel.G + [int]$pixel.B -gt 380) { $ink++ }
                }
            }
            # The shortcut badge is outside this region; a caret alone cannot pass.
            if ($ink -lt 30) { throw "Font picker $id label is missing from the rendered frame ($ink ink pixels)" }
        }
        $row = @($State.settings.font_picker.targets | Where-Object id -eq 'Row(0)')[0].bounds
        $x = [int][Math]::Floor(($row[0] + $row[2] - 20) * $scale)
        $y = [int][Math]::Floor(($row[1] + $row[3] + 40) * $scale)
        $panel = $bitmap.GetPixel($x, $y)
        $outside = $bitmap.GetPixel([int](10 * $scale), $y)
        $selected = $bitmap.GetPixel($x, [int][Math]::Floor(($row[1] + $row[3] * 0.5) * $scale))
        foreach ($comparison in @(@($panel, $outside, 'opaque panel'), @($selected, $panel, 'selected row'))) {
            $difference = [Math]::Abs([int]$comparison[0].R - [int]$comparison[1].R) + [Math]::Abs([int]$comparison[0].G - [int]$comparison[1].G) + [Math]::Abs([int]$comparison[0].B - [int]$comparison[1].B)
            if ($difference -lt 6) { throw "Font picker $($comparison[2]) is missing from the rendered frame" }
        }
    } finally { $bitmap.Dispose() }
}

function Test-AutomexiaFontPicker {
    function Wait-Font($Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(40)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 25
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Font picker did not settle: $script:testStage; $($state.settings.font_picker.notice)"
    }
    function Font-Key([int]$Key, [bool]$Control = $false, [bool]$Shift = $false) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $Control, $Shift)) { throw 'Font key lost focus' }
    }
    function Font-Click($Bounds, $Scale) {
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * 0.5) * $Scale)
        $y = [int][Math]::Round(($Bounds[1] + $Bounds[3] * 0.5) * $Scale)
        if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
            -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Font click lost focus' }
        $null = Wait-Font { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x / $Scale) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y / $Scale) -le 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) { throw 'Font pointer lost ownership' }
    }
    function Font-Row([string]$Id) {
        $state = Wait-Font { param($s) $s.settings.ready -and @($s.settings.controls | Where-Object id -eq $Id).Count -eq 1 }
        Font-Click (@($state.settings.controls | Where-Object id -eq $Id)[0].bounds) $state.scale_factor
    }
    function Font-Search([string]$Text) {
        $state = Wait-Font { param($s) $s.settings.ready -and $s.settings.font_picker.ready }
        $target = @($state.settings.font_picker.targets | Where-Object id -eq 'Search')[0]
        Font-Click $target.bounds $state.scale_factor
        Font-Key 0x41 $true
        foreach ($character in $Text.ToUpperInvariant().ToCharArray()) { Font-Key ([int]$character) }
        $null = Wait-Font { param($s) $s.settings.ready -and $s.settings.search_bytes -eq $Text.Length }
        Font-Key 0x28
    }
    function Font-SavedBytes {
        $path = Join-Path $configRoot $preferenceRelativePath
        if (Test-Path -LiteralPath $path) { return [Convert]::ToBase64String([IO.File]::ReadAllBytes($path)) }
        return ''
    }
    $script:testStage = 'open installed-font picker'
    [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1250, 820, $true)
    Send-AutomexiaTestControl 'open-terminal-appearance:font-picker'
    Font-Row 'appearance.font_size'
    $state = Wait-Font { param($s) $s.settings.ready -and $s.settings.active_category -eq 'appearance.font_size' }
    $originalFont = $state.font_primary
    $originalSaved = Font-SavedBytes
    Font-Row 'fonts.family'
    $state = Wait-Font { param($s) $s.settings.ready -and $s.settings.font_picker.ready }
    if ($state.settings.font_picker.count -lt 2) { throw 'Native installed-font inventory is empty' }
    $script:testStage = 'preview Consolas without saving'
    Font-Search 'Consolas'
    $state = Wait-Font { param($s) $s.font_primary -like '*Consolas*' -and $s.settings.font_picker.notice -like 'Preview only*' }
    if ((Font-SavedBytes) -ne $originalSaved) { throw 'Preview saved preferences' }
    $script:testStage = 'rapid search replaces preview with latest family'
    Font-Search 'Courier New'
    $state = Wait-Font { param($s) $s.font_primary -like '*Courier*' -and $s.settings.font_picker.selected -eq 'Courier New' -and $s.settings.font_picker.notice -like 'Preview only*' }
    if ((Font-SavedBytes) -ne $originalSaved) { throw 'Rapid browsing saved preferences' }
    $captureRoot = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
        Join-Path $configRoot 'font-picker-frames'
    } else { [IO.Path]::GetFullPath($ModalCaptureDirectory) }
    $null = New-Item -ItemType Directory -Force -Path $captureRoot
    foreach ($name in @('font-preview.png', 'font-preview-settled.png')) {
        if ($name -eq 'font-preview-settled.png') { Start-Sleep -Milliseconds 400 }
        $path = Join-Path $captureRoot $name
        $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, $path)
        Assert-AutomexiaFontPickerPixels $state $path
    }
    $script:testStage = 'Escape restores original font'
    Font-Key 0x1B
    $null = Wait-Font { param($s) $null -eq $s.settings.font_picker -and $s.font_primary -eq $originalFont }
    if ((Font-SavedBytes) -ne $originalSaved) { throw 'Cancel saved preferences' }
    $script:testStage = 'apply prepared font and reopen current selection'
    Font-Row 'fonts.family'
    Font-Search 'Consolas'
    $null = Wait-Font { param($s) $s.font_primary -like '*Consolas*' -and $s.settings.font_picker.notice -like 'Preview only*' }
    Font-Key 0x0D
    $null = Wait-Font { param($s) $null -eq $s.settings.font_picker -and $s.font_primary -like '*Consolas*' -and -not $s.settings.save_pending }
    if ((Font-SavedBytes) -eq $originalSaved) { throw 'Apply did not save the family override' }
    Font-Row 'fonts.family'
    $null = Wait-Font { param($s) $s.settings.font_picker.ready -and $s.settings.font_picker.current -eq 'Consolas' }
    Font-Key 0x74
    $null = Wait-Font { param($s) $s.settings.font_picker.ready -and $s.settings.font_picker.current -eq 'Consolas' }
    Font-Key 0x1B
    $null = Wait-Font { param($s) $null -eq $s.settings.font_picker -and $s.font_primary -like '*Consolas*' }
    Write-Output 'PASS: native font discovery, search, live terminal preview, rapid selection, cancellation, explicit save and refresh'
}
