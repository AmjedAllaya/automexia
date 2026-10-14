$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSEdition -ne 'Desktop') {
    & powershell.exe -NoProfile -File $PSCommandPath
    if ($LASTEXITCODE -ne 0) { throw 'Windows PowerShell WSL transport tests failed' }
    return
}
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
. (Join-Path $root 'shell-integration\windows-wsl.ps1')

# Exercise the real redirected-process adapter without requiring a Linux distro.
# This fixture is transport evidence, never a native WSL compatibility result.
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('automexia-wsl-transport-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $fixture | Out-Null
$executable = Join-Path $fixture 'wsl-fixture.exe'
$source = @'
using System;
using System.IO;
using System.Text;
using System.Threading;
public static class WslTransportFixture {
    public static int Main(string[] args) {
        File.WriteAllText(Environment.GetEnvironmentVariable("AUTOMEXIA_WSL_FIXTURE_PID"), System.Diagnostics.Process.GetCurrentProcess().Id.ToString());
        if (args.Length != 5 || args[0] != "--distribution" || args[2] != "--exec" || args[3] != "sh" || args[4] != "-s") return 91;
        if (args[1] == "hang") { Thread.Sleep(30000); return 92; }
        if (args[1] == "flood") { for (;;) Console.Out.Write(new string('x', 4096)); }
        if (args[1] == "stderr-flood") { for (;;) Console.Error.Write(new string('x', 4096)); }
        using (var bytes = new MemoryStream()) {
            Console.OpenStandardInput().CopyTo(bytes);
            byte[] input = bytes.ToArray();
            if (input.Length >= 3 && input[0] == 239 && input[1] == 187 && input[2] == 191) return 93;
            Console.OutputEncoding = new UTF8Encoding(false);
            Console.Write(Encoding.UTF8.GetString(input));
            Console.Error.Write("fixture-stderr");
        }
        return args[1] == "exit" ? 7 : 0;
    }
}
'@
$oldPidPath = $env:AUTOMEXIA_WSL_FIXTURE_PID
try {
    Add-Type -TypeDefinition $source -OutputAssembly $executable -OutputType ConsoleApplication
    $env:AUTOMEXIA_WSL_FIXTURE_PID = Join-Path $fixture 'pid.txt'
    $payload = "printf 'fixture'`n# " + [char]0x03A9 + "`n"
    foreach ($distribution in @('echo', 'exit')) {
        $result = Invoke-AutomexiaWslScript $executable $distribution $payload -TimeoutMilliseconds 5000
        $expectedCode = if ($distribution -eq 'exit') { 7 } else { 0 }
        if ($result.ExitCode -ne $expectedCode -or $result.Stdout -cne $payload -or $result.Stderr -cne 'fixture-stderr') { throw 'WSL transport changed UTF-8 bytes, argv, output, or exit status' }
    }
    foreach ($distribution in @('hang', 'flood', 'stderr-flood')) {
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $failed = $false
        try {
            Invoke-AutomexiaWslScript $executable $distribution ('x' * 1MB) -TimeoutMilliseconds 1500 | Out-Null
        } catch {
            $expectedFailure = if ($distribution -eq 'hang') { 'deadline' } else { 'output limit' }
            if ($_.Exception.Message -notmatch $expectedFailure) { throw }
            $failed = $true
        }
        if (-not $failed -or $timer.Elapsed.TotalSeconds -gt 8) { throw 'WSL transport did not fail within its budget' }
        $fixturePid = [int](Get-Content -LiteralPath $env:AUTOMEXIA_WSL_FIXTURE_PID -Raw)
        $remaining = Get-Process -Id $fixturePid -ErrorAction SilentlyContinue
        try {
            # Windows can still expose an exited process while an asynchronous
            # stream retains a handle. Existence alone does not mean it is live.
            if ($remaining -and -not $remaining.HasExited) { throw 'WSL transport leaked its owned client after failure' }
        } finally {
            if ($remaining) { $remaining.Dispose() }
        }
    }
    Write-Output 'PASS: 5 WSL transport cases (UTF-8, exit status, blocked stdin, stdout limit, stderr limit); native WSL not exercised'
} finally {
    $env:AUTOMEXIA_WSL_FIXTURE_PID = $oldPidPath
    # Exact directory created above; never remove a distro or shared fixture root.
    $resolvedFixture = [IO.Path]::GetFullPath($fixture)
    $resolvedTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
    if ([IO.Path]::GetDirectoryName($resolvedFixture) -ne $resolvedTemp -or
        [IO.Path]::GetFileName($resolvedFixture) -notmatch '^automexia-wsl-transport-[0-9a-f-]{36}$') {
        throw 'Refusing cleanup outside the owned transport fixture'
    }
    Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
}
