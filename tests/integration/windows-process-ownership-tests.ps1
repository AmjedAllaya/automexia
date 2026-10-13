param([switch]$ModelOnly, [switch]$Child)
$ErrorActionPreference = 'Stop'
if ($Child) {
    if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Native process fixture requires a disposable hosted runner' }
    Start-Sleep -Seconds 30
    return
}
. (Join-Path $PSScriptRoot 'windows-process-ownership.ps1')
function Assert-Owned($Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
function New-Row([int]$Id, [int]$Parent, [int]$Minute, [string]$Command = '', [int]$Session = 1) {
    [pscustomobject]@{ ProcessId=$Id; ParentProcessId=$Parent; SessionId=$Session;
        CreationDate=([datetime]'2026-01-01T00:00:00Z').AddMinutes($Minute); CommandLine=$Command }
}
$fixture = 'C:\qa-fixture\config'
$rootRow = New-Row 100 10 10
$rows = @($rootRow, (New-Row 200 100 5), (New-Row 300 100 11),
    (New-Row 400 300 12), (New-Row 500 300 10),
    (New-Row 600 999 11 ('shell "' + $fixture + '\child.ps1"')),
    (New-Row 700 600 12), (New-Row 800 999 11 ('shell "' + $fixture + '-other\child.ps1"')),
    (New-Row 900 999 5 ('shell "' + $fixture + '\child.ps1"')), (New-Row 1000 100 11 '' 2))
$selected = @(Select-AutomexiaOwnedProcessRecords $rows 100 $rootRow.CreationDate 1 $fixture)
Assert-Owned ((@($selected.ProcessId) -join ',') -eq '300,400,600,700') 'Stale ancestry, session or fixture-path ownership regression'
$restartedRoot = New-Row 100 10 20
$priorGeneration = New-Row 600 999 15 ('shell "' + $fixture + '\child.ps1"')
$selected = @(Select-AutomexiaOwnedProcessRecords @($restartedRoot, $priorGeneration) 100 $restartedRoot.CreationDate 1 $fixture $rootRow.CreationDate)
Assert-Owned ($selected.Count -eq 1 -and $selected[0].ProcessId -eq 600) 'Fixture ownership was lost across a recovery restart'
foreach ($badRoot in @($rootRow.CreationDate.AddSeconds(1), $rootRow.CreationDate.AddSeconds(-1))) {
    $rejected = $false
    try { Select-AutomexiaOwnedProcessRecords $rows 100 $badRoot 1 $fixture | Out-Null }
    catch { $rejected = $_.Exception.Message -eq 'Fixture root process identity changed' }
    Assert-Owned $rejected 'Reused root PID was accepted'
}
$unknownChild = New-Row 1100 100 11
$unknownChild.CreationDate = $null
$rejected = $false
try { Select-AutomexiaOwnedProcessRecords @($rootRow, $unknownChild) 100 $rootRow.CreationDate 1 '' | Out-Null }
catch { $rejected = $_.Exception.Message -eq 'Cannot verify fixture child creation time' }
Assert-Owned $rejected 'Unknown child identity was silently accepted'
$rejected = $false
try { Select-AutomexiaOwnedProcessRecords @() 100 $rootRow.CreationDate 1 '' | Out-Null }
catch { $rejected = $_.Exception.Message -eq 'Fixture root process identity changed' }
Assert-Owned $rejected 'Missing owner was treated as an empty verified tree'
$oversized = @($rootRow) + @(1..513 | ForEach-Object { New-Row (2000 + $_) 100 11 })
$rejected = $false
try { Select-AutomexiaOwnedProcessRecords $oversized 100 $rootRow.CreationDate 1 '' | Out-Null }
catch { $rejected = $_.Exception.Message -eq 'Fixture process tree exceeds its limit' }
Assert-Owned $rejected 'Process ownership limit was not enforced'

function New-FakeProcess([int]$Id, [datetime]$Born, [bool]$Exited = $false) {
    $value = [pscustomobject]@{ Id=$Id; Born=$Born; HasExited=$Exited; HandleReads=0; Kills=0; Disposed=$false }
    $value | Add-Member ScriptProperty Handle { $this.HandleReads++; return [IntPtr]1 }
    $value | Add-Member ScriptProperty StartTime {
        if ($this.HandleReads -eq 0) { throw 'Identity read before handle pin' }
        return $this.Born
    }
    $value | Add-Member ScriptMethod Kill { $this.Kills++; $this.HasExited=$true }
    $value | Add-Member ScriptMethod WaitForExit { param($milliseconds) return $this.HasExited }
    $value | Add-Member ScriptMethod Dispose { $this.Disposed=$true }
    return $value
}
$realOpener = (Get-Item Function:Open-AutomexiaProcess).ScriptBlock
$script:fixtureProcesses = @{}
function Open-AutomexiaProcess([int]$ProcessId) {
    if (-not $script:fixtureProcesses.ContainsKey($ProcessId)) { throw [ArgumentException]::new('Fixture already exited') }
    $value = $script:fixtureProcesses[$ProcessId]
    if ($value -is [Exception]) { throw $value }
    return $value
}
try {
    $record = New-Row 300 100 11
    $replacement = New-FakeProcess 300 $record.CreationDate.AddSeconds(1)
    $script:fixtureProcesses[300] = $replacement
    $pinned = @(Get-AutomexiaPinnedProcessRecords @($record))
    Assert-Owned ($pinned.Count -eq 0 -and $replacement.Disposed -and $replacement.Kills -eq 0) 'Snapshot PID replacement was adopted'
    $original = New-FakeProcess 300 $record.CreationDate
    $script:fixtureProcesses[300] = $original
    $pinned = @(Get-AutomexiaPinnedProcessRecords @($record))
    Assert-Owned ($pinned.Count -eq 1 -and $original.HandleReads -gt 0) 'Live owned process was not pinned'
    $later = New-FakeProcess 300 $record.CreationDate.AddSeconds(2)
    $script:fixtureProcesses[300] = $later
    Stop-AutomexiaPinnedProcesses $pinned
    Assert-Owned ($original.Kills -eq 1 -and $original.Disposed -and $later.Kills -eq 0) 'Cleanup rediscovered a PID instead of using its pinned owner'
    $exited = New-FakeProcess 300 $record.CreationDate $true
    $script:fixtureProcesses[300] = $exited
    Assert-Owned (@(Get-AutomexiaPinnedProcessRecords @($record)).Count -eq 0 -and $exited.Disposed) 'Exited process handle was retained'
    $script:fixtureProcesses.Remove(300)
    Assert-Owned (@(Get-AutomexiaPinnedProcessRecords @($record)).Count -eq 0) 'Exit during handle open was not handled'
    $original = New-FakeProcess 300 $record.CreationDate
    $script:fixtureProcesses[300] = $original
    $script:fixtureProcesses[400] = [ComponentModel.Win32Exception]::new(5)
    $rejected = $false
    try { Get-AutomexiaPinnedProcessRecords @($record, (New-Row 400 300 12)) | Out-Null }
    catch { $rejected = $true }
    Assert-Owned ($rejected -and $original.Disposed -and $original.Kills -eq 0) 'Failed acquisition leaked handles or hid access failure'
    $stuck = New-FakeProcess 300 $record.CreationDate
    $stuck | Add-Member ScriptMethod Kill { throw 'Synthetic termination failure' } -Force
    $rejected = $false
    try { Stop-AutomexiaPinnedProcesses @($stuck) } catch { $rejected = $true }
    Assert-Owned ($rejected -and $stuck.Disposed) 'Live cleanup failure was hidden or leaked its handle'
    $racing = New-FakeProcess 300 $record.CreationDate
    $racing | Add-Member ScriptMethod Kill { throw 'Synthetic concurrent exit' } -Force
    $racing | Add-Member ScriptMethod WaitForExit { param($milliseconds) $this.HasExited=$true; return $true } -Force
    Stop-AutomexiaPinnedProcesses @($racing)
    Assert-Owned $racing.Disposed 'Concurrent natural exit leaked its handle'
} finally { Set-Item Function:Open-AutomexiaProcess $realOpener }
Write-Host 'PASS: process ownership model, stale ancestry, PID replacement, bounded handles and failure cleanup'
if ($ModelOnly) { return }
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Native process fixture requires a disposable hosted runner' }
$self = [Diagnostics.Process]::GetCurrentProcess()
$childProcess = $null
try {
    [void]$self.Handle
    $nativePowerShell = Join-Path ([Environment]::GetFolderPath('System')) 'WindowsPowerShell\v1.0\powershell.exe'
    $childProcess = Start-Process -FilePath $nativePowerShell -ArgumentList @(
        '-NoProfile', '-File', ('"{0}"' -f $PSCommandPath), '-Child') -WindowStyle Hidden -PassThru
    [void]$childProcess.Handle
    $owned = @(Get-AutomexiaPinnedOwnedProcesses $self '')
    Assert-Owned ($childProcess.Id -in @($owned.Id)) 'Native child was not discovered and pinned'
    Stop-AutomexiaPinnedProcesses $owned
    Assert-Owned ($childProcess.WaitForExit(5000)) 'Native pinned cleanup did not retire its child'
    Write-Host 'PASS: native Windows child discovery, pinned termination and handle disposal'
} finally {
    if ($null -ne $childProcess) { Stop-AutomexiaPinnedProcesses @($childProcess) }
    $self.Dispose()
}
