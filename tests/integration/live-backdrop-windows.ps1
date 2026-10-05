# Runs inside the existing disposable configuration/native-window harness.
# Exact exterior comparison is independent of production theme/paint helpers.
function Test-AutomexiaLiveBackdrop {
    Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
public static class AutomexiaBackdropPixels {
    public static long Difference(string before, string after) {
        return Difference(before, after, false);
    }
    public static long Difference(string before, string after, bool insideCard) {
        using (var a = new Bitmap(before))
        using (var b = new Bitmap(after)) {
            if (a.Width != b.Width || a.Height != b.Height)
                throw new InvalidOperationException("Frame dimensions changed");
            long differences = 0;
            // Exclude chrome, cursor and local card shadows. The two exterior
            // strips include real terminal glyphs, not just empty background.
            for (int y = 150; y < Math.Min(400, a.Height - 100); y++) {
                for (int x = 16; x < 64; x++) {
                    int left = insideCard ? a.Width / 2 - x : x;
                    if (a.GetPixel(left, y) != b.GetPixel(left, y)) differences++;
                    int right = insideCard ? a.Width / 2 + x : a.Width - x - 1;
                    if (a.GetPixel(right, y) != b.GetPixel(right, y)) differences++;
                }
            }
            return differences;
        }
    }
}
'@
    function Wait-Backdrop([scriptblock]$Predicate) {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $state = Read-AutomexiaSnapshot
            if (& $Predicate $state) { return $state }
            Start-Sleep -Milliseconds 20
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Live backdrop did not settle: $script:testStage"
    }
    function Backdrop-Key([int]$Key) {
        if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($window, $Key, $false, $false, $false)) {
            throw 'Live backdrop keyboard input failed'
        }
    }
    $captureRoot = if ([string]::IsNullOrWhiteSpace($ModalCaptureDirectory)) {
        $configRoot
    } else { [IO.Path]::GetFullPath($ModalCaptureDirectory) }
    $null = New-Item -ItemType Directory -Path $captureRoot -Force
    function Capture-Backdrop([string]$Name) {
        $path = Join-Path $captureRoot ($Name + '.png')
        $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, $path)
        return $path
    }
    function Assert-Backdrop([string]$Before, [string]$After) {
        $difference = [AutomexiaBackdropPixels]::Difference($Before, $After)
        if ($difference -ne 0) { throw "Modal changed $difference exterior terminal pixels during $script:testStage" }
    }
    [void][AutomexiaResizeDriver]::MoveWindow($window, 40, 40, 1500, 850, $true)
    $null = Wait-Backdrop { param($s) $s.window_width -ge 1400 }
    $prior = Read-AutomexiaSnapshot
    Send-AutomexiaTestControl ("write-line:backdrop-fixture:1..12 | ForEach-Object { Write-Output ('VISIBLE TERMINAL ' * 12) }")
    $null = Wait-Backdrop { param($s) $null -ne $s.command_result_exit_code -and $s.command_result_key -ne $prior.command_result_key }
    $baseline = Capture-Backdrop 'live-terminal-before'
    $baselinePrompt = (Get-ActiveAutomexiaPanel (Read-AutomexiaSnapshot)).raw_cursor_line_text
    $samples = [Collections.Generic.List[double]]::new()
    $beforeResources = Get-AutomexiaResourceSample $process
    for ($cycle = 0; $cycle -lt 12; $cycle++) {
        $script:testStage = 'customization exterior cycle ' + $cycle
        $timer = [Diagnostics.Stopwatch]::StartNew()
        Send-AutomexiaTestControl ('open-customizations:backdrop-' + $cycle)
        $null = Wait-Backdrop { param($s) $s.settings.ready -and $s.settings.open }
        $timer.Stop()
        $samples.Add($timer.Elapsed.TotalMilliseconds)
        if ($cycle -eq 0) {
            Assert-Backdrop $baseline (Capture-Backdrop 'live-customizations')
            # Search input is modal even though the terminal is visible.
            Backdrop-Key 0x54
            $typed = Wait-Backdrop { param($s) $s.settings.search_bytes -eq 1 }
            if ((Get-ActiveAutomexiaPanel $typed).raw_cursor_line_text -ne $baselinePrompt) {
                throw 'Customization search typed into the visible terminal'
            }
        }
        Backdrop-Key 0x1B
        $null = Wait-Backdrop { param($s) -not $s.settings.open }
    }
    foreach ($action in @('open-palette', 'open-connection-hub', 'confirm-quit')) {
        $script:testStage = $action + ' exterior'
        Send-AutomexiaTestControl ($action + ':backdrop')
        $null = Wait-Backdrop { param($s) $s.last_control -eq ($action + ':backdrop') }
        Assert-Backdrop $baseline (Capture-Backdrop ('live-' + $action))
        Backdrop-Key 0x1B
        $null = Wait-Backdrop { param($s) -not $s.palette_enabled -and -not $s.confirm_quit_active -and -not $s.connection_hub_active }
    }
    $script:testStage = 'instant theme preview exterior'
    Send-AutomexiaTestControl 'open-themes:backdrop'
    $null = Wait-Backdrop { param($s) $s.settings.ready -and $null -ne $s.settings.gallery -and -not $s.settings.gallery.busy }
    Assert-Backdrop $baseline (Capture-Backdrop 'live-gallery-before')
    Backdrop-Key 0x28
    $null = Wait-Backdrop { param($s) $s.settings.gallery.selected -gt 0 }
    $preview = Capture-Backdrop 'live-gallery-preview'
    if ([AutomexiaBackdropPixels]::Difference($baseline, $preview) -lt 1000) {
        throw 'Theme preview did not repaint the visible terminal'
    }
    Backdrop-Key 0x1B
    Backdrop-Key 0x1B
    $null = Wait-Backdrop { param($s) -not $s.settings.open }
    Assert-Backdrop $baseline (Capture-Backdrop 'live-gallery-cancelled')

    $script:testStage = 'PTY output continues behind open customization'
    $prior = Read-AutomexiaSnapshot
    Send-AutomexiaTestControl ("write-line:backdrop-live:Start-Sleep -Seconds 3; 1..12 | ForEach-Object { Write-Output ('UPDATED TERMINAL ' * 12) }")
    $null = Wait-Backdrop { param($s) $s.last_control -eq 'write-line:backdrop-live:Start-Sleep -Seconds 3; 1..12 | ForEach-Object { Write-Output (''UPDATED TERMINAL '' * 12) }' }
    Send-AutomexiaTestControl 'open-customizations:backdrop-live'
    $null = Wait-Backdrop { param($s) $s.settings.ready -and $s.settings.open }
    $waiting = Capture-Backdrop 'live-output-before'
    $null = Wait-Backdrop { param($s) $s.settings.open -and $null -ne $s.command_result_exit_code -and $s.command_result_key -ne $prior.command_result_key }
    $updated = Capture-Backdrop 'live-output-after'
    if ([AutomexiaBackdropPixels]::Difference($waiting, $updated) -lt 100) {
        throw 'PTY output did not remain live behind customization'
    }
    if ([AutomexiaBackdropPixels]::Difference($waiting, $updated, $true) -ne 0) {
        throw 'Terminal updates changed pixels inside the opaque customization panel'
    }
    Backdrop-Key 0x1B
    $null = Wait-Backdrop { param($s) -not $s.settings.open }
    Assert-Backdrop $updated (Capture-Backdrop 'live-output-dismissed')
    $afterResources = Get-AutomexiaResourceSample $process
    $sorted = @($samples | Sort-Object)
    if ($sorted[11] -gt 2500) { throw 'Repeated overlay opening exceeded the native responsiveness budget' }
    foreach ($bound in @(
        @('handle_count', $MaximumHandleGrowth),
        @('thread_count', $MaximumThreadGrowth),
        @('private_bytes', $MaximumPrivateBytesGrowth),
        @('working_set_bytes', $MaximumWorkingSetGrowth),
        @('descendant_process_count', $MaximumDescendantProcessGrowth)
    )) {
        if ($afterResources[$bound[0]] - $beforeResources[$bound[0]] -gt $bound[1]) {
            throw ('Repeated overlay use exceeded the existing resource bound: ' + $bound[0])
        }
    }
    $report = [ordered]@{
        schema = 1; renderer = $expectedRendererBackend; cycles = $samples.Count
        # This includes test-control polling, not input-to-present latency.
        open_to_observed_state_ms = @{ p50 = $sorted[5]; p95 = $sorted[11] }
        before = $beforeResources; after = $afterResources
        exterior_pixels_unchanged = $true; output_live = $true; preview_live = $true
    }
    if (-not [string]::IsNullOrWhiteSpace($ResourceReport)) {
        [IO.File]::WriteAllText([IO.Path]::GetFullPath($ResourceReport), ($report | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
    }
    Write-Host ('PASS: opaque overlays retain exact live terminal exterior, modal input and live theme/output; open p50={0:N1}ms p95={1:N1}ms ({2})' -f $sorted[5],$sorted[11],$expectedRendererBackend)
}
