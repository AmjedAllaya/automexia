param([switch]$WideTable)
$ErrorActionPreference = 'Stop'
$esc = [char]27
[Console]::Write("${esc}]133;A;aid=1`a`r`n${esc}]133;P;k=c;aid=1`a/example`r`n${esc}]133;P;k=c;aid=1`alambda ${esc}]133;B`alist`r`n${esc}]133;C`a")
if ($WideTable) {
    1..32 | ForEach-Object {
        [Console]::Write("ROW-{0:D2}  -a---  2026-01-01 12:00:00  {0:D4}  artifact-{0:D2}-abcdefghijklmnopqrstuvwxyz0123456789.txt`r`n" -f $_)
    }
} else {
    1..8 | ForEach-Object { [Console]::Write("ROW-{0:D2}  retained output`r`n" -f $_) }
}
[Console]::Write("${esc}]133;D;0`a${esc}]133;A;aid=2`a`r`n${esc}]133;P;k=c;aid=2`a/example`r`n${esc}]133;P;k=c;aid=2`alambda ${esc}]133;B`a")
& "$PSScriptRoot/live-resize-ack.ps1" -ReadExitKey
