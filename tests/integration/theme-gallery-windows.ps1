# Uses the existing isolated config, owned native window and real input driver.
function Test-AutomexiaThemeGallery {
    function Wait-ThemeState([scriptblock]$Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 40
        } while ([DateTime]::UtcNow -lt $deadline)
        Write-Host ($state.settings | ConvertTo-Json -Depth 6 -Compress)
        throw "Theme gallery did not settle: $script:testStage"
    }
    function Theme-Key([int]$Key) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $false, $false)) { throw 'Theme key input failed' }
    }
    function Click-Theme($Bounds) {
        if ($null -eq $Bounds -or @($Bounds).Count -ne 4) { throw 'Missing theme control' }
        $scale = [double](Read-AutomexiaSnapshot).scale_factor
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * 0.5) * $scale)
        $y = [int][Math]::Round(($Bounds[1] + $Bounds[3] * 0.5) * $scale)
        [void][AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x + 6, $y)
        Start-Sleep -Milliseconds 80
        [void][AutomexiaResizeDriver]::MovePhysicalPointerToClient($window, $x, $y)
        $null = Wait-ThemeState { param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0] - $x/$scale) -lt 1 -and [Math]::Abs($s.settings.pointer[1] - $y/$scale) -lt 1 }
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window, $x, $y)) { throw 'Theme pointer click failed' }
    }
    function Open-Themes {
        Send-AutomexiaTestControl ('open-themes:' + [guid]::NewGuid().ToString('N'))
        return Wait-ThemeState { param($s) $s.settings.ready -and $null -ne $s.settings.gallery -and -not $s.settings.gallery.busy -and $s.settings.gallery.count -ge 6 }
    }
    function Select-Theme([string]$Name) {
        Theme-Key 0x24
        $null = Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.gallery.selected -eq 0}
        for ($i=0; $i -lt 10; $i++) {
            $state = Read-AutomexiaSnapshot
            if ($state.settings.gallery.builtin -eq ('builtin:' + $Name)) { return $state }
            $prior = [int]$state.settings.gallery.selected
            Theme-Key 0x28
            $null = Wait-ThemeState { param($s) $s.settings.ready -and $s.settings.gallery.selected -ne $prior }
        }
        throw 'Built-in theme missing'
    }
    function Theme-Capture([string]$Name) {
        if (-not [string]::IsNullOrWhiteSpace($ResultCapture)) {
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window,[IO.Path]::ChangeExtension($ResultCapture,($Name+'.png')))
        }
    }
    function Preference-Text {
        $path = Join-Path $configRoot 'state/user-preferences-v10.toml'
        for ($attempt = 0; $attempt -lt 80; $attempt++) {
            $stream = $null
            $reader = $null
            try {
                # Do not prevent the preference owner from atomically replacing
                # its snapshot. A short exclusive writer interval may still race us.
                $stream = [IO.File]::Open($path, [IO.FileMode]::Open, [IO.FileAccess]::Read,
                    [IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete)
                $reader = [IO.StreamReader]::new($stream, [Text.Encoding]::UTF8)
                return $reader.ReadToEnd()
            } catch [IO.FileNotFoundException] {
                return ''
            } catch [IO.DirectoryNotFoundException] {
                return ''
            } catch [IO.IOException] {
                $cause = $_.Exception
                if ($null -ne $cause.InnerException) { $cause = $cause.InnerException }
                $code = $cause.HResult -band 0xffff
                if ($code -notin @(32, 33) -or $attempt -eq 79) { throw }
                Start-Sleep -Milliseconds 25
            } finally {
                if ($null -ne $reader) { $reader.Dispose() }
                elseif ($null -ne $stream) { $stream.Dispose() }
            }
        }
        throw 'Preference snapshot stayed unavailable'
    }
    [void][AutomexiaResizeDriver]::MoveWindow($window,20,20,1200,780,$true)
    [void][AutomexiaResizeDriver]::SetCaptureTopmost($window,$true)
    try {
        $originalConfig = [IO.File]::ReadAllText((Join-Path $configRoot 'config.toml'))
        $script:testStage = 'open theme gallery'
        $null = Open-Themes
        $originalPreferences = Preference-Text
        foreach ($name in @('aurora-night','solar-dusk','forest-operator','arctic-glass','arctic-day')) {
            $script:testStage = 'preview ' + $name
            $state = Select-Theme $name
            Theme-Capture $name
            if ((Preference-Text) -ne $originalPreferences) { throw 'Browsing themes wrote preferences' }
        }
        $script:testStage = 'escape restores without saving'
        Theme-Key 0x1B
        $null = Wait-ThemeState {param($s) $s.settings.ready -and $null -eq $s.settings.gallery}
        if ((Preference-Text) -ne $originalPreferences) { throw 'Escape saved a theme' }
        $script:testStage = 'apply persists the selected palette'
        $null = Open-Themes
        $null = Select-Theme 'solar-dusk'
        Theme-Key 0x0D
        $null = Wait-ThemeState {param($s) -not $s.settings.open}
        $deadline = [DateTime]::UtcNow.AddSeconds(12)
        while ((Preference-Text) -notmatch 'Solar Dusk' -and [DateTime]::UtcNow -lt $deadline) {Start-Sleep -Milliseconds 50}
        if ((Preference-Text) -notmatch 'Solar Dusk') {throw 'Applied theme was not saved'}
        $script:testStage = 'customize a copy using the shared color editor'
        $state = Open-Themes
        $state = Select-Theme 'forest-operator'
        Click-Theme ($state.settings.gallery.targets | Where-Object id -eq 'Customize').bounds
        $null = Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.gallery.customizing}
        Theme-Key 0x28
        Theme-Key 0x0D
        $state = Wait-ThemeState {param($s) $s.settings.ready -and $null -ne $s.settings.color_editor}
        if (-not [AutomexiaResizeDriver]::ReplaceColorHex($window,'#6b6b6b')) {throw 'Theme color typing failed'}
        Theme-Key 0x0D
        $state = Wait-ThemeState {param($s) $s.settings.ready -and $null -eq $s.settings.color_editor -and $s.settings.gallery.customizing}
        Theme-Capture 'custom-copy'
        Click-Theme ($state.settings.gallery.targets | Where-Object id -eq 'Apply').bounds
        $null = Wait-ThemeState {param($s) -not $s.settings.open}
        $files = @(Get-ChildItem -LiteralPath (Join-Path $configRoot 'themes') -Filter '*.toml')
        if ($files.Count -ne 1 -or [IO.File]::ReadAllText($files[0].FullName) -notmatch '#6b6b6b') {throw 'Custom copy was not written canonically'}
        $script:testStage = 'configuration removes the saved override'
        $state = Open-Themes
        Click-Theme ($state.settings.gallery.targets | Where-Object id -eq 'Configuration').bounds
        $null = Wait-ThemeState {param($s) -not $s.settings.open}
        $deadline = [DateTime]::UtcNow.AddSeconds(12)
        while ((Preference-Text) -match 'theme-selection' -and [DateTime]::UtcNow -lt $deadline) {Start-Sleep -Milliseconds 50}
        if ((Preference-Text) -match 'theme-selection') {throw 'Use configuration left a theme override'}
        if ([IO.File]::ReadAllText((Join-Path $configRoot 'config.toml')) -ne $originalConfig) {throw 'Theme gallery modified config.toml'}
        Write-Host "Native theme gallery passed: $expectedRendererBackend; five previews, cancel, apply, copy, configuration."
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window,$false)
    }
}
