$script:AutomexiaWslPayloadCharacterLimit = 8MB

function Invoke-AutomexiaWslBase64Script(
    [string]$WslExecutable,
    [string]$Distribution,
    [string]$Base64Payload
) {
    if ([string]::IsNullOrWhiteSpace($WslExecutable) -or
        -not (Test-Path -LiteralPath $WslExecutable -PathType Leaf)) {
        throw 'A regular WSL executable is required.'
    }
    if ([string]::IsNullOrWhiteSpace($Distribution) -or
        $Distribution -notmatch '^[A-Za-z0-9._-]+$') {
        throw "Unsafe WSL distribution name: $Distribution"
    }
    if ([string]::IsNullOrWhiteSpace($Base64Payload) -or
        $Base64Payload.Length -gt $script:AutomexiaWslPayloadCharacterLimit -or
        $Base64Payload -notmatch '^[A-Za-z0-9+/]+={0,2}$') {
        throw 'WSL integration payload is empty, oversized, or not canonical Base64.'
    }

    $startInfo = New-Object Diagnostics.ProcessStartInfo
    $startInfo.FileName = $WslExecutable
    $startInfo.Arguments = '--distribution ' + $Distribution +
        ' --exec sh -c "tr -cd ''A-Za-z0-9+/='' | base64 -d | sh"'
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = New-Object Diagnostics.Process
    $process.StartInfo = $startInfo
    try {
        if (-not $process.Start()) {
            throw "Unable to start WSL integration process for $Distribution"
        }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $payloadBytes = [Text.Encoding]::ASCII.GetBytes($Base64Payload)
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
