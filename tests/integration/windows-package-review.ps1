# Diagnostic only. This file deliberately refuses to launch on a contributor PC.
param(
    [Parameter(Mandatory)][string]$RepositoryRoot,
    [ValidateSet('Restricted', 'RemoteSigned')][string]$Policy = 'Restricted',
    [ValidateSet('zip', 'msi')][string]$PackageKind = 'zip',
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
    [switch]$InternetMark,
    [switch]$StressOutputHighlighting,
    [string]$PackagePath,
    [ValidatePattern('^[0-9a-f]{64}$')][string]$PackageSha256,
    [ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+-alpha$')][string]$CandidateVersion,
    [ValidatePattern('^[0-9a-f]{40}$')][string]$CandidateCommit = $env:GITHUB_SHA
)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'This diagnostic may run only on a disposable GitHub-hosted Windows runner.'
}
if ($InternetMark -and $Policy -ne 'RemoteSigned') { throw 'Internet-mark comparison requires RemoteSigned.' }
if ($InternetMark -and $PackageKind -ne 'zip') { throw 'Internet-mark comparison targets extracted ZIP scripts.' }
if ($env:RUNNER_ARCH -ine $Architecture) { throw 'A native runner is required for the selected package architecture.' }
if ($PackagePath -and (-not $PackageSha256 -or -not $CandidateVersion)) { throw 'Candidate packages require an exact digest and version.' }
$root = [IO.Path]::GetFullPath($RepositoryRoot)
$work = Join-Path $env:RUNNER_TEMP ('automexia-package-review-' + [guid]::NewGuid().ToString('N'))
$reportRoot = Join-Path $root 'artifacts/windows-package-review'
New-Item -ItemType Directory -Path $work, $reportRoot -Force | Out-Null
$report = [ordered]@{
    schema = 1
    tag = $(if ($PackagePath) { 'v' + $CandidateVersion } else { '0.0.1_alpha' })
    source_commit = $(if ($PackagePath) { $CandidateCommit } else { '7b099a6be450f1cd35910e3a454da45f01d45e12' })
    architecture = $Architecture
    package = $PackageKind
    policy = $Policy
    internet_mark = [bool]$InternetMark
    stress_output_highlighting = [bool]$StressOutputHighlighting
    environment = $(if ($Architecture -eq 'arm64') { 'fresh Windows 11 ARM64 hosted VM' } else { 'fresh Windows Server hosted VM; client execution policy is an explicit test condition' })
    controls = @()
    complete = $false
}
$process = $null
$window = [IntPtr]::Zero
$stage = 'package'
$msiInstalled = $false
function Invoke-Installer([string]$Operation, [string]$Path) {
    $installer = Start-Process -FilePath "$env:SystemRoot\System32\msiexec.exe" -WindowStyle Hidden -PassThru -ArgumentList @($Operation, ('"' + $Path + '"'), '/qn', '/norestart')
    if (-not $installer.WaitForExit(120000)) { $installer.Kill(); throw 'Installer timed out in the disposable VM' }
    return $installer.ExitCode
}
function Read-Nodes {
    $native = [Windows.Automation.AutomationElement]::FromHandle($script:window)
    $walker = [Windows.Automation.TreeWalker]::RawViewWalker
    $pending = [Collections.Generic.Queue[Windows.Automation.AutomationElement]]::new()
    $pending.Enqueue($native)
    $nodes = [Collections.Generic.List[object]]::new()
    while ($pending.Count -gt 0) {
        if ($nodes.Count -ge 1024) { throw 'UIA node limit exceeded' }
        $node = $pending.Dequeue()
        $current = $node.Current
        $toggle = $null
        $state = $null
        $textPattern = $null
        $visibleText = ''
        $valuePattern = $null
        $value = ''
        if ($node.TryGetCurrentPattern([Windows.Automation.ValuePattern]::Pattern, [ref]$valuePattern)) {
            $value = $valuePattern.Current.Value
        }
        if ($node.TryGetCurrentPattern([Windows.Automation.TextPattern]::Pattern, [ref]$textPattern)) {
            $visibleText = $textPattern.DocumentRange.GetText(16384)
        }
        if ($node.TryGetCurrentPattern([Windows.Automation.TogglePattern]::Pattern, [ref]$toggle)) {
            $state = [string]$toggle.Current.ToggleState
        }
        $nodes.Add([pscustomobject]@{
            Name = $current.Name; Id = $current.AutomationId; Bounds = $current.BoundingRectangle
            Type = $current.ControlType.ProgrammaticName; Toggle = $state; Enabled = $current.IsEnabled
            Focused = $current.HasKeyboardFocus; Text = $visibleText; Value = $value
        })
        $child = $walker.GetFirstChild($node)
        while ($null -ne $child) {
            if ($nodes.Count + $pending.Count -ge 1024) { throw 'UIA node limit exceeded' }
            $pending.Enqueue($child)
            $child = $walker.GetNextSibling($child)
        }
    }
    return ,$nodes
}
function Wait-Nodes([scriptblock]$Predicate) {
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    do {
        $nodes = Read-Nodes
        $report.uia = @{
            node_count = $nodes.Count
            terminal_nodes = @($nodes | Where-Object Name -eq 'Terminal output').Count
            text_nodes = @($nodes | Where-Object { $_.Text.Length -gt 0 }).Count
            prompt_detected = @($nodes | Where-Object { $_.Text -match '(?m)(?:PS .*>|\u03bb)' }).Count -gt 0
            welcome = @($nodes | Where-Object Name -eq 'Welcome to Automexia. Press Enter to get started.').Count -gt 0
        }
        if ($script:settingId) {
            $target = @($nodes | Where-Object Id -eq $script:settingId)
            $search = @($nodes | Where-Object Name -eq 'Search settings')
            # Only fixed fixture identifiers, booleans and native enum values may
            # leave this VM. Never include arbitrary UI names, values or text.
            $report.setting_diagnostic = @{
                id = $script:settingId
                action = $script:settingAction
                target_count = $target.Count
                target_enabled = @($target | Where-Object Enabled).Count -eq 1
                target_focused = @($target | Where-Object Focused).Count -eq 1
                target_on = @($target | Where-Object Toggle -eq 'On').Count -eq 1
                target_off = @($target | Where-Object Toggle -eq 'Off').Count -eq 1
                search_count = $search.Count
                search_focused = @($search | Where-Object Focused).Count -eq 1
                search_matches_query = @($search | Where-Object Value -eq $script:settingQuery).Count -eq 1
                focused_nodes = @($nodes | Where-Object Focused).Count
                navigation_moves = $script:settingMoves
            }
        }
        if (& $Predicate $nodes) { return ,$nodes }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'UIA condition timed out'
}
function Key([uint32]$Code, [bool]$Control = $false, [bool]$Shift = $false) {
    if (-not [AutomexiaResizeDriver]::SendModifiedKeyTap($script:window, $Code, $false, $Control, $Shift)) {
        throw 'Owned window did not accept keyboard input'
    }
    Start-Sleep -Milliseconds 120
}
function Type-Text([string]$Text, [bool]$Enter = $false) {
    if (-not [AutomexiaResizeDriver]::SendActionText($script:window, $Text, $Enter)) {
        throw 'Owned window did not accept fixture input'
    }
}
function Click-Node($Node) {
    # UIA gives screen coordinates; the existing driver takes client coordinates.
    $client = [AuditCoordinates]::ScreenToClientPoint($script:window,
        [int]($Node.Bounds.X + $Node.Bounds.Width / 2), [int]($Node.Bounds.Y + $Node.Bounds.Height / 2))
    if (-not [AutomexiaResizeDriver]::ActivateWindow($script:window) -or
        -not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($script:window, $client.X + 6, $client.Y)) {
        throw 'Owned window did not accept pointer input'
    }
    Start-Sleep -Milliseconds 100
    if (-not [AutomexiaResizeDriver]::MovePhysicalPointerToClient($script:window, $client.X, $client.Y)) {
        throw 'Owned window did not accept pointer input'
    }
    Start-Sleep -Milliseconds 100
    if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($script:window, $client.X, $client.Y)) {
        throw 'Owned window did not accept pointer input'
    }
    Start-Sleep -Milliseconds 200
}
function Find-Setting([string]$Query, [string]$Id) {
    $script:settingId = $Id
    $script:settingQuery = $Query
    $script:settingAction = 'open-settings'
    $script:settingMoves = 0
    for ($i = 0; $i -lt 8; $i++) {
        if (@((Read-Nodes) | Where-Object Name -eq 'Terminal output').Count -eq 1) { break }
        Key 0x1B
    }
    Key 0x53 $true $true
    $nodes = Wait-Nodes { param($n) @($n | Where-Object Name -eq 'Search settings').Count -eq 1 }
    Click-Node @($nodes | Where-Object Name -eq 'Search settings')[0]
    $null = Wait-Nodes { param($n)
        @($n | Where-Object Focused).Count -eq 1 -and
        @($n | Where-Object { $_.Name -eq 'Search settings' -and $_.Focused }).Count -eq 1
    }
    Key 0x41 $true
    Type-Text $Query
    $script:settingAction = 'find-setting'
    $nodes = Wait-Nodes { param($n)
        @($n | Where-Object Id -eq $Id).Count -eq 1 -and
        @($n | Where-Object { $_.Name -eq 'Search settings' -and $_.Value -eq $Query }).Count -eq 1
    }
    $node = @($nodes | Where-Object Id -eq $Id)[0]
    if ($node.Type -eq 'ControlType.Button') {
        Click-Node $node
        $nodes = Wait-Nodes { param($n) @($n | Where-Object { $_.Id -eq $Id -and $_.Type -eq 'ControlType.CheckBox' }).Count -eq 1 }
        $node = @($nodes | Where-Object { $_.Id -eq $Id -and $_.Type -eq 'ControlType.CheckBox' })[0]
    }
    # UIA bounds describe the complete setting row; its center can be explanatory
    # text rather than the smaller painted input. Use the supported list keyboard
    # interaction and verify the actual focused ID before changing a value.
    $nodes = Read-Nodes
    Click-Node @($nodes | Where-Object Name -eq 'Search settings')[0]
    $script:settingAction = 'focus-search'
    $null = Wait-Nodes { param($n)
        @($n | Where-Object Focused).Count -eq 1 -and
        @($n | Where-Object { $_.Name -eq 'Search settings' -and $_.Focused }).Count -eq 1
    }
    Key 0x09
    # Input delivery and accessibility publication are asynchronous. A fixed
    # delay can expose the preceding selection and send an extra arrow key.
    # Acknowledge each transition before issuing another navigation action.
    $script:settingAction = 'enter-list'
    $nodes = Wait-Nodes { param($n)
        @($n | Where-Object Focused).Count -eq 1 -and
        @($n | Where-Object { $_.Focused -and $_.Id }).Count -eq 1
    }
    for ($i = 0; $i -lt 32; $i++) {
        $focused = @($nodes | Where-Object { $_.Id -eq $Id -and $_.Focused })
        if ($focused.Count -eq 1) { return $focused[0] }
        $previousId = @($nodes | Where-Object Focused)[0].Id
        $script:settingAction = 'move-selection'
        Key 0x28
        $script:settingMoves++
        $nodes = Wait-Nodes { param($n)
            @($n | Where-Object Focused).Count -eq 1 -and
            @($n | Where-Object { $_.Focused -and $_.Id -and $_.Id -ne $previousId }).Count -eq 1
        }
    }
    throw 'Setting keyboard focus was not reached'
}
function Start-OwnedApp {
    $script:process = [Diagnostics.Process]::Start($script:startInfo)
    $report.startup = @{ exited = $false; window_count = 0 }
    $deadline = [DateTime]::UtcNow.AddSeconds(25)
    do {
        if ($script:process.HasExited) {
            $report.startup.exited = $true
            $report.startup.exit_code = $script:process.ExitCode
            throw 'Released app exited before showing its window'
        }
        $windows = [AutomexiaNativeWindowLocator]::VisibleApplicationWindows($script:process.Id)
        $report.startup.window_count = $windows.Count
        if ($windows.Count -eq 1) {
            $script:window = $windows[0]
            # A hosted desktop can be smaller than 1280x800. Off-screen pixels
            # from a desktop capture cannot establish a footer rendering failure.
            $desktopWidth = [AutomexiaResizeDriver]::PrimaryWidth()
            $desktopHeight = [AutomexiaResizeDriver]::PrimaryHeight()
            $report.viewport = @{ desktop_width = $desktopWidth; desktop_height = $desktopHeight }
            [void][AutomexiaResizeDriver]::MoveWindow($script:window, 20, 20,
                [Math]::Min(1280, $desktopWidth - 60), [Math]::Min(800, $desktopHeight - 100), $true)
            $initial = Wait-Nodes { param($n)
                @($n | Where-Object Name -eq 'Terminal output').Count -eq 1 -or
                @($n | Where-Object Name -eq 'Welcome to Automexia. Press Enter to get started.').Count -eq 1
            }
            if (@($initial | Where-Object Name -eq 'Welcome to Automexia. Press Enter to get started.').Count -eq 1) {
                $report.welcome_seen = $true
                Key 0x0D
            }
            $null = Wait-Nodes { param($n)
                @($n | Where-Object Name -eq 'Terminal output').Count -eq 1 -and
                @($n | Where-Object { $_.Text -match '(?m)(?:PS .*>|\u03bb)' }).Count -gt 0
            }
            return
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'Released app did not show one owned window'
}
function Stop-OwnedApp {
    if ($null -ne $script:process -and -not $script:process.HasExited) {
        [void]$script:process.CloseMainWindow()
        if (-not $script:process.WaitForExit(5000)) {
            # Kill only the process tree started by this diagnostic, in this disposable VM.
            & "$env:SystemRoot\System32\taskkill.exe" /PID $script:process.Id /T /F *> $null
            if (-not $script:process.WaitForExit(5000)) { throw 'Owned process teardown timed out' }
        }
    }
}
function Open-Themes {
    for ($i = 0; $i -lt 8; $i++) {
        if (@((Read-Nodes) | Where-Object Name -eq 'Terminal output').Count -eq 1) { break }
        Key 0x1B
    }
    Key 0x50 $true $true
    $null = Wait-Nodes { param($n) @($n | Where-Object Name -eq 'Search commands').Count -eq 1 }
    Type-Text 'Theme Gallery'
    $null = Wait-Nodes { param($n) @($n | Where-Object { $_.Type -eq 'ControlType.ListItem' -and $_.Name -like 'Theme Gallery*' }).Count -eq 1 }
    Key 0x0D
    return Wait-Nodes { param($n) @($n | Where-Object { $_.Type -eq 'ControlType.Window' -and $_.Name -eq 'Themes' }).Count -gt 0 -or @($n | Where-Object Name -eq 'Use configuration').Count -gt 0 }
}
function Count-ThemeColor($Bitmap, [double[]]$Region, [int[]]$Rgb) {
    $matches = 0
    for ($y = [int]$Region[1]; $y -lt [int]$Region[3]; $y += 2) {
        for ($x = [int]$Region[0]; $x -lt [int]$Region[2]; $x += 2) {
            if ($x -lt 0 -or $y -lt 0 -or $x -ge $Bitmap.Width -or $y -ge $Bitmap.Height) { throw 'Theme sample outside captured window' }
            $pixel = $Bitmap.GetPixel($x, $y)
            if ([Math]::Abs([int]$pixel.R - $Rgb[0]) -le 3 -and
                [Math]::Abs([int]$pixel.G - $Rgb[1]) -le 3 -and
                [Math]::Abs([int]$pixel.B - $Rgb[2]) -le 3) { $matches++ }
        }
    }
    return $matches
}
try {
    $target = if ($Architecture -eq 'arm64') { 'aarch64' } else { 'x86_64' }
    $digests = @{
        'x64.zip' = 'b94baab303681884ebde7aedbae37c6cf75be265556a9009ef9b879234ece077'
        'x64.msi' = 'af7a19c7517ef03982c753651568d11769fe3e8b98add611fb680b89953d81a1'
        'arm64.zip' = '5ecc9cd47661be01c9c698df86b2d3dbb797972987da130b4f73b79628792a3b'
        'arm64.msi' = '20050f5d0aadcff7848974c3da6fafa59c37a6c537a5bdf9604370fe5cc503d1'
    }
    $download = Join-Path $work ('package.' + $PackageKind)
    if ($PackagePath) {
        if ([IO.Path]::GetExtension($PackagePath) -ne ('.' + $PackageKind)) { throw 'Candidate package kind mismatch' }
        Copy-Item -LiteralPath $PackagePath -Destination $download
        $expectedHash = $PackageSha256
    } else {
        $asset = 'automexia-terminal-0.0.1-alpha-' + $target + '-pc-windows-msvc.' + $PackageKind
        Invoke-WebRequest -UseBasicParsing -TimeoutSec 60 -Uri ('https://github.com/AmjedAllaya/automexia/releases/download/0.0.1_alpha/' + $asset) -OutFile $download
        $expectedHash = $digests[($Architecture + '.' + $PackageKind)]
    }
    $hash = (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne $expectedHash) { throw 'Package digest mismatch' }
    $report.package_sha256 = $hash
    $packageRoot = Join-Path $work 'package'
    if ($PackageKind -eq 'zip') {
        Expand-Archive -LiteralPath $download -DestinationPath $packageRoot
    } else {
        $stage = 'msi-install'
        $exitCode = Invoke-Installer '/i' $download
        $report.msi_install_exit = $exitCode
        if ($exitCode -notin @(0, 3010)) { throw 'MSI installation failed' }
        $msiInstalled = $true
        $candidates = foreach ($base in @($env:ProgramFiles, [Environment]::GetFolderPath('ProgramFilesX86'), $env:LOCALAPPDATA)) {
            if ($base) {
                foreach ($suffix in @('Automexia Terminal', 'Automexia\Terminal', 'AutomexiaTerminal', 'automexia')) {
                    $candidate = Join-Path $base $suffix
                    if (Test-Path -LiteralPath (Join-Path $candidate 'automexia.exe')) { $candidate }
                }
            }
        }
        $candidates = @($candidates | Select-Object -Unique)
        if ($candidates.Count -ne 1) { throw 'MSI did not expose one expected installation root' }
        $packageRoot = $candidates[0]
    }
    $integrationCandidates = @(
        (Join-Path $packageRoot 'shell-integration'),
        (Join-Path $packageRoot 'resources/shell-integration')
    )
    $integrationRoot = @($integrationCandidates | Where-Object { Test-Path -LiteralPath (Join-Path $_ 'powershell/automexia.ps1') })
    if ($integrationRoot.Count -ne 1) { throw 'Package integration layout is ambiguous or missing' }
    $integrationRoot = $integrationRoot[0]
    if ($InternetMark) {
        # Controlled extracted-file provenance condition, not a browser download simulation.
        Get-ChildItem -LiteralPath $integrationRoot -Recurse -File |
            Where-Object Extension -in '.ps1', '.ps1xml' | ForEach-Object {
                Set-Content -LiteralPath $_.FullName -Stream Zone.Identifier -Value "[ZoneTransfer]`r`nZoneId=3" -Encoding ASCII
            }
    }
    $configRoot = Join-Path $work 'fresh-settings'
    New-Item -ItemType Directory -Path $configRoot | Out-Null
    if (@(Get-ChildItem -LiteralPath $configRoot -Force).Count -ne 0) { throw 'Settings directory is not empty' }
    $report.started_without_settings = $true
    $stage = 'packaged-script-policy'
    $policyProbeFile = Join-Path $work 'policy-probe.json'
    $policyProbe = New-Object Diagnostics.ProcessStartInfo
    $policyProbe.FileName = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
    $policyProbe.UseShellExecute = $false
    $policyProbe.CreateNoWindow = $true
    $policyProbe.WorkingDirectory = $work
    $policyProbe.EnvironmentVariables['TERM_PROGRAM'] = 'Automexia'
    $policyProbe.EnvironmentVariables['AUTOMEXIA_CONFIG_HOME'] = $configRoot
    $policyProbe.EnvironmentVariables['AUTOMEXIA_AUDIT_SCRIPT'] = Join-Path $integrationRoot 'powershell/automexia.ps1'
    $policyProbe.EnvironmentVariables['AUTOMEXIA_AUDIT_RESULT'] = $policyProbeFile
    $policyProbe.EnvironmentVariables['PSExecutionPolicyPreference'] = $Policy
    # Release jobs use pwsh; its modules are incompatible with Windows PowerShell.
    $policyProbe.EnvironmentVariables['PSModulePath'] = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\Modules"
    # Fixed diagnostic code and exact argv; never evaluate downloaded content as a command string.
    $probeCode = '$blocked=$false;try{. $env:AUTOMEXIA_AUDIT_SCRIPT}catch [System.Management.Automation.PSSecurityException]{$blocked=$true};@{policy=(Get-ExecutionPolicy).ToString();blocked=$blocked;loaded=[bool]$global:AutomexiaShellIntegrationLoaded}|ConvertTo-Json -Compress|Set-Content -LiteralPath $env:AUTOMEXIA_AUDIT_RESULT'
    $policyProbe.Arguments = '-NoLogo -NoProfile -NonInteractive -EncodedCommand ' + [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($probeCode))
    $policyChild = [Diagnostics.Process]::Start($policyProbe)
    if (-not $policyChild.WaitForExit(20000)) { $policyChild.Kill(); throw 'Packaged script probe timed out' }
    $policyResult = Get-Content -LiteralPath $policyProbeFile -Raw | ConvertFrom-Json
    $report.packaged_script = @{ effective_policy = $policyResult.policy; blocked = $policyResult.blocked; loaded = $policyResult.loaded }
    if ($policyResult.policy -ne $Policy) { throw 'Script probe policy mismatch' }
    $expectedIntegration = $Policy -eq 'RemoteSigned' -and -not $InternetMark
    if ($policyResult.loaded -ne $expectedIntegration -or $policyResult.blocked -eq $expectedIntegration) { throw 'Packaged script violated the expected policy behavior' }
    # The direct script probe is separate from the application's first-run settings.
    if (@(Get-ChildItem -LiteralPath $configRoot -Force).Count -ne 0) { throw 'Script probe wrote application settings' }
    $stage = 'native-driver'
    # Compile the repository-owned input/capture driver; do not execute its source-built fixture.
    $harness = Get-Content -LiteralPath (Join-Path $root 'tests/integration/resize-stress-windows.ps1') -Raw
    $match = [regex]::Match($harness, "(?s)Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'\r?\n(?<code>.*?)\r?\n'@")
    if (-not $match.Success) { throw 'Cannot find the repository-owned native driver' }
    Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition $match.Groups['code'].Value
    Add-Type -Path (Join-Path $root 'tests/integration/windows-native-window-locator.cs')
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
    Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.Runtime.InteropServices;
public static class AuditCoordinates {
    [StructLayout(LayoutKind.Sequential)] struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] static extern bool ScreenToClient(IntPtr window, ref Point point);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    public static bool WindowFitsDesktop(IntPtr window) {
        var previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        try {
            Rect rect;
            return GetWindowRect(window, out rect) && rect.Left >= 0 && rect.Top >= 0 &&
                rect.Right <= GetSystemMetrics(0) && rect.Bottom <= GetSystemMetrics(1);
        } finally { SetThreadDpiAwarenessContext(previous); }
    }
    public static Point ScreenToClientPoint(IntPtr window, int x, int y) {
        var point = new Point(x, y);
        if (!ScreenToClient(window, ref point)) throw new InvalidOperationException("Coordinate conversion failed");
        return point;
    }
}
'@
    $stage = 'first-launch'
    $probeFile = Join-Path $work 'shell-probe.json'
    $startInfo = New-Object Diagnostics.ProcessStartInfo
    $startInfo.FileName = Join-Path $packageRoot 'automexia.exe'
    $startInfo.WorkingDirectory = $work
    $startInfo.UseShellExecute = $false
    foreach ($key in @($startInfo.EnvironmentVariables.Keys)) {
        if ($key -like 'AUTOMEXIA_*' -or $key -like 'AMX_*' -or $key -in @('GITHUB_TOKEN', 'GH_TOKEN', 'PSExecutionPolicyPreference')) {
            $startInfo.EnvironmentVariables.Remove($key)
        }
    }
    $startInfo.EnvironmentVariables['AUTOMEXIA_CONFIG_HOME'] = $configRoot
    $startInfo.EnvironmentVariables['AUTOMEXIA_AUDIT_RESULT'] = $probeFile
    $startInfo.EnvironmentVariables['PSExecutionPolicyPreference'] = $Policy
    $startInfo.EnvironmentVariables['PSModulePath'] = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\Modules"
    # Limit discoverable tooling to Windows and the actual package; no source-tree resource fallback.
    $startInfo.EnvironmentVariables['PATH'] = "$packageRoot;$env:SystemRoot\System32;$env:SystemRoot;$env:SystemRoot\System32\WindowsPowerShell\v1.0"
    Start-OwnedApp
    $stage = 'shell-integration'
    if ($PackagePath -and -not $expectedIntegration) {
        $null = Wait-Nodes { param($n) @($n | Where-Object { $_.Text -match 'PowerShell blocked Automexia shell integration' }).Count -gt 0 }
        $report.policy_notice_visible = $true
    }
    Type-Text '$r=@{policy=(Get-ExecutionPolicy).ToString();integrated=($env:AUTOMEXIA_SHELL_INTEGRATION -eq "1");promptHook=[bool]$global:AutomexiaShellIntegrationLoaded;editor=[string](Get-Module PSReadLine).Version};$r|ConvertTo-Json -Compress|Set-Content -LiteralPath $env:AUTOMEXIA_AUDIT_RESULT;Write-Output "AUDIT_COMMAND_COMPLETE"' $true
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while (-not (Test-Path -LiteralPath $probeFile) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 100 }
    $probe = Get-Content -LiteralPath $probeFile -Raw | ConvertFrom-Json
    $report.shell = @{ effective_policy = $probe.policy; integration_loaded = $probe.integrated; prompt_hook_loaded = $probe.promptHook; editor_version = $probe.editor }
    if ($probe.policy -ne $Policy) { throw 'Requested policy did not become effective; comparison is invalid' }
    if ($probe.integrated -ne $expectedIntegration -or $probe.promptHook -ne $expectedIntegration) { throw 'Application shell integration did not match effective policy' }
    if ($PackagePath -and $expectedIntegration -and [version]($CandidateVersion.Split('-')[0]) -ge [version]'0.0.4') {
        $editorRecipe = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'packaging/windows/powershell-editor.json') -Raw | ConvertFrom-Json
        if ($probe.editor -cne $editorRecipe.package.version) { throw 'Installed application did not select its packaged PowerShell editor' }
    }
    $stage = 'customization-controls'
    $cases = @(
        @('Show information tags', 'tags.enabled', 'boolean'),
        @('Inline tables', 'terminal.inline_tables', 'boolean'),
        @('Command timestamps', 'terminal.command_timestamps', 'boolean'),
        @('Highlight recognized output', 'terminal.output_highlighting', 'boolean'),
        @('Highlight completed commands', 'terminal.command_output_highlighting', 'boolean'),
        @('Highlight Kubernetes status', 'terminal.kubernetes_highlighting', 'boolean'),
        @('Show Footer', 'interface.footer.visible', 'boolean'),
        @('Font size', 'appearance.font_size', 'value'),
        @('Appearance', 'appearance.theme', 'value')
    )
    if ($StressOutputHighlighting) {
        # Supplemental reproduction of the candidate's intermittent restore
        # timeout. Each cycle changes, restarts, verifies and restores the value.
        $cases = @(1..12 | ForEach-Object { ,@('Highlight recognized output', 'terminal.output_highlighting', 'boolean') })
    }
    foreach ($case in $cases) {
        $entry = [ordered]@{ id = $case[1]; cycle = $report.controls.Count + 1; reached = $false; toggled = $false; persisted = $false; restored = $false }
        $report.controls += $entry
        $node = Find-Setting $case[0] $case[1]
        $entry.navigation_moves = $script:settingMoves
        $entry.reached = $true
        if (-not $node.Enabled) {
            # settings_catalog.rs intentionally disables this legacy selector in
            # a fresh configuration without an adaptive Light/Dark theme pair.
            # AccessKit exposes that explanation as UIA FullDescription, which
            # the .NET Framework UIA client does not expose through HelpText.
            if ($case[1] -eq 'appearance.theme') {
                $entry.availability = 'disabled until adaptive light and dark themes are configured'
                $entry.expected_disabled_state = $true
                continue
            }
            throw 'An expected active setting is disabled'
        }
        $property = if ($case[2] -eq 'boolean') { 'Toggle' } else { 'Value' }
        $before = $node.$property
        if ($property -eq 'Toggle' -and $before -notin @('On', 'Off')) { throw 'Setting has no native toggle state' }
        if ($property -eq 'Value' -and [string]::IsNullOrEmpty($before)) { throw 'Setting has no native value' }
        $forwardKey = if ($property -eq 'Toggle') { 0x20 } else { 0x27 }
        $backwardKey = if ($property -eq 'Toggle') { 0x20 } else { 0x25 }
        $script:settingAction = 'change-setting'
        Key $forwardKey
        $nodes = Wait-Nodes { param($n) @($n | Where-Object { $_.Id -eq $case[1] -and $_.$property -ne $before }).Count -eq 1 }
        $after = @($nodes | Where-Object Id -eq $case[1])[0].$property
        $entry.toggled = $true
        Start-Sleep -Milliseconds 600
        Key 0x1B
        Stop-OwnedApp
        Start-OwnedApp
        $restored = Find-Setting $case[0] $case[1]
        $entry.restore_navigation_moves = $script:settingMoves
        $entry.persisted = $restored.$property -eq $after
        if (-not $entry.persisted) { throw 'Setting did not survive restart' }
        # Return to the original value before moving to another independent control.
        $script:settingAction = 'restore-setting'
        Key $backwardKey
        $null = Wait-Nodes { param($n) @($n | Where-Object { $_.Id -eq $case[1] -and $_.$property -eq $before }).Count -eq 1 }
        $entry.restored = $true
        Key 0x1B
    }
    $stage = 'theme-gallery'
    $report.footer_enabled_before_themes = (Find-Setting 'Show Footer' 'interface.footer.visible').Toggle -eq 'On'
    $report.themes = @()
    foreach ($theme in @(@('Solar Dusk', @(33,26,23)), @('Arctic Day', @(233,237,241)))) {
        $entry = [ordered]@{ theme = $theme[0]; applied = $false; persisted = $false; header_color = $false; footer_color = $false }
        $report.themes += $entry
        $null = Open-Themes
        Key 0x24
        $selected = $false
        for ($i = 0; $i -lt 24; $i++) {
            $nodes = Read-Nodes
            if (@($nodes | Where-Object { $_.Focused -and $_.Name.StartsWith($theme[0] + '.') }).Count -eq 1) { $selected = $true; break }
            Key 0x28
        }
        if (-not $selected) { throw 'Built-in theme was not reachable' }
        Key 0x0D
        $null = Wait-Nodes { param($n) @($n | Where-Object Name -eq 'Terminal output').Count -eq 1 }
        $entry.applied = $true
        Start-Sleep -Milliseconds 600
        # Inspect real composited pixels inside the VM; upload booleans only.
        # Palette oracles are fixed independently of application preferences.
        $capture = Join-Path $work 'theme.png'
        $entry.window_inside_desktop = [AuditCoordinates]::WindowFitsDesktop($script:window)
        if (-not $entry.window_inside_desktop) { throw 'Window extends beyond the desktop capture area' }
        $null = [AutomexiaResizeDriver]::CaptureClientFrame($script:window, $capture)
        $bitmap = [Drawing.Bitmap]::new($capture)
        try {
            $scale = [AuditCoordinates]::GetDpiForWindow($script:window) / 96.0
            $entry.header_color = (Count-ThemeColor $bitmap @(($bitmap.Width*0.4),(4*$scale),($bitmap.Width*0.7),(28*$scale)) $theme[1]) -ge 300
            $entry.footer_color = (Count-ThemeColor $bitmap @((30*$scale),($bitmap.Height-55*$scale),($bitmap.Width*0.6),($bitmap.Height-2*$scale)) $theme[1]) -ge 300
            # Keep the original footer oracle. This extra observation distinguishes
            # an unexpected footer position from absence of its expected color.
            $entry.lower_half_surface_color = (Count-ThemeColor $bitmap @((30*$scale),($bitmap.Height*0.5),($bitmap.Width*0.6),($bitmap.Height-2*$scale)) $theme[1]) -ge 300
        } finally { $bitmap.Dispose() }
        Stop-OwnedApp
        Start-OwnedApp
        $nodes = Open-Themes
        $entry.persisted = @($nodes | Where-Object { $_.Name.StartsWith($theme[0] + '.') -and $_.Name.EndsWith('. Current theme') }).Count -eq 1
        if (-not $entry.persisted) { throw 'Applied theme did not survive restart' }
    }
    # Collect both palettes and their persistence before reporting any mismatch;
    # a later successful observation must not erase an earlier failed oracle.
    if (@($report.themes | Where-Object { -not $_.header_color -or -not $_.footer_color }).Count -gt 0) {
        throw 'Applied theme did not reach the native window palette'
    }
    $report.complete = $true
} catch {
    # Do not upload exception messages, UIA text, usernames, shell history or paths.
    $report.failed_stage = $stage
    $report.failure_type = $_.Exception.GetType().Name
    $report.failure_line = $_.InvocationInfo.ScriptLineNumber
    # Only fixed diagnostic messages may leave the VM; native exception text can contain paths.
    if ($_.Exception.Message -in @('UIA condition timed out', 'Released app exited before showing its window',
            'Released app did not show one owned window', 'Owned window did not accept keyboard input',
            'Owned window did not accept fixture input', 'Owned window did not accept pointer input',
            'Setting has no native toggle state', 'Setting did not survive restart', 'Script probe wrote application settings',
            'Built-in theme was not reachable', 'Applied theme did not reach the native window palette',
            'Applied theme did not survive restart', 'Setting has no native value', 'An expected active setting is disabled',
            'Setting keyboard focus was not reached', 'Window extends beyond the desktop capture area')) {
        $report.failure_reason = $_.Exception.Message
    }
} finally {
    try { Stop-OwnedApp } catch { $report.teardown_failed = $true; $report.complete = $false }
    if ($msiInstalled) {
        try {
            $report.msi_uninstall_exit = Invoke-Installer '/x' $download
            if ($report.msi_uninstall_exit -notin @(0, 1605, 3010)) { $report.complete = $false }
        } catch { $report.msi_uninstall_failed = $true; $report.complete = $false }
    }
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $reportRoot 'report.json') -Encoding UTF8
}
if (-not $report.complete) { throw "Package review incomplete at $stage; see the bounded report." }
