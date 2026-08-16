$script:AutomexiaWslPayloadBytesLimit = 6MB

function Invoke-AutomexiaWslScript(
    [string]$WslExecutable,
    [string]$Distribution,
    [string]$Script
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
    try {
        try {
            [Console]::InputEncoding = $utf8NoBom
            [Console]::OutputEncoding = $utf8NoBom
            if (-not $process.Start()) {
                throw "Unable to start WSL integration process for $Distribution"
            }
        } finally {
            [Console]::InputEncoding = $previousInputEncoding
            [Console]::OutputEncoding = $previousOutputEncoding
        }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $process.StandardInput.BaseStream.Write($payloadBytes, 0, $payloadBytes.Length)
        $process.StandardInput.BaseStream.Close()
        $process.WaitForExit()
        $stdout = $stdoutTask.Result.Replace([string][char]0, '')
        $stderr = $stderrTask.Result.Replace([string][char]0, '')
        return [pscustomobject]@{
            ExitCode = $process.ExitCode
            Stdout = $stdout
            Stderr = $stderr
        }
    } finally {
        $process.Dispose()
    }
}
