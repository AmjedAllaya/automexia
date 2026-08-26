param(
    [string]$Binary,
    [string]$Distro = $env:AUTOMEXIA_TEST_WSL_DISTRO
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrWhiteSpace($Binary)) {
    $Binary = Join-Path $root 'target\debug\automexia.exe'
}
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "Automexia test binary was not found at $Binary"
}
Add-Type -Path (Join-Path $PSScriptRoot 'windows-native-window-locator.cs')


$installed = @(& wsl.exe --list --quiet | ForEach-Object {
    $_.Replace(([char]0).ToString(), [string]::Empty).Trim()
} | Where-Object { $_ })
if ([string]::IsNullOrWhiteSpace($Distro)) {
    $Distro = $installed | Select-Object -First 1
}
if ([string]::IsNullOrWhiteSpace($Distro) -or
    -not ($installed | Where-Object { $_ -eq $Distro })) {
    throw "Controlled WSL clone test requires an installed distro. Set AUTOMEXIA_TEST_WSL_DISTRO; installed: $($installed -join ', ')"
}

function Read-Snapshot {
    param([int64]$After = -1, [int]$TimeoutMilliseconds = 15000)
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMilliseconds)
    do {
        if (Test-Path -LiteralPath $snapshotPath -PathType Leaf) {
            try {
                $snapshot = Get-Content -LiteralPath $snapshotPath -Raw | ConvertFrom-Json
                if ([int64]$snapshot.sequence -gt $After) { return $snapshot }
            } catch {}
        }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for WSL clone snapshot after sequence $After"
}

function Active-Panel {
    param($Snapshot)
    return @($Snapshot.panels | Where-Object { [bool]$_.active })[0]
}

function Send-AutomexiaTestControl {
    param([string]$Control)
    $stagedControl = Join-Path (
        [IO.Path]::GetDirectoryName($controlPath)) (
        '.automexia-wsl-control-{0}.tmp' -f [guid]::NewGuid().ToString('N'))
    try {
        [IO.File]::WriteAllText(
            $stagedControl,
            $Control,
            [Text.UTF8Encoding]::new($false))
        [IO.File]::Delete($controlPath)
        [IO.File]::Move($stagedControl, $controlPath)
    } finally {
        [IO.File]::Delete($stagedControl)
    }
}

$snapshotPath = Join-Path ([IO.Path]::GetTempPath()) ('automexia-wsl-clone-{0}.json' -f [guid]::NewGuid().ToString('N'))
$controlPath = Join-Path ([IO.Path]::GetTempPath()) ('automexia-wsl-control-{0}.txt' -f [guid]::NewGuid().ToString('N'))
$configRoot = Join-Path ([IO.Path]::GetTempPath()) ('automexia-wsl-config-{0}' -f [guid]::NewGuid().ToString('N'))
$previousSnapshot = $env:AUTOMEXIA_RESIZE_SNAPSHOT
$previousControl = $env:AUTOMEXIA_NATIVE_TEST_CONTROL
$previousConfigHome = $env:AUTOMEXIA_CONFIG_HOME
$process = $null

try {
    [void](New-Item -ItemType Directory -Path $configRoot)
    $wslRoot = (& wsl.exe --distribution $Distro --exec wslpath -a $root).Trim()
    $wslConfigRoot = (& wsl.exe --distribution $Distro --exec wslpath -a $configRoot).Trim()
    if (-not $wslRoot.StartsWith('/') -or -not $wslConfigRoot.StartsWith('/')) {
        throw 'Could not translate controlled test paths into WSL paths'
    }
    $bashRcWindows = Join-Path $configRoot '.automexia-test-bashrc'
    $bashRcWsl = "$wslConfigRoot/.automexia-test-bashrc"
    [IO.File]::WriteAllText(
        $bashRcWindows,
        "source '$wslRoot/shell-integration/bash/automexia.bash'`n",
        [Text.UTF8Encoding]::new($false))

    $tomlDistro = $Distro.Replace('\', '\\').Replace('"', '\"')
    $tomlRoot = $wslRoot.Replace('"', '\"')
    $tomlRc = $bashRcWsl.Replace('"', '\"')
    $config = @"
confirm-before-quit = false

[shell]
program = "wsl.exe"
args = ["--distribution", "$tomlDistro", "--cd", "$tomlRoot", "--exec", "bash", "--noprofile", "--rcfile", "$tomlRc", "-i"]
"@
    [IO.File]::WriteAllText(
        (Join-Path $configRoot 'config.toml'),
        $config,
        [Text.UTF8Encoding]::new($false))

    $env:AUTOMEXIA_RESIZE_SNAPSHOT = $snapshotPath
    $env:AUTOMEXIA_NATIVE_TEST_CONTROL = $controlPath
    $env:AUTOMEXIA_CONFIG_HOME = $configRoot
    $process = Start-Process -FilePath $Binary -WorkingDirectory $root -PassThru

    $deadline = [DateTime]::UtcNow.AddSeconds(20)
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
    if ($process.HasExited -or $applicationWindows.Count -ne 1) {
        throw "Automexia exposed $($applicationWindows.Count) live WSL application windows; expected 1"
    }
    $window = $applicationWindows[0]

    $initial = Read-Snapshot
    $readyDeadline = [DateTime]::UtcNow.AddSeconds(20)
    while (($null -eq (Active-Panel $initial).shell_distro -or
            $null -eq (Active-Panel $initial).shell_user -or
            $null -eq (Active-Panel $initial).shell_path -or
            -not [bool]$initial.full_path_visible) -and
           [DateTime]::UtcNow -lt $readyDeadline) {
        $initial = Read-Snapshot -After ([int64]$initial.sequence)
    }
    $source = Active-Panel $initial
    if ($source.shell_distro -ne $Distro -or
        -not $source.current_directory.StartsWith('/') -or
        [int64]$source.shell_pid -le 0) {
        Write-Host ($initial | ConvertTo-Json -Depth 8)
        throw 'Initial WSL distro/user/shell/cwd metadata is incomplete'
    }

    # Shell-source tests prove emitted OSC bytes, but not the ConPTY/grid/
    # renderer path. Exercise real Bash commands in this native WSL pane and
    # require every output-producing command to own a fresh painted result.
    # Silent commands must complete semantically without creating an empty
    # surface. The cases are command-class fixtures, not a command allowlist.
    $resultCases = @(
        [pscustomobject]@{
            Name = 'stdout'
            Command = "printf '%s\n' AMX_WSL_STDOUT_84201"
            Tokens = @('AMX_WSL_STDOUT_84201')
            ExitCode = 0
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'pipeline-multiline'
            Command = "printf '%s\n' amx_wsl_pipe_a_84202 amx_wsl_pipe_b_84202 | tr '[:lower:]' '[:upper:]'"
            Tokens = @('AMX_WSL_PIPE_A_84202', 'AMX_WSL_PIPE_B_84202')
            ExitCode = 0
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'stderr-failure'
            Command = "sh -c 'printf `"%s\n`" AMX_WSL_STDERR_84203 >&2; exit 7'"
            Tokens = @('AMX_WSL_STDERR_84203')
            ExitCode = 7
            HasOutput = $true
        },
        [pscustomobject]@{
            Name = 'silent-success'
            Command = 'true'
            Tokens = @()
            ExitCode = 0
            HasOutput = $false
        }
    )
    $resultProbe = $initial
    foreach ($case in $resultCases) {
        $previousPromptId = [int64]$resultProbe.latest_prompt_id
        $previousResultKey = if ($null -eq $resultProbe.command_result_key) {
            -1
        } else {
            [int64]$resultProbe.command_result_key
        }
        $control = "write-line:wsl-result-$($case.Name):$($case.Command)"
        Send-AutomexiaTestControl $control
        $resultReady = Read-Snapshot -After ([int64]$resultProbe.sequence)
        $resultDeadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $panel = Active-Panel $resultReady
            $tokensVisible = $true
            foreach ($token in $case.Tokens) {
                if (-not ([string]$panel.visible_text).Contains($token)) {
                    $tokensVisible = $false
                    break
                }
            }
            if ([bool]$case.HasOutput) {
                $resultMatches =
                    $null -ne $resultReady.command_result_key -and
                    [int64]$resultReady.command_result_key -gt $previousResultKey -and
                    [int64]$resultReady.command_result_generation -eq $previousPromptId -and
                    [int]$resultReady.command_result_exit_code -eq [int]$case.ExitCode -and
                    $null -ne $resultReady.command_result_surface -and
                    $null -ne $resultReady.command_result_divider
            } else {
                $semanticResult = @($resultReady.semantic_rows | Where-Object {
                    $_.has_result -and
                        [int64]$_.generation -eq $previousPromptId -and
                        [int]$_.result_exit_code -eq [int]$case.ExitCode
                })
                $resultMatches = $semanticResult.Count -gt 0 -and
                    ($null -eq $resultReady.command_result_generation -or
                        [int64]$resultReady.command_result_generation -ne $previousPromptId)
            }
            $complete = [string]$resultReady.last_control -eq $control -and
                [int64]$resultReady.latest_prompt_id -gt $previousPromptId -and
                $tokensVisible -and $resultMatches
            if (-not $complete) {
                $resultReady = Read-Snapshot -After ([int64]$resultReady.sequence)
            }
        } while (-not $complete -and [DateTime]::UtcNow -lt $resultDeadline)
        if (-not $complete) {
            Write-Host ($resultReady | ConvertTo-Json -Depth 10)
            throw "Native WSL command-result case '$($case.Name)' did not publish truthful visible grouping"
        }
        $resultProbe = $resultReady
    }
    $initial = $resultProbe
    $source = Active-Panel $initial

    Send-AutomexiaTestControl 'clone-right:1'
    $cloneSnapshot = Read-Snapshot -After ([int64]$initial.sequence)
    $cloneDeadline = [DateTime]::UtcNow.AddSeconds(20)
    while (([int]$cloneSnapshot.panel_count -ne 2 -or
            $null -eq (Active-Panel $cloneSnapshot).shell_path -or
            -not [bool]$cloneSnapshot.full_path_visible) -and
           [DateTime]::UtcNow -lt $cloneDeadline) {
        $cloneSnapshot = Read-Snapshot -After ([int64]$cloneSnapshot.sequence)
    }
    $clone = Active-Panel $cloneSnapshot
    if ([int]$cloneSnapshot.panel_count -ne 2 -or
        [int64]$clone.route_id -eq [int64]$source.route_id -or
        [int64]$clone.shell_pid -eq [int64]$source.shell_pid -or
        [int64]$clone.shell_pid -le 0) {
        Write-Host ($cloneSnapshot | ConvertTo-Json -Depth 8)
        throw 'WSL clone did not create an independent route and ConPTY process'
    }
    foreach ($field in @('shell_distro', 'shell_user', 'shell_path', 'current_directory', 'profile_identity')) {
        if ($clone.$field -ne $source.$field) {
            Write-Host ($cloneSnapshot | ConvertTo-Json -Depth 8)
            throw "WSL clone changed $field"
        }
    }
    if ($clone.launch_program -notmatch '(?i)(^|[\\/])wsl(\.exe)?$') {
        throw 'WSL clone silently fell back to a native Windows shell'
    }

    Write-Host "Native WSL clone passed for ${Distro}: routes $($source.route_id), $($clone.route_id)"
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        [void][AutomexiaNativeWindowLocator]::PostMessage(
            $window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        if (-not $process.WaitForExit(5000)) { Stop-Process -Id $process.Id -Force }
    }
    if ($null -eq $previousSnapshot) {
        Remove-Item Env:AUTOMEXIA_RESIZE_SNAPSHOT -ErrorAction SilentlyContinue
    } else { $env:AUTOMEXIA_RESIZE_SNAPSHOT = $previousSnapshot }
    if ($null -eq $previousControl) {
        Remove-Item Env:AUTOMEXIA_NATIVE_TEST_CONTROL -ErrorAction SilentlyContinue
    } else { $env:AUTOMEXIA_NATIVE_TEST_CONTROL = $previousControl }
    if ($null -eq $previousConfigHome) {
        Remove-Item Env:AUTOMEXIA_CONFIG_HOME -ErrorAction SilentlyContinue
    } else { $env:AUTOMEXIA_CONFIG_HOME = $previousConfigHome }
    Remove-Item -LiteralPath $snapshotPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $controlPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $configRoot -Recurse -Force -ErrorAction SilentlyContinue
}
