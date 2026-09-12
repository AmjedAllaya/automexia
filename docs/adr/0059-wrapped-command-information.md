# ADR 0059: Display-only wrapping for command information

## Decision

Command context and completion information share a measured, pane-local layout.
When a proven blank semantic prompt row is too narrow, it expands into display
rows. Labels retain their complete supplied values, prefer word boundaries and
otherwise split at grapheme boundaries. Completion text participates in the same
packing operation; it cannot reserve away all space for context. A wide pane
uses the original single-row presentation when everything fits.

This belongs to core application presentation. The optional DevOps contribution
still owns its values and refresh lifecycle; the VT engine owns native rows,
completion identities and PTY coordinates. Neither an extension-owned layout nor
another copy of terminal history would provide a safe shared coordinate owner.

## Ownership and data flow

The existing application UI module owns bounded packing and row projection.
`renderer/command_info.rs` combines the existing semantic anchors, immutable
context values and core completion labels. Only verified blank rows expand;
unproven legacy completion anchors retain their conservative existing fallback.
Core completion remains usable with the context extension disabled.

`RenderableContent` owns each pane's ephemeral projection. Grid glyphs,
backgrounds, selection/search decoration, pointer hit testing, caret positioning,
image placement and scrolling consume that projection. Inserted display slots
never repeat native cells. Images are sliced at projection discontinuities with
texture coordinates preserved. Native protocol coordinates, copied text, PTY
dimensions and terminal history do not acquire synthetic rows.

Scrolling can reach wrapped information even with no native history. Queued
wheel events accumulate, paging uses display rows, and normal input returns to
the live cursor. Alternate-screen and vi-mode native scroll semantics remain
with the terminal. Short output does not manufacture scrollable blank padding.
View state is neither persisted nor shared between panes or tabs.

## Resource and trust boundaries

No dependency, filesystem access, provider request, worker, timer or persistence
is added. Labels are bounded to 16 items and 1,024 bytes each, below which the
existing contribution validation still applies. Geometry must be finite.
Grapheme splitting always advances; an indivisible glyph wider than the usable
line scales down. Prefix storage is bounded by 65,535 native display rows and
reused. Unexpanded frames need no prefix allocation. Image work remains bounded
by the existing visible placement and viewport limits, and idle image geometry
is not recalculated. Context and completion identities remain separate even
when they share one visual band.

## Evidence and limitations

Regression ownership is in the application UI model and renderer module.
Tests exercise real parser-created anchors, complete labels, narrow/wide
restoration, native-cell invariance, queued scrolling, short viewports, image
texture cropping, real bundled-font draw data and controlled CPU text pixels.
Correctness-checked application benchmarks distinguish layout and text-shaping
cost from native PTY, GPU and compositor latency.

These deterministic checks do not certify native desktop rendering, assistive
technology, IME delivery or every shell/backend combination. Those remain
separate gates in the feature reinforcement plan. See
[the verification commands](../TESTING.md#wrapped-command-information).

The design follows the information-preservation principle in
[W3C Reflow](https://www.w3.org/WAI/WCAG21/Understanding/reflow.html), without
claiming accessibility certification. Native buffer coordinates remain distinct
from display layout, also illustrated by the
[xterm.js buffer contract](https://xtermjs.org/docs/api/terminal/interfaces/ibuffer/).

Rollback removes the application projection and restores the earlier painters;
there is no schema, profile, history or credential migration to undo.
