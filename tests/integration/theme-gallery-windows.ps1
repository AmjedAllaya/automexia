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
    function Check-ThemeChrome([string]$Name, [int[]]$Surface, [int[]]$Foreground) {
        function Capture-ThemeSurface([string]$Suffix) {
            $path = if ([string]::IsNullOrWhiteSpace($ResultCapture)) {
                Join-Path $configRoot ('theme-' + $Name + '-' + $Suffix + '.png')
            } else { [IO.Path]::ChangeExtension($ResultCapture,($Name+'-'+$Suffix+'.png')) }
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window,$path)
            return $path
        }
        function Matching-ThemePixels($Bitmap, [double[]]$Rect, [int[]]$Rgb) {
            $matches = 0
            for ($y=[int]$Rect[1]; $y -lt [Math]::Min($Bitmap.Height,$Rect[3]); $y+=2) {
                for ($x=[int]$Rect[0]; $x -lt [Math]::Min($Bitmap.Width,$Rect[2]); $x+=2) {
                    $pixel = $Bitmap.GetPixel($x,$y)
                    if ([Math]::Abs($pixel.R-$Rgb[0]) -le 3 -and [Math]::Abs($pixel.G-$Rgb[1]) -le 3 -and [Math]::Abs($pixel.B-$Rgb[2]) -le 3) { $matches++ }
                }
            }
            return $matches
        }
        $script:testStage = 'themed dense table readability ' + $Name
        $previous = Read-AutomexiaSnapshot
        # Fictional, deterministic file rows exercise dates, descenders and long names.
        Send-AutomexiaTestControl ("write-line:theme-table-" + $Name + ":Write-Output @('Mode   Last Modified       Size   Name','d----  2026-01-02 09:30    2048   glyphs_and_queries','-a---  2026-01-03 12:45    4096   application.toml','-a---  2026-01-04 16:10    8192   deployment-notes.md','-a---  2026-01-05 08:15    512    package.json','-a---  2026-01-06 10:20    128    registry.log','-a---  2026-01-07 18:25    64     typography.txt')")
        $table = Wait-ThemeState {param($s) $s.command_result_key -ne $previous.command_result_key -and $s.inline_table_count -ge 1 -and $null -ne $s.command_result_surface}
        $bitmap = [Drawing.Bitmap]::new((Capture-ThemeSurface 'table'))
        try {
            $area = @($table.command_result_surface)
            $tableScale = [double]$table.scale_factor
            $rect = @(($area[0]*$tableScale),($area[1]*$tableScale),(($area[0]+$area[2])*$tableScale),(($area[1]+$area[3])*$tableScale))
            # Literal independent palette expectations, not production tint helpers.
            $stripe = if ($Name -eq 'solar-dusk') { @(33,25,22) } else { @(235,239,244) }
            $heading = if ($Name -eq 'solar-dusk') { @(40,33,29) } else { @(228,233,237) }
            if ((Matching-ThemePixels $bitmap $rect $stripe) -lt 100 -or
                (Matching-ThemePixels $bitmap $rect $heading) -lt 100 -or
                (Matching-ThemePixels $bitmap $rect $Foreground) -lt 60) {throw 'Themed table lost subtle backgrounds or readable glyphs'}
        } finally { $bitmap.Dispose() }
        $scale = [double](Read-AutomexiaSnapshot).scale_factor
        $script:testStage = 'applied header and footer ' + $Name
        $bitmap = [Drawing.Bitmap]::new((Capture-ThemeSurface 'terminal'))
        try {
            $header = @(($bitmap.Width*0.4),(4*$scale),($bitmap.Width*0.7),(28*$scale))
            $footer = @((30*$scale),($bitmap.Height-55*$scale),($bitmap.Width*0.6),($bitmap.Height-2*$scale))
            $title = @((70*$scale),(4*$scale),(270*$scale),(40*$scale))
            if ((Matching-ThemePixels $bitmap $header $Surface) -lt 300) {throw 'Header did not follow the applied theme'}
            if ((Matching-ThemePixels $bitmap $footer $Surface) -lt 300) {throw 'Footer did not follow the applied theme'}
            if ((Matching-ThemePixels $bitmap $title $Foreground) -lt 12) {throw 'Header title lost its readable theme foreground'}
        } finally { $bitmap.Dispose() }
        $categories = @('Tabs & Windows','Panes & Sessions','Search & History','Clipboard & Input','Appearance','Customizations','Tools')
        for ($category=0; $category -lt $categories.Count; $category++) {
            $script:testStage = 'themed command category ' + $categories[$category]
            Send-AutomexiaTestControl ('open-palette:' + [guid]::NewGuid().ToString('N'))
            $null = Wait-ThemeState {param($s) $s.palette_enabled -and $s.palette_accessibility_summary.StartsWith('Command categories;')}
            for ($row=0; $row -lt $category; $row++) { Theme-Key 0x28 }
            $null = Wait-ThemeState {param($s) $s.palette_selected_index -eq $category}
            Theme-Key 0x0D
            $null = Wait-ThemeState {param($s) $s.palette_enabled -and $s.palette_accessibility_summary.StartsWith($categories[$category]+';')}
            $bitmap = [Drawing.Bitmap]::new((Capture-ThemeSurface ('category-'+$category)))
            try {
                $input = @(($bitmap.Width*0.5),(100*$scale),($bitmap.Width*0.5+30*$scale),(112*$scale))
                if ((Matching-ThemePixels $bitmap $input $Surface) -lt 30) {throw 'Menu category kept an old theme surface'}
            } finally { $bitmap.Dispose() }
            if ($category -eq 0) {
                Theme-Key 0x71
                $null = Wait-ThemeState {param($s) $s.palette_accessibility_summary.StartsWith('Edit shortcut dialog;')}
                $shortcutCapture = Capture-ThemeSurface 'shortcut-editor'
                $shortcutBitmap = [Drawing.Bitmap]::new($shortcutCapture)
                try {
                    # The fixed-size fixture exposes antialiased rounded corners.
                    # Compare all RGB samples directly with the desktop, so the
                    # GDI+ marker pattern cannot silently blacken the saved PNG.
                    $cornerX = [int][Math]::Floor(($shortcutBitmap.Width - 520*$scale)/2)
                    $cornerY = [int][Math]::Floor(($shortcutBitmap.Height - 270*$scale)/2)
                    [AutomexiaResizeDriver]::VerifyCapturedDesktopRegion(
                        $window,$shortcutCapture,$cornerX,$cornerY,24,24)
                } finally { $shortcutBitmap.Dispose() }
                Theme-Key 0x1B
                $null = Wait-ThemeState {param($s) $s.palette_accessibility_summary.StartsWith($categories[$category]+';')}
            }
            Send-AutomexiaTestControl ('dismiss-modal:' + [guid]::NewGuid().ToString('N'))
            $null = Wait-ThemeState {param($s) -not $s.palette_enabled}
        }
        $script:testStage = 'themed customization and close dialog ' + $Name
        Send-AutomexiaTestControl ('open-customizations:' + [guid]::NewGuid().ToString('N'))
        $null = Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.open}
        $null = Capture-ThemeSurface 'customizations'
        $script:testStage = 'window control styles ' + $Name
        function Open-WindowControls {
            Send-AutomexiaTestControl ('open-customizations:' + [guid]::NewGuid().ToString('N'))
            $root = Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.open -and $null -eq $s.settings.active_category}
            Click-Theme $root.settings.search_button
            if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window,0x41,$false,$true,$false)) { throw 'Caption search selection failed' }
            foreach ($key in @(0x57,0x49,0x4E,0x44,0x4F,0x57)) { Theme-Key $key }
            $root = Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.search_bytes -eq 6 -and @($s.settings.controls | Where-Object id -eq 'window-controls.style').Count -eq 1}
            Click-Theme ($root.settings.controls | Where-Object id -eq 'window-controls.style').bounds
            return Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.active_category -eq 'window-controls.style'}
        }
        $page = Open-WindowControls
        $digests = [Collections.Generic.HashSet[string]]::new()
        foreach ($style in @('soft','glass','outline','circles')) {
            for ($attempt=0; $attempt -lt 4 -and $page.settings.window_controls_style -ne $style; $attempt++) {
                Click-Theme ($page.settings.controls | Where-Object id -eq 'window-controls.style').bounds
                $prior = $page.settings.window_controls_style
                $page = Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.window_controls_style -ne $prior}
            }
            if ($page.settings.window_controls_style -ne $style) { throw 'Caption style did not apply' }
            $null = Capture-ThemeSurface ('window-controls-'+$style)
            $preview=@($page.settings.preview_bounds)
            Click-Theme @($preview[0],($preview[1]+40),[Math]::Min(140,$preview[2]),38)
            $page=Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.active_category -eq 'window-controls.style' -and -not $s.confirm_quit_active}
            # Settings is opaque and covers the caption. Close it to verify the
            # real buttons rather than sampling the overlay's unchanged surface.
            Click-Theme $page.settings.close_button
            $null = Wait-ThemeState {param($s) -not $s.settings.open}
            $bitmap = [Drawing.Bitmap]::new((Capture-ThemeSurface ('window-controls-'+$style+'-titlebar')))
            try {
                # Only the real title-bar buttons: preview text cannot satisfy this oracle.
                $bytes = [Collections.Generic.List[byte]]::new()
                for ($py=4; $py -lt [int](38*$scale); $py++) {
                    for ($px=$bitmap.Width-[int](126*$scale); $px -lt $bitmap.Width-2; $px++) {
                        $pixel=$bitmap.GetPixel($px,$py)
                        $bytes.Add($pixel.R); $bytes.Add($pixel.G); $bytes.Add($pixel.B)
                    }
                }
                $sha=[Security.Cryptography.SHA256]::Create()
                try { $digest=[Convert]::ToBase64String($sha.ComputeHash($bytes.ToArray())) } finally { $sha.Dispose() }
                if (-not $digests.Add($digest)) { throw 'Caption style did not change actual title-bar pixels' }
            } finally { $bitmap.Dispose() }
            $page = Open-WindowControls
        }
        # Return the disposable fixture to Soft before unrelated theme scenarios.
        Click-Theme ($page.settings.controls | Where-Object id -eq 'window-controls.style').bounds
        $null=Wait-ThemeState {param($s) $s.settings.ready -and $s.settings.window_controls_style -eq 'soft'}
        Theme-Key 0x1B
        $null=Wait-ThemeState {param($s) $s.settings.ready -and $null -eq $s.settings.active_category}
        Theme-Key 0x1B
        $null = Wait-ThemeState {param($s) -not $s.settings.open}
        Send-AutomexiaTestControl ('confirm-quit:' + [guid]::NewGuid().ToString('N'))
        $null = Wait-ThemeState {param($s) $s.confirm_quit_active}
        $null = Capture-ThemeSurface 'close'
        Theme-Key 0x1B
        $null = Wait-ThemeState {param($s) -not $s.confirm_quit_active}
        Send-AutomexiaTestControl ('open-connection-hub:' + [guid]::NewGuid().ToString('N'))
        $null = Wait-ThemeState {param($s) $s.connection_hub_active}
        $null = Capture-ThemeSurface 'connections'
        Theme-Key 0x1B
        $null = Wait-ThemeState {param($s) -not $s.connection_hub_active}
        Send-AutomexiaTestControl ('open-pane-search:' + [guid]::NewGuid().ToString('N'))
        $null = Wait-ThemeState {param($s) $s.search_active}
        $null = Capture-ThemeSurface 'search'
        Theme-Key 0x1B
        $null = Wait-ThemeState {param($s) -not $s.search_active}
    }
    function Preference-Text {
        $path = Join-Path $configRoot 'state/user-preferences-v11.toml'
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
        # Independent literal palette/surface oracles catch fixed header/footer
        # skins and fill values accidentally consumed as tab text colors.
        Check-ThemeChrome 'solar-dusk' @(33,26,23) @(242,230,216)
        $null = Open-Themes
        $null = Select-Theme 'arctic-day'
        Theme-Key 0x0D
        $null = Wait-ThemeState {param($s) -not $s.settings.open}
        Check-ThemeChrome 'arctic-day' @(233,237,241) @(37,55,68)
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
        Write-Host "Native theme gallery passed: $expectedRendererBackend; five previews, cancel, apply, copy, configuration, dark/light chrome and seven menu categories."
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window,$false)
    }
}
