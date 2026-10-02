# Vendor identity stays separate from Automexia product signing and versioning.
# Dot-sourcing only declares this function; validation never executes vendor code.
function Assert-ConPtyRuntime {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Architecture
    )
    $helper = Join-Path $PSScriptRoot 'windows_conpty_runtime.py'
    & python $helper verify --destination $Root --architecture $Architecture | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'ConPTY runtime hash/architecture verification failed' }
    $recipePath = Join-Path $PSScriptRoot '../../packaging/windows/conpty-runtime.json'
    $recipe = Get-Content -LiteralPath $recipePath -Raw | ConvertFrom-Json
    $architectureName = switch -CaseSensitive ($Architecture) {
        { $_ -in @('x64', 'x86_64', 'x86_64-pc-windows-msvc') } { 'x64'; break }
        { $_ -in @('arm64', 'aarch64', 'aarch64-pc-windows-msvc') } { 'arm64'; break }
        default { throw 'ConPTY requires a supported Windows architecture' }
    }
    $files = @($recipe.files | Where-Object { $_.architecture -in @($architectureName, 'any') })
    if ($files.Count -ne 3) { throw 'ConPTY runtime must contain exactly three vendor files' }
    foreach ($file in $files) {
        $path = Join-Path $Root $file.path
        $signature = Get-AuthenticodeSignature -LiteralPath $path
        if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) {
            throw "ConPTY vendor signature is invalid: $($file.path)"
        }
        # The pinned byte hash binds the complete original signed file. Checking
        # the certificate name additionally distinguishes the vendor from product.
        $publisher = $signature.SignerCertificate.GetNameInfo(
            [Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false)
        if ($publisher -cne $recipe.package.publisher) {
            throw "ConPTY vendor publisher mismatch: $($file.path)"
        }
        if ($null -eq $signature.TimeStamperCertificate) {
            throw "ConPTY vendor timestamp is missing: $($file.path)"
        }
        # Read typed X509 OIDs; PowerShell's EnhancedKeyUsageList adapter exposes
        # ObjectId as a string on some supported hosts, with no .Value property.
        $eku = @($signature.SignerCertificate.Extensions |
            Where-Object { $_.Oid.Value -eq '2.5.29.37' } |
            ForEach-Object { $_.EnhancedKeyUsages } |
            Where-Object { $_.Value -eq '1.3.6.1.5.5.7.3.3' })
        if ($eku.Count -eq 0) { throw "ConPTY vendor code-signing EKU is missing: $($file.path)" }
    }
    # Return only recipe-owned relative names; no certificate or machine paths.
    return @($files.path)
}

function Get-ConPtyPackageIdentity {
    $recipePath = Join-Path $PSScriptRoot '../../packaging/windows/conpty-runtime.json'
    $package = (Get-Content -LiteralPath $recipePath -Raw | ConvertFrom-Json).package
    return [ordered]@{ id = $package.id; version = $package.version; sha256 = $package.sha256 }
}
