# Runs only in resize-stress-windows.ps1's disposable configuration and owned process.
function Test-AutomexiaNamedProfiles {
    function Wait-ProfileState([scriptblock]$Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do { $state = Read-AutomexiaSnapshot; if (& $Predicate $state) { return $state }; Start-Sleep -Milliseconds 40 } while ([DateTime]::UtcNow -lt $deadline)
        Write-Host ("Profile fixture state: palette={0}, results={1}, settings={2}, profile_view={3}" -f $state.palette_enabled,$state.palette_total_results,$state.settings.open,($null -ne $state.settings.profiles))
        throw "Named Profiles did not settle: $script:testStage"
    }
    function Profile-Key([int]$Key, [bool]$Control = $false) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window,$Key,$false,$Control,$false)) { throw 'Profile keyboard input failed' }
    }
    function Profile-Click($Bounds) {
        if ($null -eq $Bounds -or @($Bounds).Count -ne 4) { throw 'Profile control is missing' }
        $scale = [double](Read-AutomexiaSnapshot).scale_factor
        $x = [int][Math]::Round(($Bounds[0] + $Bounds[2] * 0.5) * $scale)
        $y = [int][Math]::Round(($Bounds[1] + $Bounds[3] * 0.5) * $scale)
        [void][AutomexiaResizeDriver]::MovePhysicalPointerToClient($window,$x + 6,$y)
        Start-Sleep -Milliseconds 60
        [void][AutomexiaResizeDriver]::MovePhysicalPointerToClient($window,$x,$y)
        $null = Wait-ProfileState {param($s) $s.settings.ready -and $null -ne $s.settings.pointer -and [Math]::Abs($s.settings.pointer[0]-$x/$scale)-lt 1 -and [Math]::Abs($s.settings.pointer[1]-$y/$scale)-lt 1}
        if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($window,$x,$y)) { throw 'Profile pointer input failed' }
    }
    function Profile-Search([string]$Query) {
        $state = Read-AutomexiaSnapshot
        Profile-Click $state.settings.search_button
        Profile-Key 0x41 $true
        if (-not [AutomexiaResizeDriver]::SendProfileFixtureText($window,$Query)) { throw 'Profile search input failed' }
        return Wait-ProfileState {param($s) $s.settings.ready -and $s.settings.search_bytes -eq $Query.Length}
    }
    function Edit-ProfileField([string]$Query,[string]$Id,[string]$Value) {
        $state=Profile-Search $Query
        Profile-Click ($state.settings.controls | Where-Object id -eq $Id).bounds
        $null=Wait-ProfileState {param($s) $s.settings.ready -and $null -ne $s.settings.color_editor}
        Profile-Key 0x41 $true
        if (-not [AutomexiaResizeDriver]::SendProfileFixtureText($window,$Value)) {throw 'Profile field typing failed'}
        Profile-Key 0x0D
        $null=Wait-ProfileState {param($s) $s.settings.ready -and $null -eq $s.settings.color_editor}
    }
    function Profile-Background([string]$Name,[double]$X=0.75,[double]$Y=0.7) {
        $path=if ([string]::IsNullOrWhiteSpace($ResultCapture)) {Join-Path $configRoot ($Name+'.png')} else {[IO.Path]::ChangeExtension($ResultCapture,($Name+'.png'))}
        [void][AutomexiaResizeDriver]::CaptureClientFrame($window,$path)
        $bitmap=[Drawing.Bitmap]::new($path)
        try {return $bitmap.GetPixel([int]($bitmap.Width*$X),[int]($bitmap.Height*$Y))} finally {$bitmap.Dispose()}
    }
    $originalBackground=Profile-Background 'profile-baseline'
    $script:testStage='open profiles through application route'
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window,0x50,$false,$true,$true)) {throw 'Could not open the command palette'}
    $null=Wait-ProfileState {param($s) $s.palette_enabled}
    $script:testStage='search profiles in the keyboard-opened palette'
    if (-not [AutomexiaResizeDriver]::SendProfileFixtureText($window,'New Terminal with Profile')) {throw 'Could not search for profiles'}
    $null=Wait-ProfileState {param($s) $s.palette_enabled -and $s.palette_total_results -eq 1 -and $s.palette_accessibility_summary.StartsWith('All commands; 1 results;')}
    $script:testStage='activate profile picker from palette'
    Profile-Key 0x0D
    $initial=Wait-ProfileState {param($s) $s.settings.ready -and $null -ne $s.settings.profiles -and -not $s.settings.profiles.busy}
    $tabs=[int]$initial.window_tab_count
    Profile-Key 0x0D
    $null=Wait-ProfileState {param($s) $s.settings.ready -and $s.settings.profiles.editing}
    Edit-ProfileField 'Name' 'profiles.name' 'Focus workspace'
    Edit-ProfileField 'Executable' 'profiles.program' 'powershell.exe'
    foreach ($argument in @(@('Argument 1','profiles.arg.a0','-NoLogo'),@('Argument 2','profiles.arg.a1','-NoProfile'))) {
        $state=Profile-Search 'Add argument'
        Profile-Click ($state.settings.controls | Where-Object id -eq 'profiles.add-arg').bounds
        Edit-ProfileField $argument[0] $argument[1] $argument[2]
    }
    Edit-ProfileField 'Theme' 'profiles.theme' 'solar-dusk'
    Edit-ProfileField 'Tab icon' 'profiles.icon' 'W'
    $state=Profile-Search 'Custom tab color'
    Profile-Click ($state.settings.controls | Where-Object id -eq 'profiles.color-enabled').bounds
    $null=Wait-ProfileState {param($s) $s.settings.ready}
    Edit-ProfileField 'Tab accent' 'profiles.color' '#E8AD70'
    $script:testStage='cancel draft keeps changes until discard confirmed'
    Profile-Key 0x1B
    $null=Wait-ProfileState {param($s) $s.settings.ready -and $null -ne $s.settings.confirmation}
    Profile-Key 0x1B
    $null=Wait-ProfileState {param($s) $s.settings.ready -and $null -eq $s.settings.confirmation -and $s.settings.profiles.editing}
    Profile-Key 0x53 $true
    $script:testStage='save profile without launching'
    $saved=Wait-ProfileState {param($s) $s.settings.ready -and -not $s.settings.profiles.busy -and -not $s.settings.profiles.editing}
    if ([int]$saved.window_tab_count -ne $tabs) {throw 'Saving a profile launched a terminal'}
    $stored=Join-Path $configRoot 'profiles/profiles-v1.toml'
    $text=[IO.File]::ReadAllText($stored)
    foreach ($expected in @('name = "Focus workspace"','program = "powershell.exe"','"-NoLogo"','"-NoProfile"','theme = "solar-dusk"','icon = "W"','color = "#E8AD70"')) {if (-not $text.Contains($expected)) {throw "Saved profile differs from the visible edit: $expected"}}
    $state=Profile-Search 'Focus workspace'
    Profile-Key 0x09
    Profile-Key 0x0D
    $state=Wait-ProfileState {param($s) $s.settings.ready -and $s.settings.profiles.selected}
    if (-not [string]::IsNullOrWhiteSpace($ResultCapture)) {[void][AutomexiaResizeDriver]::CaptureClientFrame($window,$ResultCapture)}
    $script:testStage='explicit launch uses a new independently owned tab'
    Profile-Key 0x0D
    $launched=Wait-ProfileState {param($s) -not $s.settings.open -and [int]$s.window_tab_count -eq $tabs+1 -and @($s.window_tab_titles | Where-Object {$_ -ceq 'Focus workspace'}).Count -eq 1}
    if ($launched.renderer_backend -ne $expectedRendererBackend) {throw 'Unexpected renderer for profile fixture'}
    $script:testStage='profile theme reaches rendered terminal and restores on tab switch'
    $color=Profile-Background 'profile-themed'
    if ([Math]::Abs($color.R-25)-gt 3 -or [Math]::Abs($color.G-18)-gt 3 -or [Math]::Abs($color.B-15)-gt 3) {throw 'Profile theme did not reach the rendered terminal'}
    $script:testStage='temporary theme preview overrides profile colors and Escape restores them'
    Send-AutomexiaTestControl ('open-themes:profile-preview-'+[guid]::NewGuid().ToString('N'))
    $null=Wait-ProfileState {param($s) $s.settings.ready -and $null -ne $s.settings.gallery -and -not $s.settings.gallery.busy -and $s.settings.gallery.count -ge 6}
    Profile-Key 0x24
    $null=Wait-ProfileState {param($s) $s.settings.ready -and $s.settings.gallery.selected -eq 0}
    for ($i=0;$i -lt 8;$i++) {
        $state=Read-AutomexiaSnapshot
        if ($state.settings.gallery.builtin -eq 'builtin:arctic-day') {break}
        $prior=$state.settings.gallery.selected
        Profile-Key 0x28
        $null=Wait-ProfileState {param($s) $s.settings.ready -and $s.settings.gallery.selected -ne $prior}
    }
    $null=Wait-ProfileState {param($s) $s.settings.ready -and $s.settings.gallery.builtin -eq 'builtin:arctic-day'}
    # The left list's empty lower surface belongs to the window palette, not the
    # gallery's right-side sample. A light preview must replace the warm dark UI.
    $color=Profile-Background 'profile-gallery' 0.18 0.82
    if ($color.R+$color.G+$color.B -lt 510) {throw 'Profile colors prevented the instant theme preview'}
    Profile-Key 0x1B
    $null=Wait-ProfileState {param($s) $null -eq $s.settings.gallery}
    Profile-Key 0x1B
    $returned=Wait-ProfileState {param($s) -not $s.settings.open}
    if ($returned.palette_enabled) {throw 'Closing a later settings page reopened the profile launcher after launch'}
    $color=Profile-Background 'profile-after-preview'
    if ([Math]::Abs($color.R-25)-gt 3 -or [Math]::Abs($color.G-18)-gt 3 -or [Math]::Abs($color.B-15)-gt 3) {throw 'Cancelling theme preview lost profile colors'}
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window,0x09,$false,$true,$true)) {throw 'Profile tab switch failed'}
    $null=Wait-ProfileState {param($s) $s.active_window_tab_index -ne $launched.active_window_tab_index}
    $color=Profile-Background 'profile-returned'
    if ([Math]::Abs($color.R-$originalBackground.R)-gt 3 -or [Math]::Abs($color.G-$originalBackground.G)-gt 3 -or [Math]::Abs($color.B-$originalBackground.B)-gt 3) {throw 'Leaving the profile did not restore the original window theme'}
    Write-Host "Native Named Profiles passed: $expectedRendererBackend; keyboard and pointer edit, discard cancellation, private save, explicit independent terminal launch, theme application and restoration."
}
