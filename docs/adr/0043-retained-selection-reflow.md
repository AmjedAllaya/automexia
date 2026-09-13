# ADR 0043: Retained selection reflow

Status: Accepted for current source; native visual validation outstanding.

## Decision and placement

Terminal core retains non-rectangular selection anchors through column changes.
The existing grid resize owner tracks two selection cells and, when scrolled,
one first-visible cell while splitting and merging rows, beside its image
row-remap logic. Selection keeps its original
direction, cell edge and type. There is no second text buffer, substring search,
per-cell identity table, persistent state, extension, dependency or new worker.
Height-only selection rotation remains in its existing owner.
When the Vi cursor owns the selection's active endpoint, it follows that mapped
endpoint before the existing viewport clamp. An unrelated Vi cursor is not moved
to a mouse selection.

The upstream [Alacritty resize implementation](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/resize.rs)
uses the row-movement structure from which this grid derives. Automexia extends
its current owner instead of adopting another terminal implementation. A core
selection must work without an extension; a separate reflow adapter would
duplicate content ownership. The tracking storage is fixed-size, and work is
constant per row already visited by reflow. Existing no-reflow fast paths remain.

## Loss and compatibility policy

A scrolled normal buffer keeps the row containing its former first-visible
cell at the top after reflow, even when that buffer is inactive behind an
alternate-screen application. Numeric distance from the live bottom cannot
provide this identity when rows below the viewport split or merge. A merge may
bring preceding cells onto the same row; no horizontal offset is introduced.
If a taller viewport absorbs the anchor into the live screen, clamp at the live
bottom. If the tracked cell is genuinely removed as padding or evicted, retain
the existing bounded-offset fallback without inventing an anchor. Alternate
screens still have no scrollback and bottom-follow remains bottom-follow.

If either anchor cell is invalid, cropped, evicted, or discarded as reflow
padding, clear the selection instead of attaching it to replacement content.
This includes a wide glyph destroyed by the existing one-column grid policy.
The tracker does not change which terminal cells that policy retains. A
rectangular selection remains tied to physical columns and is cleared when
width changes, as before. A semantic or line selection retains its type and
recomputes its extent against the new layout.

Alternate-screen resize retains only cells that actually survive cropping; it
does not create scrollback or promise persistence across application redraws.
No resize operation writes selection text to the PTY or another pane. Reversion
needs no migration, but must retain the reproducer and record the lost behavior.

## Evidence boundaries

The original parser-to-selection regression failed on its first width change.
Coverage now includes both anchor directions, explicit cell edges, intermediate
height/width changes, repeated scrolling, semantic/line selections, wide and
combining characters, zero-history eviction, alternate-screen cropping, invalid
anchors, and an independently resized unselected control grid. A benchmark
measures reflow, copy and visible snapshot production over 10,000 history rows.

The first parser-created viewport regression failed at 120x9 before the viewport
fix. Input-assigned truecolors identify otherwise identical text independently
of row numbers. Tests compare every intermediate first row, selected bytes,
snapshot styles, live-follow sibling cells, inactive normal-buffer retention,
and real eviction/history absorption. No PTY writes are permitted. The benchmark
also measures a scrolled viewport with a saved same-host pre-change baseline.

A Windows-only conformance fixture additionally launches real PowerShell through
the existing ConPTY adapter, with profiles disabled and explicit child-local
UTF-8. It requires successful process exit and a parsed completion marker before
selecting the retained output and exercising resize. The raw default-code-page
attempt failed before resize and is retained in the private evidence record.

These tests do not prove native mouse/keyboard selection, ConPTY repaint,
renderer pixels, screen-reader behavior or original window-output-loss recovery.
Those gates remain separate. The published Linux 0.4.0 prerelease predates this
source correction.
