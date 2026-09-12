$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
if ($env:AMX_TEST_CASE -eq 'function') { function global:amx { 'AMX_EXISTING_OK' } }
if ($env:AMX_TEST_CASE -eq 'alias') {
    function global:Invoke-AmxFixture { 'AMX_EXISTING_OK' }
    Set-Alias amx Invoke-AmxFixture -Scope Global
}
if ($env:AMX_TEST_CASE -eq 'disabled') { $env:AUTOMEXIA_AMX = '0' }
. $env:AMX_TEST_SOURCE
. $env:AMX_TEST_SOURCE
[Console]::WriteLine('AMX_OUTPUT_BEGIN')
if ($env:AMX_TEST_CASE -eq 'disabled') {
    if (Test-Path Function:amx) { throw 'disabled alias exists' }
    'AMX_DISABLED_OK'
} elseif ($env:AMX_TEST_CASE -in @('function', 'alias', 'external')) {
    amx
} elseif ($env:AMX_TEST_CASE -eq 'local') {
    $captured = @(amx find file Dockerfile)
    if ($LASTEXITCODE -ne 0 -or $captured.Count -ne 1) { throw 'local search failed' }
    $captured
} elseif ($env:AMX_TEST_CASE -eq 'directory') {
    $captured = @(amx open --preview ('folder & Unicode-' + [char]0xe9))
    if ($LASTEXITCODE -ne 0 -or $captured.Count -ne 1) { throw 'directory preview failed' }
    $captured
} elseif ($env:AMX_TEST_CASE -eq 'missing') {
    $env:AUTOMEXIA_CLI = 'missing-fixture-executable'
    try { amx google fixture } catch { 'AMX_MISSING_OK' }
} elseif ($env:AMX_TEST_CASE -eq 'searches') {
    foreach ($provider in @('google', 'github', 'youtube', 'ddg')) {
        $captured = @(amx search $provider --print-url 'fixture & query')
        if ($LASTEXITCODE -ne 0 -or $captured.Count -ne 1) { throw 'search preview failed' }
        $captured
    }
    $captured = @(amx docs kubernetes --print-url 'fixture & query')
    if ($LASTEXITCODE -ne 0 -or $captured.Count -ne 1) { throw 'docs preview failed' }
    $captured
} else {
    # Capture must work as with an ordinary CLI, not only paint to the host.
    $captured = @(amx google --print-url ('caf' + [char]0xe9 + ' & rust') '+#%' 'two words')
    if ($LASTEXITCODE -ne 0) { throw 'native CLI failed' }
    if ($captured.Count -ne 1) { throw 'native preview was not returned to the pipeline' }
    $captured
}
