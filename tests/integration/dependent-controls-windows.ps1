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
