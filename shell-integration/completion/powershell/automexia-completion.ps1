# Automexia CP1 PowerShell completion adapter. PowerShell exposes no supported
# read-only registry for native argument completers, so cached scripts load only
# after the user records an explicit native-override decision during refresh.
if (-not $global:AutomexiaCompletionAdapterLoaded) {
    $global:AutomexiaCompletionAdapterLoaded = $true
    $script:AutomexiaCompletionLoaded = [Collections.Generic.List[string]]::new()
    $script:AutomexiaCompletionSkipped = [Collections.Generic.List[string]]::new()
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
        foreach ($candidate in @(
            $configRoot,
            (Join-Path $configRoot 'generated'),
            $script:AutomexiaCompletionRoot,
            (Join-Path $script:AutomexiaCompletionRoot 'powershell')
        )) {
            if (-not (Test-Path -LiteralPath $candidate)) { continue }
            $item = Get-Item -LiteralPath $candidate -Force
            if (-not $item.PSIsContainer -or
                $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
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
            if (-not (Test-Path -LiteralPath $file -PathType Leaf) -or
                -not (Test-Path -LiteralPath $digestPath -PathType Leaf) -or
                -not (Test-Path -LiteralPath $overridePath -PathType Leaf)) {
                continue
            }
            $fileItem = Get-Item -LiteralPath $file -Force
            $digestItem = Get-Item -LiteralPath $digestPath -Force
            $overrideItem = Get-Item -LiteralPath $overridePath -Force
            if ($fileItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) -or
                $digestItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) -or
                $overrideItem.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint)) {
                $script:AutomexiaCompletionSkipped.Add("${target}:link")
                continue
            }
            if ($fileItem.Length -gt 1114112 -or $digestItem.Length -gt 128 -or
                $overrideItem.Length -gt 64 -or
                ([IO.File]::ReadAllText($overridePath)).Trim() -ne 'explicit-native-override-v1') {
                $script:AutomexiaCompletionSkipped.Add("${target}:bounds")
                continue
            }
            $expected = ([IO.File]::ReadAllText($digestPath)).Trim()
            if ($expected -notmatch '^[0-9a-f]{64}$') {
                $script:AutomexiaCompletionSkipped.Add("${target}:digest")
                continue
            }
            $actual = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($actual -ne $expected) {
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
