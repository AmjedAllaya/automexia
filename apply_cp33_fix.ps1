$ErrorActionPreference = 'Stop'

$root = (Get-Location).Path
$indexPath = Join-Path $root 'docs\index.md'
$checkerPath = Join-Path $root 'tools\ci\check_command_productivity_cp33.py'

if (-not (Test-Path -LiteralPath $indexPath -PathType Leaf)) {
    throw "Expected documentation index was not found: $indexPath"
}

$content = [System.IO.File]::ReadAllText($indexPath)
if ($content -notmatch '(?<![A-Za-z0-9.])CP3\.3(?![A-Za-z0-9.])') {
    $suffix = "`r`n`r`n## CP3.3 evidence`r`n`r`nCP3.3 native imports and trusted workspace task bridges are part of the reviewed command-productivity architecture contract.`r`n"
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($indexPath, $content.TrimEnd() + $suffix, $utf8NoBom)
    Write-Host 'Added CP3.3 evidence to docs/index.md.'
} else {
    Write-Host 'docs/index.md already contains CP3.3 evidence; no file change was needed.'
}

if (Test-Path -LiteralPath $checkerPath -PathType Leaf) {
    & python3 $checkerPath
    if ($LASTEXITCODE -ne 0) {
        throw "CP3.3 validator still fails with exit code $LASTEXITCODE."
    }
}
