# Public UI branding status

Automexia's public visual system is the Liquid Hacker terminal experience:
compact, keyboard-first, high-contrast capable, and focused on terminal content.

## Current public contract

- consistent Automexia name, icon, colors, and typography;
- terminal-first windows, tabs, pane-local tabs, active outlines, and scrollbars;
- compact command palette, search, diagnostics, appearance controls, image
  preview, and quit confirmation;
- visible focus and redundant text/shape/state meaning;
- reduced motion and responsive small/high-resolution layouts;
- no credentials, hidden history, private paths, or unrelated pane content in
  chrome or diagnostics; and
- no decorative surface may change PTY, shell, process, clipboard, or native
  dialog authority.

## Verification

Renderer-neutral layout/semantic tests, exact controlled raster comparisons,
and native platform/accessibility evidence are separate requirements. Intentional
visual changes require reviewed exact baselines.

## Evidence ledger

Record the exact commit, renderer, operating system, scale, viewport, theme,
font set, controlled raster identity, accessibility tree/event result, native
frame review, and any external human-review gate. Missing platform or
assistive-technology evidence remains explicitly unverified.

This page is a current branding status, not a future UI roadmap. Private product
surfaces and commercial branding plans remain private.
