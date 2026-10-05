# Dot-sourced only by the isolated native fixture, which owns process lifetime,
# temporary preferences, input, snapshots and capture. No user files are used.
function Invoke-TagShapeScenario {
    function Send-ShapeKey([int]$Key, [bool]$Control = $false) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, ($Key -eq 0x2E), $Control, $false)) {
            throw 'Shape fixture lost foreground input ownership'
        }
        $null = Read-AutomexiaSnapshot -AfterSequence ([int64](Read-AutomexiaSnapshot).sequence)
    }
    function Set-ShapeSearch([string]$Text) {
        $before = Read-AutomexiaSnapshot
        $category = $before.settings.active_category
        Click-TagBounds $before.settings.search_button
        Send-ShapeKey 0x41 $true
        # Forward Delete clears text even when empty. Backspace in an empty
        # search deliberately navigates to the parent menu.
        Send-ShapeKey 0x2E
        $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.search_bytes -eq 0 -and $s.settings.active_category -eq $category }
        $count = 0
        foreach ($letter in $Text.ToUpperInvariant().ToCharArray()) {
            Send-ShapeKey ([int]$letter)
            $count++
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.search_bytes -eq $count }
        }
    }
    function Set-TagShape([string]$Id) {
        Set-ShapeSearch 'shape'
        $state = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.controls | Where-Object id -eq 'tags.bar-style').Count -eq 1 }
        $previous = $state.settings.tag_shape
        Click-TagBounds ($state.settings.controls | Where-Object id -eq 'tags.bar-style').bounds
        # A click advances a choice; read that result before each next key.
        $state = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $s.settings.tag_shape -ne $previous }
        for ($attempt = 0; $attempt -lt 15 -and $state.settings.tag_shape -ne $Id; $attempt++) {
            $previous = $state.settings.tag_shape
            Send-ShapeKey 0x27
            $state = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $s.settings.tag_shape -ne $previous }
        }
        if ($state.settings.tag_shape -ne $Id) { throw "Shape is not reachable through its choice control: $Id" }
    }
    function Set-TagSpacing([int]$Percent) {
        Set-ShapeSearch 'spacing'
        $state = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.controls | Where-Object id -eq 'tags.spacing').Count -eq 1 }
        Click-TagBounds ($state.settings.controls | Where-Object id -eq 'tags.spacing').bounds
        $null = Wait-TagState { param($s) $s.settings.numeric_editor.id -eq 'tags.spacing' }
        Send-ShapeKey 0x41 $true
        foreach ($digit in $Percent.ToString().ToCharArray()) { Send-ShapeKey ([int]$digit) }
        Send-ShapeKey 0x0D
        $null = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.save_pending -and $null -eq $s.settings.numeric_editor -and $s.settings.tag_spacing -eq $Percent }
    }
    $styles = @('linked-arrows', 'hexagon', 'puzzle', 'slanted', 'pill-arrows', 'ribbon', 'cut-corners', 'alternating', 'top-notch', 'wedges')
    $records = @()
    $originalConfig = (Get-FileHash -LiteralPath (Join-Path $configRoot 'config.toml') -Algorithm SHA256).Hash
    if (-not [string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
        $captureRoot = [IO.Path]::GetFullPath($ModalCaptureDirectory)
        [void][IO.Directory]::CreateDirectory($captureRoot)
    }
    foreach ($style in $styles) {
        $script:testStage = "shape $style"
        Set-TagShape $style
        foreach ($spacing in @(0, 100, 300)) {
            Set-TagSpacing $spacing
            Set-ShapeSearch ''
            $state = Wait-TagState { param($s) $s.settings.ready -and -not $s.settings.save_pending }
            $tags = @($state.settings.targets | Where-Object { -not $_.roster })
            if ($tags.Count -lt 3) { throw 'Shape change lost the visible sample tags' }
            if ($state.settings.tag_shape -ne $style) { throw 'Spacing changed the selected shape' }
            # Independent native pixel oracle: the first solid production tag
            # has a bright top-center fill, above its text and away from joints.
            # A shape flag or hit target alone passes when GPU polygons are
            # accidentally painted underneath the preview's opaque panel.
            $bounds = $tags[0].bounds
            $scale = [double]$state.scale_factor
            $fill = [AutomexiaResizeDriver]::CapturePhysicalClientRegionStats($window,
                [int](($bounds[0] + $bounds[2] * 0.5) * $scale),
                [int](($bounds[1] + 2) * $scale), 2, 2)
            if ($fill.BrightSampleCount -lt 1) { throw "Native $style preview background is missing" }
            if ($captureRoot) {
                [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot "$style-$spacing.png"))
            }
            # Open a real tag from the rendered row, then return to shared controls.
            Click-TagBounds $tags[1].bounds
            $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.active_slot -eq $tags[1].id }
            Send-ShapeKey 0x1B
            $null = Wait-TagState { param($s) $s.settings.ready -and $null -eq $s.settings.active_slot }
            $records += @{ shape = $style; spacing = $spacing; sample_tags = $tags.Count }
        }
        Click-TagBounds (Read-AutomexiaSnapshot).settings.close_button
        $null = Wait-TagState { param($s) -not $s.settings.open }
        if ($captureRoot) {
            [void][AutomexiaResizeDriver]::CaptureClientFrame($window, (Join-Path $captureRoot "$style-terminal.png"))
        }
        Send-AutomexiaTestControl "open-customizations:shape-$style"
        $rootSettings = Wait-TagState { param($s) $s.settings.ready -and @($s.settings.controls | Where-Object id -eq 'tags.enabled').Count -eq 1 }
        Click-TagBounds ($rootSettings.settings.controls | Where-Object id -eq 'tags.enabled').bounds
        $null = Wait-TagState { param($s) $s.settings.ready -and $s.settings.tag_shape -eq $style -and @($s.settings.targets | Where-Object roster).Count -ge 13 }
    }
    if ((Get-FileHash -LiteralPath (Join-Path $configRoot 'config.toml') -Algorithm SHA256).Hash -ne $originalConfig) {
        throw 'Native shape edits modified the configuration file'
    }
    if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
        [IO.File]::WriteAllText([IO.Path]::GetFullPath($ResourceReport), (@{
            schema_version = 1; mode = 'tag-shapes-only'; cases = $records;
            native_mouse_selection = $true; reopen_preserves_shape = $true;
            configuration_unchanged = $true
        } | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
    }
    Write-Host 'Native tag shapes passed: ten styles, three spacings, pointer selection and reopen'
}
