# Brand assets

`automexia-terminal-source-512.png` is the supplied transparent application
mark and the canonical raster source for non-stable packaging. Its origin and
SHA-256 digest are recorded in `ASSET-MANIFEST.toml`.

Regenerate deterministic PNG, ICO, and ICNS derivatives with:

```powershell
powershell -NoProfile -File tools/brand/generate-platform-assets.ps1
```

The generation command validates the source dimensions and alpha channel before
writing any derivative. Packaging consumes only these Automexia assets; no Rio
artwork or old placeholder is used.

Windows MSI artwork uses the same mark on a blue-black panel with cyan and violet
accents. Regenerate its 24-bit BMPs and the license RTF in a Windows build VM:

```powershell
powershell -NoProfile -File tools/brand/generate-windows-installer-assets.ps1
```

This generator uses Windows GDI+ and Segoe UI. Review the output when Windows
fonts change. The 493x312 dialog reserves its right side for live installer text;
the 493x58 banner reserves space for headings. Keep instructions out of the images.
Commit the generated files so packaging needs no graphics tools.

Stable publication remains blocked until the project receives and approves an
editable SVG logo and app mark, monochrome/light/dark variants, a native-quality
1024x1024 master, Linux SVG, and written redistribution-rights evidence. Only a
reviewer with that evidence may set `release.ready` and `rights_verified` to
`true` and mark every required manifest entry `final`.
