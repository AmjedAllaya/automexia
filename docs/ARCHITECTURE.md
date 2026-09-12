# Architecture

This document describes the public architecture of Automexia's free terminal.
It is intentionally limited to current terminal behavior and the maintenance
boundaries needed to keep that behavior secure and reliable. Unreleased
advanced features and commercial architecture are maintained privately.

## Layers

Dependency flow is one way:

```text
application
  -> terminal/session coordination
  -> renderer-neutral snapshots and input routing
  -> VT state and PTY/process adapters
  -> platform adapters

optional installed extension
  -> versioned capability contract
  -> application-owned broker
```

The VT parser, PTY owner, renderer, configuration store, and application each
have one authoritative state owner. Optional code cannot become a second owner
for terminal cells, process lifetime, routes, focus, or persisted settings.

## Terminal and session ownership

The native PTY adapter selects the core grid's resize policy before output is
served. ConPTY (including WSL on Windows) retains its history-free mutable
viewport origin; Unix PTYs retain their native reflow behavior. Wrapped history
seams use non-content padding rather than a duplicate output cache. Column
reflow also distinguishes ConPTY hard-line fill from Unix explicit whitespace:
native trailing padding cannot create extra logical output rows. Native seam
partitioning preserves the final real cell, including inter-column blanks, and
the native cursor reserves its cell at an exact shrink margin. All-blank soft
wraps retain their spaces when copied; only hard breaks create newlines. Forced wraps,
cursor distance and text extras remain owned by the grid. Prompt
editing does not authorize claiming historical repaint rows. See
[ADR 0048](adr/0048-native-resize-and-prompt-ownership.md).
For mixed-axis native resizing, column reflow precedes height reduction so live
text is not prematurely archived. Printed-space fill below the cursor uses the
native content predicate; former seam fragments retain soft wraps and non-content
padding even after moving entirely into history.
For managed ConPTY sessions, the PTY worker commits native and grid dimensions
from the same coalesced resize. UI requests update cell metrics without reflowing
ahead of that transaction. Renderer snapshots use the last committed grid and
refresh after its damage event; a failed native resize cannot advance grid size.

ConPTY prompt recovery cannot move the native protocol cursor. Input remains
ordered through the PTY write adapter before later resizes; channel batches are
bounded. Successful Windows character-grid changes allow a 50 ms asynchronous
input-settle interval for native editor compatibility, while output and shutdown
remain live. Ordinary input and Unix PTYs have no such interval. See ADR 0048
for the measured failure, tradeoff and removal criteria.

Selection endpoint tracking belongs to the existing VT grid resize traversal,
with two fixed-size trackers and no alternate terminal buffer. The
[selection reflow contract](adr/0043-retained-selection-reflow.md) defines
cropping, eviction and rectangular-selection behavior.

A session owns exactly one PTY or ConPTY endpoint, child-process lifecycle,
terminal state, ordered input queue, resize generation, and exit outcome. Panes
and tabs reference sessions through stable route identities. Closing a view
cannot silently attach another view to the same PTY or transfer ownership.

The PTY worker handles confirmed child-exit readiness before obsolete queued
input or resize work, after explicit host cancellation. One owner drains final
available output and publishes child exit, terminal close and render in that
order. Fatal transport errors reconcile an already arrived child notification;
an I/O error alone never fabricates a child status. Native adapters retain
process-tree and join ownership. Final available output can span multiple read
batches, with a separate 4 MiB ceiling, cancellation between batches and a
content-free limit warning; ordinary live-read and resize budgets are unchanged.

A normal update follows this sequence:

```text
PTY bytes
  -> bounded decoder and VT parser
  -> terminal-state mutation
  -> immutable renderer snapshot
  -> publish snapshot
  -> request a frame
```

Input follows the reverse authority direction:

```text
keyboard / pointer / IME
  -> focused application route
  -> modal and binding policy
  -> exact session input queue
  -> PTY
```

Modal surfaces consume their own input. Text from terminal output, search
results, links, previews, or installed components never becomes PTY input
without an explicit user action.

## Core and extension ownership

Explicit `amx search` and `amx docs` commands extend the existing application
CLI/default-handler owner. They share bounded query validation with `amx google`
and do no work on terminal hot paths. See
[ADR 0063](adr/0063-explicit-browser-search-routing.md).

`amx find` and `amx explain` activate installed local tools explicitly through
the app-owned bounded process adapter, not terminal hot paths or generic Quick
Action execution. WSL uses a scoped guest lease and isolated Python supervisor;
see [ADR 0064](adr/0064-bounded-explicit-local-tools.md).

`amx open` extends this explicit application boundary with directory-only
validation and the existing desktop adapter. WSL directory resolution is a fixed
isolated child of the leased helper; no startup, renderer or PTY path performs
filesystem discovery. See [ADR 0065](adr/0065-explicit-directory-handoff.md).

`amx edit` shares only the path mechanism with directory opening. Its regular-file
policy, strict user-root editor preference and fixed editor-URI authority remain
application-owned. It does not change terminal settings-editor behavior or enable
generic Quick Action execution. See [ADR 0066](adr/0066-explicit-editor-file-handoff.md).

Current capability-free shared contracts are owned by
`automexia-connectivity` and `automexia-command-productivity` where required
by existing source consumers. These package names are maintenance facts, not
public product announcements.

`automexia-extension-api::surface` owns bounded, versioned semantic table data.
The application-owned surface slot validates trusted grant binding, frame size,
generation, revision and expiry while retaining at most one snapshot. This
[contract](SEMANTIC-SURFACE-CONTRACT.md) has no automatic UI or provider transport
activation; terminal embedding surfaces keep their separate identity and owner.

Core terminal crates own behavior required by every installation: VT state,
PTY/process lifetime, input routing, panes and tabs, configuration, snapshots,
and platform integration.

Extensions are optional and versioned. They may contribute bounded data or
commands only through application-owned contracts. The application validates
capabilities, size, lifetime, route, generation, and publication before accepting
a contribution. Disabling or removing an extension must leave the terminal
usable and clean up its workers, queues, caches, and temporary state.

Network, credential, provider, filesystem, and process authority do not belong
on input, PTY, resize, startup, or renderer hot paths.

Plain operational status classification stays in the optional `automexia-devops`
extension. Its bounded, allocation-free recognizer distinguishes lifecycle from
readiness; the core grid emitter maps generic severity to active palette colours
and preserves explicit application ANSI. No provider access, worker or persisted
state participates in this row path. See
[ADR 0052](adr/0052-truthful-operational-status-colours.md).

Passive Kubernetes prompt projection uses the workspace-pinned serde-saphyr
reader in the same DevOps extension. Explicit parsing limits retain only context
and namespace; fixture and benchmark dependencies are development-only. Windows
guest filesystem reads use an application-owned, cancellable helper before GUI
startup, never provider execution in the renderer. See
[ADR 0055](adr/0055-session-local-prompt-discovery.md) and
[current scope and verification](PROMPT-CONTEXT-ASSURANCE.md).

Grapheme boundaries and label compaction belong to the capability-free
`automexia-extension-api::text` owner. Its public helpers distinguish trimmed
labels from whitespace-preserving display text and borrowed prefix slices.
Compaction counts the ellipsis within the grapheme limit; it is not a byte,
terminal-cell or pixel limit. Consumers retain original values and own their
input limits. Font measurement and visual fitting remain renderer responsibilities.
The text benchmark's Criterion dependency is development-only.
The suggestion presentation model uses the whitespace-preserving contract for
visual labels only. Its bounded full accessible name and insertion value remain
separate from shortened labels; generated ellipses never inherit match indices.
Candidate identity and acceptance authority remain with the application broker.
Connection Hub adapters retain their existing extra-ellipsis and wrapping
policies but obtain cluster boundaries from that same shared text owner. They
do not shorten the underlying connection or review model.

Sugarloaf's grid CPU path uses private cpu_raster primitives for opaque RGB
packing, rounded premultiplied source-over, grayscale masks and color glyphs.
The caller owns its destination buffer, atlas, row/cursor state and resource
lifetime. Mask text colors are straight RGBA; color-atlas bytes are already
premultiplied. Background normalization and fill policy remain in the grid.
Immediate-mode Text currently retains its own equivalent scalar blits; the
legacy rich-text CPU renderer has a distinct SWAR/SIMD rounding contract.
These are not shared GPU allocation, shader or synchronization authorities.

Sugarloaf's immediate-mode text owner retains at most 512 shaped runs and
2 MiB of text/glyph-vector payload, with capacity-aware accounting, FIFO
eviction and full text/font/style identity checks behind its hash index.
Oversized runs are usable without being cached; immutable shared runs avoid
cloning glyph arrays on a cache hit. These are retained-payload bounds, not
total process-memory or transient shaping-input limits. Shape, ascent and
private text-atlas keys use the actual rounded raster size, independently of
the grid atlas's quarter-pixel units. No GPU ownership or extension edge changes.
The existing Sugarloaf font-reload boundary also replaces the immediate Text
library, discards pending labels and invalidates font-derived caches and atlas
slots. It retains backend allocations and scale, and does not discover fonts
or initialize an unused backend. The CPU presentation cache is invalidated by
that same reload boundary, so unchanged draw-instance bytes cannot suppress
the repaint after a font change.

Responsive label fitting has a renderer-local implementation in
apps/automexia-terminal/src/renderer/text_fit.rs; the responsive surface owns
the active-font adapter. It measures whole candidates with the same DrawOpts
used to draw them, preserves whitespace and source grapheme boundaries, and
includes the marker in the measured budget. Work is limited to a 16 KiB source
view, 2,048 retained graphemes and 26 measurement probes. An incomplete boundary
at the view edge is not retained. Non-monotonic shaping can produce a conservative
fit; the contract is a confirmed finite advance within the available width,
not maximal filling, arbitrary ink-overhang clipping or additional bidi support.
This module adds no extension, persistence or filesystem authority and does not
change underlying editable or accessible values. Font resolution stays with Text.
Probe strings reuse one fallibly grown buffer capped at the source-view bytes
plus the three-byte marker. Confirmed offsets use the existing SmallVec owner
with 64 inline slots and bounded spill capacity. The final display is rebuilt
from the exact measured winner; unchanged labels continue to borrow their source.

Suggestion label drawing lives in the private renderer sibling
apps/automexia-terminal/src/renderer/suggestion_text.rs. Its thin overlay
adapters retain theme conversion, layout, hit targets and activation ownership.
Plain and matched labels use the same bounded fitter. Matched candidates are
measured as the exact styled runs that will be drawn; drawing reuses the
returned advances. Runs borrow source slices, and explicit marker provenance
keeps generated ASCII dots neutral even across Unicode Prepend boundaries.
Adjacent neutral source and marker text remain one shaped run. Literal source
dots retain their match ownership. These fitting guarantees do not certify
arbitrary glyph-ink clipping; full editable and accessible values remain intact.

The unpublished `tools/renderer-benchmarks` package measures this private module
by path inclusion, without a runtime library or application-binary build.
Only development dependencies and its explicit benchmark target are permitted;
no workspace package may depend on it. See
[ADR 0047](adr/0047-private-renderer-benchmark-boundary.md).

## Output readability

Keyboard hyperlink review extends the existing core hint owner. Bounded visible
cell/extras snapshots and cell-aware matching preserve OSC 8 destinations and
reject stale route/output/geometry before activation. Input stays modal and
default opening uses the existing platform adapter with validated targets and
content-free diagnostics. See [ADR 0061](adr/0061-keyboard-hyperlink-review.md).

Command-result row bands belong to the core renderer's private
`command_results/rows.rs` geometry owner. It splits proven visible output bounds
into inset fixed-grid bands; it never detects commands, parses tables, modifies
cells or inserts blank rows. The existing completion owner retains colour, pulse,
prompt gutter and route identity. The iterator allocates no storage, derives each
position from its index to avoid accumulated fractional drift, rejects nonfinite
geometry and caps work at 8,192 bands per surface. Existing CPU/GPU rectangle
submission consumes the same geometry. The private renderer benchmark includes
this module directly; no extension, persistence or dependency boundary changes.

The same geometry owner gives command boundaries a short leading accent rather
than a pane-spanning line: at most 48 logical pixels, at most one quarter of the
pane width, with a leading inset up to 12 pixels. Invalid or unpaintably small
rectangles are omitted. Structural pane dividers and their pointer targets stay
with the layout owner. The marker does not change completion identity or the
completion pulse. This is draw-only core feedback under
ADR 0035, not an optional extension or a terminal-grid mutation.

Command context and timestamps now share the application-owned wrapping layout
in `renderer/command_info.rs`. A verified blank prompt row can occupy several
display rows without adding VT rows. The pane's `RenderableContent` projection
maps grid paint, selection/search, pointer input, carets, images and scrolling.
Complete labels remain reachable in short windows; native protocol coordinates
and copied output remain unchanged. Unexpanded frames have an allocation-free
identity map. See [ADR 0059](adr/0059-wrapped-command-information.md) for limits,
ownership, compatibility and validation boundaries.

## VT control-string trust boundary

Terminal output is untrusted input. CSI, OSC, DCS, APC, image protocols,
hyperlinks, titles, clipboard requests, and private control sequences are parsed
with explicit byte, dimension, nesting, and state limits.

Malformed or unsupported sequences are ignored or reported safely. Control
strings cannot launch processes, evaluate a shell, read arbitrary files, or
bypass user confirmation. Terminal and parser fuzz corpora include fragmented,
oversized, Unicode, control-character, and historical failure cases.

## PTY and process lifecycle

PTY input channels retain a dedicated cancellation wakeup shared by their sender
clones. Closing a pane can therefore retire its worker even while a partial
write blocks subsequent resizes; the input queue is neither copied nor drained
by the cancellation path. The existing worker remains the sole process owner.

Processes are launched from a typed executable plus an exact argument array.
Structured actions do not use shell command concatenation or implicit Enter.

Clipboard delivery stays in the application context owner. It captures route
and terminal identity before the OS read, validates that owner again, and queues
one bounded paste frame without redirecting to later focus. Selection and scroll
changes apply only to the accepting context. See
[ADR 0042](adr/0042-route-bound-paste-transactions.md) for limits, pointer
ownership, rejection and the remaining native evidence requirements.

The session owner is responsible for:

- child identity and inherited context;
- ordered input and coalesced resize;
- bounded output delivery;
- cancellation and exit publication;
- descendant cleanup on close and shutdown; and
- joining owned readers, writers, and workers.

Unix process groups and Windows Job Objects or equivalent platform mechanisms
are used where applicable. Every ordinary and exact Windows ConPTY child is
created suspended, assigned to its session's kill-on-close Job Object, and only
then resumed. Window and application teardown first broadcast one idempotent
shutdown request to every active, background, split, pane-tab, and parked
session; only then may route destruction join workers. This keeps per-session
graceful deadlines concurrent instead of multiplying them by the number of
sessions. A failed optional feature must not interrupt the basic local shell
path. [ADR 0038](adr/0038-owned-pty-trees-and-broadcast-shutdown.md) owns this
lifecycle invariant.

## Renderer and snapshots

The private `renderer/ui_theme.rs` owner separates decorative card borders from
actionable focus outlines and owns shared palette, Hub and confirmation colours.
Theme text is contrast-corrected against the lightest shared chrome surface with
headroom for byte quantization; terminal colours remain configuration-owned.
Palette trailing labels use the existing bounded font-measured fitter and retain
full action values. This adds no dependencies, I/O, workers, animation or state
owner. Existing modal geometry, input routing and vector-icon owners are retained.

The renderer consumes immutable, generation-labelled snapshots. Expensive
layout, search, image, font, or accessibility work is bounded and cancellable.
A result is published only if its route and generation are still current.
Feature-gated native visual readiness is published only after the matching
frame has presented; a control consumed after draw-data construction forces a
new frame rather than pairing new state with old or partially rendered pixels.
The CPU renderer's frame-skip identity includes the physical surface extent,
and its reusable-frame cache advances only after successful native
presentation. Therefore an unchanged terminal model still repaints newly
exposed pixels after a client resize, while a failed present remains eligible
for an identical retry.

Frames preserve terminal-cell geometry, grapheme widths, clipping, z-order,
cursor position, selection, scroll offsets, and modal composition. Renderer
state never becomes a competing terminal-state authority.

## Windows, tabs, panes, and focus

Top-level windows own global tabs. Workspaces own panes, and panes may own local
tabs. Focus is a stable route rather than an inferred screen coordinate.
Geometric navigation, pointer focus, divider resize, search, and selection
operate on the selected route only.

Fresh splits use normal launch defaults. A clone copies only the documented
launch context and creates an independent session and PTY.

## Keyboard compatibility boundary

Automexia defaults remain authoritative. Compatibility profiles are explicit,
versioned, collision-tested, and layered before user overrides. Invalid
compilation preserves the previous complete binding registry, while native
shell control keys retain documented fallthrough.

Current source leaves bare Ctrl+R/Ctrl+D shell-owned in normal terminal input.
Both the typed registry and legacy fallback affect dispatch, and explicit user
bindings and modal ownership remain authoritative. Clone actions retain their
existing session owner and palette entry with directional Alt chords. The published
0.4.0 package predates this correction. See
[ADR 0041](adr/0041-shell-owned-history-and-eof-shortcuts.md).

The grouped palette retains one exhaustive action/category projection in its
private navigation module. Both keyboard and pointer activation use the same
application method; navigation rows cannot become terminal actions. No extension,
worker, persistence or dependency boundary is added. See
[ADR 0049](adr/0049-grouped-command-discovery-and-pane-shortcuts.md).

Pane removal sends shutdown and retires a lease without a worker join on the UI
thread. Router injects the single bounded worker registry into every launch.
One idle-blocking cleanup service retains all joins, with pre-launch capacity
reservation, bounded acknowledgements and a shared final shutdown budget.
Palette Back uses shared header draw/hit geometry; matching avoids per-candidate
query normalization and rendering filters only once. See
[ADR 0050](adr/0050-nonblocking-session-retirement.md).

Confirmed window closure dismisses native surfaces before route destruction or
service waits; explicit Quit retains one application shutdown owner. Native PTY
cleanup remains joined after dismissal. ConPTY's reader switches to bounded
discard-only draining once its consumer retires, so a full ring cannot block
native close. A live consumer receives its final buffered bytes before EOF.
Caller pipe copies are released after native child attachment; failed attachment
retains backend ownership and drain-before-close cleanup. Initialized process
attributes have one scoped owner. The VT worker parses its pending bytes before
native EOF even when a resize/frame holds the terminal lock; only that worker
waits, and child-exit verification remains independent of stream closure.

Current Windows/Linux/BSD pane defaults use Alt+R/D for clone and add Shift for
fresh; these deliberately replace shell Alt editing only in normal mode.
Every palette action reconciles its label with effective bindings during
construction/reload, not input/render. Back shares a dedicated left-arrow vector
in its fixed header and row. See
[ADR 0051](adr/0051-mnemonic-pane-shortcuts-and-honest-discovery.md).

## Configuration transaction

Core `rio-vt::config::colors` owns fixed-size ASCII RGB/RGBA conversion for both
configuration deserialization and palette defaults. Borrowed helpers and defaults
share one stack decoder; the existing owned API delegates to it. Valid conversion
needs no regex compilation, heap buffer or retained cache. Inputs are inspected
within a nine-byte token ceiling, and invalid Unicode cannot reach byte-indexed
decoding. App branding and persistence remain separate owners. See
[ADR 0058](adr/0058-bounded-colour-setup.md).

Configuration is read with bounded size and parsing depth. A candidate is
validated completely before publication. Invalid reloads keep the
last-known-good configuration and return a redacted error.

Runtime appearance preferences are application-owned and layered over
hand-edited configuration. Writes use private permissions, temporary files,
flush, atomic replacement where supported, and bounded recovery. The VT parser,
renderer, PTY layer, and extensions do not own preference storage.

The same overlay owns bounded, single-chord catalog shortcut edits. Palette
recording is capability-free; the application validates the complete candidate
against classic and typed owners before a binding-only publication. One existing
writer persists the overlay and publishes revision-tagged completion before waking
the event loop. Reset removes an override rather than modifying the base config.
See [ADR 0054](adr/0054-palette-shortcut-editor.md) for conflict, recovery and
modal-input boundaries. This adds no worker, dependency or OS hotkey registration.

Migration from compatible predecessor configuration is explicit,
non-destructive, reversible, and never overwrites the source.

## Shell integration

`amx google` delegates to the existing application's one-shot Google CLI, not
the VT parser or an extension. The CLI exits before terminal startup; native
Windows URL opening shares the existing screen handler. Shells retain quoting
and history ownership, helpers retain name collisions, and no persistent alias
or background service is added. See [ADR 0062](adr/0062-explicit-google-search-command.md).

Supported shell integration is session-local and keeps the shell's own editor,
history, completion, quoting, and pipeline semantics authoritative. Prompt
metadata is bounded, route-scoped, and treated as untrusted.

Enhanced listings decorate interactive output without changing objects or bytes
sent through pipelines. Missing integration resources degrade to an ordinary
shell rather than blocking startup.

## Image protocol and quick-look boundaries

Sixel, Kitty, and iTerm2 image data is decoded with byte, pixel, dimension,
frame, and cache limits. Images are owned by terminal state and released when
cleared, evicted, or closed.

Local raster preview requires an explicit path already visible to the user. The
preview validates identity and file type, avoids following links, bounds decoded
dimensions, and rejects replacement or stale results before publication. It has
no remote-fetch authority.

## OpenSSH inventory and interoperability

Automexia can host the system OpenSSH client like any ordinary terminal.
Credential prompts, agents, host-key policy, configuration semantics, and
network behavior remain owned by OpenSSH and the operating system.

The public inventory path reads only explicitly reviewed local configuration
files within fixed limits. It extracts bounded public metadata, never credential
material, and preserves a last-known-good view on invalid or replaced input.
Opening or searching inventory performs no connection or authentication work.

## Capabilities

Sensitive operations require explicit, narrow capabilities. A capability binds
the caller, operation, scope, route or resource, generation, lifetime, and
revocation state. Unknown, stale, expired, widened, or mismatched requests fail
closed.

No optional package receives ambient terminal content, history, clipboard,
credentials, environment, filesystem, network, process, or PTY access by
default.

## Interactive performance invariants

The following paths must stay bounded and nonblocking:

- input dispatch;
- PTY read, write, and resize;
- VT parsing and terminal-state mutation;
- snapshot publication and renderer wake;
- focus and route changes; and
- startup of the basic local terminal.

Filesystem scans, persistence, network work, authentication, dependency loading,
and optional-component work run outside these paths. Queues, caches, images,
search results, history, and worker counts have enforced ceilings.

## Accessibility boundary

Renderer-neutral UI state defines role, name, value, state, relationships,
focus order, focus restoration, live announcements, and keyboard commands.
Native platform adapters translate that model without inventing product state.

Visible changes require layout/semantic tests, exact controlled raster evidence,
and native assistive-technology review for every platform claimed by a release.
High contrast, reduced motion, long/localized text, Unicode, IME, and high scale
are first-class cases.

## Persistence

Connection Library and managed receipts use the private application module
`connections/persistence_support.rs` for bounded in-memory serialization. The
stores retain their document limits, pretty-JSON formats, validation, errors and
recovery policy. Capacity requests are fallible and capped; memory-writer flush
does not imply file sync or crash durability. SSH metadata retains an
extension-local writer instead of depending on the application.

Every persisted format has one owner, a version, strict limits, private-data
rules, corruption behavior, recovery behavior, and uninstall policy. Writes do
not contain terminal output, command history, credentials, or machine-local
identifiers unless a documented user action explicitly requires and reviews the
data.

## Build artifact lifecycle

Build, test, fuzz, package, and release outputs use repository-owned bounded
directories. Cleanup verifies exact targets and never sweeps user work. Packages
carry the expected license notices, checksums, provenance, and platform signing
requirements.

## Repository enforcement boundary

Architecture and policy checks guard dependency direction, capability changes,
unsafe code, persistence owners, documentation integrity, confidential-data
leaks, and release evidence. Tests and checkers must fail closed when required
owners, limits, or evidence are removed or weakened.

A source test, cross-compile, or mocked platform result cannot replace native
runtime evidence. Documentation must distinguish implemented source, shipped
behavior, and external release prerequisites.

## Current maintenance ownership

The existing Rust terminal/session, VT, renderer, platform and capability
boundaries remain authoritative. Fundamental input, PTY, reflow, focus and
rendering behavior belongs to core owners. Optional decoding and file
validation use the existing image owners, not the parser or paint hot path.

[Terminal maintenance status](TERMINAL-MAINTENANCE-REQUIREMENTS.md) and
[terminal interaction status](TERMINAL-INTERACTION-REQUIREMENTS.md) identify
existing source owners and known limitations. They are not implementation
plans or evidence that every native scenario has passed.

Dependencies, capabilities, persistence schemas, protocols and threading
boundaries remain subject to the existing ADR and architecture checks.

## Public architecture boundary

This page is not a complete inventory of private research or future products.
Detailed unreleased workflows, integrations, algorithms, schemas, commercial
packaging, and delivery plans must not be added to the public documentation
tree. See [Private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Command Productivity Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp32 Static Devops Pack Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp33 Native Import And Trusted Workspace Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M10 Google Cloud Adapter Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M11 Kubernetes And Openshift Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M12 Teleport Organization Identity Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M13 Provider Aware Quick Actions Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M3 M4 Reviewed Openssh Route And Trust Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M5 Reviewed Openssh Tunnel And Native Evidence Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M6 Typed Automation And Declarative Workspace Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M7 Provider Neutral Authentication Capsule Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M8 Aws Adapter Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M9 Azure Adapter Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## Command-productivity source boundaries

Quick Action scoring stays in the existing capability-free
`automexia-command-productivity::actions` model. It streams Unicode scalars rather
than materializing byte-offset tables; string-level lowercase, exact scores,
scope precedence and result limits remain unchanged. No retained cache, process,
thread, provider authority or new extension is added. Independent public-search
and maximum-input allocation oracles live in `tests/quick_action_activation.rs`
under that crate; the existing Quick Action benchmark checks scores and identities.

CP3.1 persists only reviewed opt-in aliases under the command-productivity
owner. CP3.2 adds static reviewed DevOps-pack candidates without activation or
provider authority. CP3.3 keeps native inventory and trusted workspace parsing
in capability-separated adapters; the pure compiler receives validated typed
records only. See [DEVOPS-ALIASES.md](DEVOPS-ALIASES.md).

## Session-launch source boundary

Focused table output follows [ADR 0060](adr/0060-focused-core-table-output.md):
the capability-free UI model recognizes bounded grid text, VT remains the only
terminal owner, and the application owns the route-scoped read-only snapshot,
input containment and drawing. It does not invoke provider extensions or mutate
normal scrollback. The [guide](user-guide/table-output.md) describes limits.

D0 follows ADR 0012: the terminal owns the local session and PTY while system
OpenSSH owns networking, authentication, credentials, host trust, and proxy
behavior. Optional discovery or review cannot become a second launch owner.
