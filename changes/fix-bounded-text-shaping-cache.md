# Keep text measurements and cached glyphs consistent

Immediate-mode text now keys shaping, ascent and private glyph-atlas entries by
the size actually rasterized, fixing stale measurements and pixels at nearby
font sizes. A renderer-local cache bounds retained runs by entry count and
text/glyph capacity, checks full identities despite digest collisions, and
shares immutable glyph data on hits. Original labels and extension boundaries
are unchanged. Regression tests cover exact CPU pixels, cache bounds and
cleanup; an actual-owner benchmark supports same-host comparisons.
