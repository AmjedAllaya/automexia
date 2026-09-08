Move grid CPU blending and glyph blits into a private Sugarloaf module without
changing their pixel rounding or atlas ownership. Add independent literal-pixel,
actual row/cursor and repeated-storage tests, with real grid-paint benchmarks.
