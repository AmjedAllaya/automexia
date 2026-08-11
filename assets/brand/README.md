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

Stable publication remains blocked until the project receives and approves an
editable SVG logo and app mark, monochrome/light/dark variants, a native-quality
1024x1024 master, Linux SVG, and written redistribution-rights evidence. Only a
reviewer with that evidence may set `release.ready` and `rights_verified` to
`true` and mark every required manifest entry `final`.
