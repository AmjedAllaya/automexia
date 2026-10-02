# ADR 0084: Active prompt viewport follow

Status: Accepted

## Context

ConPTY can move the beginning of a semantic prompt into retained history when a
pane becomes very small. Growing its native viewport does not pull those rows
back. The complete prompt remains in the grid, but forcing the visible viewport
to native row zero leaves only the path's suffix. Rewriting the prompt at native
row zero would move text beneath the shell's cursor and corrupt subsequent input.

The native regression reproduces this on both inbox and bundled ConPTY, using
the same fixture executable. At 140 columns by 14 rows, then 3 by 2, then 32 by
15, retained context is complete and the native cursor still owns the input.

## Decision

Keep the grid and its display offset as the sole viewport source. Distinguish
automatic active-prompt follow from manual scrollback. Automatic follow may
reveal real, contiguous rows belonging to the current semantic prompt when the
complete context and native input cursor fit together. It never reconstructs
text from a directory string or changes the native cursor, shell input, or PTY.

Only resize and completed PTY batches reconsider the bounded active prompt.
Inspect at most one viewport's rows (capped at 512 rows and 16,384 occupied
cells/context bytes), require the current generation's prompt
start and complete saved context, and leave an oversized or incomplete block
unchanged. Idle rendering performs no history search. Alternate screens, reset,
new prompt generations, command output and manual scrolling release automatic
follow; the existing bottom/follow action resumes it when appropriate.

The renderer snapshots the same source rows, styles and extras as manual
scrollback. Its cursor is translated through the display offset only during
automatic follow. Existing RowProjection then handles header/table expansion.
Selection, copy, mouse coordinates, hints and images retain their existing
display-offset mapping. The raw cursor API remains unchanged; only the renderer
uses the explicit viewport cursor. Search and VI mode preserve the displayed
source range while retiring automatic follow. Live prompt chrome and input follow use explicit follow
state rather than treating every nonzero offset as user scrollback.

## Verification and compatibility

The native fixture must preserve the exact context once, match native/raw cursor
coordinates, and accept the next character at the displayed input. Model tests
cover source mapping, styles, Unicode, manual scroll, bottom, reset, alternate
screen, generation changes, no-fit and bounded lookup. Renderer tests cover live
headers and RowProjection cursor mapping. The existing native resize gate remains
unchanged. Native checks on Windows do not claim Linux or macOS execution.

Controlled native transport checks pass with both inbox and bundled ConPTY.
The application's full native resize scenario remains a separate required gate;
this decision does not assert that gate has passed.

This is in-memory presentation state; no configuration, protocol, persisted
record, dependency or public CLI format changes. Removing the automatic follow
state restores the prior viewport behavior without data migration.
