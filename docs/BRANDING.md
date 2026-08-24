# Automexia brand

## Product message and voice

Automexia's core promise is:

> A flexible terminal that makes complex workflows faster, simpler, and easier to control.

Automexia is for anyone who turns ideas into action through commands. Public
writing should explain the practical value first: less repeated setup, clearer
organization, flexible workspaces, and visible user control. The
[Product vision](PRODUCT-VISION.md) is the authority for the purpose, audience,
values, and broader direction.

Use a simple, confident, and natural voice:

- speak about people or users unless a passage is specifically for developers;
- use concrete words such as commands, files, tasks, output, panes, and tools;
- explain technical details when someone needs them to use, evaluate, build, or
  trust the product;
- keep what works today separate from implemented-but-gated, planned, and
  research work;
- describe future media, video, AI, and ecosystem ideas as planned or research
  until their own delivery evidence exists.

Avoid inflated marketing claims, vague automation jargon, presenting future
ideas as available, or suggesting that Automexia replaces the specialist tools
a workflow depends on.

## Visual asset workflow

Automexia packages one canonical application mark across Windows, macOS, and
Linux. The repository snapshot of the supplied source is
`assets/brand/automexia-terminal-source-512.png`; its origin and SHA-256 digest
are recorded in `assets/brand/ASSET-MANIFEST.toml`.

## Regenerating platform assets

On Windows with ImageMagick 7 installed, run:

```powershell
powershell -NoProfile -File tools/brand/generate-platform-assets.ps1
```

The script validates that the source is a transparent 512x512 PNG and creates:

- Linux PNGs at 16, 32, 48, 64, 128, 256, and 512 pixels;
- a 1024-pixel derived PNG used by packaging metadata;
- a Windows ICO containing 16 through 256-pixel entries;
- a macOS ICNS containing standard 16 through 1024-pixel entries.

Run `cargo ready` after regeneration. Its repository and package gates check the
source checksum, PNG dimensions and alpha, ICO size directory, ICNS entries,
package references, and platform runtime icon wiring.

## Stable-release approval

Raster derivatives are sufficient for development and nightly packages, but do
not satisfy the stable release gate. Stable publication additionally requires:

- editable SVG logo and standalone mark sources;
- monochrome, light, and dark variants;
- a native-quality 1024x1024 master and Linux SVG;
- written redistribution-rights evidence;
- named reviewer approval and an approval timestamp.

After reviewing those inputs, a maintainer records them as `final` in
`ASSET-MANIFEST.toml`, sets `rights_verified = true`, and only then sets
`release.ready = true`. Generated files must never be marked final merely
because they pass structural validation.
