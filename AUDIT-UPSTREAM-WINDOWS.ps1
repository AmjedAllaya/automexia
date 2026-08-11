param(
    [Parameter(Mandatory=$true)][string]$ProjectRoot,
    [int]$Limit = 40
)

$ErrorActionPreference = "Stop"
$ProjectRoot = [System.IO.Path]::GetFullPath($ProjectRoot)
$UpstreamUrl = "https://github.com/raphamorim/rio.git"
$UpstreamTarget = "7d595af583f6ef1ea6036a66b367ba1e5a84d4a2"

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git is required"
}
if (-not (Test-Path (Join-Path $ProjectRoot ".git"))) {
    throw "Not a Git checkout: $ProjectRoot"
}

$remotes = @(& git -C $ProjectRoot remote)
if ($remotes -notcontains "upstream") {
    & git -C $ProjectRoot remote add upstream $UpstreamUrl
    if ($LASTEXITCODE -ne 0) { throw "could not add upstream remote" }
} else {
    & git -C $ProjectRoot remote set-url upstream $UpstreamUrl
    if ($LASTEXITCODE -ne 0) { throw "could not set upstream remote" }
}

Write-Host "Fetching Rio upstream for audit only..." -ForegroundColor Cyan
& git -C $ProjectRoot fetch upstream --prune
if ($LASTEXITCODE -ne 0) { throw "git fetch upstream failed" }

& git -C $ProjectRoot cat-file -e "$UpstreamTarget^{commit}" 2>$null
if ($LASTEXITCODE -ne 0) { throw "Pinned Automexia engine SHA is unavailable" }

$latest = (& git -C $ProjectRoot rev-parse upstream/main).Trim()
$count = (& git -C $ProjectRoot rev-list --count "$UpstreamTarget..upstream/main").Trim()

Write-Host ""
Write-Host "Automexia pinned engine : $UpstreamTarget" -ForegroundColor Green
Write-Host "Rio upstream/main       : $latest"
Write-Host "Commits beyond pin      : $count"
Write-Host ""

if ([int]$count -gt 0) {
    Write-Host "Newest upstream commits beyond the pin (audit only):" -ForegroundColor Yellow
    & git -C $ProjectRoot log --oneline --decorate --max-count=$Limit "$UpstreamTarget..upstream/main"
    if ($LASTEXITCODE -ne 0) { throw "git log failed" }
}

Write-Host ""
Write-Host "No merge, rebase, reset or checkout was performed." -ForegroundColor Green
Write-Host "To adopt a newer engine, create a separate compatibility branch and run the full Automexia gates." -ForegroundColor Cyan
