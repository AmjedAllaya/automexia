[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Arguments
)

$ErrorActionPreference = 'Stop'
$showHidden = $false
$paths = New-Object System.Collections.Generic.List[string]
foreach ($argument in $Arguments) {
    switch -Regex ($argument) {
        '^-(?:a|l|al|la)$' { if ($argument -match 'a') { $showHidden = $true }; continue }
        '^--all$' { $showHidden = $true; continue }
        '^--help$' {
            Write-Output 'Usage: ls [-a|-l|-la] [path ...]'
            Write-Output 'Automexia CMD listing: category icon and name remain together.'
            exit 0
        }
        default { $paths.Add($argument) }
    }
}

$formatPath = Join-Path $PSScriptRoot 'automexia.format.ps1xml'
if (-not (Test-Path -LiteralPath $formatPath)) {
    # Repository-source layout; installation flattens these files.
    $formatPath = Join-Path $PSScriptRoot '..\powershell\automexia.format.ps1xml'
}
if (Test-Path -LiteralPath $formatPath) {
    Update-FormatData -PrependPath $formatPath -ErrorAction Stop
}

$parameters = @{ Force = $showHidden }
if ($paths.Count -gt 0) {
    # -Path deliberately retains the familiar wildcard behavior of `ls`.
    $parameters.Path = $paths.ToArray()
}

Get-ChildItem @parameters | Format-Table
