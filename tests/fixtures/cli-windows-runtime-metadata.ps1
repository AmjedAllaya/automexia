param(
    [Parameter(Mandatory = $true)][string]$Application,
    [Parameter(Mandatory = $true)][string]$Launcher,
    [Parameter(Mandatory = $true)][string]$SuggestionHelper,
    [Parameter(Mandatory = $true)][string]$ExpectedVersion
)

$ErrorActionPreference = 'Stop'
foreach ($binary in @($Application, $Launcher, $SuggestionHelper)) {
    $metadata = [Diagnostics.FileVersionInfo]::GetVersionInfo($binary)
    if ($metadata.ProductVersion -cne $ExpectedVersion -or
        $metadata.ProductName -cne 'Automexia Terminal') {
        throw ('Missing or incorrect package identity: ' + [IO.Path]::GetFileName($binary))
    }
}
Write-Output 'PASS: all packaged Windows runtimes have product/version resources'
