# Automexia CP5.5 preview adapter. Session-only; never source from a profile.
Set-StrictMode -Version 3

$script:AutomexiaSuggestionState = 'disabled'
$script:AutomexiaSuggestionReason = 'preview-disabled'
$script:AutomexiaSuggestionChord = $null
$script:AutomexiaSuggestionRequestStream = $null

function Get-AutomexiaSuggestionHealth {
    [pscustomobject]@{
        State = $script:AutomexiaSuggestionState
        Reason = $script:AutomexiaSuggestionReason
        Bound = $null -ne $script:AutomexiaSuggestionChord
        Fallback = 'PSReadLine-native'
    }
}

function Send-AutomexiaSuggestionRequest {
    if ($script:AutomexiaSuggestionState -notin @('ready', 'ready-unbound')) { return }
    $line = $null
    $cursor = 0
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    $bytes = [Text.Encoding]::UTF8.GetBytes($line)
    $cursorBytes = [Text.Encoding]::UTF8.GetByteCount($line.Substring(0, $cursor))
    if ($bytes.Length -gt 16384) {
        $script:AutomexiaSuggestionReason = 'buffer-limit'
        return
    }
    try {
        $matches = @(TabExpansion2 -inputScript $line -cursorColumn $cursor).CompletionMatches |
            Select-Object -First 512
        $payloadStream = [IO.MemoryStream]::new()
        $payload = [IO.BinaryWriter]::new($payloadStream, [Text.Encoding]::UTF8, $true)
        $payload.Write([uint32]$bytes.Length)
        $payload.Write($bytes)
        $payload.Write([uint32]$cursorBytes)
        $payload.Write([uint32]$matches.Count)
        foreach ($match in $matches) {
            $candidate = [Text.Encoding]::UTF8.GetBytes([string]$match.CompletionText)
            if ($candidate.Length -le 1024 -and
                -not ([string]$match.CompletionText).Contains("`n") -and
                -not ([string]$match.CompletionText).Contains("`r")) {
                $payload.Write([uint32]$candidate.Length)
                $payload.Write($candidate)
            } else {
                $payload.Write([uint32]0)
            }
        }
        $payload.Flush()
        if ($payloadStream.Length -gt 524288) { throw 'suggestion batch exceeds limit' }
        $writer = [IO.BinaryWriter]::new(
            $script:AutomexiaSuggestionRequestStream,
            [Text.Encoding]::UTF8,
            $true
        )
        $writer.Write([Text.Encoding]::ASCII.GetBytes('AXSH'))
        $writer.Write([uint16]1)
        $writer.Write([byte]1)
        $writer.Write([uint32]$payloadStream.Length)
        $writer.Write($payloadStream.ToArray())
        $writer.Flush()
        $payload.Dispose()
        $payloadStream.Dispose()
    } catch {
        Disable-AutomexiaSuggestions
        $script:AutomexiaSuggestionReason = 'helper-disconnected'
    }
}

function Enable-AutomexiaSuggestions {
    [CmdletBinding()]
    param([string]$Chord)

    Disable-AutomexiaSuggestions
    if ($env:AUTOMEXIA_SUGGESTION_PREVIEW -ne '1' -or
        [string]::IsNullOrWhiteSpace($env:AUTOMEXIA_SUGGESTION_REQUEST_HANDLE)) {
        $script:AutomexiaSuggestionReason = 'preview-disabled'
        return $false
    }
    if ($PSVersionTable.PSVersion -lt [version]'7.2') {
        $script:AutomexiaSuggestionReason = 'unsupported-powershell'
        return $false
    }
    $module = Get-Module PSReadLine
    if ($null -eq $module -or $module.Version -lt [version]'2.2.2') {
        $script:AutomexiaSuggestionReason = 'unsupported-psreadline'
        return $false
    }
    try {
        $script:AutomexiaSuggestionRequestStream = [IO.Pipes.AnonymousPipeClientStream]::new(
            [IO.Pipes.PipeDirection]::Out,
            $env:AUTOMEXIA_SUGGESTION_REQUEST_HANDLE
        )
    } catch {
        $script:AutomexiaSuggestionReason = 'helper-unavailable'
        return $false
    }
    $script:AutomexiaSuggestionState = 'ready-unbound'
    $script:AutomexiaSuggestionReason = 'none'
    if ([string]::IsNullOrWhiteSpace($Chord)) { return $true }
    if ($null -ne (Get-PSReadLineKeyHandler -Chord $Chord -ErrorAction SilentlyContinue)) {
        Disable-AutomexiaSuggestions
        $script:AutomexiaSuggestionReason = 'binding-collision'
        return $false
    }
    Set-PSReadLineKeyHandler -Chord $Chord -BriefDescription AutomexiaSuggestions `
        -LongDescription 'Request local Automexia suggestions' `
        -ScriptBlock { Send-AutomexiaSuggestionRequest }
    $script:AutomexiaSuggestionChord = $Chord
    $script:AutomexiaSuggestionState = 'ready'
    return $true
}

function Disable-AutomexiaSuggestions {
    if ($null -ne $script:AutomexiaSuggestionChord) {
        $handler = Get-PSReadLineKeyHandler -Chord $script:AutomexiaSuggestionChord `
            -ErrorAction SilentlyContinue
        if ($null -ne $handler -and $handler.BriefDescription -eq 'AutomexiaSuggestions') {
            Remove-PSReadLineKeyHandler -Chord $script:AutomexiaSuggestionChord
        }
    }
    if ($null -ne $script:AutomexiaSuggestionRequestStream) {
        $script:AutomexiaSuggestionRequestStream.Dispose()
    }
    $script:AutomexiaSuggestionChord = $null
    $script:AutomexiaSuggestionRequestStream = $null
    $script:AutomexiaSuggestionState = 'disabled'
    if ($script:AutomexiaSuggestionReason -eq 'none') {
        $script:AutomexiaSuggestionReason = 'disabled'
    }
}
