param([int]$HistoryLines = 0)
$ErrorActionPreference = 'Stop'
Import-Module PSReadLine -ErrorAction Stop
# Never read or write the contributor's shell history. SaveNothing leaves this
# unique nonexistent test-local path unused; no profile or temp file is created.
$historyPath = Join-Path $PSScriptRoot ('.editor-history-' + [guid]::NewGuid().ToString('N'))
Set-PSReadLineOption -HistorySaveStyle SaveNothing -HistorySavePath $historyPath -EditMode Windows -BellStyle None
$global:EditorResizeAck = 0
Set-PSReadLineKeyHandler -Chord F12 -ScriptBlock {
    $line = ''
    $cursor = 0
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    $global:EditorResizeAck++
    # Query native editor/console state without repainting, accepting or executing input.
    $ack = 'EDITOR-ACK-{0}:{1}:{2}:{3}:{4}:{5}:{6}' -f $global:EditorResizeAck,
        ([Console]::CursorTop - [Console]::WindowTop), [Console]::CursorLeft,
        [Console]::WindowWidth, [Console]::WindowHeight, $line.Length, $cursor
    [Console]::Write(([char]27).ToString() + ']2;' + $ack + [char]7)
}
function global:prompt {
    $esc = [char]27
    [Console]::Write("${esc}]133;A;aid=1`a `r`n${esc}]133;P;k=c;aid=1`a/example`r`n${esc}]133;P;k=c;aid=1`a")
    return "lambda ${esc}]133;B`a"
}
for ($row = 0; $row -lt $HistoryLines; $row++) { [Console]::Write("retained output`r`n") }
$script:OriginalReadLine = (Get-Command PSConsoleHostReadLine).ScriptBlock
function global:PSConsoleHostReadLine {
    [Console]::Write(([char]27).ToString() + ']2;EDITOR-READY' + [char]7)
    $line = & $script:OriginalReadLine
    if ($line -cne 'aaaaaa') { [Environment]::Exit(17) }
    [Console]::Write(([char]27).ToString() + ']133;C' + [char]7)
    [Console]::Write("ACCEPTED`r`n")
    [Environment]::Exit(0)
}
