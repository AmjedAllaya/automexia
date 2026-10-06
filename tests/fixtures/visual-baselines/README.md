# Controlled visual references

Each runner-named JSON file binds 431 reviewed scenes to its actual platform,
font digests, scale, configuration, source commit and physical geometry. The
shared `images/<sha256>.png` store deduplicates identical reference images.
These are production CPU raster fixtures, not native compositor, physical
display, screen-reader or IME certification.

CI uses `tools/ci/visual_quality.py` and the existing Rust `visual-diff` owner.
It requires every reference, exact compatible identities and geometry, zero
changed pixels or channels, and all deliberate-defect checks. Missing or
incompatible references fail. CI never creates or approves replacements.

To review an update, inspect the complete candidate inventory, source identity,
test results, mutation results, representative full-resolution screenshots and
any expected/actual/diff artifacts. Explain intended differences before changing
a reference. Keep platform and font differences separate from product defects.
Never approve a reference that preserves a known defect or incomplete run.

Reference PNGs may be losslessly recompressed for repository size. Verify every
decoded RGBA pixel remains identical with the strict comparator, then bind the
stored PNG's digest in its receipt. This must not resize images, remove alpha,
quantize colors, mask regions or change comparison tolerances.

Only fictional terminal content and test configurations belong here. Store
native desktop captures and potentially identifying diagnostics privately. See
[the visual testing guide](../../../docs/TESTING.md#bound-visual-captures-and-deliberate-defects)
for coverage, commands and evidence limits.
