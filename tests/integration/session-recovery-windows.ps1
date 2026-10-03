# Invoked by resize-stress-windows.ps1 inside its owned fixture/cleanup scope.
# Reuse its native input, window discovery and presented-frame oracle.
$recoveryBackend = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
if ([string]$initial.renderer_backend -ne $recoveryBackend) {
    throw 'Recovery fixture did not use the requested renderer'
}
function Start-RecoveryFixture {
    [IO.File]::Delete($snapshotPath)
    [IO.File]::Delete($controlPath)
    $child = Start-Process -FilePath $Binary -WorkingDirectory $root -WindowStyle Hidden -PassThru
    $until = [DateTime]::UtcNow.AddSeconds(20)
    do {
        Start-Sleep -Milliseconds 50
        $child.Refresh()
        $handles = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($child.Id))
    } while ($handles.Count -eq 0 -and -not $child.HasExited -and [DateTime]::UtcNow -lt $until)
    if ($child.HasExited -or $handles.Count -eq 0) {
        if (-not $child.HasExited) {
            foreach ($ownedId in @(Get-AutomexiaOwnedProcessIds $child.Id $configRoot)) {
                Stop-Process -Id $ownedId -Force -ErrorAction SilentlyContinue
            }
            Stop-Process -Id $child.Id -Force -ErrorAction SilentlyContinue
        }
        $child.Dispose()
        throw 'Recovery fixture did not open its native window'
    }
    return @{ Process = $child; Window = $handles[0] }
}
function Wait-RecoveryFrame {
    param([scriptblock]$Predicate)
    $until = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $frame = Read-AutomexiaSnapshot
        if (& $Predicate $frame) { return $frame }
        Start-Sleep -Milliseconds 40
    } while ([DateTime]::UtcNow -lt $until)
    Write-Host ("stage={0} tabs={1} routes={2} active={3} ready={4} prompt={5}" -f $script:testStage, $frame.window_tab_count, $frame.owned_route_count, $frame.recovery_active, $frame.recovery_ready, $frame.latest_prompt_id)
    throw 'Recovery did not reach the expected presented state'
}
function Stop-RecoveryFixture {
    param([switch]$Crash)
    $owned = @(Get-AutomexiaOwnedProcessIds $process.Id $configRoot)
    if ($Crash) { Stop-Process -Id $process.Id -Force }
    else {
        [void]$process.CloseMainWindow()
        Start-Sleep -Milliseconds 200
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x59, $false, $false, $false)) { throw 'Could not confirm fixture close' }
    }
    if (-not $process.WaitForExit(15000)) { throw 'Recovery fixture did not exit' }
    foreach ($childId in $owned) {
        $remaining = Get-Process -Id $childId -ErrorAction SilentlyContinue
        if ($null -ne $remaining) { Stop-Process -Id $childId -Force -ErrorAction SilentlyContinue }
    }
}

$script:testStage = 'session recovery topology checkpoint'
if (-not [AutomexiaResizeDriver]::SendControlBurst($window, 0x54, 3)) { throw 'Could not open recovery fixture tabs' }
$frame = Wait-RecoveryFrame { param($f) [int]$f.window_tab_count -eq 4 }
Send-AutomexiaTestControl 'clone-right:recovery-split'
$frame = Wait-RecoveryFrame { param($f) [int]$f.panel_count -eq 2 }
Send-AutomexiaTestControl 'local-tab:recovery-local-tab'
$frame = Wait-RecoveryFrame { param($f) [int]$f.owned_route_count -eq 6 }
$checkpoint = Join-Path $configRoot 'state/session-v1/workspace.json'
$until = [DateTime]::UtcNow.AddSeconds(15)
do {
    Start-Sleep -Milliseconds 100
    $saved = if (Test-Path -LiteralPath $checkpoint) { [IO.File]::ReadAllText($checkpoint) | ConvertFrom-Json } else { $null }
    $savedSessions = @($saved.windows.tabs.nodes | Where-Object { $_.kind -eq 'pane' } | ForEach-Object { $_.sessions })
    $foldersReady = @($savedSessions | Where-Object { -not [string]::IsNullOrWhiteSpace($_.cwd) }).Count -eq 6
} while (($savedSessions.Count -ne 6 -or -not $foldersReady) -and [DateTime]::UtcNow -lt $until)
if ($savedSessions.Count -ne 6 -or -not $foldersReady) {
    if (-not [string]::IsNullOrWhiteSpace($ResultCapture)) {
        [IO.File]::WriteAllText("$ResultCapture.initial.json", ($saved | ConvertTo-Json -Depth 20))
    }
    throw 'The real workspace and all inactive working folders were not checkpointed'
}
$expectedTopology = $saved.windows[0].tabs | ConvertTo-Json -Depth 20 -Compress

$script:testStage = 'session recovery crash and consent'
Stop-RecoveryFixture -Crash
$started = Start-RecoveryFixture
$process = $started.Process; $window = $started.Window
$frame = Wait-RecoveryFrame { param($f) [bool]$f.recovery_ready }
if (@(Get-AutomexiaOwnedProcessIds $process.Id $configRoot).Count -ne 0) {
    throw 'A process launched before recovery consent'
}
$beforeDismissal = [IO.File]::ReadAllText($checkpoint)
[void]$process.CloseMainWindow()
if (-not $process.WaitForExit(15000)) { throw 'Closing the recovery choice did not exit' }
if ([IO.File]::ReadAllText($checkpoint) -cne $beforeDismissal) { throw 'Closing the recovery choice overwrote the saved workspace' }
$started = Start-RecoveryFixture
$process = $started.Process; $window = $started.Window
$frame = Wait-RecoveryFrame { param($f) [bool]$f.recovery_ready }
if (-not [string]::IsNullOrWhiteSpace($ResultCapture)) {
    [void][AutomexiaResizeDriver]::CaptureClientFrame($window, $ResultCapture)
}
$script:testStage = 'restoration after R confirmation'
if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x52, $false, $false, $false)) { throw 'Could not activate Restore' }
$frame = Wait-RecoveryFrame { param($f) -not [bool]$f.recovery_active -and [int]$f.owned_route_count -eq 6 -and $null -ne $f.latest_prompt_id }
if ([string]$frame.renderer_backend -ne $recoveryBackend) { throw 'Restore changed renderer backend' }
if ([int]$frame.window_tab_count -ne 4 -or [int]$frame.active_window_tab_index -ne 3 -or [int]$frame.panel_count -ne 2) {
    throw 'Restored topology or selection differs from the saved workspace'
}
$shells = @(Get-AutomexiaOwnedProcessIds $process.Id $configRoot | ForEach-Object {
    Get-Process -Id $_ -ErrorAction SilentlyContinue
} | Where-Object { $_.ProcessName -in @('powershell', 'pwsh') })
if ($shells.Count -ne 6) { throw 'Restore did not create six independent live shell processes' }
$until = [DateTime]::UtcNow.AddSeconds(15)
do {
    Start-Sleep -Milliseconds 100
    $again = [IO.File]::ReadAllText($checkpoint) | ConvertFrom-Json
    $topology = $again.windows[0].tabs | ConvertTo-Json -Depth 20 -Compress
    $freshWrite = (Get-Item -LiteralPath $checkpoint).LastWriteTimeUtc -gt $process.StartTime.ToUniversalTime()
} while (($topology -cne $expectedTopology -or -not $freshWrite) -and [DateTime]::UtcNow -lt $until)
if ($topology -cne $expectedTopology -or -not $freshWrite) {
    Write-Host "Recovery checkpoint comparison: fresh=$freshWrite topology=$($topology -ceq $expectedTopology)"
    if (-not [string]::IsNullOrWhiteSpace($ResultCapture)) {
        [IO.File]::WriteAllText("$ResultCapture.expected.json", $expectedTopology)
        [IO.File]::WriteAllText("$ResultCapture.actual.json", $topology)
    }
    throw 'Restored split ratios, local-tab order or checkpoint publication changed'
}

$script:testStage = 'session recovery normal close, escape and input isolation'
Stop-RecoveryFixture
$started = Start-RecoveryFixture
$process = $started.Process; $window = $started.Window
$frame = Wait-RecoveryFrame { param($f) [bool]$f.recovery_ready }
if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, 0x1B, $false, $false, $false)) { throw 'Could not activate Start clean' }
$frame = Wait-RecoveryFrame { param($f) -not [bool]$f.recovery_active -and $null -ne $f.latest_prompt_id }
if ([int]$frame.window_tab_count -ne 1 -or [int]$frame.owned_route_count -ne 1) { throw 'Start clean resurrected previous tabs' }
if ([string](Get-ActiveAutomexiaPanel $frame).raw_cursor_line_text -match '[rRsS]$') { throw 'Recovery key leaked into the shell' }
Stop-RecoveryFixture
$process = $null
Write-Host "PASS session recovery ($recoveryBackend): real crash, no launch before consent, tabs/splits/local tabs, normal close and Start clean"
