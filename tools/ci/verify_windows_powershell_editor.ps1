param(
    [Parameter(Mandatory = $true, ParameterSetName = 'Verify')][string]$Root,
    [Parameter(ParameterSetName = 'Verify')][switch]$Load,
    [Parameter(Mandatory = $true, ParameterSetName = 'Paths')][switch]$ModulePathOnly
)
$ErrorActionPreference = 'Stop'
if ($ModulePathOnly) {
    [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
    [Console]::Write($env:PSModulePath)
    exit 0
}
. (Join-Path $PSScriptRoot 'windows_conpty_trust.ps1')
Assert-PowerShellEditorRuntime -Root $Root | Out-Null
if ($Load) {
    $module = Import-Module PSReadLine -PassThru -ErrorAction Stop
    if ($module.Version.ToString() -cne '2.4.5') { throw 'Pinned PowerShell editor was not selected' }
    # A private module must not hide standard Windows PowerShell modules.
    Get-Command Get-ChildItem, Get-Date, Get-ExecutionPolicy -ErrorAction Stop | Out-Null
}
Write-Output 'PASS: original Microsoft editor hashes and signatures verified'
