param(
    [switch]$SyntaxOnly
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

$parseFailures = [System.Collections.Generic.List[string]]::new()
Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.ps1' |
    # Windows wildcard matching treats `*.ps1` as a prefix match and can also
    # return signed format files such as `*.ps1xml`. Require the exact source
    # extension before invoking the PowerShell parser; repository XML
    # validation covers PS1XML files separately.
    Where-Object {
        $_.Extension -eq '.ps1' -and
        $_.FullName -notmatch '[\\/](?:target|\.git)[\\/]'
    } |
    ForEach-Object {
        $tokens = $null
        $errors = $null
        [void][System.Management.Automation.Language.Parser]::ParseFile(
            $_.FullName,
            [ref]$tokens,
            [ref]$errors)
        foreach ($parseError in @($errors)) {
            $relative = $_.FullName.Substring($root.Length).TrimStart('\', '/')
            $parseFailures.Add(
                ('{0}:{1}:{2}: {3}' -f
                    $relative,
                    $parseError.Extent.StartLineNumber,
                    $parseError.Extent.StartColumnNumber,
                    $parseError.Message))
        }
    }

if ($parseFailures.Count -ne 0) {
    throw "PowerShell syntax validation failed:`n$($parseFailures -join "`n")"
}

if ($SyntaxOnly) {
    Write-Host 'PASS: all repository PowerShell sources parse'
    exit 0
}

# The production integration deliberately emits OSC identity bytes. Capture a
# real child workflow so CI proves the bytes exist without publishing host
# metadata in logs or changing the parent's console writer.
$integrationScript = Join-Path $PSScriptRoot 'test_shell_integration.ps1'
$integrationLifecycle = & powershell.exe -NoLogo -NoProfile -NonInteractive `
    -File $integrationScript 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) {
    throw 'PowerShell integration contract failed in the captured child process'
}
if ($integrationLifecycle -notmatch 'SetUserVar=automexia_shell_user=' -or
    $integrationLifecycle -notmatch 'SetUserVar=automexia_shell_path=') {
    throw 'PowerShell integration did not emit the captured identity contract'
}
$integrationLifecycle = $null
Write-Host 'PASS: all repository PowerShell sources parse and the integration contract passes'
