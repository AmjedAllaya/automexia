# Native QA process ownership. Numeric PIDs alone cannot survive process exit.
# Keep the model separate so stale parent links and PID replacement are testable.
function Get-AutomexiaProcessBirth {
    param([datetime]$Created)
    $ticks = $Created.ToUniversalTime().Ticks
    # CIM CreationDate has microsecond precision; Process.StartTime uses 100 ns.
    return [long]($ticks - ($ticks % 10))
}

function Select-AutomexiaOwnedProcessRecords {
    param(
        [object[]]$Snapshot,
        [int]$RootProcessId,
        [datetime]$RootStartTime,
        [int]$RootSessionId,
        [string]$FixtureConfigRoot,
        [datetime]$FixtureStartTime = $RootStartTime
    )
    if ($Snapshot.Count -gt 4096) { throw 'Fixture process inventory exceeds its limit' }
    $rootBirth = Get-AutomexiaProcessBirth $RootStartTime
    $fixtureBirth = Get-AutomexiaProcessBirth $FixtureStartTime
    if ($fixtureBirth -gt $rootBirth) { throw 'Fixture lifetime starts after its process owner' }
    $rows = @{}
    foreach ($row in $Snapshot) {
        $id = [int]$row.ProcessId
        if ($id -le 0) { continue }
        if ($rows.ContainsKey($id)) { throw 'Fixture process inventory repeats an identity' }
        $rows[$id] = $row
    }
    $rootRow = $rows[$RootProcessId]
    if ($null -eq $rootRow -or $null -eq $rootRow.CreationDate -or
        (Get-AutomexiaProcessBirth $rootRow.CreationDate) -ne $rootBirth -or
        [int]$rootRow.SessionId -ne $RootSessionId) {
        throw 'Fixture root process identity changed'
    }
    $owned = [Collections.Generic.HashSet[int]]::new()
    [void]$owned.Add($RootProcessId)
    $queue = [Collections.Generic.Queue[int]]::new()
    $queue.Enqueue($RootProcessId)
    if (-not [string]::IsNullOrWhiteSpace($FixtureConfigRoot)) {
        $marker = '(?i)(?:^|[\s"''])' + [regex]::Escape($FixtureConfigRoot) + '(?:[\\/\s"'']|$)'
        foreach ($row in $rows.Values) {
            if ([int]$row.ProcessId -eq $RootProcessId -or
                [int]$row.SessionId -ne $RootSessionId -or $null -eq $row.CreationDate -or
                (Get-AutomexiaProcessBirth $row.CreationDate) -lt $fixtureBirth) { continue }
            $command = [string]$row.CommandLine
            if ($command.Length -gt 32768) { throw 'Fixture process command line exceeds its limit' }
            if ($command -match $marker -and $owned.Add([int]$row.ProcessId)) {
                $queue.Enqueue([int]$row.ProcessId)
            }
        }
    }
    while ($queue.Count -gt 0) {
        if ($owned.Count -gt 512) { throw 'Fixture process tree exceeds its limit' }
        $parent = $rows[$queue.Dequeue()]
        $parentBirth = Get-AutomexiaProcessBirth $parent.CreationDate
        foreach ($child in $rows.Values) {
            if ([int]$child.ParentProcessId -ne [int]$parent.ProcessId -or
                [int]$child.SessionId -ne $RootSessionId) { continue }
            if ($null -eq $child.CreationDate) { throw 'Cannot verify fixture child creation time' }
            # Windows retains the original parent PID even after that PID is reused.
            if ((Get-AutomexiaProcessBirth $child.CreationDate) -lt $parentBirth) { continue }
            if ($owned.Add([int]$child.ProcessId)) { $queue.Enqueue([int]$child.ProcessId) }
        }
    }
    foreach ($id in @($owned | Sort-Object)) {
        if ($id -ne $RootProcessId) { $rows[$id] }
    }
}

function Get-AutomexiaOwnedProcessRecords {
    param([Diagnostics.Process]$RootProcess, [string]$FixtureConfigRoot,
        [Nullable[datetime]]$FixtureStartTime = $null)
    [void]$RootProcess.Handle
    if ($RootProcess.HasExited) { return }
    $rootStartTime = $RootProcess.StartTime
    $rootSessionId = $RootProcess.SessionId
    if ($null -eq $FixtureStartTime) { $FixtureStartTime = $rootStartTime }
    $snapshot = @(Get-CimInstance Win32_Process |
        Select-Object ProcessId, ParentProcessId, CreationDate, SessionId, CommandLine)
    if ($RootProcess.Id -notin @($snapshot.ProcessId) -and $RootProcess.HasExited) { return }
    Select-AutomexiaOwnedProcessRecords $snapshot $RootProcess.Id $rootStartTime $rootSessionId $FixtureConfigRoot $FixtureStartTime
}

function Get-AutomexiaOwnedProcessIds {
    param([Diagnostics.Process]$RootProcess, [string]$FixtureConfigRoot,
        [Nullable[datetime]]$FixtureStartTime = $null)
    # IDs are for diagnostics and counts only. Teardown uses pinned objects below.
    Get-AutomexiaOwnedProcessRecords $RootProcess $FixtureConfigRoot $FixtureStartTime | ForEach-Object { [int]$_.ProcessId }
}

function Open-AutomexiaProcess {
    param([int]$ProcessId)
    return [Diagnostics.Process]::GetProcessById($ProcessId)
}

function Get-AutomexiaPinnedProcessRecords {
    param([object[]]$Records)
    if ($Records.Count -gt 512) { throw 'Fixture process handles exceed their limit' }
    $pinned = [Collections.Generic.List[object]]::new()
    try {
        foreach ($record in $Records) {
            $candidate = $null
            $keep = $false
            try {
                $candidate = Open-AutomexiaProcess ([int]$record.ProcessId)
                # Opening Handle makes Kill/HasExited use this kernel object,
                # even if the original process exits before cleanup.
                [void]$candidate.Handle
                if ($candidate.HasExited -or
                    (Get-AutomexiaProcessBirth $candidate.StartTime) -ne
                    (Get-AutomexiaProcessBirth $record.CreationDate)) { continue }
                $pinned.Add($candidate)
                $keep = $true
            } catch [ArgumentException] {
                # The process exited between the CIM snapshot and handle open.
            } catch [InvalidOperationException] {
                # The pinned process already exited before its times were read.
            } finally {
                if ($null -ne $candidate -and -not $keep) { $candidate.Dispose() }
            }
        }
        return $pinned.ToArray()
    } catch {
        foreach ($candidate in $pinned) { $candidate.Dispose() }
        throw
    }
}

function Get-AutomexiaPinnedOwnedProcesses {
    param([Diagnostics.Process]$RootProcess, [string]$FixtureConfigRoot,
        [Nullable[datetime]]$FixtureStartTime = $null)
    $records = @(Get-AutomexiaOwnedProcessRecords $RootProcess $FixtureConfigRoot $FixtureStartTime)
    Get-AutomexiaPinnedProcessRecords $records
}

function Stop-AutomexiaPinnedProcesses {
    param([object[]]$Processes)
    $failed = 0
    $timer = [Diagnostics.Stopwatch]::StartNew()
    try {
        foreach ($owned in $Processes) {
            try { if (-not $owned.HasExited) { $owned.Kill() } }
            catch {
                # Termination can race natural exit. The wait below still
                # requires this exact pinned process to exit within the budget.
            }
        }
        foreach ($owned in $Processes) {
            $remaining = [Math]::Max(0, 5000 - $timer.ElapsedMilliseconds)
            if (-not $owned.WaitForExit([int]$remaining)) { $failed++ }
        }
    } finally {
        foreach ($owned in $Processes) { $owned.Dispose() }
    }
    if ($failed -ne 0) { throw 'Could not retire every pinned fixture process' }
}
