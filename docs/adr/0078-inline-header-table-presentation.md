# ADR 0078: Inline header-table presentation

Status: accepted. Common-code implementation with model and controlled-raster
evidence; native desktop and assistive-technology delivery remain external.

## Decision

Ordinary terminal tables with structural or typed-header evidence receive
an inline bordered presentation. Values wrap inside their own columns and each
logical row shares the tallest cell's height. This supersedes only the
focused-only scope of [ADR 0060](0060-focused-core-table-output.md); the explicit
focused viewer remains available with its existing horizontal navigation.

The existing `automexia-ui-model::tables` owner performs detection, header
confidence and bounded grapheme layout. Byte and terminal-cell ranges refer to
the original source. The application capture adapter reads complete logical
lines from the authoritative VT, preserving soft-wrap origins and source styles.
It does not parse shell commands, reexecute output, or create terminal rows.

The existing per-pane `RowProjection` owns additional display space together
with command information. Inline glyph hiding, pointer mapping and cell painting
use the same projection. Selection and copy remain terminal-owned. Generated
borders and presentation line breaks are never copied as source text. Terminal
foreground and background styles remain authoritative; unsupported decorations
retain ordinary rendering. Geometry and glyph ink are clipped to cell and pane
bounds, with one shared border between adjacent cells.

Table shading belongs to the shared application table-style owner, with quieter
automatic header and stripe tints than menu-selection surfaces. Inherited text
receives contrast correction after background composition; explicit table text,
ANSI, inverse, selection and semantic overrides retain their established order.
The existing Sugarloaf Text cell adapter accepts a terminal baseline, supplied
by the canonical cell metrics. Each shaped font run uses its own cached ascent
against that baseline; no duplicate metric cache or table row projection is added.

There are no added dependencies, workers, external capabilities, filesystem or
network operations, persistent records, or public configuration changes. Closed
or replaced pane state releases the snapshot. Unsupported program modes, uncertain
headers, insufficient physical width and resource limits retain the original
terminal presentation. VT input, cursor, normal reflow and source text are not
modified, and disabling this presentation requires no data migration.

## Resource and compatibility boundaries

Inline capture examines at most 1,024 native rows and copies at most 64 Ki
native cells near the viewport, retaining at most four surfaces. Each detected
table keeps the existing 256 KiB, 256 logical-row, 4,096-cell and 64-column
limits. Layout adds separate ceilings of 4,096 wrapped content lines and 32,768
fragments per table. Discovery tries at most eight candidate starts per contiguous
table block.
No table is made to fit by splitting a grapheme or shrinking text to invisibility.
When a captured table extends beyond 256 logical rows, the inline projection
retains its original header and optional ruler, then models only a bounded
window around visible data rows. Source-row indexes remain attached to those
rows as the viewport moves, including when the header is offscreen. A missing
header outside the copied viewport can be recovered by searching at most 1,024
preceding physical rows and 256 Ki inspected cells on a changed live or scrollback
snapshot. The header, optional top frame and ruler may soft-wrap; they share
the 64 Ki copy budget with nearby data; optional prefix reservation cannot displace
visible rows. Blank soft-wrap padding is not a table
boundary. Recent blocks receive sparse-row admission before older history, within
the same shared model-attempt limit. If
that bounded search cannot verify the header or a visible window exceeds model
limits, ordinary terminal output remains. Status tint fills each visible
data row while explicit source ANSI, inverse, and selection retain glyph
precedence; repaint uses the current source row after a watch-style rewrite.

Recognition uses distinct uppercase labels, typed data beneath mixed-case or
lowercase labels, or an explicit header ruler. Aligned whitespace, Markdown
pipes and ASCII/Unicode frames share one detector without command-specific
schemas. Ruled single-column tables and empty cells retain their source rows;
structural rulers own their horizontal edge. Ambiguous unruled string matrices,
CSV, JSON, producer-truncated output and arbitrary prose are not promoted into
an invented schema. A producer's hard line breaks
cannot be reconstructed as missing cells. Alternate-screen, mouse-reporting
and terminal vi-mode output remain outside inline presentation.

## Evidence and limits

Model tests preserve literal source ranges, shared column geometry, word and
Unicode grapheme boundaries, empty cells, impossible widths and independent
expansion limits. Application tests exercise real parser-created output and
reflow before capture, projection and inverse source mapping. Controlled raster
tests use the real text shaper with independent edge and clipping oracles,
source colours, configured selection foreground, active-pane link hover, image-only
invalidation and fixed VT glyph positions at fractional font metrics. Whole font
runs retain contextual shaping and fallback while UTF-8 cluster anchors preserve
VT cell positions. Wide trailing-cell selections highlight the entire grapheme.
The shared Text owner retains the selected font's color-glyph metadata for both
macOS raster paths; native macOS color and joining tests remain unrun.

These test layers do not establish
native compositor, physical input or operating-system accessibility delivery on
Windows, Linux or macOS. The [user guide](../user-guide/table-output.md) documents
the available interaction and limits; shared test evidence remains with the
existing assurance owners.
