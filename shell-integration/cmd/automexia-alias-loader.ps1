[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$ConfigRoot
)

$ErrorActionPreference = 'Stop'

function Write-Result([string]$State, [string]$Generation = '', [string]$Path = '', [string]$Names = '') {
    [Console]::Out.WriteLine("$State|$Generation|$Path|$Names")
}

function Get-Sha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
        $stream.Dispose()
    }
}

function Get-TextSha256([string]$Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

$currentSid = try {
    [Security.Principal.WindowsIdentity]::GetCurrent().User
} catch {
    $null
}

function Test-PrivateItem([string]$Path, [bool]$Directory, [long]$Maximum = 0) {
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    try {
        $item = Get-Item -LiteralPath $Path -Force
        if ($item.PSIsContainer -ne $Directory -or
            $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) -or
            (-not $Directory -and $item.Length -gt $Maximum) -or
            -not $currentSid) {
            return $false
        }
        $acl = [IO.FileSystemAclExtensions]::GetAccessControl($item)
        $rules = @($acl.GetAccessRules(
            $true,
            $false,
            [Security.Principal.SecurityIdentifier]
        ))
        return $acl.AreAccessRulesProtected -and $rules.Count -eq 1 -and
            $rules[0].IdentityReference -eq $currentSid -and
            $rules[0].AccessControlType -eq [Security.AccessControl.AccessControlType]::Allow
    } catch {
        return $false
    }
}

try {
    if ($ConfigRoot -notmatch '^[A-Za-z]:[\\/]' -or
        [Text.Encoding]::UTF8.GetByteCount($ConfigRoot) -gt 4096) {
        Write-Result 'UNSAFE_PATH'
        exit 2
    }
    $root = Join-Path $ConfigRoot 'generated\aliases'
    foreach ($directory in @(
        (Join-Path $ConfigRoot 'generated'),
        $root,
        (Join-Path $root 'generations')
    )) {
        if (-not (Test-PrivateItem $directory $true)) {
            Write-Result 'UNSAFE_PERMISSIONS'
            exit 2
        }
    }

    $pointer = Join-Path $root 'current'
    if (-not (Test-PrivateItem $pointer $false 80)) {
        Write-Result 'UNINITIALIZED'
        exit 0
    }
    $pointerText = [IO.File]::ReadAllText($pointer)
    if (-not $pointerText.EndsWith("`n") -or $pointerText.Contains("`r")) {
        Write-Result 'TAMPERED'
        exit 2
    }
    $generation = $pointerText.Substring(0, $pointerText.Length - 1)
    if ($generation -eq 'disabled') {
        Write-Result 'DISABLED'
        exit 0
    }
    if ($generation -notmatch '^[0-9a-f]{64}$') {
        Write-Result 'TAMPERED'
        exit 2
    }

    $generationDirectory = Join-Path $root "generations\$generation"
    if (-not (Test-PrivateItem $generationDirectory $true)) {
        Write-Result 'TAMPERED'
        exit 2
    }
    $manifest = Join-Path $generationDirectory 'generation.manifest'
    if (-not (Test-PrivateItem $manifest $false 65536) -or
        (Get-Sha256 $manifest) -ne $generation) {
        Write-Result 'TAMPERED'
        exit 2
    }
    $manifestText = [IO.File]::ReadAllText($manifest)
    $lines = @([IO.File]::ReadAllLines($manifest))
    if (-not $manifestText.EndsWith("`n") -or $manifestText.Contains("`r") -or

        $lines.Count -ne 10 -or
        $lines[0] -ne 'automexia-alias-generation-v1' -or
        $lines[1] -ne 'schema=1' -or
        $lines[2] -notmatch '^source-revision=(0|[1-9][0-9]*)$' -or
        $lines[3] -notmatch '^source-digest=[0-9a-f]{64}$' -or
        $lines[4] -ne 'generator=automexia-devops/0.4.0' -or
        $lines[5] -notmatch '^shell=powershell\|' -or
        $lines[6] -notmatch '^shell=bash\|' -or
        $lines[7] -notmatch '^shell=zsh\|' -or
        $lines[8] -notmatch '^shell=fish\|' -or
        $lines[9] -notmatch '^shell=cmd\|') {
        Write-Result 'TAMPERED'
        exit 2
    }
    $shellLines = @($lines | Where-Object { $_.StartsWith('shell=cmd|') })
    if ($shellLines.Count -ne 1) {
        Write-Result 'TAMPERED'
        exit 2
    }
    $fields = $shellLines[0].Split('|')
    if ($fields.Count -ne 8 -or
        $fields[0] -ne 'shell=cmd' -or
        $fields[1] -ne 'automexia-aliases.doskey' -or
        $fields[2] -notmatch '^[0-9a-f]{64}$' -or
        $fields[3] -notmatch '^[0-9a-f]{64}$' -or
        $fields[4] -notmatch '^[0-9]+$' -or
        $fields[5] -notmatch '^[0-9]+$') {
        Write-Result 'TAMPERED'
        exit 2
    }
    $names = if ($fields[6]) { @($fields[6].Split(',')) } else { @() }
    if ($names.Count -ne [int]$fields[4] -or
        @($names | Where-Object { $_ -notmatch '^[a-z][a-z0-9-]{1,31}$' }).Count -ne 0) {
        Write-Result 'TAMPERED'
        exit 2
    }
    $overrides = @{}
    if ($fields[7]) {
        foreach ($entry in $fields[7].Split(',')) {
            $parts = $entry.Split(':')
            if ($parts.Count -ne 2 -or $parts[0] -notmatch '^[a-z][a-z0-9-]{1,31}$' -or
                $parts[1] -notmatch '^[0-9a-f]{64}$' -or $names -notcontains $parts[0] -or
                $overrides.ContainsKey($parts[0])) {
                Write-Result 'TAMPERED'
                exit 2
            }
            $overrides[$parts[0]] = $parts[1]
        }
    }

    $shellDirectory = Join-Path $generationDirectory 'cmd'
    if (-not (Test-PrivateItem $shellDirectory $true)) {
        Write-Result 'TAMPERED'
        exit 2
    }
    $artifact = Join-Path $shellDirectory $fields[1]
    if (-not (Test-PrivateItem $artifact $false 1114112) -or
        (Get-Sha256 $artifact) -ne $fields[2]) {
        Write-Result 'TAMPERED'
        exit 2
    }

    $reserved = @(
        'assoc','break','call','cd','chcp','chdir','cls','color','copy','date','del',
        'dir','echo','endlocal','erase','exit','for','ftype','goto','if','md','mkdir',
        'mklink','move','path','pause','popd','prompt','pushd','rd','rem','ren','rename',
        'rmdir','set','setlocal','shift','start','time','title','type','ver','verify','vol'
    )
    $macros = @{}
    try {
        foreach ($line in @(& doskey.exe /macros 2>$null)) {
            $separator = $line.IndexOf('=')
            if ($separator -gt 0) {
                $macros[$line.Substring(0, $separator).ToLowerInvariant()] =
                    $line.Substring($separator + 1)
            }
        }
    } catch {}

    $collisions = [Collections.Generic.List[string]]::new()
    foreach ($name in $names) {
        $actual = $null
        $hasCollision = $false
        if ($macros.ContainsKey($name)) {
            $hasCollision = $true
        } elseif ($reserved -contains $name) {
            $hasCollision = $true
            $actual = Get-TextSha256 "cmd|Builtin|$name|shell-builtin"
        } else {
            $application = @(Get-Command -Name $name -CommandType Application -All -ErrorAction SilentlyContinue |
                Select-Object -First 1)
            if ($application.Count -eq 1 -and
                (Test-Path -LiteralPath $application[0].Source -PathType Leaf)) {
                $item = Get-Item -LiteralPath $application[0].Source -Force
                $hasCollision = $true
                if (-not $item.Attributes.HasFlag([IO.FileAttributes]::ReparsePoint) -and
                    $item.Length -le 512MB) {
                    $actual = Get-Sha256 $application[0].Source
                }
            }
        }
        if ($hasCollision -and
            (-not $overrides.ContainsKey($name) -or -not $actual -or
             $overrides[$name] -ne $actual)) {
            $collisions.Add($name)
        }
    }
    if ($collisions.Count -ne 0) {
        Write-Result 'COLLISION' $generation '' ($collisions -join ',')
        exit 0
    }

    Write-Result 'READY' $generation $artifact ($names -join ',')
} catch {
    Write-Result 'FAILED'
    exit 2
}
