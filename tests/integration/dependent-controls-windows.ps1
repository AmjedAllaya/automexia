# Runs only inside the existing isolated configuration/native-window harness.
function Test-AutomexiaDependentControls {
    function Wait-Dependency($Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(12)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 25
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Dependent controls did not settle: $script:testStage"
    }
    function Click-Dependency([string]$Id, [double]$Fraction = 0.5) {
        $state = Wait-Dependency { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq $Id).Count -eq 1 }
        $bounds = @($state.settings.controls | Where-Object id -eq $Id)[0].bounds
        $scale = [double]$state.scale_factor
        $x = [int][Math]::Round(($bounds[0] + $bounds[2] * $Fraction) * $scale)
        $y = [int][Math]::Round(($bounds[1] + $bounds[3] * 0.5) * $scale)
        if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
            -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Could not focus dependency test control' }
        $null = Wait-Dependency { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x / $scale) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y / $scale) -le 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) { throw 'Dependency test click lost window ownership' }
    }
    function Dependency-Back {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)) { throw 'Dependency test Escape failed' }
    }
    [void][AutomexiaResizeDriver]::MoveWindow($window, 40, 40, 1500, 900, $true)
    $null = Wait-Dependency { param($s) $s.window_width -ge 1400 }
    foreach ($case in @(
        @{ Category = 'terminal.command_timestamps'; Child = 'timestamps.date-format'; Independent = 'timestamps.show-status' },
        @{ Category = 'terminal.inline_tables'; Child = 'tables.banding'; Independent = 'terminal.inline_tables' },
        @{ Category = 'tags.enabled'; Child = 'tags.bar-style'; Independent = 'tags.enabled' }
    )) {
        $script:testStage = 'dependent controls ' + $case.Category
        Send-AutomexiaTestControl ('open-customizations:dependencies-' + $case.Category)
        $null = Wait-Dependency { param($s) $s.settings.ready -and $s.settings.open -and $null -eq $s.settings.active_category }
        Click-Dependency $case.Category
        $null = Wait-Dependency { param($s) $s.settings.ready -and $s.settings.active_category -eq $case.Category -and @($s.settings.controls | Where-Object id -eq $case.Child).Count -eq 1 }
        for ($cycle = 0; $cycle -lt 3; $cycle++) {
            Click-Dependency $case.Category
            $off = Wait-Dependency { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq $case.Child).Count -eq 0 }
            if (@($off.settings.controls | Where-Object id -eq $case.Independent).Count -ne 1) { throw 'Hiding a child also hid its parent or an independent result control' }
            if ($case.Category -eq 'tags.enabled' -and @($off.settings.targets).Count -ne 0) { throw 'Hidden tags still offer preview edit targets' }
            if ($cycle -eq 0 -and -not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $null = New-Item -ItemType Directory -Force -Path $ModalCaptureDirectory
                $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory ($case.Category + '-off.png')))
            }
            Click-Dependency $case.Category
            $null = Wait-Dependency { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq $case.Child).Count -eq 1 }
        }
        if ($case.Category -eq 'terminal.command_timestamps') {
            # The isolated default is the first format; its left arrow wraps to Hidden.
            Click-Dependency 'timestamps.date-format' 0.25
            $null = Wait-Dependency { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq 'timestamps.date-separator').Count -eq 0 -and @($s.settings.controls | Where-Object id -eq 'timestamps.precision').Count -eq 1 }
            Click-Dependency 'timestamps.time-format' 0.25
            $hidden = Wait-Dependency { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object { $_.id -in @('timestamps.precision','timestamps.timezone') }).Count -eq 0 }
            foreach ($parent in @('timestamps.date-format','timestamps.time-format')) {
                if (@($hidden.settings.controls | Where-Object id -eq $parent).Count -ne 1) { throw 'Hidden date/time cannot be enabled again' }
            }
            if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
                $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $ModalCaptureDirectory 'date-time-hidden.png'))
            }
            Click-Dependency 'timestamps.date-format' 0.75
            $null = Wait-Dependency { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq 'timestamps.date-separator').Count -eq 1 }
            Click-Dependency 'timestamps.time-format' 0.75
            $null = Wait-Dependency { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq 'timestamps.precision').Count -eq 1 }
        }
        Dependency-Back
        $null = Wait-Dependency { param($s) $s.settings.ready -and $null -eq $s.settings.active_category }
        Dependency-Back
        $null = Wait-Dependency { param($s) -not $s.settings.open }
    }
    Write-Output 'PASS: native dependent controls hide and return through real clicks, preserve parent/result controls, and remove hidden preview targets'
}


# Real menu edits must publish beyond the settings catalog while the overlay stays open.
function Test-AutomexiaLiveInterface {
    function Wait-Interface([scriptblock]$Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 25
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Interface did not settle: $script:testStage"
    }
    function Interface-Key([int]$Key, [bool]$Control = $false) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $Control, $false)) { throw 'Interface key lost window ownership' }
    }
    function Interface-Click($Bounds, $Scale) {
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * 0.5) * $Scale)
        $y = [int][Math]::Round(($Bounds[1] + $Bounds[3] * 0.5) * $Scale)
        if (-not [AutomexiaResizeDriver]::ActivateWindow($window) -or
            -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)) { throw 'Interface control focus failed' }
        $null = Wait-Interface { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x / $Scale) -le 1 -and [Math]::Abs($s.settings.pointer[1] - $y / $Scale) -le 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) { throw 'Interface click lost window ownership' }
    }
    function Interface-Row([string]$Id) {
        $state = Wait-Interface { param($s) $s.settings.ready -and -not $s.settings.save_pending -and @($s.settings.controls | Where-Object id -eq $Id).Count -eq 1 }
        Interface-Click (@($state.settings.controls | Where-Object id -eq $Id)[0].bounds) $state.scale_factor
    }
    function Interface-Number([string]$Id, [string]$Value) {
        Interface-Row $Id
        $null = Wait-Interface { param($s) $s.settings.numeric_editor.id -eq $Id }
        Interface-Key 0x41 $true
        if (-not [AutomexiaResizeDriver]::SendProfileFixtureText($window, $Value)) { throw 'Interface numeric input failed' }
        Interface-Key 0x0D
        $null = Wait-Interface { param($s) $s.settings.ready -and $null -eq $s.settings.numeric_editor -and -not $s.settings.save_pending }
    }
    function Interface-Capture([string]$Name) {
        $directory = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) { $configRoot } else { $ModalCaptureDirectory }
        $null = New-Item -ItemType Directory -Force -Path $directory
        $path = Join-Path $directory ($Name + '.png')
        $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, $path)
        return $path
    }
    [void][AutomexiaResizeDriver]::MoveWindow($window, 20, 20, 1100, 680, $true)
    $null = Wait-Interface { param($s) $s.window_width -ge 1000 -and $s.window_width -lt 1450 }
    $baseline = Read-AutomexiaSnapshot
    $baseTop = [double]$baseline.grid_margin.top
    $script:testStage = 'live header height'
    Send-AutomexiaTestControl 'open-terminal-appearance:live-interface'
    Interface-Row 'interface.header.background'
    Interface-Number 'interface.header.height' '72'
    $tall = Wait-Interface { param($s) $s.settings.open -and $s.grid_margin.top -ge $baseTop + 27 * $s.scale_factor }
    $null = Interface-Capture 'header-tall'
    Interface-Number 'interface.header.height' '32'
    $short = Wait-Interface { param($s) $s.settings.open -and $s.grid_margin.top -le $baseTop - 11 * $s.scale_factor }
    if ($short.rows -le $tall.rows) { throw 'Header height did not immediately change terminal rows' }
    $null = Interface-Capture 'header-short'
    $script:testStage = 'live header background'
    $before = Interface-Capture 'header-before-color'
    Interface-Row 'interface.header.background'
    $state = Wait-Interface { param($s) $s.settings.color_editor.id -eq 'interface.header.background' }
    Interface-Click $state.settings.color_editor.input $state.scale_factor
    Interface-Key 0x41 $true
    if (-not [AutomexiaResizeDriver]::SendProfileFixtureText($window, '#704020FF')) { throw 'Interface color input failed' }
    Interface-Key 0x0D
    $null = Wait-Interface { param($s) $s.settings.ready -and $null -eq $s.settings.color_editor -and -not $s.settings.save_pending }
    $after = Interface-Capture 'header-after-color'
    Add-Type -AssemblyName System.Drawing
    $a = [Drawing.Bitmap]::new($before)
    $b = [Drawing.Bitmap]::new($after)
    try {
        $differences = 0
        for ($y = 2; $y -lt 18; $y++) { for ($x = 400; $x -lt 800; $x++) {
            if ($a.GetPixel($x, $y).ToArgb() -ne $b.GetPixel($x, $y).ToArgb()) { $differences++ }
        }}
        if ($differences -lt 1000) { throw 'Header background did not repaint while settings stayed open' }
    } finally { $a.Dispose(); $b.Dispose() }
    if ((Read-AutomexiaSnapshot).rows -ne $short.rows) { throw 'Color-only edit changed terminal dimensions' }
    Interface-Key 0x1B
    $null = Wait-Interface { param($s) $s.settings.ready -and $null -eq $s.settings.active_category }
    $script:testStage = 'live footer height and visibility'
    Interface-Row 'interface.footer.visible'
    Interface-Number 'interface.footer.height' '72'
    $footerTall = Read-AutomexiaSnapshot
    Interface-Number 'interface.footer.height' '24'
    $footerShort = Wait-Interface { param($s) $s.settings.open -and $s.rows -gt $footerTall.rows }
    $null = Interface-Capture 'footer-short'
    Interface-Row 'interface.footer.visible'
    $hidden = Wait-Interface { param($s) $s.settings.open -and $s.rows -gt $footerShort.rows -and @($s.settings.controls).Count -eq 1 }
    $null = Interface-Capture 'footer-hidden'
    Interface-Row 'interface.footer.visible'
    $restored = Wait-Interface { param($s) $s.settings.open -and $s.rows -eq $footerShort.rows -and @($s.settings.controls).Count -gt 1 }
    if ($restored.owned_route_count -ne $baseline.owned_route_count) { throw 'Interface edits replaced terminal sessions' }
    Interface-Key 0x1B
    Interface-Key 0x1B
    $null = Wait-Interface { param($s) -not $s.settings.open }
    Write-Output 'PASS: native header/footer geometry, visibility and header color apply while settings remain open; sessions and color-only dimensions preserved'
}
