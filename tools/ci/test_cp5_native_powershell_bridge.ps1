# Native Windows checks for the inert CP5 PowerShell/PSReadLine bridge.
[CmdletBinding()]
param()

Set-StrictMode -Version 3
$ErrorActionPreference = 'Stop'

function Assert-Cp5 {
    param([Parameter(Mandatory)][bool]$Condition, [Parameter(Mandatory)][string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Cp5Throws {
    param([Parameter(Mandatory)][scriptblock]$Action, [Parameter(Mandatory)][string]$Message)
    try {
        & $Action
    } catch {
        return
    }
    throw $Message
}

$adapter = Join-Path $PSScriptRoot '../../shell-integration/suggestions/powershell/automexia-suggestions.ps1'
$requestServer = $null
$responseServer = $null
$savedPreview = $env:AUTOMEXIA_SUGGESTION_PREVIEW
$savedRequest = $env:AUTOMEXIA_SUGGESTION_REQUEST_HANDLE
$savedResponse = $env:AUTOMEXIA_SUGGESTION_RESPONSE_HANDLE

try {
    . (Resolve-Path -LiteralPath $adapter)
    Import-Module PSReadLine -MinimumVersion 2.2.2 -ErrorAction Stop

    Remove-Item Env:AUTOMEXIA_SUGGESTION_PREVIEW -ErrorAction SilentlyContinue
    Remove-Item Env:AUTOMEXIA_SUGGESTION_REQUEST_HANDLE -ErrorAction SilentlyContinue
    Remove-Item Env:AUTOMEXIA_SUGGESTION_RESPONSE_HANDLE -ErrorAction SilentlyContinue
    Assert-Cp5 (-not (Enable-AutomexiaSuggestions)) 'missing preview authority must fail closed'
    Assert-Cp5 ((Get-AutomexiaSuggestionHealth).Reason -eq 'preview-disabled') 'preview failure reason changed'

    $requestServer = [IO.Pipes.AnonymousPipeServerStream]::new(
        [IO.Pipes.PipeDirection]::In,
        [IO.HandleInheritability]::Inheritable
    )
    $responseServer = [IO.Pipes.AnonymousPipeServerStream]::new(
        [IO.Pipes.PipeDirection]::Out,
        [IO.HandleInheritability]::Inheritable
    )
    $env:AUTOMEXIA_SUGGESTION_PREVIEW = '1'
    $env:AUTOMEXIA_SUGGESTION_REQUEST_HANDLE = $requestServer.GetClientHandleAsString()
    $env:AUTOMEXIA_SUGGESTION_RESPONSE_HANDLE = $responseServer.GetClientHandleAsString()
    Assert-Cp5 (Enable-AutomexiaSuggestions) 'valid inherited handles were rejected'
    $requestServer.DisposeLocalCopyOfClientHandle()
    $responseServer.DisposeLocalCopyOfClientHandle()
    $health = Get-AutomexiaSuggestionHealth
    Assert-Cp5 ($health.State -eq 'ready-unbound') 'unbound PowerShell bridge state changed'
    Assert-Cp5 ($health.Reason -eq 'none') 'healthy PowerShell bridge reason changed'
    Assert-Cp5 (-not $health.Bound) 'unbound activation unexpectedly changed a key binding'
    Disable-AutomexiaSuggestions
    $health = Get-AutomexiaSuggestionHealth
    Assert-Cp5 ($health.State -eq 'disabled') 'disable did not clear bridge state'
    Assert-Cp5 ($null -eq $script:AutomexiaSuggestionRequestStream) 'request handle leaked after disable'
    Assert-Cp5 ($null -eq $script:AutomexiaSuggestionResponseStream) 'response handle leaked after disable'
    Assert-Cp5 ($null -eq $script:AutomexiaSuggestionResponseReader) 'response reader leaked after disable'

    $script:AutomexiaSuggestionResponseReader = [IO.StringReader]::new("AXSR1`tS`t2`n")
    Read-AutomexiaSuggestionResponse -OriginalLine 'private' -OriginalCursor 7 `
        -Generation 1 -SpanStartBytes 0 -SpanEndBytes 7 `
        -ReplacementStart 0 -ReplacementLength 7
    Assert-Cp5 ($script:AutomexiaSuggestionReason -eq 'no-replacement') 'authenticated helper status was not handled'
    $script:AutomexiaSuggestionResponseReader.Dispose()
    $script:AutomexiaSuggestionResponseReader = $null

    $parsed = ConvertFrom-AutomexiaSuggestionResponseLine `
        -Response "AXSR1`tR`t1`t1`t0`t1`t636166C3A92DE282AC2DF09F9A80" `
        -Generation 1 -SpanStartBytes 0 -SpanEndBytes 1
    Assert-Cp5 ($parsed.Kind -eq 'replacement') 'valid replacement was not parsed'
    $expectedInsertion = "caf$([char]0x00E9)-$([char]0x20AC)-$([char]::ConvertFromUtf32(0x1F680))"
    Assert-Cp5 ($parsed.Insertion -eq $expectedInsertion) 'strict UTF-8 replacement changed'
    foreach ($hostileHex in @(
        '80', 'C0AF', 'C2', 'E08080', 'E282', 'EDA080', 'F0808080',
        'F09F92', 'F4908080', 'FF', '09', 'C280', 'D89C', 'E280AE'
    )) {
        Assert-Cp5Throws {
            ConvertFrom-AutomexiaSuggestionResponseLine `
                -Response "AXSR1`tR`t1`t1`t0`t1`t$hostileHex" `
                -Generation 1 -SpanStartBytes 0 -SpanEndBytes 1
        } 'pure response parser accepted invalid UTF-8, control, or bidi text'
    }

    foreach ($hostile in @(
        "AXSR1`tS`t9`n",
        "AXSR1`tS`t2`textra`n",
        ("AXSR1`tS`t" + ('2' * 2176) + "`n"),
        "AXSR1`tR`t1`t0`t0`t1`t61`n",
        ('X' * 2201) + "`n",
        "AXSR1`tR`t1`t1`t0`t1`t6g`n"
    )) {
        $script:AutomexiaSuggestionResponseReader = [IO.StringReader]::new($hostile)
        Assert-Cp5Throws {
            Read-AutomexiaSuggestionResponse -OriginalLine 'x' -OriginalCursor 1 `
                -Generation 1 -SpanStartBytes 0 -SpanEndBytes 1 `
                -ReplacementStart 0 -ReplacementLength 1
        } 'malformed or oversized helper reply was accepted'
        $script:AutomexiaSuggestionResponseReader.Dispose()
        $script:AutomexiaSuggestionResponseReader = $null
    }

    Write-Output 'PASS: native PowerShell preview/handle/status/cleanup bridge paths'
} finally {
    Disable-AutomexiaSuggestions -ErrorAction SilentlyContinue
    if ($null -ne $requestServer) { $requestServer.Dispose() }
    if ($null -ne $responseServer) { $responseServer.Dispose() }
    $env:AUTOMEXIA_SUGGESTION_PREVIEW = $savedPreview
    $env:AUTOMEXIA_SUGGESTION_REQUEST_HANDLE = $savedRequest
    $env:AUTOMEXIA_SUGGESTION_RESPONSE_HANDLE = $savedResponse
}
