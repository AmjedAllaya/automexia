# Run in a Windows build environment. Uses the canonical mark and built-in GDI+.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$source = Join-Path $root 'assets\brand\automexia-terminal-source-512.png'
$output = Join-Path $root 'assets\brand\windows-installer'
$logo = [Drawing.Image]::FromFile($source)
try {
    if ($logo.Width -ne 512 -or $logo.Height -ne 512) {
        throw 'The canonical Automexia mark must be 512 x 512 pixels.'
    }
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    foreach ($height in @(312, 58)) {
        $bitmap = [Drawing.Bitmap]::new(493, $height, [Drawing.Imaging.PixelFormat]::Format24bppRgb)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $owned = [Collections.Generic.List[IDisposable]]::new()
        try {
            $graphics.Clear([Drawing.Color]::White)
            $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
            if ($height -eq 312) {
                # WiX overlays live text on the white area to the right of x=170.
                $panel = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#0B1224'))
                $owned.Add($panel)
                $graphics.FillRectangle($panel, 0, 0, 165, 312)
                $graphics.DrawImage($logo, 20, 35, 125, 125)
                $title = [Drawing.Font]::new('Segoe UI', 14, [Drawing.FontStyle]::Bold, [Drawing.GraphicsUnit]::Pixel)
                $subtitle = [Drawing.Font]::new('Segoe UI', 10, [Drawing.FontStyle]::Regular, [Drawing.GraphicsUnit]::Pixel)
                $ink = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#B8C5DD'))
                $owned.Add($title); $owned.Add($subtitle); $owned.Add($ink)
                $graphics.TextRenderingHint = [Drawing.Text.TextRenderingHint]::AntiAliasGridFit
                $graphics.DrawString('AUTOMEXIA', $title, [Drawing.Brushes]::White, 24, 177)
                $graphics.DrawString('T E R M I N A L', $subtitle, $ink, 24, 200)
                $accentArea = [Drawing.Rectangle]::new(0, 307, 165, 5)
                $name = 'dialog.bmp'
            } else {
                # Keep the title/description area white; the mark is decorative.
                $graphics.DrawImage($logo, 440, 7, 43, 43)
                $accentArea = [Drawing.Rectangle]::new(0, 55, 493, 3)
                $name = 'banner.bmp'
            }
            $accent = [Drawing.Drawing2D.LinearGradientBrush]::new(
                $accentArea, [Drawing.ColorTranslator]::FromHtml('#36C5F0'),
                [Drawing.ColorTranslator]::FromHtml('#8B5CF6'), 0.0)
            $owned.Add($accent)
            $graphics.FillRectangle($accent, $accentArea)
            $bitmap.Save((Join-Path $output $name), [Drawing.Imaging.ImageFormat]::Bmp)
        } finally {
            foreach ($item in $owned) { $item.Dispose() }
            $graphics.Dispose()
            $bitmap.Dispose()
        }
    }
} finally { $logo.Dispose() }

# WiX needs RTF for its live, scrollable license control. Keep LICENSE authoritative.
$license = [IO.File]::ReadAllText((Join-Path $root 'LICENSE')).Replace("`r`n", "`n")
if ($license -match '[^\x00-\x7F]') { throw 'Add RTF Unicode encoding before using a non-ASCII license.' }
$escaped = $license.Replace('\', '\\').Replace('{', '\{').Replace('}', '\}').Replace("`n", '\par ')
[IO.File]::WriteAllText((Join-Path $root 'packaging\windows\license.rtf'),
    ('{\rtf1\ansi\deff0{\fonttbl{\f0 Segoe UI;}}\fs18 ' + $escaped + '}'), [Text.Encoding]::ASCII)
Write-Host 'Generated Automexia Windows installer artwork and license RTF.'
