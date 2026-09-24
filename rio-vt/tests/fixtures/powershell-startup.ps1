# Leave the real ConsoleHost prompt/read callbacks untouched. Only isolate
# history and provide the command that the native test explicitly types.
Import-Module PSReadLine -ErrorAction Stop
Set-PSReadLineOption -HistorySaveStyle SaveNothing -HistorySavePath ($env:AUTOMEXIA_CONFIG_HOME + '\unused-history') -BellStyle None -ErrorAction Stop
function global:Invoke-AutomexiaStartupProbe {
    [Console]::Write("STARTUP-COMMAND-ACCEPTED`r`n")
}
