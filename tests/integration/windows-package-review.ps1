# Diagnostic only. This file deliberately refuses to launch on a contributor PC.
param(
    [Parameter(Mandatory)][string]$RepositoryRoot,
    [ValidateSet('Restricted', 'RemoteSigned')][string]$Policy = 'Restricted',
    [switch]$InternetMark
)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'This diagnostic may run only on a disposable GitHub-hosted Windows runner.'
}
if ($InternetMark -and $Policy -ne 'RemoteSigned') { throw 'Internet-mark comparison requires RemoteSigned.' }
$root = [IO.Path]::GetFullPath($RepositoryRoot)
$work = Join-Path $env:RUNNER_TEMP ('automexia-package-review-' + [guid]::NewGuid().ToString('N'))
$reportRoot = Join-Path $root 'artifacts/windows-package-review'
New-Item -ItemType Directory -Path $work, $reportRoot -Force | Out-Null
$report = [ordered]@{
    schema = 1
    tag = '0.0.1_alpha'
    source_commit = '7b099a6be450f1cd35910e3a454da45f01d45e12'
    architecture = 'x64'
    package = 'portable ZIP'
    policy = $Policy
    internet_mark = [bool]$InternetMark
    environment = 'fresh Windows Server hosted VM; client execution policy is an explicit test condition'
    controls = @()
    complete = $false
}
$process = $null
$window = [IntPtr]::Zero
$stage = 'package'
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
        if ($node.TryGetCurrentPattern([Windows.Automation.TextPattern]::Pattern, [ref]$textPattern)) {
            $visibleText = $textPattern.DocumentRange.GetText(16384)
        }
        if ($node.TryGetCurrentPattern([Windows.Automation.TogglePattern]::Pattern, [ref]$toggle)) {
            $state = [string]$toggle.Current.ToggleState
        }
        $nodes.Add([pscustomobject]@{
            Name = $current.Name; Id = $current.AutomationId; Bounds = $current.BoundingRectangle
            Type = $current.ControlType.ProgrammaticName; Toggle = $state; Enabled = $current.IsEnabled; Text = $visibleText
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
    if (-not [AutomexiaResizeDriver]::SendPhysicalLeftClick($script:window, $client.X, $client.Y)) {
        throw 'Owned window did not accept pointer input'
    }
    Start-Sleep -Milliseconds 200
}
function Find-Setting([string]$Query, [string]$Id) {
    for ($i = 0; $i -lt 8; $i++) {
        if (@((Read-Nodes) | Where-Object Name -eq 'Terminal output').Count -eq 1) { break }
        Key 0x1B
    }
    Key 0x53 $true $true
    $nodes = Wait-Nodes { param($n) @($n | Where-Object Name -eq 'Search settings').Count -eq 1 }
    Click-Node @($nodes | Where-Object Name -eq 'Search settings')[0]
    Key 0x41 $true
    Type-Text $Query
    $nodes = Wait-Nodes { param($n) @($n | Where-Object Id -eq $Id).Count -eq 1 }
    $node = @($nodes | Where-Object Id -eq $Id)[0]
    if ($node.Type -ne 'ControlType.CheckBox') {
        Click-Node $node
        $nodes = Wait-Nodes { param($n) @($n | Where-Object { $_.Id -eq $Id -and $_.Type -eq 'ControlType.CheckBox' }).Count -eq 1 }
        $node = @($nodes | Where-Object { $_.Id -eq $Id -and $_.Type -eq 'ControlType.CheckBox' })[0]
    }
    return $node
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
            [void][AutomexiaResizeDriver]::MoveWindow($script:window, 20, 20, 1280, 800, $true)
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
try {
    $zip = Join-Path $work 'package.zip'
    Invoke-WebRequest -UseBasicParsing -Uri 'https://github.com/AmjedAllaya/automexia/releases/download/0.0.1_alpha/automexia-terminal-0.0.1-alpha-x86_64-pc-windows-msvc.zip' -OutFile $zip
    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne 'b94baab303681884ebde7aedbae37c6cf75be265556a9009ef9b879234ece077') { throw 'Published package digest mismatch' }
    $report.package_sha256 = $hash
    $packageRoot = Join-Path $work 'package'
    Expand-Archive -LiteralPath $zip -DestinationPath $packageRoot
    if ($InternetMark) {
        # Controlled extracted-file provenance condition, not a browser download simulation.
        Get-ChildItem -LiteralPath (Join-Path $packageRoot 'shell-integration') -Recurse -File |
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
    $policyProbe.EnvironmentVariables['AUTOMEXIA_AUDIT_SCRIPT'] = Join-Path $packageRoot 'shell-integration/powershell/automexia.ps1'
    $policyProbe.EnvironmentVariables['AUTOMEXIA_AUDIT_RESULT'] = $policyProbeFile
    $policyProbe.EnvironmentVariables['PSExecutionPolicyPreference'] = $Policy
    # Fixed diagnostic code and exact argv; never evaluate downloaded content as a command string.
    $probeCode = '$blocked=$false;try{. $env:AUTOMEXIA_AUDIT_SCRIPT}catch [System.Management.Automation.PSSecurityException]{$blocked=$true};@{policy=(Get-ExecutionPolicy).ToString();blocked=$blocked;loaded=[bool]$global:AutomexiaShellIntegrationLoaded}|ConvertTo-Json -Compress|Set-Content -LiteralPath $env:AUTOMEXIA_AUDIT_RESULT'
    $policyProbe.Arguments = '-NoLogo -NoProfile -NonInteractive -EncodedCommand ' + [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($probeCode))
    $policyChild = [Diagnostics.Process]::Start($policyProbe)
    if (-not $policyChild.WaitForExit(20000)) { $policyChild.Kill(); throw 'Packaged script probe timed out' }
    $policyResult = Get-Content -LiteralPath $policyProbeFile -Raw | ConvertFrom-Json
    $report.packaged_script = @{ effective_policy = $policyResult.policy; blocked = $policyResult.blocked; loaded = $policyResult.loaded }
    if ($policyResult.policy -ne $Policy) { throw 'Script probe policy mismatch' }
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
    [DllImport("user32.dll")] static extern bool ScreenToClient(IntPtr window, ref Point point);
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
    # Limit discoverable tooling to Windows and the actual package; no source-tree resource fallback.
    $startInfo.EnvironmentVariables['PATH'] = "$packageRoot;$env:SystemRoot\System32;$env:SystemRoot;$env:SystemRoot\System32\WindowsPowerShell\v1.0"
    Start-OwnedApp
    $stage = 'shell-integration'
    Type-Text '$r=@{policy=(Get-ExecutionPolicy).ToString();integrated=($env:AUTOMEXIA_SHELL_INTEGRATION -eq "1");promptHook=[bool]$global:AutomexiaShellIntegrationLoaded};$r|ConvertTo-Json -Compress|Set-Content -LiteralPath $env:AUTOMEXIA_AUDIT_RESULT;Write-Output "AUDIT_COMMAND_COMPLETE"' $true
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while (-not (Test-Path -LiteralPath $probeFile) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 100 }
    $probe = Get-Content -LiteralPath $probeFile -Raw | ConvertFrom-Json
    $report.shell = @{ effective_policy = $probe.policy; integration_loaded = $probe.integrated; prompt_hook_loaded = $probe.promptHook }
    if ($probe.policy -ne $Policy) { throw 'Requested policy did not become effective; comparison is invalid' }
    $stage = 'customization-controls'
    $cases = @(
        @('Show information tags', 'tags.enabled'),
        @('Inline tables', 'terminal.inline_tables'),
        @('Command timestamps', 'terminal.command_timestamps')
    )
    foreach ($case in $cases) {
        $entry = [ordered]@{ id = $case[1]; reached = $false; toggled = $false; persisted = $false }
        $report.controls += $entry
        $node = Find-Setting $case[0] $case[1]
        $entry.reached = $true
        $before = $node.Toggle
        if ($before -notin @('On', 'Off')) { throw 'Setting has no native toggle state' }
        Click-Node $node
        $nodes = Wait-Nodes { param($n) @($n | Where-Object { $_.Id -eq $case[1] -and $_.Toggle -ne $before }).Count -eq 1 }
        $after = @($nodes | Where-Object Id -eq $case[1])[0].Toggle
        $entry.toggled = $true
        Start-Sleep -Milliseconds 600
        Key 0x1B
        Stop-OwnedApp
        Start-OwnedApp
        $restored = Find-Setting $case[0] $case[1]
        $entry.persisted = $restored.Toggle -eq $after
        if (-not $entry.persisted) { throw 'Setting did not survive restart' }
        # Return to the original value before moving to another independent control.
        Click-Node $restored
        $null = Wait-Nodes { param($n) @($n | Where-Object { $_.Id -eq $case[1] -and $_.Toggle -eq $before }).Count -eq 1 }
        Key 0x1B
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
            'Setting has no native toggle state', 'Setting did not survive restart', 'Script probe wrote application settings')) {
        $report.failure_reason = $_.Exception.Message
    }
} finally {
    try { Stop-OwnedApp } catch { $report.teardown_failed = $true; $report.complete = $false }
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $reportRoot 'report.json') -Encoding UTF8
}
if (-not $report.complete) { throw "Package review incomplete at $stage; see the bounded report." }
