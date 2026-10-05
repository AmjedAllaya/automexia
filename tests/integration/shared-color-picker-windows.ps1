# Dot-sourced only by the isolated native-window harness.
function Test-AutomexiaSharedColorPicker {
    function Wait-Color($Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 25
        } while ([DateTime]::UtcNow -lt $deadline)
        Write-Host ("State: open={0}, ready={1}, category={2}, controls={3}, editor={4}, draft={5}, search={6}, tab-menu={7}" -f $state.settings.open, $state.settings.ready, $state.settings.active_category, (($state.settings.controls | ForEach-Object id) -join ','), $state.settings.color_editor.id, ($state.settings.color_editor.draft_color -join ','), $state.settings.search_bytes, $state.tab_appearance_open)
        throw "Shared color picker did not settle: $script:testStage"
    }
    function Color-Key([int]$Key) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $false, $false)) { throw 'Color key lost focus' }
    }
    function Color-Click($Bounds, $Scale) {
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * 0.5) * $Scale)
        $y = [int][Math]::Round(($Bounds[1] + $Bounds[3] * 0.5) * $Scale)
        if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
            -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Color click lost focus' }
        $null = Wait-Color { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x / $Scale) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y / $Scale) -le 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) { throw 'Color pointer lost ownership' }
    }
    function Color-Row([string]$Id) {
        $script:testStage = "row $Id"
        $state = Wait-Color { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq $Id).Count -eq 1 }
        Color-Click (@($state.settings.controls | Where-Object id -eq $Id)[0].bounds) $state.scale_factor
    }
    function Color-Control([string]$Focus) {
        $state = Wait-Color { param($s) $s.settings.ready -and @($s.settings.color_editor.palette.controls | Where-Object focus -eq $Focus).Count -eq 1 }
        Color-Click (@($state.settings.color_editor.palette.controls | Where-Object focus -eq $Focus)[0].bounds) $state.scale_factor
    }
    $script:testStage = 'open shared color editor'
    [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1250, 820, $true)
    Send-AutomexiaTestControl 'open-customizations:shared-color-picker'
    Color-Row 'appearance.font_size'
    # Search narrows the real feature catalog and reveals a lower color row.
    $state = Wait-Color { param($s) $s.settings.ready -and $s.settings.active_category -eq 'appearance.font_size' }
    Color-Click $state.settings.search_button $state.scale_factor
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window,0x41,$false,$true,$false)) { throw 'Color search selection failed' }
    foreach ($key in @(0x46,0x4F,0x52,0x45,0x47,0x52,0x4F,0x55,0x4E,0x44)) { Color-Key $key }
    $null = Wait-Color { param($s) $s.settings.search_bytes -eq 10 }
    Color-Row 'fonts.colors.foreground'
    $state = Wait-Color { param($s) $s.settings.ready -and $s.settings.color_editor.id -eq 'fonts.colors.foreground' }
    $script:testStage = "suggestion draft and cancellation"
    $old = $state.settings.color_editor.draft_color -join ','
    Color-Key 0x28
    Color-Key 0x0D
    $state = Wait-Color { param($s) $s.settings.ready -and $null -ne $s.settings.color_editor -and ($s.settings.color_editor.draft_color -join ',') -ne $old }
    Color-Key 0x1B
    $null = Wait-Color { param($s) $s.settings.ready -and $null -eq $s.settings.color_editor }
    Color-Row 'fonts.colors.foreground'
    $state = Wait-Color { param($s) $s.settings.ready -and ($s.settings.color_editor.draft_color -join ',') -eq $old }
    $script:testStage = 'custom color and persisted favorite'
    if (-not [AutomexiaResizeDriver]::ReplaceColorHex($window, '#ABCDEF')) { throw 'Color custom input failed' }
    $state = Wait-Color { param($s) $s.settings.ready -and ($s.settings.color_editor.draft_color -join ',') -eq '171,205,239,255' }
    Color-Key 0x0D
    $null = Wait-Color { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $null -eq $s.settings.color_editor }
    $preferences = Join-Path $configRoot 'state/user-preferences-v12.toml'
    if ((Get-Content -LiteralPath $preferences -Raw) -notmatch 'color-favorites\s*=\s*\[\s*\[\s*171,\s*205,\s*239,\s*255') { throw 'Custom color did not persist as favorite' }
    Color-Row 'fonts.colors.foreground'
    Color-Control 'Favorites'
    $state = Wait-Color { param($s) $s.settings.ready -and $s.settings.color_editor.palette.favorites -and @($s.settings.color_editor.palette.controls | Where-Object focus -like 'Swatch*').Count -eq 1 }
    if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
        $null = New-Item -ItemType Directory -Force -Path $ModalCaptureDirectory
        $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory 'favorites.png'))
    }
    Color-Control 'FavoriteToggle'
    $null = Wait-Color { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.color_editor.palette.controls | Where-Object focus -like 'Swatch*').Count -eq 0 }
    Color-Control 'Suggested'
    $state = Wait-Color { param($s) $s.settings.ready -and -not $s.settings.color_editor.palette.favorites }
    if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) { $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory 'suggested.png')) }
    Color-Key 0x1B
    Color-Key 0x1B
    Color-Key 0x1B
    $null = Wait-Color { param($s) -not $s.settings.open }
    $script:testStage = 'tab appearance delegates to shared picker'
    $state = Read-AutomexiaSnapshot
    $x = [int](140 * $state.scale_factor); $y = [int](24 * $state.scale_factor)
    if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Tab appearance pointer failed' }
    $null = Wait-Color { param($s) [Math]::Abs($s.pointer.x - $x) -le 1 -and [Math]::Abs($s.pointer.y - $y) -le 1 }
    if (-not [AutomexiaResizeDriver]::SendPhysicalRightClick($window, $x, $y)) { throw 'Tab appearance click failed' }
    $null = Wait-Color { param($s) $s.tab_appearance_open }
    $script:testStage = 'tab F2 opens shared editor'
    Color-Key 0x71
    $state = Wait-Color { param($s) $s.settings.ready -and $s.settings.color_editor.id -eq 'tab.color' }
    $script:testStage = 'apply tab color'
    if (-not [AutomexiaResizeDriver]::ReplaceColorHex($window, '#789ABC')) { throw 'Tab custom input failed' }
    $null = Wait-Color { param($s) ($s.settings.color_editor.draft_color -join ',') -eq '120,154,188,255' }
    Color-Key 0x0D
    $null = Wait-Color { param($s) -not $s.settings.open }
    $script:testStage = 'reopen saved tab color'
    if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Tab appearance reopen pointer failed' }
    $null = Wait-Color { param($s) [Math]::Abs($s.pointer.x - $x) -le 1 -and [Math]::Abs($s.pointer.y - $y) -le 1 }
    if (-not [AutomexiaResizeDriver]::SendPhysicalRightClick($window, $x, $y)) { throw 'Tab appearance reopen click failed' }
    $null = Wait-Color { param($s) $s.tab_appearance_open }
    Color-Key 0x71
    $state = Wait-Color { param($s) $s.settings.ready -and $s.settings.color_editor.id -eq 'tab.color' -and ($s.settings.color_editor.draft_color -join ',') -eq '120,154,188,255' }
    Color-Key 0x1B
    $null = Wait-Color { param($s) -not $s.settings.open }
    Write-Output 'PASS: shared suggestions, draft cancellation, custom apply, persisted favorites, removal and tab color delegation'
}
