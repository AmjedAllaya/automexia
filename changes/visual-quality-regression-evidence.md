# Bind visual regression evidence to the real renderer

- Capture terminal cells and settings through their production CPU painters,
  across five themes and five scales, with Unicode and native font identities.
- Extend the existing exact-pixel comparator with bounded environment, image
  digest and physical geometry checks. Prove detection with deliberately broken
  glyphs, colors, cursor, spacing, selection and box seams.
- Preserve native Windows shell, theme, opacity and keyboard evidence, and add
  stable Linux X11 window captures to the existing AT-SPI probe. Hosted controlled
  captures remain separate from native hardware, IME and screen-reader approval.
- Fix the first plain output losing semantic colors after a wrapped prompt,
  including returning from WSL to PowerShell. Keep application ANSI untouched.
- Suppress concealed SGR 8 glyphs and decorations before shared CPU/GPU emission,
  including selection, while preserving cell positions and backgrounds. Check
  both delivered raster pixels and the native Linux window alongside AT-SPI.
- Harden native color sampling and persistence oracles so small glyphs and the
  current preference schema are actually checked. Missing reviewed baselines
  remain explicitly uncertified and are never generated as automatic approvals.
- Update macOS window-adapter calls for the installed Objective-C bindings,
  preserving retained ownership and using the existing main-thread dispatcher.
- Record actual settings fallback fonts, exercise the installed emoji chain,
  and fix independent macOS cursor hotspot coordinates, with native tests for
  missing and malformed coordinate values.
- Reuse retained CoreText fallback handles for coverage checks and preserve
  their optional file identity without reading font bytes on the render path.
