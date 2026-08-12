$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

$parseFailures = [System.Collections.Generic.List[string]]::new()
Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.ps1' |
    Where-Object { $_.FullName -notmatch '[\\/](?:target|\.git)[\\/]' } |
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

& (Join-Path $PSScriptRoot 'test_shell_integration.ps1')
Write-Host 'PASS: all repository PowerShell sources parse and the integration contract passes'
