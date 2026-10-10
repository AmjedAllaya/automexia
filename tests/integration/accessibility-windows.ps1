# Invoked only by resize-stress-windows.ps1. Reuse its isolated configuration,
# owned process, native HWND, bounded control channel and teardown.
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

function Read-OwnedAccessibility {
    $nativeRoot = [Windows.Automation.AutomationElement]::FromHandle($window)
    $walker = [Windows.Automation.TreeWalker]::RawViewWalker
    $pending = [Collections.Generic.Queue[Windows.Automation.AutomationElement]]::new()
    $pending.Enqueue($nativeRoot)
    $nodes = [Collections.Generic.List[object]]::new()
    while ($pending.Count -gt 0 -and $nodes.Count -lt 1024) {
        $node = $pending.Dequeue()
        $current = $node.Current
        $text = ''
        $value = ''
        $pattern = $null
        if ($node.TryGetCurrentPattern([Windows.Automation.TextPattern]::Pattern, [ref]$pattern)) {
            $text = $pattern.DocumentRange.GetText(20000)
        }
        if ($node.TryGetCurrentPattern([Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
            $value = $pattern.Current.Value
            if ($value.Length -gt 8192) { throw 'UIA value exceeded the application bound' }
        }
        $invoke = $null
        [void]$node.TryGetCurrentPattern([Windows.Automation.InvokePattern]::Pattern, [ref]$invoke)
        $nodes.Add([pscustomobject]@{ Name = $current.Name; Type = $current.ControlType.ProgrammaticName;
            Focused = $current.HasKeyboardFocus; Bounds = $current.BoundingRectangle; Text = $text; Value = $value; Invoke = $invoke })
        $child = $walker.GetFirstChild($node)
        while ($null -ne $child) {
            if ($nodes.Count + $pending.Count -ge 1024) { throw 'UIA tree exceeded the test bound' }
            $pending.Enqueue($child)
            $child = $walker.GetNextSibling($child)
        }
    }
    return ,$nodes
}

function Wait-OwnedAccessibility {
    param([scriptblock]$Predicate, [string]$Failure)
    $until = [DateTime]::UtcNow.AddSeconds(12)
    do {
        $nodes = Read-OwnedAccessibility
        if (& $Predicate $nodes) { return ,$nodes }
        [void][AutomexiaResizeDriver]::PostMessage($window, 0x000F, [IntPtr]::Zero, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $until)
    # Names/text are intentionally absent from failure logs: even isolated
    # shells may publish a real local username or working directory.
    throw $Failure
}

$script:testStage = 'native UIA activation'
$null = Wait-OwnedAccessibility { param($nodes) @($nodes | Where-Object { $_.Name -eq 'Terminal output' }).Count -eq 1 } 'Native UIA terminal was not activated'

$script:testStage = 'native UIA caption controls'
$captions = Wait-OwnedAccessibility { param($nodes)
    @($nodes | Where-Object { $_.Name -in @('Minimize window', 'Maximize window', 'Close window') -and
        $_.Type -eq 'ControlType.Button' -and $null -ne $_.Invoke }).Count -eq 3
} 'Native UIA caption buttons lack their activation pattern'
foreach ($transition in @(@('Maximize window', 'Restore window'), @('Restore window', 'Maximize window'))) {
    $before = Wait-OwnedAccessibility { param($nodes)
        @($nodes | Where-Object { $_.Name -eq $transition[0] -and $null -ne $_.Invoke }).Count -eq 1
    } 'Native UIA caption action unavailable'
    @($before | Where-Object { $_.Name -eq $transition[0] })[0].Invoke.Invoke()
    $null = Wait-OwnedAccessibility { param($nodes)
        @($nodes | Where-Object { $_.Name -eq $transition[1] -and $null -ne $_.Invoke }).Count -eq 1 -and
        @($nodes | Where-Object { $_.Name -eq $transition[0] }).Count -eq 0
    } 'Native UIA maximize/restore did not change the native window'
}

# Fixed fixture text only; execute in the harness-owned disposable shell.
$fixture = '"AX_FIXTURE \u4e2d\u6587 e\u0301 \u0645\u0631\u062d\u0628\u0627 \u05e9\u05dc\u05d5\u05dd \ud83d\udc69\u200d\ud83d\udcbb"' | ConvertFrom-Json
$fixtureBytes = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($fixture))
# The command echo contains only encoded bytes; it cannot satisfy the output
# oracle. Select UTF-8 explicitly for the fixture's Console.WriteLine transport.
$command = '[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); [Console]::WriteLine([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String("' + $fixtureBytes + '")))'
Send-AutomexiaTestControl ('write-line:accessibility-unicode:' + $command)
$script:testStage = 'native UIA Unicode text ranges'
$unicode = Wait-OwnedAccessibility { param($nodes) @($nodes | Where-Object { $_.Text.Contains($fixture) }).Count -gt 0 } 'Native UIA text range lost Unicode fixture text'

# This injects a winit preedit event through the existing test adapter; it proves
# native rendering/projection, not delivery by a real Windows IME service.
$preedit = '"\u65e5\u672c e\u0301 \ud83d\udc69\u200d\ud83d\udcbb"' | ConvertFrom-Json
$encoded = [BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($preedit)).Replace('-', '')
Send-AutomexiaTestControl ('ime-preedit-hex:accessibility-composition:' + $encoded)
$null = Wait-OwnedAccessibility { param($nodes)
    @($nodes | Where-Object { $_.Name -eq 'Input method composition' -and $_.Value -eq $preedit }).Count -eq 1
} 'Native UIA composition did not retain complete Unicode text'
if (-not [string]::IsNullOrWhiteSpace($FrameCapture)) {
    $null = [AutomexiaResizeDriver]::CaptureClientFrame($window, [IO.Path]::GetFullPath($FrameCapture))
}
Send-AutomexiaTestControl 'ime-cancel:accessibility-composition'
$null = Wait-OwnedAccessibility { param($nodes)
    @($nodes | Where-Object { $_.Name -eq 'Input method composition' }).Count -eq 0
} 'Cancelled IME composition remained in the native tree'

foreach ($size in @(@(1280, 800), @(960, 700))) {
    [void][AutomexiaResizeDriver]::MoveWindow($window, 80, 80, $size[0], $size[1], $true)
    Send-AutomexiaTestControl ('open-palette:accessibility-' + $size[0])
    $script:testStage = 'native UIA modal isolation'
    $palette = Wait-OwnedAccessibility { param($nodes)
        @($nodes | Where-Object { $_.Name -eq 'Search commands' }).Count -eq 1 -and
        @($nodes | Where-Object { $_.Name -eq 'Terminal output' }).Count -eq 0
    } 'Native UIA palette missing, or covered terminal remained exposed'
    if (@($palette | Where-Object { $_.Name -eq 'Close window' }).Count -ne 0) {
        throw 'Native UIA modal exposes covered caption actions'
    }
    $options = @($palette | Where-Object { $_.Type -eq 'ControlType.ListItem' })
    if (@($palette | Where-Object { $_.Type -eq 'ControlType.List' }).Count -ne 1) {
        throw 'Native UIA palette options have no unique selection container'
    }
    if ($options.Count -eq 0) { throw 'Native UIA palette has no individual options' }
    foreach ($option in $options) {
        if ($option.Bounds.Width -le 0 -or $option.Bounds.Height -le 0) { throw 'Native UIA option has no painted bounds' }
    }
    Send-AutomexiaTestControl ('dismiss-modal:accessibility-' + $size[0])
    $null = Wait-OwnedAccessibility { param($nodes)
        @($nodes | Where-Object { $_.Name -eq 'Terminal output' }).Count -eq 1 -and
        @($nodes | Where-Object { $_.Name -eq 'Search commands' }).Count -eq 0
    } 'Native UIA did not restore terminal after modal close'
}

Send-AutomexiaTestControl 'open-customizations:accessibility-settings'
$script:testStage = 'native UIA settings controls'
$settingsNodes = Wait-OwnedAccessibility { param($nodes)
    @($nodes | Where-Object { $_.Name -eq 'Search settings' }).Count -eq 1 -and
    @($nodes | Where-Object { $_.Name -eq 'Terminal output' }).Count -eq 0
} 'Native UIA settings controls were not projected'
if (@($settingsNodes | Where-Object { $_.Type -eq 'ControlType.Button' }).Count -lt 3) {
    throw 'Native UIA settings were flattened into a summary'
}
Write-Host 'PASS: native Windows UIA activation, Unicode TextPattern, modal isolation, individual controls, resize and teardown ownership'
Write-Host 'Narrator/NVDA usability, native IME service input, AX and AT-SPI are separate evidence requirements.'
