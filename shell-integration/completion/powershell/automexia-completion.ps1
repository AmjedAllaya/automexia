# Automexia CP1 PowerShell completion adapter. PowerShell exposes no supported
# read-only registry for native argument completers, so cached scripts load only
# after the user records an explicit native-override decision during refresh.
if (-not $global:AutomexiaCompletionAdapterLoaded) {
    $global:AutomexiaCompletionAdapterLoaded = $true
    $script:AutomexiaCompletionLoaded = [Collections.Generic.List[string]]::new()
    $script:AutomexiaCompletionSkipped = [Collections.Generic.List[string]]::new()
    function script:Get-AutomexiaCompletionFileSha256([string]$Path) {
        # Avoid optional module auto-loading on the shell startup path.
        $stream = [IO.File]::OpenRead($Path)
        $sha = [Security.Cryptography.SHA256]::Create()
        try {
            return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
        } finally {
            $sha.Dispose()
            $stream.Dispose()
        }
    }

    $configRoot = if ($env:AUTOMEXIA_CONFIG_HOME) {
        $env:AUTOMEXIA_CONFIG_HOME
    } elseif ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA 'Automexia\Terminal'
    } else {
        $null
    }
    $script:AutomexiaCompletionRoot = if ($configRoot) {
        Join-Path $configRoot 'generated\completion'
    } else {
        $null
    }

    function Test-AutomexiaCompletionDirectorySafe {
        if (-not $configRoot -or -not $script:AutomexiaCompletionRoot) { return $false }
        # Cached completion is local-only; a UNC root would perform network I/O at startup.
        if ($configRoot -notmatch '^[A-Za-z]:[\\/]') { return $false }
        if ([Text.Encoding]::UTF8.GetByteCount($configRoot) -gt 4096) {
            return $false
        }
        foreach ($candidate in @(
            $configRoot,
            (Join-Path $configRoot 'generated'),
            $script:AutomexiaCompletionRoot,
            (Join-Path $script:AutomexiaCompletionRoot 'powershell')
        )) {
            try {
                # File.GetAttributes is one direct local filesystem probe. The
                # provider-based Test-Path/Get-Item pair doubled shell-startup
                # I/O and made the completion adapter miss its 50 ms budget.
                $attributes = [IO.File]::GetAttributes($candidate)
            } catch [IO.FileNotFoundException] {
                continue
            } catch [IO.DirectoryNotFoundException] {
                continue
            } catch {
                return $false
            }
            if (($attributes -band [IO.FileAttributes]::Directory) -eq 0 -or
                ($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                return $false
            }
        }
        return $true
    }
    $script:AutomexiaCompletionPathSafe = Test-AutomexiaCompletionDirectorySafe
    if (-not $script:AutomexiaCompletionPathSafe) {
        $script:AutomexiaCompletionSkipped.Add('state:unsafe-path')
    }

    if ($script:AutomexiaCompletionPathSafe -and
        $env:AUTOMEXIA_COMPLETION_DISABLED -ne '1' -and
        -not (Test-Path -LiteralPath (Join-Path $script:AutomexiaCompletionRoot '.disabled') -PathType Leaf)) {
        foreach ($target in @('kubectl', 'oc', 'helm')) {
            $file = Join-Path $script:AutomexiaCompletionRoot "powershell\$target.ps1"
            $digestPath = "$file.sha256"
            $overridePath = "$file.allow-override"
            $fileItem = [IO.FileInfo]::new($file)
            $fileItem.Refresh()
            if (-not $fileItem.Exists) {
                continue
            }
            $digestItem = [IO.FileInfo]::new($digestPath)
            $digestItem.Refresh()
            if (-not $digestItem.Exists) {
                continue
            }
            $overrideItem = [IO.FileInfo]::new($overridePath)
            $overrideItem.Refresh()
            if (-not $overrideItem.Exists) {
                continue
            }
            if (($fileItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
                ($digestItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
                ($overrideItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                $script:AutomexiaCompletionSkipped.Add("${target}:link")
                continue
            }
            if ($fileItem.Length -gt 1114112 -or $digestItem.Length -gt 192 -or
                $overrideItem.Length -gt 64 -or
                ([IO.File]::ReadAllText($overridePath)).Trim() -ne 'explicit-native-override-v1') {
                $script:AutomexiaCompletionSkipped.Add("${target}:bounds")
                continue
            }
            $digestText = [IO.File]::ReadAllText($digestPath)
            $expectedDigests = @($digestText.TrimEnd("`n").Split("`n"))
            $invalidDigest = $false
            foreach ($expectedDigest in $expectedDigests) {
                if ($expectedDigest -notmatch '^[0-9a-f]{64}$') {
                    $invalidDigest = $true
                    break
                }
            }
            if (-not $digestText.EndsWith("`n") -or
                $expectedDigests.Count -lt 1 -or $expectedDigests.Count -gt 2 -or
                $invalidDigest) {
                $script:AutomexiaCompletionSkipped.Add("${target}:digest")
                continue
            }
            $actual = Get-AutomexiaCompletionFileSha256 $file
            if ($expectedDigests -notcontains $actual) {
                $script:AutomexiaCompletionSkipped.Add("${target}:tampered")
                continue
            }
            . $file
            $script:AutomexiaCompletionLoaded.Add($target)
        }
    }

    function global:Get-AutomexiaCompletionHealth {
        [CmdletBinding()]
        param()
        [pscustomobject]@{
            Shell = 'PowerShell'
            Version = $PSVersionTable.PSVersion.ToString()
            State = if (-not $script:AutomexiaCompletionPathSafe) {
                'UnsafePath/NativeFallback'
            } elseif ($env:AUTOMEXIA_COMPLETION_DISABLED -eq '1' -or
                ($script:AutomexiaCompletionRoot -and (Test-Path -LiteralPath (Join-Path $script:AutomexiaCompletionRoot '.disabled')))) {
                'Disabled'
            } else { 'Enabled' }
            Loaded = @($script:AutomexiaCompletionLoaded)
            Skipped = @($script:AutomexiaCompletionSkipped)
            NativeOverrideRequired = $true
        }
    }
}
