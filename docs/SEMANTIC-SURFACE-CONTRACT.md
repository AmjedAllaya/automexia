# Semantic surface contract

This is a source-level extension contract and host admission model, not an
activated table browser. Existing terminal output and pane behavior are unchanged.

## Table data

A version-1 `SurfaceUpdate` carries a surface ID, nonzero generation and revision,
an extension/session/capsule/operation binding, and a loading, replace, failed or
close event. `SemanticTable` owns a schema and rows. Its schema declares stable
column IDs, titles, terminal-cell widths, priority, alignment, overflow policy,
responsive policy and data kind. Lower priority values mean more important
columns. Stable row handles are independent of order; optional resource handles
are opaque and are never commands, paths or authorization.

| Contract | Limit |
| --- | --- |
| Encoded host frame | 8 MiB |
| Columns / rows / cells | 64 / 20,000 / 320,000 |
| Total row cell text | 4 MiB UTF-8 |
| Individual cell / title / identifier | 1,024 / 128 / 64 UTF-8 bytes |

Column titles must contain non-whitespace text. IDs use ASCII letters, digits,
dot, underscore or hyphen. Display text rejects controls and bidi-direction
overrides; ordinary Unicode, combining marks and emoji remain supported. Producers
must deliberately select safe display fields; credentials and raw authentication
objects do not belong in these cells. Display data may still be confidential.
This local UI contract is not a release manifest or permission to log, persist
or publish its contents. The syntax checker cannot identify secrets in plain text.
Table diagnostics contain only row/column counts; identifier diagnostics are
redacted. This does not alter data shown to the authorized user.

Text, identifier and status labels retain exact display text. Missing differs
from empty. Status uses the existing generic semantic severity. Percentages use
integer basis points (0–10,000); bytes are unsigned integers; duration is unsigned
milliseconds; timestamps are signed Unix milliseconds. Numbers preserve signed
or unsigned integers, or coefficient times 10 to the negative scale (0–9).
No float conversion loses large integer precision.

## Host acceptance

`SemanticSurfaceSlot` receives a trusted, independently obtained UI-overlay grant
scoped to one session. The host owns one slot per operation and supplies its
binding and fresh generation; frames cannot supply grants. Wrong binding,
generation, non-increasing revisions, missing/denied/expired grants, oversized
frames and invalid data are rejected with fixed redacted errors. Revision gaps
are allowed for coalesced snapshots; ordering at a transport boundary is a
separate concern. AllowOnce permits one accepted update only. Loading or failed
updates preserve the last good snapshot. Close/revoke removes it permanently.

The caller must decode outside terminal hot paths and revoke on session, route,
capsule or extension retirement. There is no automatic registration, persistence,
worker, network connection, global registry or rendering in this model. Readiness
of this boundary does not establish interactive UI or native accessibility.

## Typed presentation state

The admitted host slot owns one `TablePresentation` from `automexia-ui-model`.
Its snapshot accessor borrows that same table; no duplicate text/grid cache is
created. The model exposes exact typed cells and a borrowed visible row range,
bounded to 1,024 rows and 16,384 terminal columns. These are presentation ceilings,
not changes to the 20,000-row data contract or the separate 256-row shell capture.
Schema preferred widths and one-cell separators define horizontal positions;
cells remain intact rather than being split into new table rows. Cell formatting,
responsive hide/details policies and drawing are not activated by this model.

Directional selection and paging do not wrap at dataset edges. Explicit scrolling
can leave selection offscreen; input selection reveals it again. Empty data has
no selection, and invalid indices do nothing. Replacement preserves the selected
row and viewport anchor by row plus resource handle within the same schema ID.
Deleted or reused resources clear selection instead of choosing a neighbor. A new
schema ID resets selection and scroll offsets. Replacement scans at most the
admitted row count, outside input/render paths. Input, fit and visible row reads
perform constant work; column projection traverses at most 64 schema entries.

Host navigation requires the exact displayed revision and a ready surface.
`presentation()` issues that revision, phase and immutable table borrow together.
Delayed input, loading/failure states, expiry, clock rollback and closed slots
cannot change selection. Last-good data stays readable until authorization ends.
Selection is not authorization to execute a resource action. Native pointer
adapters must resolve logical rows from current layout, bind press/release to the
same view identity, and carry the displayed revision rather than substituting a
new one at dispatch. No keyboard shortcut, clipboard write or cluster request is
introduced here. The ordinary terminal table view remains unchanged.

## Verification

Run `cargo test -p automexia-extension-api` and
`cargo test -p automexia-terminal --test semantic_surface_admission`.
Run `cargo test -p automexia-ui-model --test semantic_table` for typed navigation,
refresh, borrowed cells, extreme dimensions and fixed-seed mixed transitions.
Run `cargo bench -p automexia-ui-model --bench semantic_table_presentation`
for checked navigation/projection and replacement/drop at 0/1/100/2,000/20,000 rows.
These optimized model timings include correctness assertions; they do not measure
native rendering, input-to-photon latency, API traffic or process memory.
`cargo bench -p automexia-extension-api --bench semantic_surfaces` measures the
bounded table decoder, typed schema/row construction and validation separately,
and bounded diagnostic summaries. Constructor measurements exclude fixture cloning
and keep one input alive per iteration; timer overhead matters for tiny cases.
The `semantic_surfaces` fuzz target has a minimal feature
set and reviewed seeds under `fuzz/seeds/semantic_surfaces`; generated corpus and
crashes belong in ignored managed output. Allocation tests measure owned bytes
and cleanup in a fixed fixture, not process RSS or native graphics resources.
