# Automexia CP5.5 preview adapter. Session-only; never source from a profile.
Set-StrictMode -Version 3

$script:AutomexiaSuggestionState = 'disabled'
$script:AutomexiaSuggestionReason = 'preview-disabled'
$script:AutomexiaSuggestionChord = $null
$script:AutomexiaSuggestionRequestStream = $null
$script:AutomexiaSuggestionResponseStream = $null
$script:AutomexiaSuggestionResponseReader = $null
$script:AutomexiaSuggestionGeneration = [uint64]0

function Get-AutomexiaSuggestionHealth {
    [pscustomobject]@{
        State = $script:AutomexiaSuggestionState
        Reason = $script:AutomexiaSuggestionReason
        Bound = $null -ne $script:AutomexiaSuggestionChord
        Fallback = 'PSReadLine-native'
    }
}

function Read-AutomexiaBoundedResponseLine {
    if ($null -eq $script:AutomexiaSuggestionResponseReader) {
        throw 'helper response reader is unavailable'
    }
    $line = [Text.StringBuilder]::new(128)
    for ($index = 0; $index -le 2175; $index++) {
        $value = $script:AutomexiaSuggestionResponseReader.Read()
        if ($value -lt 0) { throw 'unterminated helper response' }
        if ($value -eq 10) { return $line.ToString() }
        if ($index -eq 2175) { throw 'helper response exceeds its byte limit' }
        if ($value -eq 13 -or $value -gt 127) { throw 'helper response is not strict ASCII' }
        [void]$line.Append([char]$value)
    }
    throw 'helper response exceeds its byte limit'
}
function ConvertFrom-AutomexiaSuggestionResponseLine {
    param(
        [Parameter(Mandatory)][string]$Response,
        [Parameter(Mandatory)][uint64]$Generation,
        [Parameter(Mandatory)][uint32]$SpanStartBytes,
        [Parameter(Mandatory)][uint32]$SpanEndBytes
    )

    $fields = $Response.Split("`t")
    if ($fields.Count -eq 3 -and $fields[0] -eq 'AXSR1' -and $fields[1] -eq 'S' -and
        $fields[2] -match '^[1-6]$') {
        return [pscustomobject]@{
            Kind = 'status'
            StatusCode = [byte]$fields[2]
        }
    }
    if ($fields.Count -ne 7 -or $fields[0] -ne 'AXSR1' -or $fields[1] -ne 'R') {
        throw 'invalid helper response'
    }
    $responseGeneration = [uint64]$fields[2]
    $requestId = [uint64]$fields[3]
    $responseStart = [uint32]$fields[4]
    $responseEnd = [uint32]$fields[5]
    $hex = $fields[6]
    if ($responseGeneration -ne $Generation -or $requestId -eq 0 -or
        $responseStart -ne $SpanStartBytes -or $responseEnd -ne $SpanEndBytes -or
        $hex.Length -eq 0 -or $hex.Length -gt 2048 -or
        $hex.Length % 2 -ne 0 -or $hex -notmatch '^[0-9A-F]+$') {
        throw 'stale or invalid helper replacement'
    }

    $strictUtf8 = [Text.UTF8Encoding]::new($false, $true)
    $insertion = $strictUtf8.GetString([Convert]::FromHexString($hex))
    if ([string]::IsNullOrEmpty($insertion) -or
        $insertion -match '[\p{Cc}\u061C\u200E\u200F\u202A-\u202E\u2066-\u2069]') {
        throw 'unsafe helper replacement'
    }
    [pscustomobject]@{
        Kind = 'replacement'
        RequestId = $requestId
        Insertion = $insertion
    }
}

function Read-AutomexiaSuggestionResponse {
    param(
        [Parameter(Mandatory)][string]$OriginalLine,
        [Parameter(Mandatory)][int]$OriginalCursor,
        [Parameter(Mandatory)][uint64]$Generation,
        [Parameter(Mandatory)][uint32]$SpanStartBytes,
        [Parameter(Mandatory)][uint32]$SpanEndBytes,
        [Parameter(Mandatory)][int]$ReplacementStart,
        [Parameter(Mandatory)][int]$ReplacementLength
    )

    $parsed = ConvertFrom-AutomexiaSuggestionResponseLine `
        -Response (Read-AutomexiaBoundedResponseLine) `
        -Generation $Generation -SpanStartBytes $SpanStartBytes `
        -SpanEndBytes $SpanEndBytes
    if ($parsed.Kind -eq 'status') {
        $script:AutomexiaSuggestionReason = 'no-replacement'
        return
    }

    $currentLine = $null
    $currentCursor = 0
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState(
        [ref]$currentLine,
        [ref]$currentCursor
    )
    if ($currentLine -cne $OriginalLine -or $currentCursor -ne $OriginalCursor) {
        $script:AutomexiaSuggestionReason = 'stale-editor-state'
        return
    }
    [Microsoft.PowerShell.PSConsoleReadLine]::Replace(
        $ReplacementStart,
        $ReplacementLength,
        $parsed.Insertion
    )
    [Microsoft.PowerShell.PSConsoleReadLine]::SetCursorPosition(
        $ReplacementStart + $parsed.Insertion.Length
    )
    $script:AutomexiaSuggestionReason = 'accepted'
}
function Send-AutomexiaSuggestionRequest {
    if ($script:AutomexiaSuggestionState -ne 'ready') { return }
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
        $completion = TabExpansion2 -inputScript $line -cursorColumn $cursor
        $replacementStart = [Math]::Max(0, [int]$completion.ReplacementIndex)
        $replacementLength = [Math]::Max(0, [int]$completion.ReplacementLength)
        $replacementEnd = $replacementStart + $replacementLength
        if ($replacementStart -gt $cursor -or $replacementEnd -lt $cursor -or
            $replacementEnd -gt $line.Length) {
            throw 'native completion returned an invalid replacement span'
        }
        $spanStartBytes = [Text.Encoding]::UTF8.GetByteCount(
            $line.Substring(0, $replacementStart)
        )
        $spanEndBytes = [Text.Encoding]::UTF8.GetByteCount(
            $line.Substring(0, $replacementEnd)
        )
        $candidateBytes = [Collections.Generic.List[byte[]]]::new()
        foreach ($match in @($completion.CompletionMatches) | Select-Object -First 512) {
            $text = [string]$match.CompletionText
            $candidate = [Text.Encoding]::UTF8.GetBytes($text)
            if ($candidate.Length -gt 0 -and $candidate.Length -le 1024 -and
                $text -notmatch '[\u0000-\u001f\u007f\r\n\u202a-\u202e\u2066-\u2069]') {
                $candidateBytes.Add($candidate)
            }
        }

        $script:AutomexiaSuggestionGeneration++
        if ($script:AutomexiaSuggestionGeneration -eq 0) { throw 'generation exhausted' }
        $payloadStream = [IO.MemoryStream]::new()
        $payload = [IO.BinaryWriter]::new($payloadStream, [Text.Encoding]::UTF8, $true)
        $payload.Write([uint32]$bytes.Length)
        $payload.Write($bytes)
        $payload.Write([uint32]$cursorBytes)
        $payload.Write([uint64]$script:AutomexiaSuggestionGeneration)
        $payload.Write([uint32]$spanStartBytes)
        $payload.Write([uint32]$spanEndBytes)
        $payload.Write([byte]0) # no selection
        $payload.Write([byte]5) # shell-specific quote context
        $payload.Write([uint32]$candidateBytes.Count)
        foreach ($candidate in $candidateBytes) {
            $payload.Write([uint32]$candidate.Length)
            $payload.Write($candidate)
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
        Read-AutomexiaSuggestionResponse -OriginalLine $line -OriginalCursor $cursor `
            -Generation $script:AutomexiaSuggestionGeneration `
            -SpanStartBytes $spanStartBytes -SpanEndBytes $spanEndBytes `
            -ReplacementStart $replacementStart -ReplacementLength $replacementLength
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
        [string]::IsNullOrWhiteSpace($env:AUTOMEXIA_SUGGESTION_REQUEST_HANDLE) -or
        [string]::IsNullOrWhiteSpace($env:AUTOMEXIA_SUGGESTION_RESPONSE_HANDLE)) {
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
        $script:AutomexiaSuggestionResponseStream = [IO.Pipes.AnonymousPipeClientStream]::new(
            [IO.Pipes.PipeDirection]::In,
            $env:AUTOMEXIA_SUGGESTION_RESPONSE_HANDLE
        )
        $script:AutomexiaSuggestionResponseReader = [IO.StreamReader]::new(
            $script:AutomexiaSuggestionResponseStream,
            [Text.Encoding]::ASCII,
            $false,
            4096,
            $true
        )
    } catch {
        Disable-AutomexiaSuggestions
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
    if ($null -ne $script:AutomexiaSuggestionResponseReader) {
        $script:AutomexiaSuggestionResponseReader.Dispose()
    }
    if ($null -ne $script:AutomexiaSuggestionResponseStream) {
        $script:AutomexiaSuggestionResponseStream.Dispose()
    }
    if ($null -ne $script:AutomexiaSuggestionRequestStream) {
        $script:AutomexiaSuggestionRequestStream.Dispose()
    }
    $script:AutomexiaSuggestionChord = $null
    $script:AutomexiaSuggestionRequestStream = $null
    $script:AutomexiaSuggestionResponseStream = $null
    $script:AutomexiaSuggestionResponseReader = $null
    $script:AutomexiaSuggestionState = 'disabled'
    if ($script:AutomexiaSuggestionReason -eq 'none') {
        $script:AutomexiaSuggestionReason = 'disabled'
    }
}
