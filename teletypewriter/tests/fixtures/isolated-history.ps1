param([Parameter(Mandatory = $true)][string]$HistoryPath)

$ErrorActionPreference = 'Stop'
Import-Module PSReadLine
# Do not read or append the contributor's persistent history. Explicit Emacs
# mode makes the EOF assertion independent of Windows editing-mode defaults.
Set-PSReadLineOption -HistorySavePath $HistoryPath -HistorySaveStyle SaveNothing -EditMode Emacs
[Microsoft.PowerShell.PSConsoleReadLine]::ClearHistory()
function global:prompt { 'AMX_PTY_PROMPT> ' }
