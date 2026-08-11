param(
    [string]$ProjectRoot = (Join-Path $PWD "automexia-terminal-source"),
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$ForkUrl = "https://github.com/AmjedAllaya/rio.git"
$UpstreamUrl = "https://github.com/raphamorim/rio.git"
$UpstreamTarget = "7d595af583f6ef1ea6036a66b367ba1e5a84d4a2"
$BranchPrefix = "automexia/release-audited-v0.3.13"

$AutomexiaManagedPaths = @(
    "frontends/rioterm/Cargo.toml",
    "frontends/rioterm/src/main.rs",
    "frontends/rioterm/src/context/renderable.rs",
    "frontends/rioterm/src/context/mod.rs",
    "frontends/rioterm/src/renderer/mod.rs",
    "frontends/rioterm/src/renderer/utils.rs",
    "frontends/rioterm/src/renderer/command_palette.rs",
    "frontends/rioterm/src/router/mod.rs",
    "frontends/rioterm/src/screen/mod.rs",
    "frontends/rioterm/src/grid_emit.rs",
    "frontends/rioterm/src/bindings/mod.rs",
    "rio-window/src/platform_impl/windows/util.rs",
    "sugarloaf/src/renderer/mod.rs",
    "frontends/rioterm/src/context/title.rs",
    "frontends/rioterm/src/renderer/devops_status.rs",
    "frontends/rioterm/src/extensions/mod.rs",
    "frontends/rioterm/src/extensions/manager.rs",
    "frontends/rioterm/src/extensions/devops.rs",
    "frontends/rioterm/src/automexia/mod.rs",
    "frontends/rioterm/src/automexia/api.rs",
    "frontends/rioterm/src/automexia/state.rs",
    "frontends/rioterm/src/automexia/runtime.rs",
    "frontends/rioterm/src/automexia/shell.rs",
    "frontends/rioterm/src/automexia/theme.rs",
    "frontends/rioterm/src/automexia/ui.rs",
    "frontends/rioterm/src/automexia/marketplace.rs",
    "frontends/rioterm/src/automexia/builtins/mod.rs",
    "frontends/rioterm/src/automexia/builtins/devops.rs",
    "frontends/rioterm/src/automexia/builtins/devops/mod.rs",
    "frontends/rioterm/src/automexia/builtins/devops/model.rs",
    "frontends/rioterm/src/automexia/builtins/devops/context.rs",
    "frontends/rioterm/src/automexia/builtins/devops/semantics.rs"
)

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$GitArguments,
        [string]$WorkingDirectory = ""
    )

    if ($WorkingDirectory) {
        & git -C $WorkingDirectory @GitArguments
    } else {
        & git @GitArguments
    }

    if ($LASTEXITCODE -ne 0) {
        throw "git failed: git $($GitArguments -join ' ')"
    }
}

function Get-DirtyPaths {
    param(
        [Parameter(Mandatory = $true)]
        [string]$WorkingDirectory
    )

    $lines = & git -C $WorkingDirectory status --porcelain=v1 --untracked-files=all
    if ($LASTEXITCODE -ne 0) {
        throw "git failed: git status --porcelain=v1 --untracked-files=all"
    }

    $paths = @()
    foreach ($line in $lines) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line.Length -lt 4) {
            continue
        }
        $path = $line.Substring(3).Trim()
        if ($path -match ' -> ') {
            $path = ($path -split ' -> ', 2)[1]
        }
        $path = $path.Trim('"').Replace('\', '/')
        $paths += $path
    }
    return $paths
}

function Get-GitOutput {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$GitArguments,
        [string]$WorkingDirectory = ""
    )

    if ($WorkingDirectory) {
        $output = & git -C $WorkingDirectory @GitArguments
    } else {
        $output = & git @GitArguments
    }

    if ($LASTEXITCODE -ne 0) {
        throw "git failed: git $($GitArguments -join ' ')"
    }

    return (($output | Out-String).Trim())
}

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git is required. Install Git for Windows first."
}

$pythonLauncher = $null
if (Get-Command py -ErrorAction SilentlyContinue) {
    $pythonLauncher = "py"
} elseif (Get-Command python -ErrorAction SilentlyContinue) {
    $pythonLauncher = "python"
} else {
    throw "Python 3 is required to apply the Automexia source integration."
}

if (-not $SkipBuild -and -not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust/Cargo is required for the build. Re-run with -SkipBuild to prepare source only."
}

Write-Host "Verifying Automexia integration package..." -ForegroundColor Cyan
foreach ($verificationScript in @("verify_package.py", "verify_patcher.py", "verify_regressions.py", "verify_architecture.py", "verify_devops_extension.py", "verify_visual_system.py", "verify_source_quality.py", "verify_shell_behavior.py")) {
    $verificationPath = Join-Path $PSScriptRoot $verificationScript
    if ($pythonLauncher -eq "py") {
        & py -3 $verificationPath
    } else {
        & python $verificationPath
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Automexia package verification failed: $verificationScript"
    }
}

$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)
$PackageRoot = [System.IO.Path]::GetFullPath($PSScriptRoot)

# A common mistake is to point -ProjectRoot at this extracted package itself.
# Make that safe: keep the package immutable and place the real Rio checkout
# in a child directory instead of rejecting a non-empty package directory.
$isPackageRoot = ($ProjectRoot.TrimEnd('\\') -ieq $PackageRoot.TrimEnd('\\')) -and
    (Test-Path (Join-Path $ProjectRoot "apply_automexia.py")) -and
    (Test-Path (Join-Path $ProjectRoot "BOOTSTRAP-WINDOWS.ps1")) -and
    (-not (Test-Path (Join-Path $ProjectRoot ".git")))

if ($isPackageRoot) {
    $ProjectRoot = Join-Path $ProjectRoot "rio-source"
    Write-Host "-ProjectRoot pointed at the extracted package." -ForegroundColor Yellow
    Write-Host "Using source checkout: $ProjectRoot" -ForegroundColor Yellow
}

$gitDir = Join-Path $ProjectRoot ".git"
if (-not (Test-Path $gitDir)) {
    if (Test-Path $ProjectRoot) {
        $items = @(Get-ChildItem -Force $ProjectRoot -ErrorAction SilentlyContinue)
        if ($items.Count -gt 0) {
            throw "Destination exists and is not an empty Git checkout: $ProjectRoot`nChoose a new/empty directory or point to an existing Rio Git checkout."
        }
    } else {
        $parent = Split-Path -Parent $ProjectRoot
        if ($parent -and -not (Test-Path $parent)) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }
    }

    Write-Host "Cloning Automexia Rio fork..." -ForegroundColor Cyan
    Invoke-Git -GitArguments @("clone", "--", $ForkUrl, $ProjectRoot)
}

if (-not (Test-Path (Join-Path $ProjectRoot "Cargo.toml")) -or
    -not (Test-Path (Join-Path $ProjectRoot "frontends\rioterm\src\main.rs"))) {
    throw "ProjectRoot is a Git checkout, but it does not look like the Rio source tree: $ProjectRoot"
}

Push-Location $ProjectRoot
try {
    $resumeAutomexia = $false
    $dirtyPaths = @(Get-DirtyPaths -WorkingDirectory $ProjectRoot)
    if ($dirtyPaths.Count -gt 0) {
        $unexpectedDirty = @($dirtyPaths | Where-Object {
            $candidate = $_
            (-not $candidate.StartsWith(".backups/")) -and
            ($AutomexiaManagedPaths -notcontains $candidate)
        })
        $mainSource = Join-Path $ProjectRoot "frontends\rioterm\src\main.rs"
        $rendererSource = Join-Path $ProjectRoot "frontends\rioterm\src\renderer\mod.rs"
        $hasAutomexiaMarker =
            (Select-String -Path $mainSource -SimpleMatch "mod automexia;" -Quiet -ErrorAction SilentlyContinue) -or
            (Select-String -Path $mainSource -SimpleMatch "mod extensions;" -Quiet -ErrorAction SilentlyContinue) -or
            (Select-String -Path $rendererSource -SimpleMatch "devops_enabled" -Quiet -ErrorAction SilentlyContinue)

        if ($unexpectedDirty.Count -eq 0 -and $hasAutomexiaMarker) {
            $resumeAutomexia = $true
            Write-Host "Detected an interrupted/previous Automexia integration; resuming it safely." -ForegroundColor Yellow
            if (@($dirtyPaths | Where-Object { $_.StartsWith(".backups/") }).Count -gt 0) {
                Write-Host "Legacy in-checkout .backups data is preserved and ignored by resume." -ForegroundColor DarkGray
            }
        } else {
            $details = ($dirtyPaths -join "`n  ")
            throw "Checkout has unrelated uncommitted changes. Commit/stash them before bootstrap.`n  $details"
        }
    }

    $origin = Get-GitOutput -WorkingDirectory $ProjectRoot -GitArguments @("remote", "get-url", "origin")
    if (-not $origin) {
        throw "Rio checkout has no origin remote."
    }

    $remoteText = Get-GitOutput -WorkingDirectory $ProjectRoot -GitArguments @("remote")
    $remotes = @($remoteText -split '\r?\n')
    if ($remotes -notcontains "upstream") {
        Invoke-Git -WorkingDirectory $ProjectRoot -GitArguments @("remote", "add", "upstream", $UpstreamUrl)
    } else {
        Invoke-Git -WorkingDirectory $ProjectRoot -GitArguments @("remote", "set-url", "upstream", $UpstreamUrl)
    }

    Write-Host "Fetching fork and official upstream..." -ForegroundColor Cyan
    Invoke-Git -WorkingDirectory $ProjectRoot -GitArguments @("fetch", "origin", "--prune")
    Invoke-Git -WorkingDirectory $ProjectRoot -GitArguments @("fetch", "upstream", "--prune")

    # The audited target must be available after fetching upstream.
    & git -C $ProjectRoot cat-file -e "$UpstreamTarget^{commit}" 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "Audited upstream commit $UpstreamTarget is not available after fetch."
    }

    if ($resumeAutomexia) {
        # An interrupted/previous Automexia run already created the integration branch
        # and advanced it to the audited target. Do not switch branches while its
        # managed files are dirty; verify ancestry and finish the idempotent apply.
        & git -C $ProjectRoot merge-base --is-ancestor $UpstreamTarget HEAD
        if ($LASTEXITCODE -ne 0) {
            throw "Interrupted Automexia checkout does not contain audited upstream target $UpstreamTarget."
        }
        $branch = Get-GitOutput -WorkingDirectory $ProjectRoot -GitArguments @("branch", "--show-current")
        if (-not $branch) {
            throw "Interrupted Automexia checkout is detached; switch to its integration branch first."
        }
        $resumeHead = Get-GitOutput -WorkingDirectory $ProjectRoot -GitArguments @("rev-parse", "HEAD")
        Write-Host "Resuming branch $branch at $resumeHead." -ForegroundColor Cyan
        if ($resumeHead -ne $UpstreamTarget) {
            Write-Host "Legacy Automexia checkout uses a descendant base ($resumeHead), not the exact v0.3 pin." -ForegroundColor Yellow
            Write-Host "This resume path preserves your existing work. For fully reproducible v0.3 source, bootstrap into a fresh directory." -ForegroundColor Yellow
        }
    } else {
        # Automexia stable bootstraps are reproducible: always branch from the
        # exact audited Rio commit. origin/main may move independently without
        # changing the code that a given Automexia version builds.
        $stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
        $branch = "$BranchPrefix-$stamp"
        Invoke-Git -WorkingDirectory $ProjectRoot -GitArguments @("switch", "-c", $branch, $UpstreamTarget)
        Write-Host "Pinned Automexia base to audited Rio commit $UpstreamTarget." -ForegroundColor Cyan
    }

    Write-Host "Applying Automexia architecture, keyboard, /market and built-in extensions..." -ForegroundColor Cyan
    if ($pythonLauncher -eq "py") {
        if ($resumeAutomexia) {
            & py -3 (Join-Path $PSScriptRoot "apply_automexia.py") $ProjectRoot --allow-dirty
        } else {
            & py -3 (Join-Path $PSScriptRoot "apply_automexia.py") $ProjectRoot
        }
    } else {
        if ($resumeAutomexia) {
            & python (Join-Path $PSScriptRoot "apply_automexia.py") $ProjectRoot --allow-dirty
        } else {
            & python (Join-Path $PSScriptRoot "apply_automexia.py") $ProjectRoot
        }
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Automexia source integration failed."
    }

    # Source transformation is a generation step. Normalize the generated Rust
    # with the exact rustfmt selected by the checkout's rust-toolchain.toml
    # before any non-mutating release/check gates inspect it.
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        & (Join-Path $PSScriptRoot "FORMAT-WINDOWS.ps1") -ProjectRoot $ProjectRoot
    } else {
        Write-Warning "Cargo is unavailable, so generated Rust source could not be rustfmt-normalized. Re-run bootstrap after installing Rust before BUILD-WINDOWS.ps1."
    }

    Write-Host "Verifying warning-clean source transforms..." -ForegroundColor Cyan
    $warningVerifier = Join-Path $PSScriptRoot "verify_warning_cleanup.py"
    if ($pythonLauncher -eq "py") {
        & py -3 $warningVerifier $ProjectRoot
    } else {
        & python $warningVerifier $ProjectRoot
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Automexia warning-clean source verification failed."
    }

    Write-Host "Verifying Automexia architecture boundaries..." -ForegroundColor Cyan
    $architectureVerifier = Join-Path $PSScriptRoot "verify_architecture.py"
    if ($pythonLauncher -eq "py") {
        & py -3 $architectureVerifier $ProjectRoot
    } else {
        & python $architectureVerifier $ProjectRoot
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Automexia architecture verification failed."
    }

    Write-Host "Verifying Automexia DevOps extension behavior..." -ForegroundColor Cyan
    $devopsVerifier = Join-Path $PSScriptRoot "verify_devops_extension.py"
    if ($pythonLauncher -eq "py") {
        & py -3 $devopsVerifier $ProjectRoot
    } else {
        & python $devopsVerifier $ProjectRoot
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Automexia DevOps extension verification failed."
    }

    Write-Host "Verifying Automexia unified visual system..." -ForegroundColor Cyan
    $visualVerifier = Join-Path $PSScriptRoot "verify_visual_system.py"
    if ($pythonLauncher -eq "py") {
        & py -3 $visualVerifier $ProjectRoot
    } else {
        & python $visualVerifier $ProjectRoot
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Automexia unified visual-system verification failed."
    }

    Write-Host "Verifying Automexia source/shell quality invariants..." -ForegroundColor Cyan
    $qualityVerifier = Join-Path $PSScriptRoot "verify_source_quality.py"
    if ($pythonLauncher -eq "py") {
        & py -3 $qualityVerifier $ProjectRoot
    } else {
        & python $qualityVerifier $ProjectRoot
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Automexia source-quality verification failed."
    }

    if (-not $SkipBuild) {
        & (Join-Path $PSScriptRoot "BUILD-WINDOWS.ps1") -ProjectRoot $ProjectRoot
        if ($LASTEXITCODE -ne 0) {
            throw "Build failed."
        }
    }

    Write-Host ""
    Write-Host "Automexia Terminal source is ready." -ForegroundColor Green
    Write-Host "Project: $ProjectRoot"
    Write-Host "Branch : $branch"
    Write-Host "Base   : $(& git -C $ProjectRoot rev-parse HEAD)"
    if (-not $SkipBuild) {
        Write-Host "Binary : $(Join-Path $ProjectRoot 'dist\AutomexiaTerminal.exe')"
    }
} finally {
    Pop-Location
}
