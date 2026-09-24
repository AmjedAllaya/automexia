# Bordered inline header tables

Recognized header tables gain borders and per-cell wrapping in narrow panes,
with aligned row heights and complete grapheme boundaries. Source colours,
selection and clipboard text remain terminal-owned; generated borders and line
breaks do not alter the VT grid or original output. The focused table viewer
remains available.

Recognition handles whitespace, Markdown and ASCII/Unicode frames, with typed or
ruled headers independent of command names. Contextual text shaping and wide-cell
selection stay within the shared cell geometry. Recognition and expansion are bounded. Unsupported text, program modes or
geometry retain ordinary terminal presentation. Model and integration checks
are separate from native desktop and assistive-technology evidence; integration
tests exercise actual parser-created output and font glyphs.
