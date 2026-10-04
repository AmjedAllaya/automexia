# Invoked by resize-stress-windows.ps1 inside its owned fixture/cleanup scope.
# Reuse its native input, window discovery and presented-frame oracle.
Add-Type -AssemblyName System.Security
$recoveryBackend = if ($UseCpuRenderer) { 'cpu' } else { 'wgpu' }
if ([string]$initial.renderer_backend -ne $recoveryBackend) {
    throw 'Recovery fixture did not use the requested renderer'
}
function Read-RecoveryCheckpoint {
    if (-not (Test-Path -LiteralPath $checkpoint)) { return $null }
    $envelope = [IO.File]::ReadAllText($checkpoint) | ConvertFrom-Json
    if ($envelope.version -eq 1) { return $envelope }
    if ($envelope.protection -ne 'dpapi-user-zlib-v1') { throw 'Checkpoint protection is missing' }
    $encrypted = [Convert]::FromBase64String($envelope.payload)
    $compressed = [Security.Cryptography.ProtectedData]::Unprotect($encrypted, $null, [Security.Cryptography.DataProtectionScope]::CurrentUser)
    $inputStream = [IO.MemoryStream]::new($compressed, 2, ($compressed.Length - 6))
    $decoder = [IO.Compression.DeflateStream]::new($inputStream, [IO.Compression.CompressionMode]::Decompress)
    $reader = [IO.StreamReader]::new($decoder)
    try { return ($reader.ReadToEnd() | ConvertFrom-Json).snapshot }
    finally { $reader.Dispose(); $decoder.Dispose(); $inputStream.Dispose(); [Array]::Clear($compressed, 0, $compressed.Length) }
}
function Get-RecoverySentinelCount {
    param($Snapshot)
    $matches = 0
    foreach ($session in @($Snapshot.windows.tabs.nodes.sessions)) {
        foreach ($row in @($session.history.rows)) {
            if ($null -eq $row.cells) { continue }
            $bytes = [Convert]::FromBase64String($row.cells)
            $text = [Text.StringBuilder]::new()
            for ($cell=0; $cell -lt $bytes.Length; $cell+=8) {
                $codepoint = [BitConverter]::ToUInt64($bytes, $cell) -band 0x1fffff
                if ($codepoint -gt 0) { [void]$text.Append([char]::ConvertFromUtf32([int]$codepoint)) }
            }
            if ($text.ToString().Contains('RECOVERY_HISTORY_SENTINEL')) { $matches++ }
        }
    }
    return $matches
}
function Get-RecoveryTopology {
    param($Snapshot)
    # History changes when new prompts appear; topology must remain identical.
    $copy = $Snapshot.windows[0].tabs | ConvertTo-Json -Depth 30 | ConvertFrom-Json
    foreach ($node in @($copy.nodes)) {
        foreach ($session in @($node.sessions)) { if ($null -ne $session) { $session.PSObject.Properties.Remove('history') } }
    }
    return ($copy | ConvertTo-Json -Depth 30 -Compress)
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
$frame = Wait-RecoveryFrame { param($f) $null -ne $f.latest_prompt_id }
if (-not [AutomexiaResizeDriver]::SendRecoverySentinel($window)) { throw 'Recovery fixture input failed' }
$checkpoint = Join-Path $configRoot 'state/session-v1/workspace.json'
$until = [DateTime]::UtcNow.AddSeconds(15)
do {
    Start-Sleep -Milliseconds 100
    $saved = Read-RecoveryCheckpoint
    $savedSessions = @($saved.windows.tabs.nodes | Where-Object { $_.kind -eq 'pane' } | ForEach-Object { $_.sessions })
    $foldersReady = @($savedSessions | Where-Object { -not [string]::IsNullOrWhiteSpace($_.cwd) }).Count -eq 6 -and (Get-RecoverySentinelCount $saved) -ge 2
} while (($savedSessions.Count -ne 6 -or -not $foldersReady) -and [DateTime]::UtcNow -lt $until)
if ($savedSessions.Count -ne 6 -or -not $foldersReady) {
    if (-not [string]::IsNullOrWhiteSpace($ResultCapture)) {
        [IO.File]::WriteAllText("$ResultCapture.initial.json", ($saved | ConvertTo-Json -Depth 20))
    }
    throw 'The real workspace and all inactive working folders were not checkpointed'
}
$expectedTopology = Get-RecoveryTopology $saved
$expectedSentinelCount = Get-RecoverySentinelCount $saved
if (@($savedSessions | Where-Object { $null -ne $_.history -and $_.history.rows.Count -gt 0 }).Count -ne 6) { throw 'Terminal history was not checkpointed' }

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
    # Preserve the first frame as well as the following presented frame, so
    # capture timing cannot hide a transient or persistent text-rendering defect.
    $presented = Read-AutomexiaSnapshot -AfterSequence ([int64]$frame.sequence)
    if (-not [bool]$presented.recovery_ready) { throw 'Recovery choice disappeared before presentation' }
    [void][AutomexiaResizeDriver]::CaptureClientFrame($window, "$ResultCapture.presented.png")
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
    $again = Read-RecoveryCheckpoint
    $topology = Get-RecoveryTopology $again
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

$script:testStage = 'consumed recovery cleanup'
$archivePath = Join-Path $configRoot 'state/session-v1/restore.json'
$until = [DateTime]::UtcNow.AddSeconds(15)
while ((Test-Path -LiteralPath $archivePath) -and [DateTime]::UtcNow -lt $until) { Start-Sleep -Milliseconds 100 }
if (Test-Path -LiteralPath $archivePath) { throw 'Successful recovery retained its consumed archive' }
$restoredSessions = @($again.windows.tabs.nodes | Where-Object { $_.kind -eq 'pane' } | ForEach-Object { $_.sessions })
if (@($restoredSessions | Where-Object { $null -ne $_.history -and $_.history.rows.Count -gt 1 }).Count -ne 6) { throw 'Restored output did not survive the fresh shell launch' }

if ((Get-RecoverySentinelCount $again) -ne $expectedSentinelCount) { throw 'Saved command/output was lost or replayed during recovery' }
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
$script:testStage = 'quiet restart preserves manual recovery'
$started = Start-RecoveryFixture
$process = $started.Process; $window = $started.Window
$frame = Wait-RecoveryFrame { param($f) -not [bool]$f.recovery_active -and $null -ne $f.latest_prompt_id }
if ([bool]$frame.recovery_ready) { throw 'A trivial terminal reopen asked to restore' }
if (-not (Test-Path -LiteralPath $archivePath)) { throw 'Start clean erased the manual recovery candidate' }
$originalWindow = $window
$originalShells = @(Get-AutomexiaOwnedProcessIds $process.Id $configRoot)
$script:testStage = 'manual recovery into additional windows'
Send-AutomexiaTestControl 'restore-previous:manual-restore'
$until = [DateTime]::UtcNow.AddSeconds(30)
do {
    Start-Sleep -Milliseconds 100
    $handles = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
} while ($handles.Count -lt 2 -and [DateTime]::UtcNow -lt $until)
if ($handles.Count -ne 2 -or $originalWindow -notin $handles) { throw 'Manual restore replaced the current window' }
$frame = Wait-RecoveryFrame { param($f) -not [bool]$f.recovery_active -and [int]$f.owned_route_count -eq 6 }
foreach ($childId in $originalShells) { if ($null -eq (Get-Process -Id $childId -ErrorAction SilentlyContinue)) { throw 'Manual restore terminated current work' } }
Send-AutomexiaTestControl 'restore-previous:duplicate-restore'
Start-Sleep -Milliseconds 500
if (@([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id)).Count -ne 2) { throw 'Consumed session restored twice' }
foreach ($handle in $handles) {
    [void][AutomexiaResizeDriver]::PostMessage($handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 200
    [void][AutomexiaResizeDriver]::SendModifiedKeyTap($handle, 0x59, $false, $false, $false)
}
if (-not $process.WaitForExit(15000)) { throw 'Manual restore windows did not close' }
$process = $null
Write-Host "PASS session recovery ($recoveryBackend): crash and close recovery, consent, layout, inert history, consumed-copy cleanup, quiet restart and manual restore preserving current work"
