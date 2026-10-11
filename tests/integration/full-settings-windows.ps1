# Runs only in the existing isolated configuration/native-window harness.
function Test-AutomexiaFullSettings {
    function Wait-Settings($Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 30
        } while ([DateTime]::UtcNow -lt $deadline)
        $diagnostic = @{ stage = $script:testStage; sequence = $state.sequence; open = $state.settings.open; ready = $state.settings.ready; query_bytes = $state.settings.search_bytes; pointer = $state.settings.pointer; palette = $state.palette_enabled } | ConvertTo-Json -Depth 4 -Compress
        throw "Full Settings did not settle: $diagnostic"
    }
    function Settings-Key([byte]$Key, [bool]$Ctrl = $false, [bool]$Shift = $false) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $Ctrl, $Shift)) {
            throw 'Settings test lost keyboard ownership'
        }
    }
    function Settings-Click($Bounds) {
        if ($null -eq $Bounds -or @($Bounds).Count -ne 4) { throw 'Settings control is missing' }
        $scale = [double](Read-AutomexiaSnapshot).scale_factor
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * 0.5) * $scale)
        $y = [int][Math]::Round(($Bounds[1] + $Bounds[3] * 0.5) * $scale)
        # Reopening clears the modal's hover position. Moving to the same OS
        # coordinate sends no event, so first establish a distinct owned point.
        $nudgeX = $x + 6
        if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
            -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $nudgeX, $y)) { throw 'Settings control focus failed' }
        $null = Wait-Settings { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and
            [Math]::Abs($s.settings.pointer[0] - $nudgeX / $scale) -le 1 -and
            [Math]::Abs($s.settings.pointer[1] - $y / $scale) -le 1 }
        if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Settings control pointer failed' }
        $null = Wait-Settings { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and
            [Math]::Abs($s.settings.pointer[0] - $x / $scale) -le 1 -and
            [Math]::Abs($s.settings.pointer[1] - $y / $scale) -le 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) { throw 'Settings pointer ownership failed' }
    }
    function Search-Settings([string]$Query) {
        Settings-Click (Read-AutomexiaSnapshot).settings.search_button
        Settings-Key 0x41 $true
        if (-not [AutomexiaResizeDriver]::SendProfileFixtureText($window, $Query)) { throw 'Settings search input failed' }
        return Wait-Settings { param($s) $s.settings.ready -and $s.settings.search_bytes -eq $Query.Length }
    }
    $configPath = Join-Path $configRoot 'config.toml'
    $before = (Get-FileHash -LiteralPath $configPath -Algorithm SHA256).Hash
    $script:testStage = 'open full Settings through existing keyboard shortcut'
    Settings-Key 0x53 $true $true
    $null = Wait-Settings { param($s) $s.settings.ready -and $s.settings.open }
    $script:testStage = 'find general control and verify default ownership'
    $state = Search-Settings 'copy-on-select'
    $control = @($state.settings.controls | Where-Object id -eq 'general.copy_on_select')
    if ($control.Count -ne 1 -or $control[0].origin -ne 'Default') { throw 'Full Settings/default provenance unavailable' }
    Settings-Click $control[0].bounds
    $state = Wait-Settings { param($s) $s.settings.ready -and -not $s.settings.save_pending -and
        @($s.settings.controls | Where-Object { $_.id -eq 'general.copy_on_select' -and $_.origin -eq 'UI override' }).Count -eq 1 }
    $savedPath = Join-Path $configRoot $preferenceRelativePath
    if ([IO.File]::ReadAllText($savedPath) -notmatch 'copy-on-select = true') { throw 'General setting was not persisted' }
    $script:testStage = 'reset inherits without rewriting config'
    Settings-Click $state.settings.reset_button
    $state = Wait-Settings { param($s) $s.settings.ready -and $null -ne $s.settings.confirmation }
    Settings-Click $state.settings.confirmation.accept
    $null = Wait-Settings { param($s) $s.settings.ready -and -not $s.settings.save_pending -and
        @($s.settings.controls | Where-Object { $_.id -eq 'general.copy_on_select' -and $_.origin -eq 'Default' }).Count -eq 1 }
    if ([IO.File]::ReadAllText($savedPath) -match 'copy-on-select') { throw 'Reset retained an override' }
    $script:testStage = 'launch scope and reopen retain inherited state'
    $state = Search-Settings 'history line limit'
    $control = @($state.settings.controls | Where-Object id -eq 'general.history_limit')
    if ($control.Count -ne 1 -or $control[0].scope -ne 'Launch only: new terminal') { throw 'History launch scope missing' }
    $capturePath = if ([string]::IsNullOrWhiteSpace($ResultCapture)) { Join-Path $configRoot 'full-settings-control.png' } else { $ResultCapture }
    [void][AutomexiaResizeDriver]::CaptureClientFrame($window, $capturePath)
    $bitmap = [Drawing.Bitmap]::new($capturePath)
    try {
        # This harness fixes a dark default palette. Check ink strictly inside
        # each numeric zone, excluding rounded borders and neighboring labels.
        $b = $control[0].bounds
        $scale = [double]$state.scale_factor
        foreach ($zone in @(@('decrement', 0.05, 0.23), @('value', 0.29, 0.71), @('increment', 0.78, 0.95))) {
            $ink = 0
            for ($y = [int][Math]::Ceiling(($b[1] + $b[3] * 0.18) * $scale); $y -lt [int][Math]::Floor(($b[1] + $b[3] * 0.82) * $scale); $y++) {
                for ($x = [int][Math]::Ceiling(($b[0] + $b[2] * $zone[1]) * $scale); $x -lt [int][Math]::Floor(($b[0] + $b[2] * $zone[2]) * $scale); $x++) {
                    $pixel = $bitmap.GetPixel($x, $y)
                    if ([Math]::Min($pixel.R, [Math]::Min($pixel.G, $pixel.B)) -gt 100) { $ink++ }
                }
            }
            if ($ink -lt 8) { throw "Numeric $($zone[0]) is missing from the native frame" }
        }
    } finally { $bitmap.Dispose() }
    $script:testStage = 'Escape closes full Settings'
    Settings-Key 0x1B
    $null = Wait-Settings { param($s) -not $s.settings.open }
    $script:testStage = 'keyboard shortcut reopens full Settings'
    Settings-Key 0x53 $true $true
    $null = Wait-Settings { param($s) $s.settings.ready -and $s.settings.open }
    $script:testStage = 'search inherited control after reopening'
    $state = Search-Settings 'copy-on-select'
    if (@($state.settings.controls | Where-Object { $_.id -eq 'general.copy_on_select' -and $_.origin -eq 'Default' }).Count -ne 1) {
        throw 'Reopening Settings did not retain inheritance'
    }
    if ((Get-FileHash -LiteralPath $configPath -Algorithm SHA256).Hash -ne $before) { throw 'Settings rewrote config.toml' }
    Settings-Key 0x1B
    Write-Output 'PASS: Full Settings keyboard entry, search, provenance, save/reset, launch scope, numeric pixels, reopen and config preservation'
}
