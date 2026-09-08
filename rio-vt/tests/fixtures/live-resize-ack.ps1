param([switch]$ReadExitKey, [switch]$CumulativeReceipt)
$ErrorActionPreference = 'Stop'
$esc = [char]27
[Console]::Write("${esc}]2;RESIZE-READY`a")
# Buffered, non-echoing input preserves output and cursor position even when
# probes arrive before ReadKey starts. Acknowledgments never add screen rows.
$observed = ''
for ($step = 0; $step -lt 12; $step++) {
    $key = [Console]::ReadKey($true)
    if ($CumulativeReceipt) { $observed += "RESIZE-PROBE-${step}=$([int]$key.KeyChar);" }
    # A cumulative receipt preserves every consumed key and its ordinal even
    # when ConPTY collapses intermediate title updates in one native repaint.
    [Console]::Write("${esc}]2;${observed}RESIZE-ACK-${step}`a")
}
if ($ReadExitKey) { $null = [Console]::ReadKey($true) }
