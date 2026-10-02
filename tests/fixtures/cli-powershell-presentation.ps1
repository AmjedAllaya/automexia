$ErrorActionPreference = 'Stop'
$env:TERM_PROGRAM = 'Automexia'
$env:AUTOMEXIA_SHELL_INTEGRATION = '1'
$env:AUTOMEXIA_PLAIN_LS = '1'
Remove-Item Env:COLUMNS, Env:LINES -ErrorAction SilentlyContinue
. $env:AMX_TEST_SOURCE
[Console]::WriteLine('AMX_NATIVE_BEGIN')
switch ($env:AMX_PRESENTATION_CASE) {
    'help' { amx --help }
    'logo' { amx logo }
    'about' { amx about }
    default { throw 'unknown presentation case' }
}
if ($LASTEXITCODE -ne 0) { throw 'native amx command failed' }
[Console]::WriteLine('AMX_NATIVE_DONE')
