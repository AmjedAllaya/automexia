# Terminal interaction requirements

Status: technical usability corrections and ordinary terminal improvements.
These requirements do not announce newly available actions or configuration
keys. [Keyboard](KEYBOARD.md), [Configuration](CONFIGURATION.md), and the
[user guide](user-guide/index.md) describe the current implementation.

This document complements [Terminal maintenance requirements](TERMINAL-MAINTENANCE-REQUIREMENTS.md).
The product remains a keyboard-first terminal with lightweight in-terminal
surfaces. Existing pointer behavior remains supported, but no essential action
may require a pointer. A separate graphical application framework or web
frontend is not required for these corrections.

## Ownership and inspected limitations

Source observations use the same `3cb729af27` baseline as the maintenance
requirements. Revalidate them before implementation.

| Area | Current authoritative owner | Correction boundary |
|---|---|---|
| Palette | `apps/automexia-terminal/src/renderer/command_palette.rs`, action dispatch in `screen/mod.rs` | Improve discovery and navigation without a second executor |
| Shortcuts | `automexia-keybindings/src/`, application `bindings/registry.rs` and legacy `bindings/mod.rs` | Use effective binding state; account for the legacy fallback |
| Runtime preferences | `apps/automexia-terminal/src/automexia/preferences.rs` | Existing schema 1 stores font size and appearance; additional persisted controls need explicit schema/design review |
| Context and compact layout | Application `renderer/`, `context/renderable.rs`, `grid_emit.rs` | Render existing validated session state; do not create a second context store |
| Local preview | Application `image_preview.rs`, `automexia-image/`, Sugarloaf image consumers | Retain bounded worker, route generation, current file authority and cache limits |
| Shell listings | Existing shell integrations and terminal output | Preserve shell objects/bytes and copy semantics |
| Semantics and accessibility | Renderer-neutral state and existing native adapters | One focus/layout model for keyboard, paint, hit testing and accessibility |

The existing palette uses frontend `PaletteAction` values; the typed keyboard
registry has its own action contracts. A single effective action description is
the required convergence point, not a claim that both paths are already fully
unified. Adapt through the application owner and preserve existing dispatch.

## I1: Make command discovery accurate and keyboard-complete

### Discovery and presentation

Keep one searchable palette for existing terminal actions. Use understandable
groups such as windows, tabs/panes, search, appearance, and local files, without
forcing users to browse a hierarchy for an action they can search directly.

For each visible action, expose a stable identity, concise label, applicable
scope, effective shortcut or “unbound,” availability, and an actionable reason
when unavailable. Labels must distinguish a fresh split from a clone, a
pane-local tab from a window tab, and selected-pane search from workspace
search. Do not silently substitute another operation for an unavailable action.

Resolve availability from the actual build/platform, active input mode,
selection/target, and current capability checks. An installed component,
registered action, or successful model test is not proof an operation is
enabled. Hide implementation-only entries that have no supported user path;
where a familiar action is temporarily unavailable, show its reason.

### Keyboard and action contract

Search must preserve deterministic ordering, selected identity, visible focus,
and query text when the result list refreshes. Support arrows, confirm, cancel,
and back navigation with an explicit rule for each nested surface. Esc returns
to the previous level or closes the top surface; it must not also send Escape
to the terminal. Closing restores the original input owner if still valid,
otherwise an explicitly chosen surviving terminal target.

Take a target snapshot when the action is accepted and validate its route,
generation, scope and capability again before dispatch. Search/filter/open do
not execute, launch, read the clipboard, inspect the filesystem, or query a
provider. Existing privileged actions retain the application broker and typed
argument boundary. Capability checks do not belong in the renderer.

Use the effective keyboard registry and its legacy adapter for shortcut badges,
help and collision explanations. Do not maintain another static table of
supposedly active chords. The conflict correction and migration expectations
are in [R4](TERMINAL-MAINTENANCE-REQUIREMENTS.md#r4-restore-native-shell-control-keys-by-default).

### Acceptance

Test empty/one/many results, long and localized labels, repeated labels with
distinct scope, unavailable actions, stale targets, profile changes, user
unbinds, invalid reload, nested back/cancel, and route closure during activation.
Assert one action dispatch at most, exact target, visible selected row, restored
focus, and zero PTY bytes from navigation or cancellation. Search latency and
memory remain bounded when the catalog is at its supported limit.

## I2: Make ordinary preferences discoverable and reversible

### Existing implementation to preserve

Hand-edited `config.toml` remains the declarative authority. The existing
application-owned runtime overlay persists font size and appearance without
rewriting comments or unrelated configuration. Its inspected schema version
is 1 and maximum payload is 16 KiB. Do not claim that arbitrary context,
preview, or keybinding preferences already persist in that format.

Expose supported settings through keyboard actions and the configured editor.
A value not supported by current source must be labelled as a requirement here,
not shown as a copyable TOML option in a current reference.

### Required maintenance refinements

Where ordinary appearance controls are exposed, define their label, type,
valid range, default, ownership, platform scope, reset behavior, and whether
the change is immediate or requires restart. Font size, theme, line spacing,
contrast, background treatment, and compact terminal chrome must remain
consistent across existing panes and newly created windows.

A preference must not disable essential focus indication, hide the only way to
close a surface, falsify session identity, or bypass input/capability safety.
Offer useful small choices rather than making every layout invariant mutable.

For additional runtime-editable values, extend the existing validated
application transaction only after deciding whether they belong in the
declarative config, runtime overlay, or transient surface state. Do not create
per-renderer, per-extension, and per-window files for the same preference.

Define candidate validation, atomic publication, persistence outcome, and
rollback. If a runtime change is active but saving fails, visibly distinguish
“applied for this session” from “saved”; never imply persistence succeeded.
Reset removes only the relevant override and immediately resolves the remaining
configuration layers.

### Persistence and migration acceptance

Test startup, restart, multiple windows, concurrent updates, default/reset,
invalid values, unknown/duplicate keys, oversized or corrupt input, permission
and disk-full failures, interrupted writes, linked/replaced files, and supported
predecessor versions. Preserve last-known-good state and user-authored content.
A format change needs an ADR and migration/rollback ownership, not an
unreviewed schema increment in a rendering patch.

Verify exact effective values and storage-tree changes independently.
Documentation must show actual defaults, precedence, saving failures, reset,
disable and recovery only when the corresponding behavior exists.

## I3: Keep context, paths and compact layouts readable

### Context already owned by the session

Display only validated session information available to the current terminal:
for example the shell, working directory, active pane/local tab, command
status, and other already supported shell-integration metadata. Stale or
unavailable metadata must be identifiable; a guessed value is not live state.

Keep one owner for the existing session model. A context bar is a presentation,
not a new database, provider client, authentication authority, or shell prompt
engine. Missing integration must leave the normal terminal usable. Metadata
refresh must not perform filesystem or external-command work during paint.

### Responsive behavior

Define large, compact and minimum-usable layouts in the existing geometry
model. Reserve space for the terminal's cursor, active focus, modal controls and
critical status before decorative items. Lower-priority context can be elided
or hidden at narrow sizes, with keyboard access to retained full information.

Long paths preserve useful root and trailing components where possible. Elide
by valid text/grapheme boundaries. Copy/open actions use the validated full
value, never the shortened display label. Do not perform a filesystem probe
just to decide how a path is drawn.

Path placement choices apply only to application-owned context/list rows.
Automexia must not rewrite arbitrary shell prompt bytes to move the directory
inside or outside a prompt. This distinction also applies to custom prompts
and remote sessions.

Pane-local tabs, footers, search, transient notices and previews must share
space predictably. A transient surface must not repeatedly shrink and expand
the PTY or steal focus on arrival. Nonessential animation cannot delay typing.
Minimal display still needs a keyboard way to reveal hidden context.

### Acceptance

Use tiny panes through 8K-equivalent layouts, many splits, long labels and
paths, mixed-width Unicode, font changes, 100–300% scale and 200%/400%
accessibility scenarios where applicable. Assert shared geometry, clipping,
focus visibility, correct ellipsis, full-value copy, no overlap with the cursor,
and no extra filesystem/process calls on resize or paint. Test delayed/stale
metadata and route replacement without cross-pane labels.

## I4: Improve local image-preview placement without widening authority

### Existing path

Quick look already uses a bounded worker, generation-aware completion,
geometry anchors, thumbnail cache and renderer resource ownership. Its
inspected worker queue holds 16 owners and hover stability delay is 100 ms.
[Image previews](IMAGE-PREVIEWS.md) remains authoritative for formats,
file/decode/cache limits, current keyboard behavior, and native evidence.

Keep local quick look distinct from inline terminal graphics. No change here
authorizes remote fetches, directory scans, additional codecs, or bypassing
file identity and link checks.

### Required interaction corrections

Anchor the preview to the actual retained filename/path row when that row is
known. Prefer a placement above it that leaves the filename readable; use below
or a bounded in-pane fallback when necessary. If no usable space exists,
provide an explicit accessible outcome rather than covering the only input
area. Never infer the target solely from an old pointer pixel coordinate after
resize, reflow or scroll.

Keyboard activation must retain the selected path's target identity and owner,
not depend on where the pointer last happened to be. If a reliable row anchor
cannot be recovered, use a clearly defined selected-target fallback or require
reselection. Do not pretend a geometric anchor is already a persistent
semantic row identity.

Keep fit/aspect ratio, title/metadata space, clipping and dismissal consistent.
If zoom/fit controls are added to the existing preview, they operate only on
already authorized decoded content and stay within current texture/resource
budgets. They are requirements, not newly available keybindings.

Image-to-image navigation uses already known eligible paths in the documented
scope. It must not silently expand to enumerating a directory or reading hidden
scrollback. Hover cannot capture navigation keys; pinning must be explicit.

On dismissal, stale generation, route closure, permission failure or file
replacement, cancel or reject pending work and remove active surface/GPU
resources. Only the separately bounded reusable thumbnail cache may survive.

### Acceptance

Test pointer and keyboard activation, above/below/fallback placement, one-cell
edges, long filenames, zoom/font/DPI changes, reflow, file replacement,
malformed/oversized files, load failure, empty target, remote-looking paths,
mouse-reporting TUIs, fast navigation, route/window isolation, and shutdown.

Check filename visibility, exact anchor/clip geometry, focus restoration and
no PTY resize/input. Compare bounded worker/mailbox/cache accounting and GPU
release after repeated open/dismiss cycles; a visually closed card is not
sufficient cleanup evidence.

## I5: Preserve shell output and full-screen TUI behavior

A terminal must remain useful for ordinary installed command-line tools.
Test representative editors, pagers, file managers and resource-monitoring TUIs
inside a normal PTY, including an installed K9s where an isolated fixture is
available. This is terminal compatibility, not a promise of a native provider
workspace or a bundled external executable.

Alternate-screen transitions, cursor modes, bracketed paste, mouse reporting,
scrolling, keyboard ownership, resize, focus and return to the shell must
remain intact. Optional application chrome must not consume a TUI's keys or
reinterpret its output as application commands.

For existing enhanced shell listings, preserve the shell's objects and pipeline
bytes. Decorative glyphs belong inside the intended name field, with bounded
display-width-aware formatting. At narrow widths retain a useful name/ellipsis;
do not create a misleading detached icon-only column. Copied paths and explicit
actions use validated full targets, not decorated display text.

Do not parse arbitrary command output into an authoritative structured result.
Keep unsupported or ambiguous output as ordinary terminal text. A maintenance
fix to listing geometry does not require a new structured-output subsystem.

Acceptance compares raw/piped shell results with integration enabled/disabled,
narrow/wide listings, Unicode and control-character filenames, nonexistent and
replaced targets, selection/copy, and normal/alternate-screen round trips.
Use isolated fixtures and no live infrastructure for public evidence.

## I6: Use one geometry and accessibility model

Use existing renderer-neutral state for pane transforms, cells, cursor,
selection, links, IME, overlay anchors, clipping, z-order, focus and semantic
roles. Consumers must not independently round the same scaled position in
different ways. A visible active border and semantic focus must identify the
same owner after every navigation and layout change.

Every actionable surface needs keyboard entry, a readable accessible name,
visible current selection, cancellation, and predictable focus restoration.
Loading, empty, error, unavailable, stale and confirmation states need explicit
semantics; repeated background status must not overwhelm live announcements.

Use readable contrast in dark, light and high-contrast themes. Status must not
depend only on red/green coloring. Respect reduced motion and keep focused
controls unobscured. WCAG's
[focus-not-obscured guidance](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html)
is a useful interaction reference; this document does not claim a desktop
application's accessibility conformance from that reference alone.

### Three independent evidence layers

1. Renderer-neutral assertions for geometry, hierarchy, clipping, z-order,
   semantic relationships, keyboard ownership and focus restoration.
2. Deterministic controlled raster comparisons with exact RGBA equality.
   Freeze nondeterminism or use environment-specific baselines rather than
   broad tolerances or masks.
3. Native rendered frames and assistive-technology checks on each claimed
   platform: Narrator/NVDA, VoiceOver, or Orca as applicable.

Include tiny and large viewports, high scale, long/localized strings, combining
text, IME composition, multiple panes/tabs/windows, theme changes, modal
stacking, pointer overrides and enabled/reduced motion. Native manual evidence
remains required where automation cannot establish actual usability.

## Technical change review

Before implementing an interaction correction, record:

- the current behavior and concrete user-visible failure or friction;
- core or existing optional-owner placement and affected consumers;
- the smallest model/adapter change and why no new frontend/executor is needed;
- input, capability, persistence and resource boundaries;
- supported actions, labels, modes, effective shortcuts and fallback states;
- exact regression scenarios and forbidden side effects;
- native/accessibility evidence and any external prerequisites; and
- guide/reference updates that become valid only when code and evidence agree.

These requirements extend existing terminal mechanisms. They do not approve a
new dependency, persisted format, capability, third-party runtime, graphical
frontend, or public feature-availability claim. Material boundary changes still
require the existing architecture and ADR review.
