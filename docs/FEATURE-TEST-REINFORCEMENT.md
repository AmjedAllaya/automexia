# Public feature-test reinforcement

This reinforcement plan covers only the free terminal capabilities in
[the public feature catalog](FEATURES.md). Advanced unreleased and commercial
feature assurance remains private.

The machine-enforced feature matrix may contain internal source owners, but
public documentation must not turn those owners into product announcements.

## Required assurance pattern

For every public behavior:

1. reproduce the real user path;
2. add the smallest deterministic regression test;
3. cover boundary, malformed, stale, cancelled, repeated, and concurrent cases;
4. assert intended effects and forbidden side effects;
5. use an independent oracle;
6. exercise cleanup, disable, recovery, rollback, and restart where applicable;
7. measure the real owning path when performance or resources can regress; and
8. retain native, visual, accessibility, signing, hardware, and long-duration
   work as explicit external gates until it actually runs.

## Public feature owners

### identity-config-migration

Environment tests reject a whole invalid batch before startup mutation or launch,
including every platform override. Preserve empty values, Unicode, additional
equals signs and duplicate ordering. Creation tests require exact complete bytes,
concurrent no-overwrite publication, relative targets, missing parents, existing
sentinels, linked/directory destinations, failure cleanup and isolated fixture
ownership. Native links/private modes and macOS startup errors require their
actual platforms; a privilege-skipped branch is not passing native evidence.

Welcome creation must use one bounded application worker, with a checked operation
number, weak route identity and cancellation. Cover close before and after result
publication, stale replacement, siblings, exhausted IDs, held Enter and release
after completion, handler failure and closed wake targets. Require result-before-
wake and completion drain even after target closure. Native busy/error rendering,
focus and assistive technology remain distinct from model and source-policy tests.


Colour setup assurance exercises the public helpers, literal defaults and serde
palette path. Independent channel and ASCII-grammar oracles cover RGB/RGBA,
all channel values, malformed Unicode, controls, markers and oversized input.
The six-scalar/eight-byte Unicode digit fixture must return an error, never panic.
First-use and repeated default/helper construction must allocate zero heap
buffers; a test-only System allocator counter excludes parallel test activity.
Correctness-checked palette benchmarks retain same-host before/after evidence;
development-profile setup timing is not native window startup or GPU latency.
Keep [the test commands](TESTING.md#bounded-colour-setup) and
[ADR 0058](adr/0058-bounded-colour-setup.md) with this owner.

Shortcut editor assurance includes double-click/F2 capture, mode and profile
conflicts, stale queued edits, no terminal input from IME/paste/drop, Reset retry,
revision-tagged durable failure, and separate-process restart. Binding publication
must not resize panes or reload fonts. Native pointer, pixels, keyboard layouts
and screen-reader delivery remain external; see [the test protocol](TESTING.md#shortcut-editor-assurance).

Reinforce first-run identity, non-overwrite, strict bounded parsing,
transactional reload, private permissions, atomic replacement,
last-known-good recovery, migration preview/apply/rollback, and side-by-side
coexistence. Assert that real profile paths and environment values never enter
fixtures or reports.

### terminal-protocols-grid-history

Inline header tables preserve authoritative VT scalar widths, source bytes and
ANSI styles while wrapping complete graphemes within aligned cells. Exercise
actual parser/reflow capture, partial soft-wrap scroll round trips, empty-cell
hit mapping, mixed-script fallback, mouse-mode invalidation and source copying.
Shared borders and clipped glyph quads need independent raster bounds at
fractional scales; selected substrings must paint only their source cells.
The bounded model and snapshot benchmarks do not establish native compositor,
physical input or assistive-technology delivery. See
[the protocol](TESTING.md#inline-header-tables).

Keyboard hyperlink review must retain exact OSC 8 targets over history, soft
wraps, CJK and combining cells, with one label per contiguous anchor. Enforce
capture/regex bounds and byte-to-cell mapping. Replaced cells or extras must
invalidate activation; use parser-created snapshots rather than injected matches.

Command-information wrapping adds display rows only. The shared projection tests
must preserve exact native cells, ordering, cursor, history and copy semantics;
the live WSL resize ladder remains required alongside the CPU raster oracle.

Real WSL multi-column listings must preserve literal and erased column gaps
through consecutive mixed-axis changes, including one-cell panes, long mixed
file/directory names and large restoration. Require eight fixed seeds as well
as targeted wider/shorter, native blank-fill and historical short-fragment cases
across history/live seams. Exercise an exact-margin native cursor cell, blank
soft-wrap fragments, pre-resize selections and repeated tiny-pane roundtrips.
Compare copied text independently of pixels: a correct grid can still serialize
an all-blank soft fragment as an incorrect hard break.

Native resize coverage must exercise both coalesced bursts and independently
published intermediate worker sizes, observing the real grid without injecting
its state. Keep strict output and prompt-adjacency assertions after fixture
acknowledgments. Failure-only diagnostics record the step, dimensions and a
bounded later-state observation without turning the original failure into a
pass. Later isolated or concurrent passes do not clear an unexplained failure.

A parser-created viewport identity journal must retain the first visible cell
in the first projected row after every resize, as far as history permits. Use
independent input colors for repeated text and inspect renderer-facing snapshot
styles as well as live-follow sibling cells. Cover one-column output, height-only
changes, blank hard lines, selection, inactive normal buffers, alternate screens,
and actual anchor eviction/absorption. Reject PTY writes. Saved same-host
scrolled/unselected/selected benchmarks measure the combined core path; preserve
separate native redraw, pixel and accessibility gates.

Reinforce fragmented and malformed control sequences, Unicode widths,
combining/bidi/control input, alternate-screen transitions, scrollback, reflow,
search, selection, cursor state, and exact visible cells. Fuzz every structured
terminal-input boundary with historical failures retained.

Dependency updates must also preserve every generated emoji variation lookup.
Enumerate all 708 source-table entries independently of PHF's hash lookup, compare
the public presentation result, and reject invalid and suffixed keys. Run with
default features enabled and disabled. A compiler pass did not detect the PHF
0.14/table mismatch; preserve the matching runtime unless regeneration and these
complete lookup checks establish compatibility. Linux windowing dependencies
must retain one compatible Smithay/calloop/decoration generation, with native
window and renderer evidence kept separate from host compilation.

The parser-driven retained-output journal preserves explicit hard breaks,
intentional empty lines, spaces across soft wraps, fragmented Unicode and ANSI
colors across intermediate width/height changes in both directions. Its oracle
is fault-tested against text, hard-break, whitespace and style mutations.
Substring counts or restored-width-only checks cannot replace this assertion.
This does not establish selection retention, native shell repaint, renderer
geometry or pixels; those remain distinct evidence owners.

Selection reflow uses a separate parser-created selection journal: select once,
resize both directions, and assert exact copy bytes and anchor sides at every
intermediate geometry. Include wide/combining characters, semantic and line
selections, alternate-screen cropping, invalid points, zero-history eviction and
rectangular-selection invalidation. Compare all retained cells with an independently
resized unselected terminal. Measure combined reflow/copy/visible-snapshot work
over deep bounded history; do not call a parser-only pass native pixel evidence.
Check the next Vi motion as well as its cursor/anchor coordinates, preserving
unrelated cursor ownership. The Windows PowerShell/ConPTY fixture uses bounded
UTF-8 output and requires both parsed completion and successful child exit before
retained-output reflow; live redraw and native selection gestures remain separate.

For command completion metadata, include multiple adjacent prompt lifecycles
whose source result and preceding boundary share a physical prompt row. Resize
between narrow and wide grids, navigate in both directions, and independently
compare stable IDs, timestamps, boundaries, visible rows, and unchanged PTY
bytes after each transition.

### pty-scheduler-process-lifecycle

Channel readiness must survive first registration racing each send API and final
sender destruction, without a later message or disconnect masking the result.
Gate the actual native poller with bounded per-receiver checkpoints. Run both Loom
models with the Loom configuration enabled; retain canaries for old snapshot order
and either missing fence. Source publication order alone is insufficient. Queue
admission, accounting and channel-transition performance require separate evidence.
See [ADR 0074](adr/0074-channel-registration-publication.md).

Unix native boundaries require status/no-record/retry/invalid-field lookup
oracles, concurrent caller-owned terminal names, failed-launch descriptor recovery,
argument/environment/cwd and executable-script compatibility. Child probes must
check controlling-terminal/session/foreground group identity and closed standard
streams. Retain exact-launch early-error cleanup and distinguish model-only macOS
argv checks from native host evidence. See
[ADR 0069](adr/0069-transactional-unix-pty-launch.md).

Windows pipe storage must retain a fixed capacity and safe cross-thread
ownership without manual Send/Sync or overlapping whole-buffer borrows. Cover
zero capacity, saturation, contiguous wraparound, empty operations and unchanged
destination suffixes against an independent FIFO. Gate cross-thread publication
with bounded acknowledgments and join both endpoints. Native pipe/ConPTY tests
must retain exact final bytes, EOF and cleanup; compare the existing PTY startup
and sustained-output benchmark with a same-host baseline. See
[ADR 0068](adr/0068-safe-windows-pipe-buffer.md).
Windows launch tests also require native ordinal environment-name ordering,
last-override values, empty double terminators and rejection before child lookup.
Use independent exact/over-limit UTF-16, quoting-expansion, entry-count and signed
geometry oracles. A public-constructor allocation observer with its own canary
must reject oversized explicit batches before wide copies. Native inherited
snapshots and total process memory remain distinct bounds. See
[ADR 0071](adr/0071-validated-windows-launch-boundaries.md).


Multi-column WSL listing coverage must execute both raw native and real worker
resize delivery, with explicit installed-tool prerequisites and no silent
substitution. Check no-resize probe invariance, varied filenames, erased gaps,
history seams, exact cursor margins, two-row panes, every original column and
filename count, final exit and temporary-directory cleanup. Keep the native
desktop and other host/architecture gates explicit.

Queue native probes before the input consumer blocks, then compare every
consumed key value and ordinal in a bounded cumulative acknowledgment. Keep
per-step acknowledgment, cursor/output invariance, real CMD line editing and
successful child exit separate. A title-only channel can coalesce intermediate
updates; neither the last counter alone nor arbitrary sleeps prove no lost input.

Real ConsoleHost editor tests must silently shrink/grow before typing, compare
native and VT cursor rows, preserve isolated history, and finish repeated child
and worker cleanup. Keep the native protocol cursor unchanged by prompt-metadata
repair at every fragment boundary. Use fake-clock input-settle deadlines and
buffered-input ordering tests to reject busy polls, lost expiry wakeups, resets
from failed/duplicate resizes and input overtaking. Preserve ordinary-input and
Unix zero-delay behavior; native frames and assistive-technology checks remain
separate external evidence.

Native broken-pipe error-then-drop tests must preserve single join ownership for
both directions. Repeat editor lifecycles in a shared test process as well as
isolated native cases; retain and diagnose the first abort before claiming a pass.

Caller-handle recovery must use an isolated native process-count oracle after
fully joined create/resize/exit/drop cycles, plus failed child attachment. Require
bounded stable infrastructure readiness and exact recovery on every measured
cycle. Final-output lock contention must traverse the actual worker's reader,
parser and grid: hold the terminal lock until final bytes and EOF arrive, then
compare literal text. Include zero reads, would-block, native EOF and fatal errors.
Native pipe tail tests alone do not prove the worker retained its pending bytes.

Confirmed child-exit precedence must traverse the actual worker and poll owner.
Queue input and child readiness together; fail writes or the final read, then
require exact final bytes and one exit-before-close-before-render sequence.
Cover exit arriving during a failed write, absent status and explicit host
cancellation. Repeat the real live PowerShell resize and queued-input exit cases
with the workspace feature set and no retries. Distinguish native process exit,
notification latency and actual worker join; diagnostic probes must forward
real I/O and expose only counts, error kinds and signaled state, never content.
Retain an intermittent failure even if later repetitions pass; they cannot prove
its original trigger. Final-tail coverage must span multiple ordinary read
batches, compare exact 4 MiB budget edges and cancellation, and retain every
one of 1,024 real native output rows plus the final marker. A short final marker
alone cannot prove the entire stream survived. Native desktop appearance and
other platforms remain separate evidence gates.

Pane-close regressions must distinguish immediate UI retirement from eventual
native process termination. Gate real worker-body and native thread-local
destruction independently: a finished hint is not an actual join acknowledgement.
Cover 256-slot reservation saturation, failed launches, panic, queued cleanup,
capacity recovery, repeated close/open, bounded stale acknowledgements and one
shared final deadline. Verify exact pre-close OS process handles, not only the
worker count. No application input/close path may join a foreign worker.

Confirmed-close coverage must distinguish native surface dismissal from process
cleanup. Test single-window close, explicit Quit and the final callback, with
cancelled confirmation and sibling windows preserved. Require dismissal before
service/PTY waits; a queued native destructor is not immediate disappearance.
The controlled Windows harness has a separate 500 ms dismissal ceiling.
Saturate the actual native output ring, retire its consumer, require completion
acknowledgement, and verify exact final bytes before EOF in live consumers.
Repeat real ConPTY shutdown and fallback drop with captured process handles.
Keep existing native grace/force budgets and actual join/TLS evidence intact.

Live resize evidence is owned by `rio-vt/tests/live_resize.rs`; exact parser,
selection, search, snapshot and native-seam regressions are owned by
`rio-vt/tests/resize_repaint.rs`. Keep the native child alive through all resize
transitions and require bounded acknowledgments plus successful exit. Cover
fragmented full/trailing EL without historical-output prompt ownership, native
history/live seams, unused bottom rows, Unicode and intentional blank lines.
Long-table native fixtures cover 32 padded rows crossing the viewport/history
seam, both worker scheduling modes, duplicate rejection and exact order. Verify
silent probe cursor/row invariance without resizing; keep Enter-during-resize
and final successful child release separate from that probe contract. Replay
one/two-cell transitions and consecutive wider/shorter changes, not only paired
shrink/restore. Former padded seams that move deeper into history must keep
their continuation; native spaces below the cursor must not archive live text.
Compare exact live/history pixels and correctness-checked extreme replay timing.
Replay the real fictional repaint at every byte split; retain Unicode, colours, blanks,
cursor distance and Unix explicit-space controls. The default resize-stress
command must run library, replay, live-shell and native-editor binaries. Exact
dispatch/failure tests and checker mutations prevent GUI-only native coverage.
The `grid_resize_snapshot_*` benchmark retains one bounded terminal across
cycles; `grid_table_roundtrip_checked_*` verifies exact copied input and bounded
history on every iteration outside its grid/snapshot timing. Neither is a native
PTY latency benchmark. Native desktop/GPU, accessibility and unexecuted Linux/macOS environments
remain external evidence, not inferred passes.

Reinforce exact executable/argument launch, ordered input, resize/output storms,
interrupt, EOF, exit, cancellation, close, and shutdown on each native adapter.
Assert no shell evaluation, implicit Enter, stale publication, orphan process,
leaked handle, worker, or temporary file. Treat Linux master-side `EIO` after a
one-shot slave closes as a platform EOF signal without weakening exact output,
completion-marker, child-exit, or cleanup oracles; do not treat temporary
zero-byte ConPTY reads as permanent closure.

Exercise ordinary and exact Windows ConPTY sessions under kill-on-close Job
ownership plus the Unix process-group owner. Broadcast one idempotent shutdown
request to active, background, split, pane-tab, parked, and top-level-window
sessions before sequential joins. Test one and many sessions, repeated requests,
window close, explicit quit, and the final event-loop callback. Capture exact
temporary-fixture process identities before close so ConPTY reparenting cannot
escape a parent-only oracle; require every identity and owner to exit inside a
declared many-session wall-clock ceiling.

Exercise the parked split/local-tab exit journal through the real context and
Taffy owners: all exit orders, selected and inactive routes, unknown/late exits,
independent windows, undo/redo and repeated cleanup. Compare real shutdown
channels and weak terminal references: surviving channels remain empty and
connected, while only the removed owner is dropped. Require live window size,
scale, margins and font metrics on restore even when padding did not change;
restore refresh precedes visibility. Saved zoom/unzoom styles must also use the
new extent and DPI. Native sibling output, process identities,
resource ceilings and pixels remain independent of these model-path checks.

### renderer-fonts-responsive-ui

Inline header tables preserve authoritative VT scalar widths, source bytes and
ANSI styles while wrapping complete graphemes within aligned cells. Exercise
actual parser/reflow capture, partial soft-wrap scroll round trips, empty-cell
hit mapping, mixed-script fallback, mouse-mode invalidation and source copying.
Shared borders and clipped glyph quads need independent raster bounds at
fractional scales; selected substrings must paint only their source cells.
The bounded model and snapshot benchmarks do not establish native compositor,
physical input or assistive-technology delivery. See
[the protocol](TESTING.md#inline-header-tables).

Keyboard hyperlink preview uses real parser targets and the production font
fitter. Compare exact restored pixels, mutate one channel, and cover tiny through
8K geometry plus 100Ã¢â‚¬â€œ400% scale. Full destinations stay separate from shortened
labels; native desktop and screen-reader delivery remain external evidence.

Shared command-information wrapping must preserve every supplied context label
and full completion label through narrow/wide restoration. Assert parser-created
blank-row ownership, independent display origins, actual grid/text pixels,
queued scrolling, short windows without native history, fractional-scale ink,
and non-intersection with prompt/output. Identity layout must avoid prefix
allocation. Keep native compositor and assistive-technology validation external
until executed; see [the protocol](TESTING.md#wrapped-command-information).

The table resize raster includes an independent literal seam-gap viewport at
three scales. Require matching dimensions and zero mismatches, and inspect fresh
rasters. CPU raster evidence does not certify a native desktop compositor,
minimize/restore gesture or GPU driver.

Shortcut editor assurance includes double-click/F2 capture, mode and profile
conflicts, stale queued edits, no terminal input from IME/paste/drop, Reset retry,
revision-tagged durable failure, and separate-process restart. Binding publication
must not resize panes or reload fonts. Native pointer, pixels, keyboard layouts
and screen-reader delivery remain external; see [the test protocol](TESTING.md#shortcut-editor-assurance).

Font upgrades must retain one compatible Skrifa/read-fonts generation across
Swash, Sugarloaf and the glyph protocol. Run the unchanged all-feature dependency
policy plus real glyph, fallback, fitting and raster tests; compiling two parser
generations successfully does not satisfy the reviewed dependency contract.

Command-versus-pane distinction requires short inset command markers, literal
48-pixel/quarter-pane caps, invalid and tiny geometry, exact marker pixel coverage
and preserved structural hit targets. Traverse adjacent parsed output and silent
commands, reflow, scroll and previous/next navigation; preserve timestamp identity,
copy bytes and cursor. Keep marker geometry benchmarks and native inset checks;
mutation tests must reject removal of the non-colour distinction requirement.
Native frame and screen-reader evidence stays separate from controlled specimens.

Shared chrome requires independent quantized contrast on all surfaces, separate
decorative and focus roles, literal RGBA token evidence, and real-font trailing
label fitting at fractional scales. Retain complete action values, hit geometry,
existing icon meaning and configured terminal colours/fonts. Reject one-channel
token drift and loss of all-surface contrast requirements in checker mutations.
Inspect controlled specimens without treating them as native app captures; exact
CPU/GPU frames and assistive-technology delivery remain external when unavailable.

Command-result row bands require parser-created Kubernetes, container and generic
table fixtures, Unicode/wrapped rows, retained selection/copy and scroll/resize
projection. Assert independent literal row coordinates and exact pixel-centre
coverage, including every gap and surface edge. Cover single/clipped rows,
fractional metrics, invalid/nonfinite geometry, the 8,192-band ceiling, all result
tones and reduced motion. Benchmark the actual allocation-free iterator in the
private renderer harness. Keep terminal bytes/cell positions unchanged; do not
infer table schemas or insert blank rows. A fictional controlled preview and
geometry raster do not replace native CPU/GPU frames, theme/scale review or
assistive-technology delivery. Missing native evidence stays external.

Focused core table output is separate from these inline row bands. Reinforce
its parser-created real tab stops, retained selection/cursor/copy, conservative
recognition, byte/row/cell/column ceilings, Unicode clipping, modal input and
key-release ownership, fractional scrolling, extreme viewport geometry and
route cleanup. Compare literal columns and row edges, parser-to-pixel exact
round trips and a one-pixel separator mutation; benchmark bounded capture and
resize/scroll with unchanged source assertions. Checker mutations must reject
loss of activation, local input containment, visual evidence or benchmark guards.
Native GPU, desktop focus, physical trackpad, theme/scale and screen-reader
delivery remain external until actual artifacts establish those properties.

Keep literal responsive whitespace, marker-budget, unchanged-storage and
Unicode cuts independent of the renderer-local helper. Reproduce the old scalar
and rounded-font-width failures. Verify finite whole-candidate advances with the
exact drawing options, bounded partial-context handling, both edges, source
retention, generated-marker identity and conservative non-monotonic fitting.
Pin resource ceilings independently; exercise oversized single clusters and
regional-indicator context. Preserve exact characterized probe order and retained
source identity. Assert bounded amortized capacity, borrowed reused probe storage,
inline/spilled offset limits and no stale bytes after overshoot. Compare
prepared-font fresh-owner CPU pixels across styles, fractional sizes and scales;
keep ink clipping and native UI claims
separate. The helper benchmark must include its production
owner directly and remain registered as renderer performance evidence; a
same-count replacement with a non-benchmark reference must fail validation.
The isolated development harness additionally requires Cargo-metadata mutations
rejecting publication, missing/extra/conditional targets, non-development or
unreviewed dependencies, inverse renamed edges and a duplicate application
harness. Confirm the selected build graph does not compile the application,
without changing release optimization or weakening the complete workspace gate.

Suggestion fitting must preserve literal cuts and whitespace, measure the exact
styled runs that are drawn and retain borrowed source storage. Reproduce
generated-dot highlight leakage and nonfinite-budget draws; preserve existing
pixels on rejection. Cover Prepend/control boundaries, literal source dots and
real-font source ceilings. Compare independent literal normal/highlighted CPU
draws across sizes/scales, preserve full values, overlay geometry and broker
no-execution tests, and benchmark the actual label/description drawing owner.
Keep ink clipping and native accessibility as separate evidence requirements.

CPU primitive characterizations require independent literal channel/alpha
pixels, repeated source-over, mask/color atlas samples, all edges/corners,
empty/offscreen dimensions, atlas boundaries and destination suffixes.
Exercise actual atlas insertion, row writes, cursor draw order/colors, clearing
and repeated warmed storage retention. Benchmark the real cached-grid paint
and row-update/paint owners separately from immediate text; do not infer global
allocator, native window or GPU results from CPU buffer tests.

Immediate text-cache regressions compare cached measurement and exact CPU
pixels with fresh bundled-font owners after adjacent rounded-size and scale
transitions. Require nonempty finalized frames, forced digest collisions,
entry and retained-byte ceilings, spare-capacity accounting, oversized-run
bypass, FIFO eviction and released immutable runs. Register the actual Text
measurement/draw benchmark with the renderer performance owner; its mutation
must reject a missing benchmark even if the reference count stays unchanged.
Font replacement must alternate visibly distinct prepared fonts and variable
weights, compare fresh-owner CPU pixels and widths after each transition,
release old shape/font identities and queued base/modal labels, retain atlas
allocations and scale, and leave unused backends uninitialized. Review the
application reload call and CPU frame-skip invalidation together. Benchmark
replacement plus raster separately from full-owner reconstruction. Retained
payload bounds are not total allocator/RSS evidence; interactive font reload,
native GPU/window and assistive-technology validation remain separate contracts.

Reinforce immutable generation-labelled snapshots, font fallback, cell geometry,
clipping, z-order, cursor, selection, themes, high contrast, reduced motion, and
small through high-resolution layouts. Require renderer-neutral state, exact
controlled rasters, and native frames.

Project row-anchored metadata through one frame-local ownership pass. Exercise
input permutations and malformed duplicates, then prove each result identity is
painted once and every measured label rectangle is pairwise disjoint after
reflow. Expose co-located prompt-context rectangles and reject cross-owner
intersection under one shared reservation. Freeze command duration as well as
wall time before exact raster comparison. A latest-result-only hook is not a
multi-result or cross-overlay paint oracle.

Publish a feature-gated native checkpoint only after the matching frame has
presented successfully. Controls consumed after overlay construction must force
a subsequent frame; skipped or dropped frames publish nothing. Retained native
captures establish foreground ownership, reject detectable dialog occlusion,
and require two consecutive identical full-frame pixel digests before exact
backend comparison and independent frame inspection. Two identical occluded or
stale captures are not visual proof.

Frame-skip identities must include the physical framebuffer extent, and the
cache may record only a successfully presented frame. Exercise unchanged
content across initial sizing and later resize, acquisition or presentation
failure followed by identical retry, a completely opaque on-screen client, and
zero-tolerance full-client comparison across available renderer backends. Hold
live editor input at a fixed unsent sentinel so cursor-line drift cannot
masquerade as a renderer difference.

### windows-tabs-sessions-input

Inline header tables preserve authoritative VT scalar widths, source bytes and
ANSI styles while wrapping complete graphemes within aligned cells. Exercise
actual parser/reflow capture, partial soft-wrap scroll round trips, empty-cell
hit mapping, mixed-script fallback, mouse-mode invalidation and source copying.
Shared borders and clipped glyph quads need independent raster bounds at
fractional scales; selected substrings must paint only their source cells.
The bounded model and snapshot benchmarks do not establish native compositor,
physical input or assistive-technology delivery. See
[the protocol](TESTING.md#inline-header-tables).

Exercise the actual classic hyperlink binding on every platform table and the
shared key-intent owner: Tab/Back, labels, Enter, copy, Escape, repeats and
releases. Gate route/viewport changes before effects, consume IME and pointer
events, preserve terminal selection, and retain consumed releases after close.
The explicit correctness-checked hyperlink benchmark measures bounded capture
and navigation only; native OS handler/clipboard latency needs native evidence.

Wrapped headers share one inverse coordinate map with pointer selection, paging,
scrollbars and IME. Accepted paste follows only its captured pane; rejected paste
and sibling panes retain their view. Short panes must reach every context line
without inventing native scrollback. Native desktop interaction remains external.

Shortcut editor assurance includes double-click/F2 capture, mode and profile
conflicts, stale queued edits, no terminal input from IME/paste/drop, Reset retry,
revision-tagged durable failure, and separate-process restart. Binding publication
must not resize panes or reload fonts. Native pointer, pixels, keyboard layouts
and screen-reader delivery remain external; see [the test protocol](TESTING.md#shortcut-editor-assurance).

Keep header Back available independently of result scrolling and active search.
Keep event normalization outside the shortcut candidate loop and action cloning
inside the matched branch. Source-order mutation guards complement, not replace,
the configured platform tables, overrides, physical keys and mode tests.
Test category and font/extension parent restoration at narrow and HiDPI sizes,
input/Back/ESC non-intersection, no executable action from Back and unchanged
Unicode scores against an independent pre-optimization oracle. Benchmark the
real model navigation sequence separately from native frame/input latency.

Grouped palette regressions require exhaustive category coverage, bidirectional
Back restoration, global command/category search, empty results, held Enter,
query limits, Shift+Tab, paging, responsive pointer hit tests and wheel reset.
Navigation rows expose no executable action. Keyboard and pointer dispatch must
share the application activation owner. Check fresh and clone chords independently
on every platform, retired shifted punctuation, disabled splits, mode suppression,
typed tombstones, explicit overrides and strict-profile isolation. Verify the
approved Alt+R/D clone and Shift-for-fresh matrix against actual configured
defaults as well as isolated platform tables. Cover all effective shortcut labels:
missing defaults, hardware Copy/Paste fallback, search-only keys, wrong modes,
typed chains, reload storage bounds and distinct ClearHistory/ClearScreen actions.
Require complete classic palette defaults, source-free shortcut chips, literal CPU
glyph pixels and palette-only Enter fallback without recreating removed keys.
Assert Escape cancellation and exact new destructive-action mode guards, including
modifier supersets, user overrides and strict-profile isolation.
Require a left-arrow Back in both header and row, pinned stroke geometry and
unchanged hit targets; source and coordinate guards do not certify pixels. Native frames,
physical layouts and accessibility events remain separate gates; a privacy-safe
semantic summary is not proof of native screen-reader delivery.

Shell-control regressions must exercise complete Windows, macOS and Unix
default tables with splits enabled and disabled, plus typed Automexia fallback,
pinned profiles, explicit user overrides and reset. Assert exact control bytes
and no clone action in normal and alternate-screen input. Test palette reload
against actual configured clone mappings, typed tombstones and overlapping
sequences; unbound actions remain discoverable. Never migrate a user mapping by
guessing its provenance. Native shell/PTY and physical keyboard/IME evidence are
separate requirements, not inferred from a binding-table pass.

Clipboard regressions must traverse the real terminal mode parser and capture
one complete queue transaction, including bracket delimiters and exact Unicode
bytes. Assert sibling silence after a focus change between target capture and
delivery, and reject removed, replaced, closing and disconnected destinations.
Cover empty/byte-limit text, ESC/ETX injection, raw input and newline semantics;
rejection must preserve selection and scroll. Pointer tests must distinguish
terminal cells from pane chrome and preserve copy-before-paste selection and
per-button release ownership. Native clipboard/IME evidence remains separate.

Reinforce independent windows, global and pane-local tabs, fresh/cloned splits,
route and focus isolation, geometric navigation, pointer routing, divider
resize, clipboard, IME, search scope, overlays, and focus restoration.

For previous/next command shortcuts, perform an actual native resize first,
assert the freshly reflowed frame, and inspect every intermediate and restored
frame for result-ID uniqueness, non-intersecting badge geometry, stable pane and
prompt ownership, and absence of terminal input.

Require the state checkpoint to follow the corresponding successful present,
then stabilize the retained native image with two exact full-frame digests. On
multi-pane exit, require broadcast-first PTY shutdown and exact owned-process
cleanup within the native wall-clock ceiling.

### ghostty-compatibility-g0-g6

Reinforce documented Ghostty-style keyboard compatibility with exact binding
resolution, left/right modifier handling, pane isolation, focus restoration,
clipboard behavior, IME safety, and absence of unintended PTY input. Treat
unsupported Ghostty behavior as an explicit compatibility limit.

### prompt-context-devops-semantics

Joint context/completion packing measures real fonts and preserves supplied label
bytes across wrapping, including timestamps and combining graphemes. Context and
completion rectangles must not intersect; disabling context restores the same
native grid without changing command-result ownership.

Reinforce bounded route-scoped prompt metadata, directory and status updates,
Git state, long and hostile labels, stale generation rejection, and redaction.
Context rendering must never change commands or trigger network work.

The shared context wire boundary is covered by the extension-contract-runtime
section below; count admission must reject before decoding an excess payload,
without changing current prompt wrapping or native grid behavior.

Kubernetes prompt discovery must reproduce real local `kubectl config set-context`
writes, same-directory replacement/clear, native first-source-wins semantics,
reordered/quoted/inline YAML and JSON, default/missing namespace, invalid input,
credential opacity and long-context namespace visibility. Guest filesystem reads
must remain isolated from the input path, with deadline/cancellation/output limits,
exact helper identity, child reaping and no Windows-host cluster substitution.
Use `automexia-devops/tests/kubernetes_prompt.rs`, the native WSL helper rehearsal,
and its correctness-checked benchmarks. `check_prompt_discovery.py` and its
mutation tests guard source boundaries but are not runtime evidence. Exercise
real Bash/Zsh/Fish/PowerShell hooks with same-directory changes, clearing, export
attributes, Unicode/metacharacters, byte limits, opt-out and encoder-free replay.
Use `tools/ci/test_shell_location_hints.py` for tests and validated replay timings.
Measure startup separately from cached replay and immediate identity projection.
Title-only storms must retain pending work and cached context; every real source
change must still invalidate it. Initial progress must precede guest reads, retain
the operation latch, and never overwrite an existing namespace during refresh.
Check cancelled/stale progress, initial and final renderer consumption, and cached
cross-pane reuse without treating an incomplete snapshot as a completed result.
For Zsh, test the actual encoder's hash table in the calling fixture process;
checking only the parent misses enumeration inside frame-capture subshells.
Assert unrelated PATH commands are not preloaded and user hash options remain.
Fragment path-pair commit frames through the VT; reject generic serialization,
debug disclosure, network/device path authority and stale guest hints in CMD.
Check the enlarged bundled glyph's ink/label bounds at multiple row heights and
scales, and the namespace-only label with full accessible context. Native desktop pixels and screen-reader
delivery remain distinct evidence requirements.

Operational status colour assurance must distinguish lifecycle from readiness:
completed pods/clean exits, full/partial/invalid readiness, init failures,
condition polarity, paused/starting/unhealthy containers, unknown table states,
misleading names, kind-prefixed pods, zero-count log prefixes, declared log levels,
negated success and mixed failure counts.
Require literal status oracles, parser-to-grid colour delivery, exact controlled
CPU pixels, configured/explicit ANSI preservation, unchanged source/copy and
disabled-extension behavior. Exercise bounded hostile rows and benchmark the
allocation-free classifier without conflating helper timings with native frame
latency. Native GPU and screen-reader evidence remains a separate gate.

### openssh-inventory-persistence

SSH's intentionally extension-local bounded JSON writer needs literal format,
validation-order, UTF-8, empty/exact/over-limit, rejected-write integrity and
large-then-small reservation checks. Keep its 8 MiB byte/capacity contract
separate from native permissions, recovery and allocator-RSS evidence.

Reinforce explicit file selection, bounded includes and record counts,
duplicates, malformed data, links, replacement, revocation, public metadata,
last-known-good recovery, and absence of secrets, passive scanning, login, or
network authority.

### extension-contract-runtime

SSH planning remains non-activated. Its canonical dependency/source mutations,
model/property scope and actual-generator controlling-PTY tests preserve native
review ownership, permission masks, prompt hooks, redirection and scoped remote
metadata. The existing application Criterion group measures only bounded local
planning operations. Live SSH/native GUI and controlled latency are not established
by these tests. See [current evidence commands](SSH-INTEGRATION-LIBRARY.md#verification).

Worker tests must gate full queues, blocked handlers, reentrant and panicking
registration, native thread-local destruction and callback lifetime beyond join.
Require actual join before acknowledgement, no stale replacement, reuse of one
cleanup service, charged owner admission after drop, failure isolation and bounded
shutdown waits. Exercise the application review-owner drop while its handler is
blocked. See [ADR 0073](adr/0073-bounded-extension-worker-retirement.md); no test
may equate a finished Rust function with native thread cleanup.

Reinforce version negotiation, strict message schemas and ceilings, exact
capability denial, session isolation, cancellation, queue saturation, stale
generation rejection, crash/restart, disable, uninstall, and shutdown. Optional
failure must leave the core terminal functional.

Context contribution tests use literal v1 envelopes and an observed fragmented
reader to prove rejection before the 65th segment is decoded, including huge and
malformed tails. Preserve exact in-limit values/order, version/schema validation
and typed constructor parity. Source-dispatch/limit/oracle/benchmark mutations
guard the real decoder path. Checked decode/drop timings cover 0/1/32/64 entries;
count admission is not a substitute for separate transport byte/deadline budgets.
See `docs/CONTEXT-CONTRIBUTION-CONTRACT.md` for the current contract and commands.

The shared text owner has independent literal-cluster characterization for
trimmed and whitespace-preserving labels, borrowed prefix storage, zero/one,
exact and over-limit values, combining marks, emoji, Indic text and long inputs.
Its same-host benchmark compares short and long compaction without implying
pixel fitting, screen-reader correctness or an allocator/RSS measurement.

Semantic surface contracts additionally require literal wire oracles, duplicate
member/identity rejection, exact typed units, per-sequence and aggregate budgets,
permission binding, stale/replayed revisions, expiry, once grants, revocation,
last-good retention, bounded redacted diagnostics and repeated snapshot cleanup.
Memory-accounting fixtures must report counter errors without unwinding inside
allocator callbacks and disable measurement before failed-path fixture cleanup.
Benchmark typed validation separately from decoding. Keep reviewed fuzz seeds and
decode/validation benchmarks for 0/1/100/2,000/20,000 rows. An admission-only model
is not an activated browser, renderer, multi-surface registry or native AT proof.

Typed table presentation needs exact borrowed-value and pointer-identity tests,
zero/tiny/maximum/over-limit viewport bounds, non-wrapping navigation, invalid
indices, mixed-axis fixed-seed transitions, reorder/delete/resource-handle reuse,
schema changes and independent scrolled-anchor preservation. Traverse fake
provider frames through host admission and revision-bound navigation; loading,
failure, expiry, clock rollback and malformed refresh must not retarget selection.
Benchmark checked navigation/projection separately from replacement/drop for
0/1/100/2,000/20,000 rows, with flat sampling and one prepared replacement at a
time. Keep raw shell capture bounds unchanged. Models are not native input,
screen-reader, compositor, clipboard, provider or sustained RSS evidence.

### image-protocols-local-preview

Kitty size declarations cannot allocate unreceived bytes. Test pre-decode
rejection, exact/over-limit actual payload accumulation, checked length overflow,
transfer isolation, inherited quiet/identifier replies and recovery after
rejection. Test graphics-enabled and minimal feature configurations. These
per-transfer tests do not certify aggregate retention or decoder peak memory.

Inline image quads use the same command-row projection and crop texture slices
around inserted rows. Regressions check exact coordinates and UV preservation;
offscreen file-list candidates must not participate in visible preview navigation.

Reinforce input, dimension, pixel, frame, cache, and lifetime limits; malformed
protocol images; local regular-file identity; links; replacement; permission
errors; cancellation; stale results; clipping; scrolling; eviction; and cleanup.
Compression upgrades must preserve the reviewed PNG/Flate2 miniz_oxide family.
Run the unchanged dependency policy and hostile decompression tests, and resolve
both root and fuzz metadata with locked full dependency graphs. Metadata with
`--no-deps` is not proof that the complete fuzz lockfile resolves correctly.

### shell-integration-listings

Exercise the actual application bootstrap with the real PowerShell integration
in repository and flattened resource layouts. Prompt, input, completion and CMD
helpers must survive bootstrap return and repeated loading, including literal
space/bracket paths. Native ConsoleHost/PSReadLine must remain idle without user
input, execute one explicitly submitted command once, and return to stable input.
Use isolated configuration/history, bounded child cleanup, and content-free
diagnostics. Windows PowerShell and explicitly selected PowerShell 7 are separate
native evidence; unexecuted operating systems remain external.

Google command reinforcement: exact fixed-origin query encoding, argument/byte
boundaries, control rejection, content-free failure/debug, preview with no
browser/config side effects and correctness-checked encoding timing. Native
post-build smoke must source the shipped adapters twice and test existing
functions/aliases/executables, disable and missing executable. WSL path flags
remain one-way and collision-free. Mutation-test removal of limits, preview,
exact output and readiness dispatch. Shell history and external browser data
are separate from Automexia persistence; no live queries belong in fixtures.
Keep unexecuted native desktops and interactive CMD activation external.

The same controls apply to every `amx search` provider and `amx docs` tool.
Assert exact fixed-origin query parameters, GitHub repository scope, independent
official-site filtering, unknown-provider rejection without fallback and shared
validator use. Real post-build shell fixtures must execute every search route;
mutating away their dispatch, preview isolation or native oracle must fail.
Benchmark checked search and documentation URL pairs without opening a browser.

Local-tool reinforcement: exact rg/tldr argv, default ignores plus negative
privacy rules, bounded JSON/NUL parsing, terminal-control/bidi escaping, no
automatic install/update and no browser fallback. Require real installed-client
evidence for early and late binary detection: keep a match before a distant NUL
beside valid text, then require only the valid file's exact results. The parser
must reject orphan, duplicate, unfinished, invalid-offset and post-summary records,
including interleaved files, before publication. Count discarded matches against
the budget. Mutation-test completion-before-publication and binary filtering;
benchmark every output row, not just count or endpoints. Native offline-client
fixtures must prove exact argv, unsupported-client refusal, missing-cache and
invalid-text failure, escaped controls and absence of execution or update calls.
Require permanent exact documented-search examples, nested Git fetch/insteadOf
behavior distinct from push URLs, independently decoded editor file identities
and rejection of project preferences overriding user disable. Require actual
child/process handle evidence for cancellation, timeout, stream saturation, descendant cleanup
and guest-lease EOF. Windows cleanup tests must acquire the acknowledged live process
handle before allowing parent exit or requesting cancellation, and retain it
through the immediate post-capture assertion. A PID lookup after cleanup can
observe a reused identity. Run four concurrent workers over 25 cycles each,
including descendants with inherited and closed output pipes; EOF alone is not
native process-exit evidence. A later diagnostic wait must not turn an initial
failure into success. Require the Windows observer to attach before resume, pin
bounded member handles before termination, and gate completion on both native
signals and an empty job. Over-limit and unqueryable state must not report success.
Use an isolated native fixture to assert exact handle recovery after each capture
and measure checked capture latency. Do not move this work onto the GUI path.
The WSL import-path canary must prove project Python modules
cannot execute; a default safe host environment is not a sufficient oracle.
Also test the earlier interpreter-selection boundary with real project-local
`python3` canaries and dot/empty/relative PATH entries. Reject an all-relative
PATH before launch; preserve explicitly configured absolute tool precedence.
Test POSIX splitting on Windows, byte/count/control limits and redacted errors.
Mutations must catch skipped, reordered and bypassed bootstrap validation, not
only presence of `-I`. Benchmark exact filtered output separately from WSL launch.
The native shell smoke runs a real search from an isolated project directory.
The separate `check_amx_guest_native.py` entrypoint exercises Linux supervision;
it cannot be replaced by a Windows-only mock. Keep a real tealdeer cache and
unexecuted native Unix adapters external until tested. Parser timing is distinct
from process/filesystem latency. Mutate each boundary and its test dispatch.

Directory opening must preserve native and guest path identity, resolve symlinks
in the owning OS, reject files and ambiguous Windows components, and keep preview
free of desktop launches and storage mutations. Assert exact destinations and
redacted errors/debug over default, relative, absolute, Unicode, missing and
over-limit paths. Keep the fixed guest probe isolated from project imports and
under the existing lease owner. Mutation-test its route, limits, preview, path
validation and real CLI/guest oracle. Measure checked conversion separately from
filesystem/desktop latency; unexecuted native desktop and Unix Rust evidence
remains external.

Editor handoff uses the same path owner with a distinct regular-file kind.
Assert exact VS Code/Insiders URI authority, escaping, Unicode, literal percent
bytes and positive line/column bounds. Reject ambiguous filenames and workspace
manifests. Strict bounded user-root preferences, explicit override and disable
must not become project-script authority or fall back on invalid configuration.
Require real CLI and shipped-shell preview tests, guest file-symlink resolution,
hostile Python import canaries and unchanged files/settings. WSL test preferences
must reach the isolated host root through explicit one-way path translation.
Mutate editor dispatch, file-kind checks, config limits/version/disable, encoding,
preview, native oracles and benchmark correctness. URI timing does not measure
editor startup; no-launch tests do not prove visible desktop activation. Native
macOS/Linux desktop, editor prompts and actual cursor placement remain external.

Repository navigation must use one read-only Git remote lookup in the owning
session. Assert exact GitHub/GitLab root and issue destinations, explicit remote
selection and no fallback when a remote is missing. Reject credentials, unreviewed
hosts, schemes, ports, traversal, controls and over-limit names/URLs/components.
Include mixed-case SSH DNS hosts on both services, exact repository path casing
and mixed-case lookalikes; HTTPS/SCP-only tests cannot cover SSH parser behavior.
Use real temporary Git repositories and unchanged-file/config oracles; guest
results must come from guest metadata rather than the host checkout. Never fetch,
authenticate or call a repository-existence API to implement offline preview.
Mutation-test the fixed Git argv, guest allowlist, URL policy, preview, native
shell/guest dispatch and checked parsing benchmark. Browser visibility, repository
existence/permissions and unexecuted native platforms remain separate evidence.

Use live eza output over isolated fictional variable-length filenames to cover
multi-column resize behavior, not only equal-width synthetic rows. Preserve icons
and colors, verify every filename occurs once and restore exact initial cell
spacing. The parent owns fixture cleanup; never inspect a contributor directory
or rerun a command to conceal broken terminal reflow.

Reinforce supported-shell startup, prompt boundaries, object-preserving
pipelines, quoting, Unicode, missing-resource fallback, explicit disable/remove,
and cleanup. The native shell remains the independent command-editor oracle.
PowerShell assurance must capture real identity control bytes in a child process,
verify internally that user and path fields exist, and discard those bytes before
they reach local or hosted logs. CMD identity fixtures must use stable fictional
values rather than contributor account or executable-path data.

### packaging-release-provenance

Reinforce artifact identity, licenses, checksums, SBOM, provenance, signatures,
install, upgrade, rollback, uninstall, startup hooks, user-data policy, and final
cleanup. Missing native signing or notarization evidence remains not run.
The native nFPM revision is one repository-owned input for both DEB and RPM
names; the post-download assembler must consume the exact native filenames, and
mutation coverage must reject a missing, duplicate, zero, or split revision.

Linux Early Access authorization additionally validates the pinned owner's
author/merger/sender/original/rerun identities, a same-repository merged release
PR, and exact current-main identity under ADR 0040. The existing
`tools/ci/public_distribution.py` / `tools/ci/test_public_distribution.py` pair
owns realistic event and CLI tests, 1 MiB/32-depth/32,768-node bounds, duplicate
and malformed JSON rejection, no writes, and redacted diagnostics. Mutations
must reject removed, duplicated, reordered or ignored authorization and forged
contexts while preserving manual rehearsal isolation and all artifact gates.
Only the real post-merge run supplies signed-release evidence.
The explicitly approved public-archive solo policy requires zero approvals,
no mandatory second reviewer, and retains PRs, resolved threads, signatures,
linear squash history, no bypass and protected tags. Mutate every field's
absence, value and JSON type. Public documentation uses the separate read-only
`check_public_release_metadata.py` / `test_public_release_metadata.py` owner:
check exact file inventory, logo bytes, accessible image text, local anchors,
six pinned download URLs, trusted verification key and private-path rejection.
Check live anonymous links and the actual GitHub-rendered page separately;
offline tests neither activate downloads nor certify rendered pixels.
The real classic-protection self-approval blocker is also a regression seed:
require bounded classic review evidence as well as rulesets, reject type/value
drift, and mutate its API fetch and CLI wiring without expanding App authority.


Minimal public metadata tests must reject full inventories, unknown manifest
fields, private identifiers and paths in documents even after rehashing,
incorrect review-policy digests, removed notices, altered verification comments,
and retention outside the private source repository. Preserve schema-1 first
release compatibility without allowing a downgrade for another version. Run
real ephemeral-key Minisign verification and tamper rejection where available;
mocked signatures do not prove cryptographic interoperability.

Private SBOM privacy tests must retain the recomputed-checksum leak regression,
real preparation CLI, full dependency/license/file-hash preservation, cross-format
input identities, virtual versus host scan paths, nested/encoded unsafe metadata,
duplicate/dangling references, malformed and bounded input, atomic-write failure
and untouched private inputs. The shared release-trust suite must accept Syft's
versionless file/root entries without counting them as dependencies. Workflow
mutations reject raw-output uploads, generator drift, skipped/duplicated/moved
preparation, suppressed failure and missing scratch cleanup before claiming
current-source verification. Hosted publication remains separate evidence.
Read-only App governance must combine REST rules with repository/identity-bound
GraphQL bypass evidence. Preserve the real omitted REST field and redacted
nonempty GraphQL actor as regression seeds. Test zero/nonzero/inconsistent
counts, missing/duplicate/substituted identities, incomplete pages, query errors,
1 MiB/depth/node limits, strict redacted CLI failure and no writes. Mutation-test
query removal, comments, error suppression, deadline changes, stale evidence,
reordering and permission expansion. A native App positive control must detect
an actual actor on an isolated disabled rule; remove that exact rule and revoke
the diagnostic token afterward. Never modify live main/tag rules for this test.
Draft lifecycle tests must use GitHub's observed temporary release and asset
locators before tag creation, not a published-only fixture relabeled as draft.
Bind the create response URL and numeric ID through lookup, complete signed
bundle verification, the exact publication write and immutable response. Reject
missing/changed IDs, cross-draft locators, malformed temporary URLs, stale
published URLs, digest drift, stable/latest-channel drift, skipped or reordered
checks, and extra writes. Verify the actual least-privilege App draft path before
discarding a failed draft; keep final version-pinned URLs mandatory and do not
claim publication until release and every asset attestation pass.
`tools/ci/test_free_plan_contract.py` also removes and inverts the stable-lane
Linux-namespace exclusion; the selector must reject that overlap before stable
authorization rather than failing an unrelated branch grammar later.

### contributor-automation-quality-policy

Model benchmark dependencies must stay development-only. Reuse the shared
Criterion-kind validator for extension API and UI model packages; test runtime,
build, renamed, target-specific and missing/malformed declarations, and mutate
the owner dispatch so an allowlist entry alone never permits runtime authority.

`tools/ci/qa_process.py` and `tools/ci/test_qa_process.py` reinforce the existing
QA owner with parent-exit retained-pipe regressions, exact descendant identities,
gated admission, separate exit receipts, joined readers, argument/capture limits,
interruption, concurrency rejection and repeated cleanup. Test completion and
deadline separately; preserve final bytes and nonzero status. Native Windows
handles or Linux pidfds must be acquired before allowing the parent to exit.
Failed cleanup retains ownership and blocks more launches. Mutation-check native
admission/cleanup order as supplementary evidence, not a substitute for live
process tests. Record startup overhead and repeated handle/thread counts; keep
macOS and unavailable hosted validation external. See
[ADR 0057](adr/0057-contained-contributor-qa-processes.md).

QA diagnostic privacy tests exercise case-folded and escaped local prefixes
through the real failed-subprocess capture path. Use fictional roots and exact
logical-label expectations, preserve the original nonzero exit and failed
verdict, and cover nested home/workspace precedence, literal regex characters
and unchanged unrelated text. Keep output ceilings and source-identity checks;
scan generated evidence without deleting or reclassifying earlier failures.

`.github/scripts/check_free_plan_contract.py` and
`tools/ci/test_free_plan_contract.py` own compiler assurance. Compare every stable workflow's effective Rustup
selector with the bounded canonical pin, not just find an install command.
Mutate missing, nested, duplicate, floating and mismatched selectors; malformed,
oversized and missing pin files; legacy overrides; global-default changes; and
cache generations that no longer follow the compiler. Keep failure messages
path-free and explicit nightly overrides unchanged. Host toolchain inspection
and hosted execution remain distinct from the static contract mutation suite.
Validate workflow expressions with Actionlint as an independent parser. Execute
the real cache initializer in native Linux Bash and compare exact environment
file bytes; reject misplaced initialization, duplicate writes, clobbered prior
values and unsupported job-level `env` context references.

The shared Markdown-anchor owner needs independent literal fixtures and actual
consumer mutations for Unicode, markup, duplicate headings, missing references,
invalid UTF-8 and missing files. Reject fenced-only references and test both
natural-suffix collision orderings. All three production checkers must
use the same parser without turning their tests into its own result oracle.

Verify resolved rustup selection and actual rustc/Cargo release and commit fields,
not an installed-tool label. Mutate workflow/TOML drift, duplicate/oversized
structure, skipped or reordered probes, compiler overrides, ignored failures,
MSRV removal and stale cache labels or current compiler documentation. Exercise
real probe overflow/deadline cleanup and workflow-label disclosure canaries
with redacted errors. The explicit locked MSRV build remains distinct from the
development/release pin even when their current versions match.
Mutation-test compatible Python bootstrap before every compiler probe, including
the native Ubuntu 22.04 runner. Preserve full-history secret scanning with only
exact reviewed historical fingerprints; reject expanded exceptions and prove
real new generic-key/token findings in the same paths remain detectable.

Mutation-test repository validators so deleted owners, weakened limits, stale
paths, fabricated evidence, missing private-documentation exclusions, and
confidential-data canaries fail closed.

Public-documentation mutations cover restored future announcements, renamed
pages and changelog entries, duplicate or unreviewed status rows, and unresolved
merge markers inside and outside code fences. Current limitations and missing
release evidence remain allowed. Diagnostics must not echo rejected content.
These checks supplement human publication review; they do not prove the absence
of every possible confidential statement. Nonactivation, architecture, security
and release-evidence gates remain enforced.

The documentation checker/hygiene pairs own publication regressions. The
existing `test_phase_implementation_audit.py` and
`test_production_operations_po0.py` suites additionally enforce current-status
row identity and the absence of runtime activation.

For caches and build storage, additionally mutate content identity, integrity
manifests, atomic publication, link/reparse rejection, traversal ceilings,
current and dirty worktree protection, live/dead owner probes, leases, grace
periods, dry-run/apply behavior, exact deletion allowlists, de-duplicated usage,
success/failure cleanup, and the dormant pre-push hook. Compare the generated
tree and hashes independently. Exercise native process ownership on each
claimed platform and keep unexecuted hosts external.

Cache regressions also require new-name admission during deletion, retained
named locks, concurrent collectors, native subprocess exclusion, iterator
consumption and closure, queued-directory limits, lease-handle overflow, and
lock release after inventory or deletion failure. An independent temporary
storage tree checks forbidden deletion and unchanged bytes. Older clients that
introduce unseen lease names remain outside the admission protocol.

The summarized workspace-test owner must also keep its 30-minute deadline,
16 MiB stdout ceiling, live compiler diagnostics, Unix process group or Windows
Job Object, and real success/deadline/overflow child-process tests. Mutation
coverage must reject a missing or relaxed limit, wrapper, cleanup path, partial
failure diagnostic, or real-process oracle.

### stabilization-release-assurance

QA evidence must reject empty or stale JUnit, duplicate identities, inconsistent
counts, invalid encoding, DTD/entity declarations and over-limit reports. Preserve valid
failed reports and require fresh report identity after each nextest run. Execute
XML-safe redaction roundtrips on written text, attributes and CDATA, preserving
exact test outcomes and enforcing the serialized artifact ceiling. Execute
all benchmark scenarios in test mode separately from controlled timing evidence;
continue independent tests after failures without weakening flaky-result gates.
Exercise exact free-space boundaries and overridden, uncreated build targets;
never silently clean unrelated caches or describe an interrupted run as passed.
Benchmark inventory must prune excluded trees before descent, with independent
directory-operation counts that remain constant as cache fixtures grow. Denied
source reads must fail closed instead of silently reducing coverage.

Private provider transient deletion failures must retain authoritative records
until a retry really removes the owned file. Test native sharing violations,
immediate handle retirement, all five retirement paths, unauthorized-caller
isolation and retry before expiry without reactivation after clock rollback,
repeated revoke/shutdown, exact file/root removal and capacity recovery; preserve
content-free checkpoints and unexplained historical stalls as unresolved evidence.

Keep first failures, compare same-host performance baselines, require bounded
resource growth and cleanup, and bind release evidence to the exact commit and
package. Cross-compiles and mocked UI evidence do not satisfy native gates.

### ffi-wasm-embedding

Where retained for compatibility, reinforce ABI/version negotiation, bounded
input/output, ownership, cancellation, panic containment, hostile input,
disable/unload, and absence of ambient authority.

## Source-owned reinforcement plans

These anchors preserve machine-enforced traceability for implemented or
explicitly nonactivated source owners. They are assurance boundaries, not a
public product schedule or commercial feature announcement.

### command-productivity-cp0-policy

Reinforce the pure policy/schema boundary with exact limits, hostile structured
input, deterministic decisions, and proof of no process, network, credential,
profile, or execution authority.
The renderer benchmark-only workspace member is not a missing-runtime-source
exemption: scan its Rust sources after validating its exact non-product target
contract. Mutate automatic target discovery, build/runtime sources, target path,
hidden features, integer/boolean confusion, malformed/oversized manifests and
symlinks. Preserve ordinary missing-source rejection, global file-count limits
and redacted errors; the architecture metadata checker separately guards edges.

### command-productivity-cp2-persistence

Reinforce bounded private persistence, revision compare-and-swap, atomic update,
last-known-good recovery, permissions, corruption, concurrency, rollback,
disable, removal, restart, and exact residue checks.

### command-productivity-cp22-quick-actions

Reinforce list, search, review, insert-without-Enter, copy, placeholder binding,
risk confirmation, import/export, keyboard access, redaction, and route/generation
isolation without granting launch authority.

Search allocation changes must preserve exact scalar scores and string-level
Unicode lowercase (including final sigma), word boundaries, repeated and missing
subsequences, maximum candidates/queries and deterministic result limits. Keep
the independent indexed oracle, fixed-seed public search properties, allocating
canary/unwind reset and repeated maximum-input allocation ceiling in
`quick_action_activation.rs`. Correctness-checked search benchmarks must retain
action identities and scores; native latency evidence remains separate.

### command-productivity-cp4-provider-actions

Reinforce that provider-aware public actions consume bounded caller-owned cached
observations, report stale/unknown state, preserve external authentication, and
cannot perform hidden discovery or execution.

### command-productivity-cp31-persistent-aliases

Reinforce exact preview, shell-specific compilation, collision ownership,
explicit activation, private publication, native parsing, rollback, disable,
uninstall, startup bounds, and absence of raw shell evaluation.

### command-productivity-cp32-devops-packs

Reinforce immutable reviewed manifests, disabled-by-default actions, exact
provenance/digests, conservative risk floors, explicit enablement, provider-free
inspection, update planning, and safe removal.

### command-productivity-cp33-native-imports-workspace

Reinforce user-supplied bounded inventories, strict simple-command parsing,
explicit selection, dry-run, collision review, workspace trust/revocation,
source-change invalidation, and no native discovery or task execution.

### command-productivity-cp1-native-completion

Reinforce shell-native ownership, generated artifact integrity, completion
fallback, buffer/cursor preservation, bounded startup, explicit install/remove,
and no provider, network, credential, or implicit execution while typing.

### command-productivity-cp50-research

Keep research evidence clearly non-implementation: source dates, alternatives,
security/performance constraints, unresolved platform questions, and the exact
gate required before a proposal may change public behavior.

### command-productivity-cp51-proposal

Suggestion-model regressions must keep full bounded accessible text and insertion
bytes while visually compacting graphemes. Test exact/over byte limits, invalid
offscreen candidates, preserved whitespace, sorted unique visible match indices,
and unhighlighted generated ellipses. Verify keyboard and pointer acceptance
through the real validation/ranking/controller path, exact replacement bytes,
no execution and final worker cleanup. Model evidence is not a native
screen-reader claim and does not change preview activation.

Require proposal-only owners to remain nonactivated until architecture,
capability, persistence, native-platform, UX, performance, and negative-side-
effect contracts are accepted and independently evidenced.

### situation-aware-production-operations-po0-proposal

Keep this proposal nonactivated and noncommercial in public source. Assurance
checks verify separation from credential, provider, approval, remote-execution,
and terminal hot-path authority; detailed future planning remains private.

### ecosystem-d7-cp6-proposal

Optional signed settings metadata uses independent schema-1 declarations while
preserving manifest v1, accepted digest, receipt/signature framing and release
denial. Cover literal Boolean/Choice/Integer defaults, exact byte/count/text/range
bounds, duplicate/unknown members, Unicode, identity and receipt relabeling.
Signed-member tampering must fail package verification; malformed optional metadata
must remain unavailable without retroactively rejecting a valid legacy package.
The fuzz target reaches the pure decoder directly because its hostile package
path has no trusted publishers. Real signed fixtures prove projection integrity;
fuzz compilation does not establish an executed campaign or installed Settings.

All-feature host assurance must cover pre-arming interrupts, shared-engine ticks,
reused cancellation tokens and worker-unwind cleanup. Require invocation-local
completion, a persistent absolute deadline, store-local interruption decisions
and joined watchdogs. Use a finite emergency fuel budget in deliberate lost-tick
mutations; retain the native infinite-loop failure separately. Default-feature
QA is not evidence for code behind the component-host feature, and none of these
source tests grants product activation.

Keep optional ecosystem proposals behind explicit capability, signing,
sandboxing, lifecycle, disable/uninstall, resource, and failure-isolation gates.
This anchor is not an availability claim.

### stabilization-release-assurance-s1-s2

Use explicit fixture clocks for dated evidence and validate a positive baseline
before negative mutations. Pin exact age/skew boundaries independently and
retain the real-clock expired-evidence rejection; calendar aging must not make
an unrelated mutation pass. Synthetic fixtures never certify native runs.

Reinforce staged stabilization with exact failure retention, same-host baselines,
native evidence, packaged-artifact identity, bounded resources, cleanup, and
explicit external gates before release claims advance.

QA source evidence must use content-bound dirty fingerprints, not status-line
hashes. Mutate an already-dirty file without changing its status and prove the
identity changes. Exercise untracked/deleted/renamed files, Unicode names,
byte/file/total limits, link rejection, unavailable Git, subprocess deadlines and
cleanup. An actual QA report must fail on before/after identity drift even when
all mocked test commands pass; unavailable identities must never compare equal
as passing evidence. Keep source contents and private paths out of reports.
Exercise logical artifact announcements through the actual CLI on successful
and failed runs, including optional bundle creation; reject checkout disclosure.

### connection-hub-f2-models

Reinforce pure bounded models for public connection metadata, provenance,
freshness, unknown state, stable identity, duplicates, hostile labels, and
absence of credentials, login, network, filesystem mutation, or process launch.

### connection-hub-f3-catalog

Reinforce immutable snapshots, deterministic filtering/grouping, route and
generation isolation, size ceilings, stale-result rejection, and truthful
empty/loading/unavailable states.

### connection-hub-f3-composition

Reinforce single-owner composition of public connection sources with explicit
precedence, conflicts, provenance, cancellation, publication-before-wake, and
last-known-good behavior.

### connection-hub-m1-product

Keep independent binary-owned truncation and wrapping fixtures with literal
combining, joined emoji and regional-indicator clusters. Retain whitespace,
extra-ellipsis and zero/one/exact-limit policies; wrapped review text must
concatenate to the original bytes. Preserve focus, pointer targets and full
model values; grapheme counts do not prove pixel fitting or native rendering.

Reinforce the read-only keyboard workflow, focus restoration, accessibility,
responsive layouts, review-before-activation, and proof that browsing or
selection sends no PTY input and starts no connection.

### connection-hub-f3-library

Exercise the actual pretty-JSON serializer with a half-budget-plus-one string
and its final quote; assert exact bytes and capacity within the document limit.
Keep independent literal format/escaping and validation-order fixtures. Shared
writer properties must cover zero/exact/over-limit, UTF-8, accumulated writes,
overflow rejection, unchanged accepted bytes on error and in-memory flush.
Keep native recovery/permission and allocator-RSS claims separate.

Reinforce bounded user-private connection metadata, atomic persistence,
conflicts, permissions, import/export, rollback, disable, uninstall, corruption,
and strict exclusion of secret material.

### connection-automation-m6-workspaces

Reinforce declarative topology, profile/recipe review, generation binding,
bounded restore planning, revision conflicts, cancellation, recovery, and no
implicit PTY/process start from list, show, edit, or restore review.

### provider-auth-m7-capsules

Reinforce external credential ownership, opaque references, identity/context
freshness, per-pane isolation, expiry/revocation, offline/unknown states,
redaction, and absence of provider-global context mutation.

### direct-openssh-m3-review

Reinforce exact executable/argument review, host-key and route evidence,
cancel/timeout/cleanup, external agent/key ownership, hostile metadata, and no
shell command-string evaluation or implicit connection.

### provider-aws-m8-source

Reinforce bounded cached public AWS observations, explicit profile/account/region
provenance, stale/expired/unknown states, per-pane isolation, redaction, and no
credential copying or background provider work on terminal hot paths.

### provider-azure-m9-source

Reinforce bounded cached public Azure observations, explicit tenant/subscription
provenance, stale/expired/unknown states, per-pane isolation, redaction, and
external CLI/identity authority.

### provider-gcp-m10-source

Reinforce bounded cached public Google Cloud observations, explicit project/
account provenance, stale/expired/unknown states, per-pane isolation, redaction,
and external CLI/identity authority.

### provider-kubernetes-openshift-m11-source

Reinforce bounded context/cluster/namespace metadata, exec-auth opacity,
certificate/path redaction, stale source handling, per-pane isolation, and no
global kubeconfig mutation or implicit cluster operation.

Credential-redaction fixtures must contain every sentinel asserted absent from
public output. Test inline token, key, certificate, username and password
independently so one field cannot mask another field's presence classification.
Check exact public user projections, merged snapshots, debug output and rejected
document diagnostics. Parser benchmarks independently validate their results
before timing empty, comment-heavy, dense credential, oversized and malformed
workloads; byte scanning alone is not resource-scaling evidence.

### provider-teleport-m12-source

Reinforce bounded public profile/proxy metadata, external Teleport identity and
certificate authority, expiry/unknown states, redaction, route review, and no
credential custody or hidden login.

## Documentation reinforcement

Every documentation change must prove:

- all public Markdown loads as UTF-8;
- titles and code fences are structurally valid;
- relative file and heading links resolve;
- no machine-local or confidential values are present;
- no unreleased advanced or commercial plans appear publicly;
- the lossless private archive is present and ignored; and
- the public index, feature catalog, architecture, testing, and roadmap agree.

Exact private schemas, algorithms, provider matrices, future workflows, market
plans, pricing, and commercial packaging must never be copied into a public
fixture merely to satisfy a checker.
