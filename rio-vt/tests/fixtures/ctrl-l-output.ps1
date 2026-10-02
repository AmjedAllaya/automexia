$ErrorActionPreference = 'Stop'
$escape = [char]27
$bell = [char]7
[Console]::Write("${escape}]133;A${bell}${escape}]133;B${bell}${escape}]133;C${bell}")
for ($index = 0; $index -lt 40; $index++) {
    [Console]::Write("${escape}[48;2;8;48;38mclear-fixture-row-$index${escape}[K${escape}[0m`r`n")
}
[Console]::Write("${escape}]133;D;0${bell}")
[Console]::WriteLine('CLEAR-FIXTURE-READY')
$env:AUTOMEXIA_SHELL_INTEGRATION = '1'
. (Join-Path $PSScriptRoot '..\..\..\shell-integration\powershell\automexia.ps1')
