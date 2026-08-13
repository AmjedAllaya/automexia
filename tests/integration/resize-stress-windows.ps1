param(
    [string]$Binary
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrWhiteSpace($Binary)) {
    $Binary = Join-Path $root 'target\debug\automexia.exe'
}
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "Automexia test binary was not found at $Binary"
}

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class AutomexiaResizeDriver {
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool MoveWindow(
        IntPtr hWnd, int x, int y, int width, int height, bool repaint);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(
        IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);

}
'@

function Get-ActiveAutomexiaPanel {
    param($Snapshot)
    return @($Snapshot.panels | Where-Object { [bool]$_.active })[0]
}

function Test-AllAutomexiaPaneContexts {
    param($Snapshot)
    if ([int]$Snapshot.panel_count -le 0 -or
        @($Snapshot.panels).Count -ne [int]$Snapshot.panel_count) {
        return $false
    }
    foreach ($panel in @($Snapshot.panels)) {
        if ($null -eq $panel.context_session_id -or
            [int64]$panel.context_session_id -ne [int64]$panel.route_id -or
            @($panel.context_segments).Count -eq 0) {
            return $false
        }
    }
    return $true
}

$snapshotPath = Join-Path ([System.IO.Path]::GetTempPath()) (
    'automexia-resize-{0}.json' -f [guid]::NewGuid().ToString('N'))
$controlPath = Join-Path ([System.IO.Path]::GetTempPath()) (
    'automexia-control-{0}.txt' -f [guid]::NewGuid().ToString('N'))
$configRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
    'automexia-resize-config-{0}' -f [guid]::NewGuid().ToString('N'))
$previousSnapshot = $env:AUTOMEXIA_RESIZE_SNAPSHOT
$previousControl = $env:AUTOMEXIA_NATIVE_TEST_CONTROL
$previousConfigHome = $env:AUTOMEXIA_CONFIG_HOME
$process = $null
$window = [IntPtr]::Zero
$lastSnapshot = $null
$testStage = 'startup'

function Read-AutomexiaSnapshot {
    param(
        [int64]$AfterSequence = -1,
        [int]$TimeoutMilliseconds = 10000
    )

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        if (Test-Path -LiteralPath $snapshotPath -PathType Leaf) {
            try {
                $snapshot = Get-Content -LiteralPath $snapshotPath -Raw | ConvertFrom-Json
                $script:lastSnapshot = $snapshot
                if ([int64]$snapshot.sequence -gt $AfterSequence) {
                    return $snapshot
                }
            } catch {
                # The render thread may be replacing this small JSON payload.
            }
        }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $deadline)

    if ($null -ne $process) {
        $process.Refresh()
        Write-Host "Automexia process exited: $($process.HasExited)"
    }
    Write-Host "Native resize test stage: $script:testStage"
    if ($null -ne $script:lastSnapshot) {
        Write-Host ($script:lastSnapshot | ConvertTo-Json -Depth 8)
    } elseif (Test-Path -LiteralPath $snapshotPath -PathType Leaf) {
        Write-Host 'Last raw renderer snapshot:'
        Write-Host (Get-Content -LiteralPath $snapshotPath -Raw)
    }
    throw "Timed out waiting for an Automexia renderer snapshot after sequence $AfterSequence during $script:testStage"
}

function Send-AutomexiaTestControl {
    param([string]$Control)
    [System.IO.File]::WriteAllText(
        $controlPath,
        $Control,
        [System.Text.UTF8Encoding]::new($false))
    if ($window -ne [IntPtr]::Zero) {
        # WM_PAINT is posted asynchronously. It wakes the feature-gated control
        # reader without adding a synchronous resize to the latency result.
        [void][AutomexiaResizeDriver]::PostMessage(
            $window, 0x000F, [IntPtr]::Zero, [IntPtr]::Zero)
    }
}

try {
    [void](New-Item -ItemType Directory -Path $configRoot)
    $integration = (Join-Path $root 'shell-integration\powershell\automexia.ps1').Replace('\', '/')
    $config = @"
[shell]
program = "powershell.exe"
args = ["-NoLogo", "-NoProfile", "-NoExit", "-Command", ". '$integration'"]
"@
    [System.IO.File]::WriteAllText(
        (Join-Path $configRoot 'config.toml'),
        $config,
        [System.Text.UTF8Encoding]::new($false))

    $env:AUTOMEXIA_RESIZE_SNAPSHOT = $snapshotPath
    $env:AUTOMEXIA_NATIVE_TEST_CONTROL = $controlPath
    $env:AUTOMEXIA_CONFIG_HOME = $configRoot
    $process = Start-Process -FilePath $Binary -WorkingDirectory $root -PassThru

    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        Start-Sleep -Milliseconds 50
        $process.Refresh()
        $window = $process.MainWindowHandle
    } while ($window -eq [IntPtr]::Zero -and -not $process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    if ($process.HasExited) {
        throw "Automexia exited before its native window became ready (exit $($process.ExitCode))"
    }
    if ($window -eq [IntPtr]::Zero) {
        throw 'Automexia did not expose a native window within 15 seconds'
    }

    # A native window can be drawable before PowerShell has emitted its first
    # prompt. Wait for the prompt without sending input so this test also proves
    # that startup metadata/path discovery is automatic rather than action-led.
    $script:testStage = 'initial prompt'
    $initial = Read-AutomexiaSnapshot
    $promptDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (($null -eq $initial.latest_prompt_id -or
            [int]$initial.latest_prompt_start_count -ne 1 -or
            -not [bool]$initial.full_path_visible) -and
           [DateTime]::UtcNow -lt $promptDeadline) {
        $initial = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
    }
    if ($null -eq $initial.latest_prompt_id -or
        [int]$initial.latest_prompt_start_count -ne 1 -or
        -not [bool]$initial.full_path_visible) {
        Write-Host ($initial | ConvertTo-Json -Depth 4)
        throw 'PowerShell did not publish one complete prompt automatically after startup'
    }

    $initialPanel = Get-ActiveAutomexiaPanel $initial
    if ($null -eq $initialPanel -or [int]$initial.panel_count -ne 1) {
        throw 'The initial native session snapshot is incomplete'
    }
    if ([int64]$initialPanel.shell_pid -le 0) {
        throw 'The initial ConPTY child process ID was not recorded'
    }

    # Create a top-level tab through the exact Ctrl+T lifecycle. The renderer
    # snapshot is taken immediately after the control is consumed, before any
    # later OS resize can accidentally repair stale geometry.
    $script:testStage = 'create top-level tab at current viewport'
    $windowTabControl = 'window-tab:top-create'
    Send-AutomexiaTestControl $windowTabControl
    $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
    $topTabDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([string]$topTab.last_control -ne $windowTabControl -or
            [int]$topTab.window_tab_count -ne 2 -or
            [int]$topTab.active_window_tab_index -ne 1 -or
            [int]$topTab.panel_count -ne 1) -and
           [DateTime]::UtcNow -lt $topTabDeadline) {
        $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$topTab.sequence)
    }
    if ([string]$topTab.last_control -ne $windowTabControl -or
        [int]$topTab.window_tab_count -ne 2 -or
        [int]$topTab.active_window_tab_index -ne 1 -or
        [int]$topTab.panel_count -ne 1) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'Ctrl+T lifecycle did not create and select exactly one top-level tab'
    }
    if ([Math]::Abs([double]$topTab.grid_width - [double]$topTab.window_width) -gt 1.0 -or
        [Math]::Abs([double]$topTab.grid_height - [double]$topTab.window_height) -gt 1.0) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'A new top-level tab inherited terminal dimensions instead of the current window viewport'
    }
    if ([string]$topTab.active_tab_profile -notmatch '(?i)powershell|pwsh') {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'A new top-level tab did not expose its PowerShell launch identity before shell output'
    }
    $topPanel = Get-ActiveAutomexiaPanel $topTab
    $topRect = @($topPanel.layout_rect)
    $expectedBottom = [double]$topTab.grid_height - [double]$topTab.grid_margin.bottom
    $actualBottom = [double]$topTab.grid_margin.top + [double]$topRect[1] + [double]$topRect[3]
    $configuredBottomInset = $expectedBottom - $actualBottom
    if ($topRect.Count -ne 4 -or $configuredBottomInset -lt -1.0 -or $configuredBottomInset -gt 32.0) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'The new-tab pane/footer boundary does not reach the current viewport bottom'
    }

    # Wait without input for the new shell too. This makes the following
    # history checks use the newly created session and catches blank first-frame
    # regressions independently of profile startup speed.
    while (($null -eq $topTab.latest_prompt_id -or
            [int]$topTab.latest_prompt_start_count -ne 1 -or
            -not [bool]$topTab.full_path_visible) -and
           [DateTime]::UtcNow -lt $topTabDeadline) {
        $topTab = Read-AutomexiaSnapshot -AfterSequence ([int64]$topTab.sequence)
    }
    if ($null -eq $topTab.latest_prompt_id -or
        [int]$topTab.latest_prompt_start_count -ne 1 -or
        -not [bool]$topTab.full_path_visible) {
        Write-Host ($topTab | ConvertTo-Json -Depth 8)
        throw 'The new Ctrl+T session did not publish its complete prompt automatically'
    }
    $initial = $topTab
    $initialPanel = Get-ActiveAutomexiaPanel $initial

    # Prove native PowerShell history navigation remains interactive after a
    # completed command. Unit tests cover the physical key mappings; raw bytes
    # here exercise the same ConPTY, PSReadLine, VT, and renderer path without
    # focus-sensitive synthetic keyboard automation.
    $historyToken = 'AMX_HISTORY_73491'
    $historyCommand = "Write-Output '$historyToken'"
    $historyControl = "write-line:history-seed:$historyCommand"
    $script:testStage = 'history seed command'
    Send-AutomexiaTestControl $historyControl
    $historyReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$initial.sequence)
    $controlDeadline = [DateTime]::UtcNow.AddSeconds(3)
    while ([string]$historyReady.last_control -ne $historyControl -and
           [DateTime]::UtcNow -lt $controlDeadline) {
        $historyReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    }
    if ([string]$historyReady.last_control -ne $historyControl) {
        Write-Host ($historyReady | ConvertTo-Json -Depth 8)
        throw 'The native driver did not observe the history seed control input'
    }
    $historyDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64]$historyReady.latest_prompt_id -le [int64]$initial.latest_prompt_id -and
           [DateTime]::UtcNow -lt $historyDeadline) {
        $historyReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    }
    if ([int64]$historyReady.latest_prompt_id -le [int64]$initial.latest_prompt_id) {
        Write-Host ($historyReady | ConvertTo-Json -Depth 8)
        throw 'PowerShell did not complete the history seed command'
    }

    $upTimer = [Diagnostics.Stopwatch]::StartNew()
    # ConPTY requests DEC private mode 9001, so native Windows keys are
    # transported as lossless KEY_EVENT_RECORD sequences rather than legacy
    # VT bytes. Up Arrow: VK_UP=38, scan=72, enhanced-key state=256.
    $upControl = 'write-hex:history-up:1b5b33383b37323b303b313b3235363b315f1b5b33383b37323b303b303b3235363b315f'
    $script:testStage = 'Up Arrow recall'
    Send-AutomexiaTestControl $upControl
    $upRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    $upDeadline = [DateTime]::UtcNow.AddSeconds(3)
    $upControlObservedMilliseconds = $null
    while (([string](Get-ActiveAutomexiaPanel $upRecall).cursor_line_text) -notlike "*$historyToken*" -and
           [DateTime]::UtcNow -lt $upDeadline) {
        if ($null -eq $upControlObservedMilliseconds -and
            [string]$upRecall.last_control -eq $upControl) {
            $upControlObservedMilliseconds = $upTimer.ElapsedMilliseconds
        }
        $upRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$upRecall.sequence)
    }
    $upTimer.Stop()
    if ($null -eq $upControlObservedMilliseconds -and
        [string]$upRecall.last_control -eq $upControl) {
        $upControlObservedMilliseconds = $upTimer.ElapsedMilliseconds
    }
    if ($null -eq $upControlObservedMilliseconds) {
        throw 'The native driver did not observe the Up Arrow control input'
    }
    $upShellMilliseconds = [Math]::Max(
        0, $upTimer.ElapsedMilliseconds - $upControlObservedMilliseconds)
    if (([string](Get-ActiveAutomexiaPanel $upRecall).cursor_line_text) -notlike "*$historyToken*") {
        Write-Host ($upRecall | ConvertTo-Json -Depth 8)
        throw 'Up Arrow did not recall the latest PowerShell command'
    }
    if ($upShellMilliseconds -gt 1500) {
        throw "Up Arrow shell/VT recall exceeded 1.5 seconds ($upShellMilliseconds ms; $($upTimer.ElapsedMilliseconds) ms including test-control delivery)"
    }

    # Cancel the recalled line, clear its old screen occurrence, then search by
    # a unique fragment. Seeing it afterward proves Ctrl+R produced a live
    # PSReadLine match instead of merely finding stale terminal output.
    $script:testStage = 'cancel recalled history'
    Send-AutomexiaTestControl 'write-hex:history-cancel-up:03'
    $cancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$upRecall.sequence)
    $cancelDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int64]$cancelled.latest_prompt_id -le [int64]$historyReady.latest_prompt_id -and
           [DateTime]::UtcNow -lt $cancelDeadline) {
        $cancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelled.sequence)
    }
    $script:testStage = 'clear history display'
    Send-AutomexiaTestControl 'write-line:history-clear:Clear-Host'
    $cleared = Read-AutomexiaSnapshot -AfterSequence ([int64]$cancelled.sequence)
    $clearDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([int64]$cleared.latest_prompt_id -le [int64]$cancelled.latest_prompt_id -and
           [DateTime]::UtcNow -lt $clearDeadline) {
        $cleared = Read-AutomexiaSnapshot -AfterSequence ([int64]$cleared.sequence)
    }

    # Ctrl+R: VK_R=82, scan=19, Unicode Ctrl+R=18, left-control state=8.
    $ctrlRRecord = [Text.Encoding]::ASCII.GetBytes("$([char]27)[82;19;18;1;8;1_")
    $searchPayload = [byte[]]($ctrlRRecord + [Text.Encoding]::UTF8.GetBytes($historyToken))
    $searchHex = -join ($searchPayload | ForEach-Object { $_.ToString('x2') })
    $searchTimer = [Diagnostics.Stopwatch]::StartNew()
    $searchControl = "write-hex:history-search:$searchHex"
    $script:testStage = 'Ctrl+R history search'
    Send-AutomexiaTestControl $searchControl
    $searchRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$cleared.sequence)
    $searchDeadline = [DateTime]::UtcNow.AddSeconds(3)
    $searchControlObservedMilliseconds = $null
    while (([string](Get-ActiveAutomexiaPanel $searchRecall).cursor_line_text) -notlike "*$historyToken*" -and
           [DateTime]::UtcNow -lt $searchDeadline) {
        if ($null -eq $searchControlObservedMilliseconds -and
            [string]$searchRecall.last_control -eq $searchControl) {
            $searchControlObservedMilliseconds = $searchTimer.ElapsedMilliseconds
        }
        $searchRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchRecall.sequence)
    }
    $searchTimer.Stop()
    if ($null -eq $searchControlObservedMilliseconds -and
        [string]$searchRecall.last_control -eq $searchControl) {
        $searchControlObservedMilliseconds = $searchTimer.ElapsedMilliseconds
    }
    if ($null -eq $searchControlObservedMilliseconds) {
        throw 'The native driver did not observe the Ctrl+R control input'
    }
    $searchShellMilliseconds = [Math]::Max(
        0, $searchTimer.ElapsedMilliseconds - $searchControlObservedMilliseconds)
    if (([string](Get-ActiveAutomexiaPanel $searchRecall).cursor_line_text) -notlike "*$historyToken*") {
        Write-Host ($searchRecall | ConvertTo-Json -Depth 8)
        throw 'Ctrl+R did not find the seeded PowerShell history command'
    }
    if ($searchShellMilliseconds -gt 1500) {
        throw "Ctrl+R shell/VT search exceeded 1.5 seconds ($searchShellMilliseconds ms; $($searchTimer.ElapsedMilliseconds) ms including test-control delivery)"
    }
    $script:testStage = 'cancel history search'
    Send-AutomexiaTestControl 'write-hex:history-cancel-search:03'
    $historyDone = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchRecall.sequence)

    # Create a tab inside the selected pane through the same implementation
    # path as Ctrl+Alt+T. It must own a new ConPTY/route while preserving the
    # selected PowerShell launch intent and must not add another split panel.
    $script:testStage = 'create pane-local tab'
    Send-AutomexiaTestControl 'local-tab:local-create'
    $localCreated = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    $localDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([int]$localCreated.panel_count -ne 1 -or
            [int](Get-ActiveAutomexiaPanel $localCreated).local_tab_count -ne 2 -or
            [int64](Get-ActiveAutomexiaPanel $localCreated).route_id -eq [int64]$initialPanel.route_id -or
            -not [bool]$localCreated.full_path_visible) -and
           [DateTime]::UtcNow -lt $localDeadline) {
        $localCreated = Read-AutomexiaSnapshot -AfterSequence ([int64]$localCreated.sequence)
    }
    $localPanel = Get-ActiveAutomexiaPanel $localCreated
    if ([int]$localCreated.panel_count -ne 1 -or [int]$localPanel.local_tab_count -ne 2) {
        Write-Host ($localCreated | ConvertTo-Json -Depth 10)
        throw 'Ctrl+Alt+T local-tab path did not create exactly one sibling in the selected pane'
    }
    $localRoutes = @($localPanel.local_tabs | ForEach-Object { [int64]$_.route_id } | Sort-Object -Unique)
    $localPids = @($localPanel.local_tabs | ForEach-Object { [int64]$_.shell_pid } | Sort-Object -Unique)
    if ($localRoutes.Count -ne 2 -or $localPids.Count -ne 2 -or $localPids[0] -le 0) {
        throw 'Pane-local PowerShell tabs reused a route or ConPTY process'
    }
    if ($localPanel.current_directory -ne $initialPanel.current_directory -or
        $localPanel.launch_program -ne $initialPanel.launch_program -or
        $localPanel.profile_identity -ne $initialPanel.profile_identity -or
        (($localPanel.launch_args | ConvertTo-Json -Compress) -ne
         ($initialPanel.launch_args | ConvertTo-Json -Compress))) {
        Write-Host ($localCreated | ConvertTo-Json -Depth 10)
        throw 'Pane-local tab did not preserve the selected PowerShell profile and directory'
    }

    # Return to the source and close the inactive sibling by index. This is the
    # native regression for the old cascade-close failure: the active source
    # route/PID and the window must survive unchanged.
    $script:testStage = 'select pane-local source'
    Send-AutomexiaTestControl 'select-local:local-source:0'
    $localSource = Read-AutomexiaSnapshot -AfterSequence ([int64]$localCreated.sequence)
    $localSourceDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $localSource).route_id -ne
           [int64]$initialPanel.route_id -and [DateTime]::UtcNow -lt $localSourceDeadline) {
        $localSource = Read-AutomexiaSnapshot -AfterSequence ([int64]$localSource.sequence)
    }
    $script:testStage = 'close inactive pane-local tab'
    Send-AutomexiaTestControl 'close-local:local-close-inactive:1'
    $localClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$localSource.sequence)
    $localCloseDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([int](Get-ActiveAutomexiaPanel $localClosed).local_tab_count -ne 1 -or
            [int64](Get-ActiveAutomexiaPanel $localClosed).route_id -ne [int64]$initialPanel.route_id) -and
           [DateTime]::UtcNow -lt $localCloseDeadline) {
        $localClosed = Read-AutomexiaSnapshot -AfterSequence ([int64]$localClosed.sequence)
    }
    $survivingLocalPanel = Get-ActiveAutomexiaPanel $localClosed
    if ([int]$localClosed.panel_count -ne 1 -or
        [int]$survivingLocalPanel.local_tab_count -ne 1 -or
        [int64]$survivingLocalPanel.route_id -ne [int64]$initialPanel.route_id -or
        [int64]$survivingLocalPanel.shell_pid -ne [int64]$initialPanel.shell_pid) {
        Write-Host ($localClosed | ConvertTo-Json -Depth 10)
        throw 'Closing an inactive pane-local tab changed or closed the active source session'
    }
    $historyDone = $localClosed

    # Deterministic binding tests prove Ctrl+Alt+R clones while bare Ctrl+R
    # remains shell-owned. This feature-gated,
    # renderer-neutral control invokes the same clone-right action path without
    # relying on focus-sensitive synthetic keyboard input.
    $script:testStage = 'clone split right'
    Send-AutomexiaTestControl 'clone-right:1'
    $rightClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    $cloneDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([int]$rightClone.panel_count -ne 2 -or
            $null -eq (Get-ActiveAutomexiaPanel $rightClone).shell_user -or
            -not [bool]$rightClone.full_path_visible) -and
           [DateTime]::UtcNow -lt $cloneDeadline) {
        $rightClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$rightClone.sequence)
    }
    $rightPanel = Get-ActiveAutomexiaPanel $rightClone
    if ([int]$rightClone.panel_count -ne 2 -or $null -eq $rightPanel) {
        Write-Host ($rightClone | ConvertTo-Json -Depth 8)
        throw 'The clone-right action did not create an independent right split'
    }
    if ([int64]$rightPanel.route_id -eq [int64]$initialPanel.route_id -or
        [int64]$rightPanel.shell_pid -eq [int64]$initialPanel.shell_pid -or
        [int64]$rightPanel.shell_pid -le 0) {
        throw 'The right clone reused its source route or ConPTY process'
    }
    if ($rightPanel.current_directory -ne $initialPanel.current_directory -or
        $rightPanel.launch_program -ne $initialPanel.launch_program -or
        $rightPanel.profile_identity -ne $initialPanel.profile_identity -or
        (($rightPanel.launch_args | ConvertTo-Json -Compress) -ne
         ($initialPanel.launch_args | ConvertTo-Json -Compress))) {
        Write-Host ($rightClone | ConvertTo-Json -Depth 8)
        throw 'The right clone did not preserve PowerShell/profile/current-directory launch intent'
    }

    # Input and visible history must remain isolated. Write a marker only to
    # the clone, then return to the source with the unchanged Shift+F6 split
    # navigation shortcut and prove the marker is absent there.
    $marker = 'AUTOMEXIA_CLONE_ONLY_73491'
    $script:testStage = 'write to cloned split'
    Send-AutomexiaTestControl "write-line:2:Write-Output $marker"
    $cloneOutput = Read-AutomexiaSnapshot -AfterSequence ([int64]$rightClone.sequence)
    $outputDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not ((Get-ActiveAutomexiaPanel $cloneOutput).visible_text -like "*$marker*") -and
           [DateTime]::UtcNow -lt $outputDeadline) {
        $cloneOutput = Read-AutomexiaSnapshot -AfterSequence ([int64]$cloneOutput.sequence)
    }
    if (-not ((Get-ActiveAutomexiaPanel $cloneOutput).visible_text -like "*$marker*")) {
        throw 'The cloned PowerShell PTY did not receive its independent input'
    }

    $script:testStage = 'return to source split'
    Send-AutomexiaTestControl 'select-prev:3'
    $sourceAgain = Read-AutomexiaSnapshot -AfterSequence ([int64]$cloneOutput.sequence)
    $sourceDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int64](Get-ActiveAutomexiaPanel $sourceAgain).route_id -ne
           [int64]$initialPanel.route_id -and [DateTime]::UtcNow -lt $sourceDeadline) {
        $sourceAgain = Read-AutomexiaSnapshot -AfterSequence ([int64]$sourceAgain.sequence)
    }
    $sourcePanel = Get-ActiveAutomexiaPanel $sourceAgain
    if ([int64]$sourcePanel.route_id -ne [int64]$initialPanel.route_id) {
        throw 'Shift+F6 did not return focus to the source split'
    }
    if ($sourcePanel.visible_text -like "*$marker*") {
        throw 'Clone-only terminal output contaminated the source session'
    }

    # Add a lower independent clone before the storm so layout, prompt, PTY,
    # and session isolation are exercised together under rapid resizing.
    $script:testStage = 'clone split down'
    Send-AutomexiaTestControl 'clone-down:4'
    $lowerClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$sourceAgain.sequence)
    $lowerDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (([int]$lowerClone.panel_count -ne 3 -or
            -not [bool]$lowerClone.full_path_visible -or
            -not (Test-AllAutomexiaPaneContexts $lowerClone)) -and
           [DateTime]::UtcNow -lt $lowerDeadline) {
        $lowerClone = Read-AutomexiaSnapshot -AfterSequence ([int64]$lowerClone.sequence)
    }
    if ([int]$lowerClone.panel_count -ne 3) {
        Write-Host ($lowerClone | ConvertTo-Json -Depth 8)
        throw 'The clone-down action did not create an independent lower split'
    }
    $routeIds = @($lowerClone.panels | ForEach-Object { [int64]$_.route_id } | Sort-Object -Unique)
    $processIds = @($lowerClone.panels | ForEach-Object { [int64]$_.shell_pid } | Sort-Object -Unique)
    if ($routeIds.Count -ne 3 -or $processIds.Count -ne 3 -or $processIds[0] -le 0) {
        throw 'Cloned panels do not have three independent routes and ConPTY processes'
    }
    if (-not (Test-AllAutomexiaPaneContexts $lowerClone)) {
        Write-Host ($lowerClone | ConvertTo-Json -Depth 8)
        throw 'Every visible pane must expose a route-matched operational context snapshot'
    }
    $sizes = @(
        @(320, 220),
        @(1920, 1080),
        @(420, 260),
        @(1600, 900),
        @(280, 200),
        @(1280, 720)
    )
    $script:testStage = 'resize storm'
    for ($iteration = 0; $iteration -lt 240; $iteration++) {
        $size = $sizes[$iteration % $sizes.Count]
        if (-not [AutomexiaResizeDriver]::MoveWindow(
                $window, 40, 40, $size[0], $size[1], $true)) {
            $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            throw "MoveWindow failed during iteration $iteration with Win32 error $code"
        }
        if ($iteration -eq 80) {
            # Create another clone while window-size events are still arriving;
            # this exercises descriptor launch, Taffy layout, ConPTY startup,
            # and prompt seeding in the middle of the storm.
            Send-AutomexiaTestControl 'clone-right:5'
            Start-Sleep -Milliseconds 150
        }
        if ($iteration -eq 160) {
            Send-AutomexiaTestControl 'select-prev:6'
        }
        Start-Sleep -Milliseconds 4
    }

    $storm = Read-AutomexiaSnapshot -AfterSequence ([int64]$lowerClone.sequence)
    $stormDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ([int]$storm.panel_count -ne 4 -and [DateTime]::UtcNow -lt $stormDeadline) {
        $storm = Read-AutomexiaSnapshot -AfterSequence ([int64]$storm.sequence)
    }

    if (-not [AutomexiaResizeDriver]::MoveWindow($window, 40, 40, 1400, 900, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Final MoveWindow failed with Win32 error $code"
    }
    $script:testStage = 'final restored size'
    $final = Read-AutomexiaSnapshot -AfterSequence ([int64]$storm.sequence)
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([double]$final.window_width -lt 1200 -or
            [double]$final.window_height -lt 700 -or
            $null -eq $final.latest_prompt_id -or
            [int]$final.latest_prompt_start_count -ne 1 -or
            -not [bool]$final.full_path_visible) -and
           [DateTime]::UtcNow -lt $deadline) {
        $final = Read-AutomexiaSnapshot -AfterSequence ([int64]$final.sequence)
    }
    $process.Refresh()

    if ($process.HasExited) { throw 'Automexia exited during the resize storm' }
    if ([int]$final.columns -lt 1 -or [int]$final.rows -lt 1) {
        throw "Invalid final grid dimensions: $($final.columns)x$($final.rows)"
    }
    if ([int]$final.cursor_column -lt 0 -or [int]$final.cursor_column -ge [int]$final.columns) {
        throw "Final cursor column $($final.cursor_column) is outside the grid"
    }
    if ([int]$final.cursor_row -lt 0 -or [int]$final.cursor_row -ge [int]$final.rows) {
        throw "Final cursor row $($final.cursor_row) is outside the grid"
    }
    if ([int]$final.latest_prompt_start_count -gt 1) {
        throw "The active prompt was duplicated $($final.latest_prompt_start_count) times"
    }
    if ($null -ne $final.active_prompt_gap_rows -and
        [int]$final.active_prompt_gap_rows -gt 1) {
        Write-Host ($final | ConvertTo-Json -Depth 4)
        throw "Resize left $($final.active_prompt_gap_rows) blank rows between completed output and the active prompt"
    }
    if ([bool]$final.prompt_active -and -not [bool]$final.full_path_visible) {
        Write-Host ($final | ConvertTo-Json -Depth 4)
        throw 'The complete active path did not return after restoring a usable window size'
    }
    if ([int]$final.panel_count -ne 4) {
        throw 'A cloned session disappeared during the resize storm'
    }
    $finalRoutes = @($final.panels | ForEach-Object { [int64]$_.route_id } | Sort-Object -Unique)
    $finalProcesses = @($final.panels | ForEach-Object { [int64]$_.shell_pid } | Sort-Object -Unique)
    if ($finalRoutes.Count -ne 4 -or $finalProcesses.Count -ne 4 -or $finalProcesses[0] -le 0) {
        throw 'Interleaved clone/resize operations lost route or PTY isolation'
    }
    if (-not (Test-AllAutomexiaPaneContexts $final)) {
        Write-Host ($final | ConvertTo-Json -Depth 8)
        throw 'A visible pane lost or inherited another route operational context during resize'
    }

    Write-Host (
        'Native resize/history stress passed: sequence {0}, grid {1}x{2}, prompt {3}, Up shell/VT {4}ms ({5}ms total), Ctrl+R shell/VT {6}ms ({7}ms total)' -f
        $final.sequence, $final.columns, $final.rows, $final.latest_prompt_id,
        $upShellMilliseconds, $upTimer.ElapsedMilliseconds,
        $searchShellMilliseconds, $searchTimer.ElapsedMilliseconds)
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(5000)) {
            Stop-Process -Id $process.Id -Force
        }
    }
    if ($null -eq $previousSnapshot) {
        Remove-Item Env:AUTOMEXIA_RESIZE_SNAPSHOT -ErrorAction SilentlyContinue
    } else {
        $env:AUTOMEXIA_RESIZE_SNAPSHOT = $previousSnapshot
    }
    if ($null -eq $previousControl) {
        Remove-Item Env:AUTOMEXIA_NATIVE_TEST_CONTROL -ErrorAction SilentlyContinue
    } else {
        $env:AUTOMEXIA_NATIVE_TEST_CONTROL = $previousControl
    }
    if ($null -eq $previousConfigHome) {
        Remove-Item Env:AUTOMEXIA_CONFIG_HOME -ErrorAction SilentlyContinue
    } else {
        $env:AUTOMEXIA_CONFIG_HOME = $previousConfigHome
    }
    Remove-Item -LiteralPath $snapshotPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $controlPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $configRoot -Recurse -Force -ErrorAction SilentlyContinue
}
