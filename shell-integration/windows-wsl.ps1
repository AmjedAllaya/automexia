$script:AutomexiaWslPayloadBytesLimit = 6MB
$script:AutomexiaWslOutputCharsLimit = 65536

function Invoke-AutomexiaWslScript(
    [string]$WslExecutable,
    [string]$Distribution,
    [string]$Script,
    [ValidateRange(100, 300000)][int]$TimeoutMilliseconds = 60000
) {
    if ([string]::IsNullOrWhiteSpace($WslExecutable) -or
        -not (Test-Path -LiteralPath $WslExecutable -PathType Leaf)) {
        throw 'A regular WSL executable is required.'
    }
    if ([string]::IsNullOrWhiteSpace($Distribution) -or
        $Distribution -notmatch '^[A-Za-z0-9._-]+$') {
        throw "Unsafe WSL distribution name: $Distribution"
    }
    if ([string]::IsNullOrWhiteSpace($Script) -or
        $Script.IndexOf([char]0) -ge 0) {
        throw 'WSL integration script is empty or contains a null byte.'
    }
    $payloadBytes = [Text.UTF8Encoding]::new($false).GetBytes($Script)
    if ($payloadBytes.Length -gt $script:AutomexiaWslPayloadBytesLimit) {
        throw 'WSL integration script exceeds the 6 MiB safety ceiling.'
    }

    $startInfo = New-Object Diagnostics.ProcessStartInfo
    $startInfo.FileName = $WslExecutable
    # The distribution token is allowlisted above. The fixed sh -s command
    # consumes raw UTF-8 from stdin; no command text, decoder, or nested shell
    # is constructed from the payload.
    $startInfo.Arguments = '--distribution ' + $Distribution + ' --exec sh -s'
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $startInfo
    # Windows PowerShell 5.1 lacks ProcessStartInfo.StandardInputEncoding and
    # otherwise creates the redirected stdin writer with a UTF-8 BOM. Pin the
    # console encodings only while Process.Start constructs its stream objects,
    # then restore them before any payload is written.
    $utf8NoBom = [Text.UTF8Encoding]::new($false)
    $previousInputEncoding = [Console]::InputEncoding
    $previousOutputEncoding = [Console]::OutputEncoding
    $started = $false
    $timer = [Diagnostics.Stopwatch]::StartNew()
    try {
        try {
            [Console]::InputEncoding = $utf8NoBom
            [Console]::OutputEncoding = $utf8NoBom
            if (-not $process.Start()) {
                throw "Unable to start WSL integration process for $Distribution"
            }
            $started = $true
        } finally {
            [Console]::InputEncoding = $previousInputEncoding
            [Console]::OutputEncoding = $previousOutputEncoding
        }
        $streams = @(
            foreach ($reader in @($process.StandardOutput, $process.StandardError)) {
                $buffer = New-Object char[] 4096
                [pscustomobject]@{
                    Reader = $reader
                    Buffer = $buffer
                    Text = [Text.StringBuilder]::new()
                    Pending = $reader.ReadAsync($buffer, 0, $buffer.Length)
                    Done = $false
                }
            }
        )
        # A distro which never reads stdin must not trap a synchronous 6 MiB
        # write. One deadline covers stdin, both bounded readers and client exit.
        $write = $process.StandardInput.BaseStream.WriteAsync($payloadBytes, 0, $payloadBytes.Length)
        $inputClosed = $false
        while ($true) {
            if ($timer.ElapsedMilliseconds -ge $TimeoutMilliseconds) {
                throw 'WSL integration exceeded its deadline. Check the selected distro and retry.'
            }
            if (-not $inputClosed -and $write.IsCompleted) {
                $write.GetAwaiter().GetResult()
                $process.StandardInput.BaseStream.Close()
                $inputClosed = $true
            }
            foreach ($stream in $streams) {
                if (-not $stream.Done -and $stream.Pending.IsCompleted) {
                    $count = $stream.Pending.GetAwaiter().GetResult()
                    if ($count -eq 0) {
                        $stream.Done = $true
                    } else {
                        if ($stream.Text.Length + $count -gt $script:AutomexiaWslOutputCharsLimit) {
                            throw 'WSL integration exceeded its output limit.'
                        }
                        [void]$stream.Text.Append($stream.Buffer, 0, $count)
                        $stream.Pending = $stream.Reader.ReadAsync($stream.Buffer, 0, $stream.Buffer.Length)
                    }
                }
            }
            if ($inputClosed -and $streams[0].Done -and $streams[1].Done -and $process.HasExited) { break }
            Start-Sleep -Milliseconds 5
        }
        return [pscustomobject]@{
            ExitCode = $process.ExitCode
            Stdout = $streams[0].Text.ToString().Replace([string][char]0, '')
            Stderr = $streams[1].Text.ToString().Replace([string][char]0, '')
        }
    } finally {
        try {
            if ($started -and -not $process.HasExited) {
                # Own this client only. Never terminate a distro shared with
                # other terminals as a side effect of an installer failure.
                $process.Kill()
                if (-not $process.WaitForExit(2000)) { throw 'WSL client cleanup did not finish.' }
            }
        } finally {
            $process.Dispose()
        }
    }
}
