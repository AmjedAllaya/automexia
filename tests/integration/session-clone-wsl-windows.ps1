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
    do {
        Start-Sleep -Milliseconds 50
        $process.Refresh()
        $window = $process.MainWindowHandle
    } while ($window -eq [IntPtr]::Zero -and -not $process.HasExited -and [DateTime]::UtcNow -lt $deadline)
    if ($process.HasExited -or $window -eq [IntPtr]::Zero) {
        throw 'Automexia did not expose a live WSL test window'
    }

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

    [IO.File]::WriteAllText(
        $controlPath,
        'clone-right:1',
        [Text.UTF8Encoding]::new($false))
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
        [void]$process.CloseMainWindow()
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
