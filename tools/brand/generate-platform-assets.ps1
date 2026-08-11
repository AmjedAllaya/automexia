param(
    [string]$Source = 'assets/brand/automexia-terminal-source-512.png'
)

$ErrorActionPreference = 'Stop'
if (-not (Get-Command magick -ErrorAction SilentlyContinue)) {
    throw 'ImageMagick 7 is required to generate Automexia platform assets.'
}

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$sourcePath = (Resolve-Path (Join-Path $root $Source)).Path
$brand = Join-Path $root 'assets\brand'
$pngRoot = Join-Path $brand 'png'
New-Item -ItemType Directory -Force -Path $pngRoot | Out-Null

$sourceDescription = & magick identify -format '%w %h %[channels]' $sourcePath
if ($LASTEXITCODE -ne 0) { throw 'ImageMagick could not inspect the brand source.' }
$sourceParts = $sourceDescription -split ' '
if ($sourceParts[0] -ne '512' -or $sourceParts[1] -ne '512') {
    throw "The canonical raster source must be exactly 512x512; found $($sourceParts[0])x$($sourceParts[1])."
}
if ($sourceParts[2] -notmatch 'a') {
    throw 'The canonical raster source must have an alpha channel.'
}

$sizes = @(16, 32, 48, 64, 128, 256, 512, 1024)
foreach ($size in $sizes) {
    $output = Join-Path $pngRoot "automexia-terminal-$size.png"
    & magick $sourcePath -filter Lanczos -resize "${size}x${size}" -strip $output
    if ($LASTEXITCODE -ne 0) { throw "ImageMagick failed for ${size}px PNG" }
}
Copy-Item -LiteralPath (Join-Path $pngRoot 'automexia-terminal-1024.png') -Destination (Join-Path $brand 'automexia-terminal-1024.png') -Force

$ico = Join-Path $brand 'automexia-terminal.ico'
& magick $sourcePath -filter Lanczos -define icon:auto-resize=256,128,64,48,32,16 -strip $ico
if ($LASTEXITCODE -ne 0) { throw 'ImageMagick failed to create the ICO container' }

function Write-BigEndianUInt32([System.IO.BinaryWriter]$writer, [uint32]$value) {
    $bytes = [BitConverter]::GetBytes($value)
    [Array]::Reverse($bytes)
    $writer.Write($bytes)
}

$icnsEntries = @(
    @{ Type = 'icp4'; Size = 16 },
    @{ Type = 'icp5'; Size = 32 },
    @{ Type = 'icp6'; Size = 64 },
    @{ Type = 'ic07'; Size = 128 },
    @{ Type = 'ic08'; Size = 256 },
    @{ Type = 'ic09'; Size = 512 },
    @{ Type = 'ic10'; Size = 1024 }
)
$payloadLength = 8
foreach ($entry in $icnsEntries) {
    $entry.Path = Join-Path $pngRoot "automexia-terminal-$($entry.Size).png"
    $entry.Bytes = [IO.File]::ReadAllBytes($entry.Path)
    $payloadLength += 8 + $entry.Bytes.Length
}
$stream = [IO.File]::Create((Join-Path $brand 'automexia-terminal.icns'))
try {
    $writer = [IO.BinaryWriter]::new($stream)
    $writer.Write([Text.Encoding]::ASCII.GetBytes('icns'))
    Write-BigEndianUInt32 $writer $payloadLength
    foreach ($entry in $icnsEntries) {
        $writer.Write([Text.Encoding]::ASCII.GetBytes($entry.Type))
        Write-BigEndianUInt32 $writer (8 + $entry.Bytes.Length)
        $writer.Write($entry.Bytes)
    }
    $writer.Flush()
}
finally {
    $stream.Dispose()
}

Write-Host 'Generated Automexia PNG, ICO, and ICNS assets from the canonical raster source.'
