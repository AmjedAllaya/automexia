param(
    [string]$Binary,
    [ValidateRange(100, 10000)]
    [int]$PowerShellHistoryBudgetMilliseconds = 1500,
    [string]$ResourceReport,
    [string]$FrameCapture,
    [ValidateRange(32, 4096)]
    [int64]$MaximumHandleGrowth = 384,
    [ValidateRange(8, 512)]
    [int64]$MaximumThreadGrowth = 48,
    [ValidateRange(67108864, 4294967296)]
    [int64]$MaximumPrivateBytesGrowth = 536870912,
    [ValidateRange(67108864, 4294967296)]
    [int64]$MaximumWorkingSetGrowth = 536870912,
    [ValidateRange(2, 128)]
    [int64]$MaximumDescendantProcessGrowth = 16
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrWhiteSpace($Binary)) {
    $Binary = Join-Path $root 'target\debug\automexia.exe'
}
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "Automexia test binary was not found at $Binary"
}

Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
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


    [StructLayout(LayoutKind.Sequential)]
    public struct Rect {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }



    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetClientRect(IntPtr hWnd, out Rect rect);

    [StructLayout(LayoutKind.Sequential)]
    private struct Point {
        public int X;
        public int Y;
    }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientToScreen(IntPtr hWnd, ref Point point);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr GetDC(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern int ReleaseDC(IntPtr hWnd, IntPtr hdc);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(
        IntPtr hWnd, IntPtr insertAfter, int x, int y, int width, int height, uint flags);

    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int index);

    [DllImport("gdi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool BitBlt(
        IntPtr destination, int x, int y, int width, int height,
        IntPtr source, int sourceX, int sourceY, uint operation);

    public sealed class FrameStats {
        public int Width;
        public int Height;
        public int SampleCount;
        public int DistinctColorBuckets;
        public int DominantColorBucket;
        public int LuminanceSpread;
    }

    public static FrameStats CaptureClientFrame(IntPtr hWnd, string outputPath) {
        Rect rect;
        if (!GetClientRect(hWnd, out rect)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        int width = rect.Right - rect.Left;
        int height = rect.Bottom - rect.Top;
        if (width < 1 || height < 1) {
            throw new InvalidOperationException("Automexia client frame has no drawable area");
        }

        Point origin = new Point { X = 0, Y = 0 };
        if (!ClientToScreen(hWnd, ref origin)) {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }

        using (var bitmap = new Bitmap(width, height, PixelFormat.Format32bppArgb)) {
            using (var graphics = Graphics.FromImage(bitmap)) {
                IntPtr destination = graphics.GetHdc();
                IntPtr screen = GetDC(IntPtr.Zero);
                if (screen == IntPtr.Zero) {
                    graphics.ReleaseHdc(destination);
                    throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
                }
                try {
                    const uint SourceCopy = 0x00CC0020;
                    const uint CaptureLayered = 0x40000000;
                    if (!BitBlt(
                        destination, 0, 0, width, height,
                        screen, origin.X, origin.Y, SourceCopy | CaptureLayered)) {
                        throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
                    }
                } finally {
                    ReleaseDC(IntPtr.Zero, screen);
                    graphics.ReleaseHdc(destination);
                }
            }

            var buckets = new HashSet<int>();
            var bucketCounts = new Dictionary<int, int>();
            int dominantColorBucket = -1;
            int dominantColorCount = 0;
            int minimumLuminance = 255;
            int maximumLuminance = 0;
            int samples = 0;
            for (int y = 0; y < height; y += 8) {
                for (int x = 0; x < width; x += 8) {
                    Color color = bitmap.GetPixel(x, y);
                    int bucket = ((color.R >> 4) << 8) | ((color.G >> 4) << 4) | (color.B >> 4);
                    buckets.Add(bucket);
                    int count;
                    bucketCounts.TryGetValue(bucket, out count);
                    count++;
                    bucketCounts[bucket] = count;
                    if (count > dominantColorCount) {
                        dominantColorCount = count;
                        dominantColorBucket = bucket;
                    }
                    int luminance = (color.R * 54 + color.G * 183 + color.B * 19) >> 8;
                    minimumLuminance = Math.Min(minimumLuminance, luminance);
                    maximumLuminance = Math.Max(maximumLuminance, luminance);
                    samples++;
                }
            }
            if (!String.IsNullOrWhiteSpace(outputPath)) {
                string directory = Path.GetDirectoryName(Path.GetFullPath(outputPath));
                if (!String.IsNullOrWhiteSpace(directory)) {
                    Directory.CreateDirectory(directory);
                }
                bitmap.Save(outputPath, ImageFormat.Png);
            }
            return new FrameStats {
                Width = width,
                Height = height,
                SampleCount = samples,
                DistinctColorBuckets = buckets.Count,
                DominantColorBucket = dominantColorBucket,
                LuminanceSpread = maximumLuminance - minimumLuminance,
            };
        }
    }

    public static bool SetCaptureTopmost(IntPtr hWnd, bool topmost) {
        IntPtr insertAfter = topmost ? new IntPtr(-1) : new IntPtr(-2);
        const uint NoMove = 0x0002;
        const uint NoSize = 0x0001;
        const uint NoActivate = 0x0010;
        const uint ShowWindow = 0x0040;
        return SetWindowPos(
            hWnd, insertAfter, 0, 0, 0, 0,
            NoMove | NoSize | NoActivate | ShowWindow);
    }

    public static int PrimaryWidth() {
        return GetSystemMetrics(0);
    }

    public static int PrimaryHeight() {
        return GetSystemMetrics(1);
    }



}
'@
Add-Type -Path (Join-Path $PSScriptRoot 'windows-native-window-locator.cs')

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

function Get-AutomexiaDescendantCount {
    param([int]$RootProcessId)

    $processes = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $frontier = @($RootProcessId)
    $seen = [Collections.Generic.HashSet[int]]::new()
    while ($frontier.Count -gt 0) {
        $parent = [int]$frontier[0]
        if ($frontier.Count -eq 1) {
            $frontier = @()
        } else {
            $frontier = @($frontier[1..($frontier.Count - 1)])
        }
        foreach ($child in @($processes | Where-Object { [int]$_.ParentProcessId -eq $parent })) {
            $childId = [int]$child.ProcessId
            if ($seen.Add($childId)) {
                $frontier += $childId
            }
        }
    }
    return $seen.Count
}

function Get-AutomexiaResourceSample {
    param([Diagnostics.Process]$AutomexiaProcess)

    $AutomexiaProcess.Refresh()
    if ($AutomexiaProcess.HasExited) {
        throw "Automexia exited while collecting resources during $script:testStage"
    }
    return [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        handle_count = [int64]$AutomexiaProcess.HandleCount
        thread_count = [int64]$AutomexiaProcess.Threads.Count
        private_bytes = [int64]$AutomexiaProcess.PrivateMemorySize64
        working_set_bytes = [int64]$AutomexiaProcess.WorkingSet64
        descendant_process_count = [int64](Get-AutomexiaDescendantCount $AutomexiaProcess.Id)
    }
}

function Wait-AutomexiaWindowCount {
    param(
        [int]$Expected,
        [int]$TimeoutMilliseconds = 15000
    )

    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        $process.Refresh()
        if ($process.HasExited) {
            throw "Automexia exited while waiting for $Expected visible windows during $script:testStage"
        }
        $windows = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
        if ($windows.Count -eq $Expected) {
            return $windows
        }
        Start-Sleep -Milliseconds 25
    } while ([DateTime]::UtcNow -lt $deadline)

    $windows = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
    $descriptions = @($windows | ForEach-Object { [AutomexiaNativeWindowLocator]::DescribeWindow($_) }) -join "; "
    throw "Expected $Expected visible Automexia windows during $script:testStage, found $($windows.Count): $descriptions"
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
                $snapshot = [IO.File]::ReadAllText(
                    $snapshotPath,
                    [Text.Encoding]::UTF8) | ConvertFrom-Json
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
        Write-Host ([IO.File]::ReadAllText($snapshotPath, [Text.Encoding]::UTF8))
    }
    throw "Timed out waiting for an Automexia renderer snapshot after sequence $AfterSequence during $script:testStage"
}

function Send-AutomexiaTestControl {
    param([string]$Control)
    $stagedControl = Join-Path (
        [System.IO.Path]::GetDirectoryName($controlPath)) (
        '.automexia-control-{0}.tmp' -f [guid]::NewGuid().ToString('N'))
    try {
        [System.IO.File]::WriteAllText(
            $stagedControl,
            $Control,
            [System.Text.UTF8Encoding]::new($false))
        [System.IO.File]::Delete($controlPath)
        [System.IO.File]::Move($stagedControl, $controlPath)
    } finally {
        [System.IO.File]::Delete($stagedControl)
    }
    if ($window -ne [IntPtr]::Zero) {
        # WM_PAINT is posted asynchronously. It wakes the feature-gated control
        # reader without adding a synchronous resize to the latency result.
        [void][AutomexiaResizeDriver]::PostMessage(
            $window, 0x000F, [IntPtr]::Zero, [IntPtr]::Zero)
    }
}

try {
    [void](New-Item -ItemType Directory -Path $configRoot)
    # Exercise the installed flat layout, not repository-only relative fallbacks.
    # CMD identity contains account-specific Base64 values generated by the
    # installer, so the native test must reproduce that exact deployed contract.
    $integrationRoot = Join-Path $configRoot 'shell-integration'
    [void](New-Item -ItemType Directory -Path $integrationRoot)
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\powershell\automexia.ps1') -Destination $integrationRoot
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\powershell\automexia.format.ps1xml') -Destination $integrationRoot
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\cmd\automexia-ls.cmd') -Destination $integrationRoot
    Copy-Item -LiteralPath (Join-Path $root 'shell-integration\cmd\automexia-ls.ps1') -Destination $integrationRoot

    $cmdSource = [IO.File]::ReadAllText(
        (Join-Path $root 'shell-integration\cmd\automexia.cmd'),
        [Text.Encoding]::UTF8)
    $cmdSource = $cmdSource.Replace(
        '__AUTOMEXIA_CMD_USER_BASE64__',
        [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([Environment]::UserName)))
    $cmdSource = $cmdSource.Replace(
        '__AUTOMEXIA_CMD_PATH_BASE64__',
        [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($env:ComSpec)))
    $cmdSource = [regex]::Replace($cmdSource, "\r?\n", [Environment]::NewLine)
    [IO.File]::WriteAllText(
        (Join-Path $integrationRoot 'automexia.cmd'),
        $cmdSource,
        [Text.Encoding]::ASCII)

    $cmdListingFixture = Join-Path $configRoot 'cmd-listing-fixture'
    [void](New-Item -ItemType Directory -Path $cmdListingFixture)
    [void](New-Item -ItemType Directory -Path (Join-Path $cmdListingFixture 'apps'))
    [IO.File]::WriteAllText(
        (Join-Path $cmdListingFixture 'Cargo.toml'),
        '[workspace]',
        [Text.Encoding]::ASCII)

    $integration = (Join-Path $integrationRoot 'automexia.ps1').Replace('\', '/')
    $config = @"
confirm-before-quit = false

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

    # Process.MainWindowHandle can transiently select Winit's internal event
    # target because it is created before Automexia's titled application HWND.
    # Enumerate process windows and exclude that infrastructure window so all
    # resize, input, frame, and close assertions target the real terminal.
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    $applicationWindows = @()
    do {
        Start-Sleep -Milliseconds 50
        $process.Refresh()
        if (-not $process.HasExited) {
            $applicationWindows = @(
                [AutomexiaNativeWindowLocator]::VisibleApplicationWindows($process.Id))
        }
    } while ($applicationWindows.Count -eq 0 -and
             -not $process.HasExited -and
             [DateTime]::UtcNow -lt $deadline)
    if ($process.HasExited) {
        throw "Automexia exited before its native window became ready (exit $($process.ExitCode))"
    }
    if ($applicationWindows.Count -ne 1) {
        throw "Automexia exposed $($applicationWindows.Count) application windows during startup; expected 1"
    }
    $window = $applicationWindows[0]

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
    $script:testStage = 'initial resource baseline'
    $resourceBaseline = Get-AutomexiaResourceSample $process

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
    # completed command. The recall path uses real window messages below; the
    # feature-gated controls only seed/cancel deterministically and publish
    # renderer-neutral snapshots without OCR.
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

    # Establish whether latency is in generic frontend -> PTY delivery or in a
    # PSReadLine history action. A printable key uses the same channel, ConPTY,
    # VT parser, damage, and renderer path as normal interactive typing.
    $typingTimer = [Diagnostics.Stopwatch]::StartNew()
    $typingControl = 'write-text:latency-printable:q'
    Send-AutomexiaTestControl $typingControl
    $typingSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    $typingDeadline = [DateTime]::UtcNow.AddSeconds(3)
    $typingControlObservedMilliseconds = $null
    while (([string](Get-ActiveAutomexiaPanel $typingSnapshot).cursor_line_text) -notlike '*q*' -and
           [DateTime]::UtcNow -lt $typingDeadline) {
        if ($null -eq $typingControlObservedMilliseconds -and
            [string]$typingSnapshot.last_control -eq $typingControl) {
            $typingControlObservedMilliseconds = $typingTimer.ElapsedMilliseconds
        }
        $typingSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$typingSnapshot.sequence)
    }
    $typingTimer.Stop()
    if ($null -eq $typingControlObservedMilliseconds -and
        [string]$typingSnapshot.last_control -eq $typingControl) {
        $typingControlObservedMilliseconds = $typingTimer.ElapsedMilliseconds
    }
    if ($null -eq $typingControlObservedMilliseconds) {
        throw 'The native driver did not observe the printable input control'
    }
    $typingShellMilliseconds = [Math]::Max(
        0, $typingTimer.ElapsedMilliseconds - $typingControlObservedMilliseconds)
    if (([string](Get-ActiveAutomexiaPanel $typingSnapshot).cursor_line_text) -notlike '*q*') {
        throw 'Printable input did not reach PowerShell and the renderer'
    }
    if ($typingShellMilliseconds -gt 500) {
        throw "PowerShell plain typed input exceeded 500 ms ($typingShellMilliseconds ms; $($typingTimer.ElapsedMilliseconds) ms total)"
    }
    $eraseControl = 'write-hex:latency-erase:1b5b383b31343b383b313b303b315f1b5b383b31343b303b303b303b315f'
    Send-AutomexiaTestControl $eraseControl
    $erased = Read-AutomexiaSnapshot -AfterSequence ([int64]$typingSnapshot.sequence)
    $eraseDeadline = [DateTime]::UtcNow.AddSeconds(3)
    while ((([string](Get-ActiveAutomexiaPanel $erased).cursor_line_text) -like '*q*' -or
            [string]$erased.last_control -ne $eraseControl) -and
           [DateTime]::UtcNow -lt $eraseDeadline) {
        $erased = Read-AutomexiaSnapshot -AfterSequence ([int64]$erased.sequence)
    }
    if (([string](Get-ActiveAutomexiaPanel $erased).cursor_line_text) -like '*q*') {
        throw 'Backspace did not clear the printable latency probe'
    }

    $upTimer = [Diagnostics.Stopwatch]::StartNew()
    # The Windows key encoder is covered by Rust unit tests. Inject its CSI Up
    # sequence through Automexia's input queue here so this headless native test
    # deterministically covers ConPTY, PSReadLine, VT, damage, and rendering
    # without depending on the desktop foreground-lock policy.
    $script:testStage = 'Up Arrow recall'
    Send-AutomexiaTestControl 'write-hex:history-up:1b5b41'
    $upRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyReady.sequence)
    $upDeadline = [DateTime]::UtcNow.AddSeconds(3)
    $upRawObservedMilliseconds = $null
    while (([string](Get-ActiveAutomexiaPanel $upRecall).cursor_line_text) -notlike "*$historyToken*" -and
           [DateTime]::UtcNow -lt $upDeadline) {
        if ($null -eq $upRawObservedMilliseconds -and
            [string](Get-ActiveAutomexiaPanel $upRecall).raw_cursor_line_text -like "*$historyToken*") {
            $upRawObservedMilliseconds = $upTimer.ElapsedMilliseconds
        }
        $upRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$upRecall.sequence)
    }
    if ($null -eq $upRawObservedMilliseconds -and
        [string](Get-ActiveAutomexiaPanel $upRecall).raw_cursor_line_text -like "*$historyToken*") {
        $upRawObservedMilliseconds = $upTimer.ElapsedMilliseconds
    }
    $upTimer.Stop()
    $upShellMilliseconds = $upTimer.ElapsedMilliseconds
    if (([string](Get-ActiveAutomexiaPanel $upRecall).cursor_line_text) -notlike "*$historyToken*") {
        Write-Host ($upRecall | ConvertTo-Json -Depth 8)
        throw 'Up Arrow did not recall the latest PowerShell command'
    }
    if ($upShellMilliseconds -gt $PowerShellHistoryBudgetMilliseconds) {
        throw "Up Arrow recall exceeded the $PowerShellHistoryBudgetMilliseconds ms Windows PowerShell budget ($upShellMilliseconds ms; raw terminal observed at $upRawObservedMilliseconds ms)"
    }
    # The control contains distinct down/up records back-to-back, matching a
    # physical tap without holding the key through the repeat interval.
    $upReleased = $upRecall

    # Cancel the recalled line, clear its old screen occurrence, then search by
    # a unique fragment. Seeing it afterward proves Ctrl+R produced a live
    # PSReadLine match instead of merely finding stale terminal output.
    $script:testStage = 'cancel recalled history'
    Send-AutomexiaTestControl 'write-hex:history-cancel-up:1b5b36373b34363b333b313b383b315f1b5b36373b34363b303b303b383b315f'
    $cancelled = Read-AutomexiaSnapshot -AfterSequence ([int64]$upReleased.sequence)
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
    # Send the physical key-down and the search text as separate controls. In
    # DECSET 9001 mode every typed character must remain a Win32 input record;
    # mixing raw UTF-8 into that stream is invalid and previously added a
    # misleading ConsoleHost timeout to this latency measurement.
    $searchTimer = [Diagnostics.Stopwatch]::StartNew()
    $ctrlRControl = 'write-hex:history-search-open:1b5b38323b31393b31383b313b383b315f1b5b38323b31393b303b303b383b315f'
    $script:testStage = 'Ctrl+R history search open'
    Send-AutomexiaTestControl $ctrlRControl
    $searchOpened = Read-AutomexiaSnapshot -AfterSequence ([int64]$cleared.sequence)
    $searchOpenDeadline = [DateTime]::UtcNow.AddSeconds(3)
    while ([string]$searchOpened.last_control -ne $ctrlRControl -and
           [DateTime]::UtcNow -lt $searchOpenDeadline) {
        $searchOpened = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchOpened.sequence)
    }
    if ([string]$searchOpened.last_control -ne $ctrlRControl) {
        throw 'The native driver did not observe the Ctrl+R key-down event'
    }
    $searchControl = "write-text:history-search-text:$historyToken"
    $script:testStage = 'Ctrl+R history search text'
    Send-AutomexiaTestControl $searchControl
    $searchRecall = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchOpened.sequence)
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
    if ($searchShellMilliseconds -gt $PowerShellHistoryBudgetMilliseconds) {
        throw "Ctrl+R search exceeded the $PowerShellHistoryBudgetMilliseconds ms Windows PowerShell budget ($searchShellMilliseconds ms; $($searchTimer.ElapsedMilliseconds) ms including test-control delivery)"
    }
    $script:testStage = 'cancel history search'
    $searchReleased = $searchRecall
    $cancelSearchControl = 'write-hex:history-cancel-search:1b5b36373b34363b333b313b383b315f1b5b36373b34363b303b303b383b315f'
    Send-AutomexiaTestControl $cancelSearchControl
    $historyDone = Read-AutomexiaSnapshot -AfterSequence ([int64]$searchReleased.sequence)
    $cancelSearchDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$historyDone.last_control -ne $cancelSearchControl -or
            [int64]$historyDone.latest_prompt_id -le [int64]$searchReleased.latest_prompt_id -or
            ([string](Get-ActiveAutomexiaPanel $historyDone).cursor_line_text) -like "*$historyToken*" -or
            ([string](Get-ActiveAutomexiaPanel $historyDone).visible_text) -like '*bck-i-search*') -and
           [DateTime]::UtcNow -lt $cancelSearchDeadline) {
        $historyDone = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    }
    if ([string]$historyDone.last_control -ne $cancelSearchControl -or
        [int64]$historyDone.latest_prompt_id -le [int64]$searchReleased.latest_prompt_id -or
        ([string](Get-ActiveAutomexiaPanel $historyDone).cursor_line_text) -like "*$historyToken*" -or
        ([string](Get-ActiveAutomexiaPanel $historyDone).visible_text) -like '*bck-i-search*') {
        Write-Host ($historyDone | ConvertTo-Json -Depth 10)
        throw 'Ctrl+R teardown did not restore a clean PowerShell prompt'
    }

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

    # Enter the real interactive CMD child through PowerShell's bare cmd alias.
    # It must remain inside this ConPTY and publish the same renderer metadata
    # without waiting for a second keypress.
    $script:testStage = 'interactive CMD startup parity'
    $cmdTypeControl = 'write-text:cmd-type:cmd'
    Send-AutomexiaTestControl $cmdTypeControl
    $cmdTyped = Read-AutomexiaSnapshot -AfterSequence ([int64]$historyDone.sequence)
    $cmdTypeDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((([string]$cmdTyped.last_control -ne $cmdTypeControl) -or
            -not ([string](Get-ActiveAutomexiaPanel $cmdTyped).cursor_line_text).Contains('cmd')) -and
           [DateTime]::UtcNow -lt $cmdTypeDeadline) {
        $cmdTyped = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdTyped.sequence)
    }
    if ([string]$cmdTyped.last_control -ne $cmdTypeControl -or
        -not ([string](Get-ActiveAutomexiaPanel $cmdTyped).cursor_line_text).Contains('cmd')) {
        Write-Host ($cmdTyped | ConvertTo-Json -Depth 10)
        throw 'Interactive PowerShell did not visibly accept the CMD command text'
    }

    $cmdEnterControl = 'write-line:cmd-submit:'
    Send-AutomexiaTestControl $cmdEnterControl
    $cmdReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdTyped.sequence)
    $cmdDeadline = [DateTime]::UtcNow.AddSeconds(20)
    while (([string]$cmdReady.last_control -ne $cmdEnterControl -or
            (Get-ActiveAutomexiaPanel $cmdReady).shell_name -ne 'CMD' -or
            -not [bool](Get-ActiveAutomexiaPanel $cmdReady).shell_integration -or
            -not [bool](Get-ActiveAutomexiaPanel $cmdReady).shell_prompt_active -or
            -not (@((Get-ActiveAutomexiaPanel $cmdReady).context_segments) -contains [Environment]::UserName) -or
            [int]@((Get-ActiveAutomexiaPanel $cmdReady).context_segments).Count -lt 3 -or
            -not [bool]$cmdReady.full_path_visible) -and
           [DateTime]::UtcNow -lt $cmdDeadline) {
        $cmdReady = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdReady.sequence)
    }
    $cmdPanel = Get-ActiveAutomexiaPanel $cmdReady
    if ([string]$cmdReady.last_control -ne $cmdEnterControl -or
        $cmdPanel.shell_name -ne 'CMD' -or
        -not [bool]$cmdPanel.shell_integration -or
        -not [bool]$cmdPanel.shell_prompt_active -or
        -not (@($cmdPanel.context_segments) -contains [Environment]::UserName) -or
        [int]@($cmdPanel.context_segments).Count -lt 3 -or
        -not [bool]$cmdReady.full_path_visible -or
        [string]::IsNullOrWhiteSpace([string]$cmdPanel.shell_user) -or
        [IO.Path]::GetFileName([string]$cmdPanel.shell_path) -ine 'cmd.exe' -or
        -not ([string]$cmdPanel.cursor_line_text).Contains([char]0x03BB)) {
        Write-Host ($cmdReady | ConvertTo-Json -Depth 10)
        throw 'Interactive CMD did not publish its shell, user, path, prompt, and complete working directory automatically'
    }

    # Prove the display-only DOSKEY helper is active in the real pane and keeps
    # category/file glyphs directly beside names.
    $folderGlyph = [char]::ConvertFromUtf32(0xF19F6)
    $rustGlyph = [char]0xE7A8
    $script:testStage = 'interactive CMD icon listing'
    Send-AutomexiaTestControl ('write-line:cmd-list:ls "{0}"' -f $cmdListingFixture)
    $cmdListing = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdReady.sequence)
    $cmdListingDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((-not ((Get-ActiveAutomexiaPanel $cmdListing).visible_text.Contains("$folderGlyph apps\")) -or
            -not ((Get-ActiveAutomexiaPanel $cmdListing).visible_text.Contains("$rustGlyph Cargo.toml")) -or
            -not [bool](Get-ActiveAutomexiaPanel $cmdListing).shell_prompt_active) -and
           [DateTime]::UtcNow -lt $cmdListingDeadline) {
        $cmdListing = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdListing.sequence)
    }
    $cmdListingPanel = Get-ActiveAutomexiaPanel $cmdListing
    if (-not $cmdListingPanel.visible_text.Contains("$folderGlyph apps\") -or
        -not $cmdListingPanel.visible_text.Contains("$rustGlyph Cargo.toml")) {
        Write-Host ($cmdListing | ConvertTo-Json -Depth 10)
        throw 'Interactive CMD ls did not render category and Rust icons immediately beside names'
    }

    # Exit must restore the parent metadata on PowerShell's very next prompt;
    # stale CMD identity is a failure even if another keystroke would repair it.
    $script:testStage = 'restore PowerShell after CMD exit'
    Send-AutomexiaTestControl 'write-line:cmd-exit:exit'
    $powerShellRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$cmdListing.sequence)
    $restoreDeadline = [DateTime]::UtcNow.AddSeconds(15)
    while (((Get-ActiveAutomexiaPanel $powerShellRestored).shell_name -ne 'PowerShell' -or
            -not [bool](Get-ActiveAutomexiaPanel $powerShellRestored).shell_prompt_active -or
            -not [bool]$powerShellRestored.full_path_visible) -and
           [DateTime]::UtcNow -lt $restoreDeadline) {
        $powerShellRestored = Read-AutomexiaSnapshot -AfterSequence ([int64]$powerShellRestored.sequence)
    }
    $restoredPanel = Get-ActiveAutomexiaPanel $powerShellRestored
    if ($restoredPanel.shell_name -ne 'PowerShell' -or
        -not [bool]$restoredPanel.shell_prompt_active -or
        -not [bool]$powerShellRestored.full_path_visible) {
        Write-Host ($powerShellRestored | ConvertTo-Json -Depth 10)
        throw 'PowerShell metadata did not replace CMD identity immediately after exit'
    }
    $historyDone = $powerShellRestored

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

    $finalWindowWidth = [Math]::Min(
        1400, [AutomexiaResizeDriver]::PrimaryWidth())
    $finalWindowHeight = [Math]::Min(
        900, [AutomexiaResizeDriver]::PrimaryHeight())
    if (-not [AutomexiaResizeDriver]::MoveWindow(
        $window, 0, 0, $finalWindowWidth, $finalWindowHeight, $true)) {
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
        Write-Host ($final | ConvertTo-Json -Depth 8)
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

    # Enter and leave the exact borderless-fullscreen path used by F11 and
    # Alt+Enter. The renderer's dominant composited color must not change, the
    # client must cover the display, and Windows must expose Automexia's scoped
    # DisplayRequired request only for the fullscreen lifetime.
    $script:testStage = 'fullscreen display brightness stability'
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for the windowed brightness sample (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $windowedBrightnessFrame = [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }

    $fullscreenControl = 'toggle-fullscreen:9050'
    Send-AutomexiaTestControl $fullscreenControl
    $fullscreenSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$final.sequence)
    $fullscreenDeadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        if ([string]$fullscreenSnapshot.last_control -ne $fullscreenControl -or
            -not [bool]$fullscreenSnapshot.fullscreen_display_request_active) {
            $fullscreenSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$fullscreenSnapshot.sequence)
        }
        $fullscreenBrightnessFrame = [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
        $fullscreenSettled = (
            [Math]::Abs($fullscreenBrightnessFrame.Width - [AutomexiaResizeDriver]::PrimaryWidth()) -le 2 -and
            [Math]::Abs($fullscreenBrightnessFrame.Height - [AutomexiaResizeDriver]::PrimaryHeight()) -le 2)
        $fullscreenReady = (
            $fullscreenSettled -and
            [string]$fullscreenSnapshot.last_control -eq $fullscreenControl -and
            [bool]$fullscreenSnapshot.fullscreen_display_request_active)
        if (-not $fullscreenReady) {
            Start-Sleep -Milliseconds 50
        }
    } while (-not $fullscreenReady -and [DateTime]::UtcNow -lt $fullscreenDeadline)
    if (-not $fullscreenSettled -or
        [string]$fullscreenSnapshot.last_control -ne $fullscreenControl -or
        -not [bool]$fullscreenSnapshot.fullscreen_display_request_active) {
        throw "Fullscreen did not settle with an active DisplayRequired request at the display bounds: $($fullscreenBrightnessFrame.Width)x$($fullscreenBrightnessFrame.Height)"
    }

    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for the fullscreen brightness sample (Win32 error $code)"
    }
    try {
        Start-Sleep -Milliseconds 100
        $fullscreenBrightnessFrame = [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
    }
    if ($windowedBrightnessFrame.DominantColorBucket -lt 0 -or
        $fullscreenBrightnessFrame.DominantColorBucket -ne $windowedBrightnessFrame.DominantColorBucket) {
        throw "Fullscreen changed the rendered dominant color bucket from $($windowedBrightnessFrame.DominantColorBucket) to $($fullscreenBrightnessFrame.DominantColorBucket)"
    }

    $restoreControl = 'toggle-fullscreen:9051'
    Send-AutomexiaTestControl $restoreControl
    $restoredSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$fullscreenSnapshot.sequence)
    $restoreDeadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        if ([string]$restoredSnapshot.last_control -ne $restoreControl -or
            [bool]$restoredSnapshot.fullscreen_display_request_active) {
            $restoredSnapshot = Read-AutomexiaSnapshot -AfterSequence ([int64]$restoredSnapshot.sequence)
        }
        $restoredBrightnessFrame = [AutomexiaResizeDriver]::CaptureClientFrame($window, $null)
        $restoredSettled = (
            [Math]::Abs($restoredBrightnessFrame.Width - $windowedBrightnessFrame.Width) -le 2 -and
            [Math]::Abs($restoredBrightnessFrame.Height - $windowedBrightnessFrame.Height) -le 2)
        $restoreReady = (
            $restoredSettled -and
            [string]$restoredSnapshot.last_control -eq $restoreControl -and
            -not [bool]$restoredSnapshot.fullscreen_display_request_active)
        if (-not $restoreReady) {
            Start-Sleep -Milliseconds 50
        }
    } while (-not $restoreReady -and [DateTime]::UtcNow -lt $restoreDeadline)
    if (-not $restoredSettled -or
        [string]$restoredSnapshot.last_control -ne $restoreControl -or
        [bool]$restoredSnapshot.fullscreen_display_request_active) {
        throw "Fullscreen exit did not restore the windowed bounds and release DisplayRequired: $($restoredBrightnessFrame.Width)x$($restoredBrightnessFrame.Height)"
    }
    if ($restoredBrightnessFrame.DominantColorBucket -ne $windowedBrightnessFrame.DominantColorBucket) {
        throw "Fullscreen exit did not restore the rendered dominant color bucket: $($restoredBrightnessFrame.DominantColorBucket)"
    }

    $requestReleased = -not [bool]$restoredSnapshot.fullscreen_display_request_active
    # Exercise the complete native quick-look path: feature-gated control,
    # bounded background decode, route-scoped overlay upload, split-relative
    # placement, renderer snapshot, and composited frame. The path is a
    # repository-owned public brand asset and contains no user content.
    $script:testStage = 'native image quick look'
    $previewAsset = Join-Path $root 'assets\brand\png\automexia-terminal-128.png'
    $previewControl = "preview-image:9100:$previewAsset"
    Send-AutomexiaTestControl $previewControl
    $preview = Read-AutomexiaSnapshot -AfterSequence ([int64]$final.sequence)
    $previewDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (([string]$preview.last_control -ne $previewControl -or
            -not [bool]$preview.image_preview.visible -or
            -not [bool]$preview.image_preview.overlay_present) -and
           [DateTime]::UtcNow -lt $previewDeadline) {
        $preview = Read-AutomexiaSnapshot -AfterSequence ([int64]$preview.sequence)
    }
    if (-not [bool]$preview.image_preview.visible -or
        -not [bool]$preview.image_preview.overlay_present -or
        [int]$preview.image_preview.decoded_dimensions[0] -ne 128 -or
        [int]$preview.image_preview.decoded_dimensions[1] -ne 128) {
        Write-Host ($preview | ConvertTo-Json -Depth 8)
        throw 'Native image quick look did not publish its decoded route-scoped GPU overlay'
    }

    $framePath = if ([string]::IsNullOrWhiteSpace($FrameCapture)) {
        $null
    } else {
        [IO.Path]::GetFullPath($FrameCapture)
    }
    # The renderer snapshot is published before the GPU presentation is
    # necessarily observable in the desktop compositor. Keep the visual
    # threshold strict, but allow one bounded presentation-settle window
    # instead of treating a transient single-color capture as the final frame.
    $script:testStage = 'final painted frame settle'
    $frameDeadline = [DateTime]::UtcNow.AddSeconds(5)
    $frameStopwatch = [Diagnostics.Stopwatch]::StartNew()
    $frameAttempts = 0
    if (-not [AutomexiaResizeDriver]::SetCaptureTopmost($window, $true)) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "Could not expose Automexia for composited frame capture (Win32 error $code)"
    }
    try {
        do {
            $frameAttempts++
            $frameStats = [AutomexiaResizeDriver]::CaptureClientFrame(
                $window, $framePath)
            $frameIsValid = (
                $frameStats.Width -ge 100 -and
                $frameStats.Height -ge 100 -and
                $frameStats.SampleCount -ge 100 -and
                $frameStats.DistinctColorBuckets -ge 8 -and
                $frameStats.LuminanceSpread -ge 32)
            if (-not $frameIsValid) {
                Start-Sleep -Milliseconds 100
            }
        } while (-not $frameIsValid -and [DateTime]::UtcNow -lt $frameDeadline)
    } finally {
        [void][AutomexiaResizeDriver]::SetCaptureTopmost($window, $false)
        $frameStopwatch.Stop()
    }
    if (-not $frameIsValid) {
        throw "Final painted client frame did not settle within 5 seconds after $frameAttempts attempts: $($frameStats.Width)x$($frameStats.Height), samples=$($frameStats.SampleCount), buckets=$($frameStats.DistinctColorBuckets), luminance-spread=$($frameStats.LuminanceSpread)"
    }

    $script:testStage = 'dismiss native image quick look'
    $dismissControl = 'dismiss-preview:9101'
    Send-AutomexiaTestControl $dismissControl
    $dismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$preview.sequence)
    $dismissDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (([string]$dismissed.last_control -ne $dismissControl -or
            [bool]$dismissed.image_preview.visible -or
            [bool]$dismissed.image_preview.overlay_present) -and
           [DateTime]::UtcNow -lt $dismissDeadline) {
        $dismissed = Read-AutomexiaSnapshot -AfterSequence ([int64]$dismissed.sequence)
    }
    if ([bool]$dismissed.image_preview.visible -or
        [bool]$dismissed.image_preview.overlay_present) {
        throw 'Native image quick look did not remove its GPU overlay after dismissal'
    }

    # Custom-chrome hit geometry is covered deterministically in Rust across
    # physical DPI scales. Exercise the native OS teardown route here without
    # relying on foreground-locked desktop pointer injection.
    $script:testStage = 'native close isolates Ctrl+Shift+N window'
    Send-AutomexiaTestControl 'new-window:9001'
    $windows = Wait-AutomexiaWindowCount -Expected 2
    $newWindow = @($windows | Where-Object { $_ -ne $window })[0]
    if (-not [AutomexiaResizeDriver]::PostMessage(
        $newWindow, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw 'Could not post WM_CLOSE to the secondary Automexia window'
    }
    $remaining = Wait-AutomexiaWindowCount -Expected 1
    $process.Refresh()
    if ($process.HasExited -or $remaining[0] -ne $window) {
        throw 'Native window close terminated or replaced the original Automexia window'
    }

    $script:testStage = 'post-storm resource ceiling'
    Start-Sleep -Milliseconds 500
    $resourceFinal = Get-AutomexiaResourceSample $process
    $resourceLimits = [ordered]@{
        handle_growth = $MaximumHandleGrowth
        thread_growth = $MaximumThreadGrowth
        private_bytes_growth = $MaximumPrivateBytesGrowth
        working_set_growth = $MaximumWorkingSetGrowth
        descendant_process_growth = $MaximumDescendantProcessGrowth
    }
    $resourceDelta = [ordered]@{
        handle_growth = $resourceFinal.handle_count - $resourceBaseline.handle_count
        thread_growth = $resourceFinal.thread_count - $resourceBaseline.thread_count
        private_bytes_growth = $resourceFinal.private_bytes - $resourceBaseline.private_bytes
        working_set_growth = $resourceFinal.working_set_bytes - $resourceBaseline.working_set_bytes
        descendant_process_growth = $resourceFinal.descendant_process_count - $resourceBaseline.descendant_process_count
    }
    foreach ($name in $resourceLimits.Keys) {
        if ([int64]$resourceDelta[$name] -gt [int64]$resourceLimits[$name]) {
            throw "Native resource ceiling exceeded for $name`: $($resourceDelta[$name]) > $($resourceLimits[$name])"
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
        $reportPath = [IO.Path]::GetFullPath($ResourceReport)
        $reportDirectory = [IO.Path]::GetDirectoryName($reportPath)
        if (-not [string]::IsNullOrWhiteSpace($reportDirectory)) {
            New-Item -ItemType Directory -Force -Path $reportDirectory | Out-Null
        }
        $report = [ordered]@{
            schema_version = 1
            panel_count_at_final_sample = [int]$final.panel_count
            baseline = $resourceBaseline
            final = $resourceFinal
            delta = $resourceDelta
            ceilings = $resourceLimits
            fullscreen_brightness = [ordered]@{
                windowed_size = @($windowedBrightnessFrame.Width, $windowedBrightnessFrame.Height)
                fullscreen_size = @($fullscreenBrightnessFrame.Width, $fullscreenBrightnessFrame.Height)
                restored_size = @($restoredBrightnessFrame.Width, $restoredBrightnessFrame.Height)
                dominant_color_bucket = $windowedBrightnessFrame.DominantColorBucket
                display_request_released = $requestReleased
            }
            painted_frame = [ordered]@{
                width = $frameStats.Width
                height = $frameStats.Height
                sample_count = $frameStats.SampleCount
                distinct_color_buckets = $frameStats.DistinctColorBuckets
                dominant_color_bucket = $frameStats.DominantColorBucket
                luminance_spread = $frameStats.LuminanceSpread
                attempts = $frameAttempts
                settle_milliseconds = $frameStopwatch.ElapsedMilliseconds
                artifact = if ($null -eq $framePath) { $null } else { [IO.Path]::GetFileName($framePath) }
            }
        } | ConvertTo-Json -Depth 5
        $temporaryReport = "$reportPath.$PID.tmp"
        [IO.File]::WriteAllText($temporaryReport, $report, [Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $temporaryReport -Destination $reportPath -Force
    }

    Write-Host (
        'Native CMD/resize/history/fullscreen/multi-window stress passed: sequence {0}, grid {1}x{2}, prompt {3}, Up shell/VT {4}ms ({5}ms total), Ctrl+R shell/VT {6}ms ({7}ms total)' -f
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
