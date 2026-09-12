# Testing

## Keyboard hyperlinks

```text
cargo test -p automexia-terminal --bin automexia --locked keyboard_links -- --nocapture
cargo test -p automexia-terminal --bin automexia --locked keyboard_links_benchmark -- --ignored --nocapture
python tools/ci/check_feature_test_reinforcement.py
python tools/ci/test_feature_test_reinforcement.py
```

The first command runs real-parser, keyboard-policy, default platform-table,
resource-bound and controlled preview-pixel regressions. The benchmark is
explicitly ignored by ordinary tests: require its actual nonzero test count and
correctness assertions. It measures capture plus 100 exact snapshot/navigation
operations, not native browser opening or compositor latency. For a fresh
fictional CPU preview, set `AUTOMEXIA_HYPERLINK_PREVIEW` to a temporary PNG target
before the first command, inspect that image and remove it after review.
Native WGPU/CPU desktop frames, browser/clipboard behavior, alternative keyboard
layouts and Narrator/NVDA/VoiceOver/Orca delivery require separate native runs.
Do not promote controlled pixels or all-platform binding tables into OS claims.

## Bounded colour setup

```text
cargo test -p rio-vt --test color_conversion --locked
cargo test -p rio-vt --lib --locked config::colors::tests
cargo test -p rio-backend --lib --locked config::
cargo bench -p rio-vt --bench vt_input --profile dev --locked -- color_setup --noplot
cargo bench -p automexia-terminal --bench automexia_services --profile dev --locked -- color_setup --noplot
```

The integration target checks literal numerical RGB/RGBA values, every channel
value, malformed Unicode, size and marker boundaries, serde defaults and a
fixed-seed independent ASCII grammar. Its thread-local System allocation counter
includes first-use and 32 repeated default/helper calls with a zero-allocation
ceiling. This measures allocation calls on that path, not whole-process RSS.
The Unicode regression reproduces the former validator/decoder mismatch before
the production fix. Correctness assertions execute inside benchmark iterations.
Use Criterion's `--save-baseline` before editing and `--baseline` afterward on
the same host, compiler, profile and feature set. These development-profile
benchmarks measure setup only; optimized builds, full window startup, native
compositors and controlled S2 evidence are separate claims.

## Bounded Quick Action scoring

```text
cargo test -p automexia-command-productivity --test quick_action_activation --locked
cargo bench -p automexia-command-productivity --bench quick_actions --profile dev --locked -- bounded_search_scoring --noplot
```

The public search path is checked against an independent indexed-scalar oracle,
literal scores, contextual Greek lowercase, missing/repeated subsequences,
word boundaries, deterministic ties and the 128-result ceiling. Fixed-seed
properties complement maximum-length ASCII and Unicode candidates and queries.
A test-only thread-local System allocator measures the largest allocation over
32 repeated searches: each allocation must fit an 8 KiB ceiling for that fixture.
This is not a total-memory or RSS ceiling. An allocating canary and unwind reset
test protect the instrumentation. The old character table exceeds this ceiling.

The existing capability-free command-productivity owner streams scalar values
using the standard library; no cache, worker, I/O or package boundary is added.
String-level lowercase remains authoritative; per-character folding can change
[context-dependent Unicode results](https://doc.rust-lang.org/std/primitive.str.html#method.to_lowercase).
Trust, session, provider and execution policies remain with their existing owners.
Benchmark iterations assert exact scores and action identities on a 1,024-action
inventory and a maximum-length candidate. Same-host setup/search measurements
do not certify native input-to-frame latency or provider/network performance.

## Shortcut editor assurance

Current source adds palette badge double-click and contextual F2 editing. Owners
are `bindings/shortcut.rs`, `renderer/command_palette/shortcut_editor.rs`, the
application binding-only transaction and `automexia/preferences.rs`.

Run the following before a shortcut-editor change is considered ready for review:

```text
cargo test -p automexia-terminal --lib --bin automexia --locked
cargo test -p automexia-terminal --test user_preferences_restart --locked
cargo test -p automexia-terminal --bin automexia --locked benchmark_shortcut_record_validate_and_cancel -- --ignored --nocapture
python tools/ci/check_feature_test_reinforcement.py
python tools/ci/test_feature_test_reinforcement.py
cargo ready
```

The state tests exercise the live palette key/gesture owner, actual Windows,
Linux/BSD and macOS default tables, typed profiles, explicit unbind/text conflicts,
reserved and hostile keys, Enter repeat, reset/retry, stale queued edits and
tiny-to-8K geometry. Independent original-table comparisons cover every classic
mode-bit combination and all combinations of typed Search/Vi/alternate screen;
changing normal-mode Paste or split keys must neither remove nor duplicate their
existing actions in those modes. The writer test holds the real private-store lock to force a
failed write, verifies unchanged primary data, consumes the error and requires
flush to remain failed before a successful retry. A separate test process saves
the shortcut and confirms restart/reset without modifying `config.toml`.
Source-order mutation tests supplement, not replace, these behavioral oracles.
The benchmark includes correctness assertions and reports model latency only.

Native acceptance checklist (not established by a model or cross-platform table):

1. Use a disposable config root with no personal startup scripts. Build the exact
   candidate binary. Record its revision and renderer; do not change real settings.
2. Open the palette, enter Panes & Sessions, and double-click the clone-right key
   badge. First click must only select; second opens Edit shortcut, never a pane.
   Repeat with F2. Both must preserve the query, category and selection on Back.
3. Record a free combination, such as Ctrl+Shift+F9 if your environment leaves it
   free. Verify the displayed modifiers, Save, then wait for Saved. Close the
   palette and use the chord: exactly one cloned pane appears. The old default is
   no longer an alias in normal mode. Reset and verify the original mapping.
4. Retry with a conflict (for example the active palette launcher), plain text,
   Ctrl+R, modifier-only keys, dead keys, IME, paste, right/middle click and a file
   drop. No command runs or receives bytes while editing; the draft is not saved.
5. Hold Enter across recording/review, press Esc before saving, and return after
   Save. Test Reset before any draft and after a failed write, including clicking
   Reset from each keyboard focus and retrying via Enter or the primary button.
   No old candidate may replace a requested reset. Verify current/new-window and
   process-restart behavior; keep a sibling terminal input sentinel unchanged.
6. Change bindings in another window or reload the config while recording or
   with a queued save. The stale draft/receipt must not overwrite the new state.
   For an incompatible startup overlay, expect a warning and base bindings,
   not a failed terminal launch or silent deletion of the stored preferences.
7. Resize before clicking, scroll between clicks, switch focus and return. Check
   tiny, narrow and maximized windows at 100–400% scale on CPU and WGPU. Compare
   frozen full-frame pixels with zero tolerance and inspect both images; model
   rectangles and literal CPU badge glyph tests are not full-dialog evidence.
8. With Narrator/NVDA, VoiceOver and Orca on their native platforms, check the
   dialog name, current/candidate value, conflict/save announcements, focus order,
   return focus and 200%/400% scale. A test-only semantic summary is not proof of
   OS accessibility delivery. Record each unavailable combination as external.

Keep native desktop, assistive-technology and keyboard-layout results separate
from library tests. No native editor acceptance is implied by these commands.

Shortcut coverage includes all classic catalog actions against actual platform
tables, exact mode/override guards, source-free labels, palette-only Enter, and
literal CPU glyph pixels at 100–400%. The palette model benchmark is separate
from native desktop input latency. See ADR 0051 for remaining native gates.

## QA diagnostic privacy

Benchmark inventory prunes build, tool and private directories before entering
them; ignored-file filtering after recursive traversal is insufficient. Native
directory-operation tests keep source visits constant as cache fixtures grow,
and unreadable source directories fail the inventory instead of disappearing
from its results.
Full Python discovery names each active test in the bounded diagnostic log so a
deadline does not leave only an anonymous sequence of completed-test dots.
The documentation publication checker likewise prunes private, build and tool
roots before traversal. Its distinct root-only exclusion policy stays local;
nested public source directories remain covered, and read failures are errors.

The QA runner rejects missing, stale, empty, malformed, oversized, invalid-UTF-8,
entity-bearing and count-inconsistent JUnit reports. Reports are bounded to
100,000 XML nodes and eight nesting levels. Suite/test identities must
be unique, and reported outcomes must match actual cases. Valid failed reports
are retained as failure evidence, never converted into passing tests. The exact
previous report digest prevents a failed build from reusing an older report.
Redaction operates on XML text and attributes before serialization. Parse the
written artifact again in regressions, including CDATA and failure messages;
valid input alone does not prove that a redacted report remains consumable.
Native nextest runs use `--no-fail-fast`; retries still fail the flaky-result
gate. Ordinary QA also runs every enabled workspace benchmark in test mode;
this exercises correctness assertions, not a controlled performance comparison.
The polling benchmark requires every registration exactly once, a bounded poll
deadline and joined producer threads before the next iteration. Its timing now
includes complete worker cleanup; do not compare it to old detached-worker
numbers as if the measured workloads were identical.

Before Cargo steps, QA checks a 4 GiB minimum free-space reserve for workspace
artifacts and environment-selected target/build directories. Uncreated targets
are checked against their nearest existing ancestor. Results contain numeric
byte counts, not machine paths; insufficient or unavailable space blocks build
commands without deleting caches. The reserve is not a guarantee that a cold
build fits. Monitor long builds and use the documented storage preview workflow.

Run `python3 -m unittest discover -s tools/ci -p test_qa.py` after changing the
QA runner. Fictional-path regressions cover workspace/home precedence, case
changes, slash variants, escaped tracebacks and literal regex characters. A real
failing subprocess verifies that captured logs and command labels are redacted
while the original nonzero exit and failed verdict remain intact. Existing
output ceilings, timeout cleanup and source-identity checks remain mandatory.
Re-scan generated evidence before sharing it; a later passing run does not
clear an unexplained earlier failure.

## Native acknowledgment and private cleanup regressions

Run `cargo test -p rio-vt --test live_resize --locked -- --test-threads=1`.
The Windows fixture keeps CMD-produced output and the CMD `SET /P` editor case.
Non-echoing probes use the same buffered `Console.ReadKey` helper as PowerShell:
`PAUSE` can discard typeahead between READY and the next read. The buffered
regression queues twelve keys and compares a cumulative native receipt containing
every literal key value and ordinal, while requiring unchanged output/cursor and
successful child exit. Per-step tests independently check each acknowledgment.
The cumulative form accounts for native coalescing of rapid title changes;
accepting only the last counter would not establish exact consumed input.

Run `cargo test -p automexia-terminal --lib --locked provider_transients::tests`.
Windows coverage holds a real deny-sharing file handle across revoke and
shutdown, requires retained cleanup ownership on failure, releases the handle,
then verifies successful retry, empty records and removed private root. The
test matrix covers revoke, disable, session retirement, expiry and shutdown: handles
are immediately unusable even if file deletion fails, cleanup can retry before
the original expiry, and wrong-session/generation callers cannot retire them.
Restored file access or clock rollback must never reactivate a retired handle.
Repeated capacity/disable/shutdown cycles use exact file counts and content-free phase
checkpoints. A later pass does not explain a previously observed long filesystem
stall; retain that failure until its cause is identified. These are process and
filesystem tests, not desktop pixels or assistive-technology certification.

## Operational status colour regression

Run the focused classifier and actual grid-renderer tests:

```text
cargo test -p automexia-devops --locked
cargo test -p automexia-terminal --bin automexia --locked semantic_status_tests
```

The first exercises lifecycle/readiness matrices, status/name precedence,
conditions, container health, kind-prefixed pod names, zero-count/log-prefix
boundaries, mixed failure counts, byte limits and properties. The second covers
real fragmented VT input, visible snapshots,
issued grid glyph colours, exact CPU pixel comparison, configured colours,
explicit ANSI, disable and unchanged copied text. The pixel oracle supplies
literal expected colours independently; shared font geometry is not an
independent shaping oracle. Optional `AUTOMEXIA_STATUS_PREVIEW` writes a
fictional-data CPU capture to the selected private evidence path.

Measure against a same-host saved baseline:

```text
cargo bench -p automexia-terminal --bench automexia_services --locked -- semantic_statuses
```

These timings measure
the classifier, not native end-to-end latency. Native GPU presentation and
assistive technologies must still be checked on each claimed platform.

For a safe manual smoke test, print these literal rows in an Automexia pane
without connecting to a cluster: `batch 0/1 Completed` (cyan), `api 1/1 Running`
(green), `api 0/1 Running` (amber), `api 0/1 Init:Error` (red). In POSIX shells
use `printf '%s\n' 'batch 0/1 Completed'`; in PowerShell use
`Write-Output 'batch 0/1 Completed'`. Repeat with the other rows, explicit ANSI,
DevOps disabled, selection/copy, and the documented native display matrix.

## Shared chrome appearance

Run `cargo test -p automexia-terminal --bin automexia --locked renderer::` after
changing shared colours or palette typography. The theme tests use an independent
f64 sRGB oracle after byte quantization, dark/light/custom configuration inputs,
all shared surfaces, literal RGBA tokens and a one-channel mutation. Palette
tests retain navigation/hit geometry and measure real bundled-font key labels at
100–400% scale, including Unicode, long shortcuts and bounded trailing widths.

The `chrome_controlled_style_specimen` test rasterizes fictional labels using
production theme tokens and the bundled font. Set `AUTOMEXIA_CHROME_PREVIEW` to an
absolute private generated PNG destination to inspect the specimen; unset it
afterward. Normal tests write no preview. This fixture is not an application or
native-window capture and does not establish rounded-corner antialiasing, GPU
pixels, native focus delivery or screen-reader behavior. Capture those separately
through the controlled native suite before making a release claim.

The existing private renderer benchmark measures bounded fitting and real-font
CPU text drawing. Token consolidation adds no runtime dependency or background
work; a fitting microbenchmark is not proof of interactive frame latency.
The xtask modal ownership guard checks the shared scrim import and finite opaque
surface literals; mutations reject removed imports, competing declarations,
transparent, malformed, duplicate and nonfinite tokens. It complements, rather
than replaces, runtime drawing and input/compositor tests.

## Live resize preservation

`cargo xtask test resize-stress` runs the library stress tests **and** the
`resize_repaint`, `live_resize` and `pane_editor_resize` integration binaries.
Previously its default library-only name filter omitted the native cases;
full workspace QA ran them separately. Exact dispatch tests and mutation checks
now reject missing native suites and swallowed failures. `--native-gui` adds
desktop validation; it does not enable otherwise missing process tests.

`cargo test -p rio-vt --test pane_editor_resize --locked` exercises the actual
Windows PowerShell ConsoleHost and PSReadLine input loop through the application
worker. Four independently scheduled cases each repeat ten times: empty view,
low prompt, full view and scrollback. Each silently shrinks the surviving pane,
restores or changes its geometry, then types and checks exact editable text,
native cursor, VT cursor and dimensions before accepting the known fixture.
History uses a unique nonexistent test-local path with saving disabled; neither
user profiles nor history are changed. The cases retain bounded acknowledgments,
successful child exit and worker joins. Native PTY cases share the nextest
native-process group; no retries are used as proof of correctness.

The metadata/no-metadata repaint comparison additionally checks the protocol
cursor after every fragmentation boundary. Performer fake-clock tests cover
50 ms deadline edges, read-only registration at expiry, no busy waiting, ordinary
input, bounded batches and buffered-input ordering. These process/model checks
do not establish desktop pixels, physical keyboard gestures or accessibility.
The worker-boundary fixture injects failed resizes and partial/WouldBlock writes;
it verifies exact native acceptance order, unchanged failed/duplicate/pixel-only
deadlines, and cancellation behind blocked input and later resizes. A separate
poller test proves cloned senders wake cancellation despite an undrained input
backlog, without moving or copying that backlog.
Empty input is ignored at the worker boundary so a zero-byte write cannot hold
later geometry changes hostage.

Native Windows pipe tests explicitly close each peer, wait for the worker to
finish, consume the broken-pipe error, and then destroy the adapter. This catches
a second attempt to consume an already-joined worker during teardown. Run
`cargo test -p teletypewriter --locked` and run the editor suite above in its
default shared-process mode as well as nextest's isolated cases. Keep an observed
abort recorded even if a later diagnostic run passes; investigate cleanup rather
than reducing repetitions or hiding it behind test serialization.

`cargo test -p rio-vt --test resize_repaint --locked` exercises raw-byte native
repaint fragmentation, exact logical output, intentional blank lines, Unicode,
pre-existing selection, bidirectional search, navigation, snapshots and separate
screen/history clears. It covers both Unix reflow and ConPTY viewport policies.

`cargo test -p rio-vt --test live_resize --locked` keeps actual shells alive while
alternating narrow/wide and short/tall sizes. Windows runs PowerShell and CMD;
Unix runs Bash, and macOS additionally runs Zsh. The fixtures use fixed fictional
content, bounded acknowledgments/output and successful child-exit checks.
Both eight-short-row and 32-long-table-row fixtures run. The latter crosses the
history boundary and checks every row after shrink/restore, rejecting duplicates.
The captured table repaint is separately replayed at every byte split; padded
hard lines, forced wraps, Unicode, colours, blanks, cursor distance and Unix
explicit spaces have independent expected-content assertions.
The `resized_native_table_matches_independent_viewport_pixels` application test
traverses fragmented bytes, core reflow, native repaint, visible snapshots and
the CPU glyph renderer at 100%, 125% and 200% scale. Its expected viewport is
literal fixture text in a fresh, never-resized grid. It checks the narrow live
viewport, the restored live viewport and restored scrollback, allowing zero
changed pixels. Set `AUTOMEXIA_RESIZE_PREVIEW` to a private PNG path to inspect
the fictional restored view. Shared font shaping is not an independent font
oracle, and CPU pixels do not certify a native GPU desktop.
CMD uses a non-echoing key probe. Its no-resize regression verifies that all
twelve acknowledgments preserve both exact rows and cursor position. The old
empty Enter probe moved the native cursor and could scroll during repaint;
it was not a valid side-effect-free acknowledgment. A separate worker fixture
still exercises Enter during resize. Final CMD release uses a line read after
all viewport assertions, avoiding PAUSE's buffered-key exit race.
Windows-backed fixtures additionally traverse the application's real PTY worker:
three-size bursts, alternating scheduling yields, input ordering barriers, exact
retained rows, and final child release only after the last output assertion.
The repeated CMD case also checks bounded worker shutdown. The direct adapter
baseline remains distinct; it cannot substitute for worker coalescing coverage.
Core unit tests verify deferred geometry, synchronous standalone behavior and
fractional cell metrics. Search tests cover every padding endpoint in both
directions, including empty ranges and matches completed before the boundary.
Stale or out-of-grid search endpoints must return no match without dereferencing
invalid cells. Retain a separate valid-input test that actually reaches the
regex runtime complexity limit; bounds rejection is not evidence for that limit.

The Windows PowerShell, CMD and WSL ConPTY fixtures have been executed, including
the long-table cases. All eight non-opt-in Windows cases passed fifty no-retry
repetitions. Earlier Linux Bash Unix-PTY evidence covers the short fixture under
WSL, not a physical desktop; the expanded Unix long-table and macOS cases still
require execution on their target platforms.
The `live_resize` fixtures do not exercise PSReadLine; `pane_editor_resize`
separately exercises the native editor as described above. Neither establishes
desktop drag gestures, GPU frames, screen readers or a native macOS host.
Keep those evidence gates open.

The WSL tests are opt-in: set `AUTOMEXIA_TEST_WSL_DISTRIBUTION` to an installed
test distribution and `AUTOMEXIA_TEST_WSL_FIXTURE` to that distribution's path to
`rio-vt/tests/fixtures/live-resize-output.sh`. Then run:

```text
cargo test -p rio-vt --test live_resize --locked native_live_wsl -- --ignored
```

The real-listing case additionally requires eza in that distribution. It creates
28 varied fictional filenames under a randomly named, workspace-local temporary
directory and checks cleanup. It does not load user shell profiles or inspect a
real project. Four WSL tests must execute, not a successful zero-test filter.
The listing campaign covers widths from 99 down to 2 columns, restoration to
100 columns, and 8- and 2-row panes. It checks the raw adapter, queued resize
bursts and each intermediate worker commit. Every final assertion precedes
child release, with exact filename uniqueness, original gaps and prompt adjacency.
The title-only probe first proves unchanged rows and cursor without a resize.

The extreme listing case starts at 146 by 16 cells and includes long mixed file
and directory names. A handpicked sequence and eight fixed-seed sequences each
exercise 48 transitions through one/two-cell viewports and sizes up to 512 by 96.
Consecutive mixed-axis changes do not reset to the original size between steps.
Run all raw, burst and intermediate-commit paths. Exact order, spacing, filename
uniqueness and prompt adjacency must hold at every acknowledgment, not only at
the final restore. The previous pairwise campaign missed premature live-prefix
archiving, native printed-space fill and short historical continuation loss.

Deterministic replays cover wider/shorter and narrower/shorter native redraws,
space-filled rows below the cursor and a former seam becoming fully historical.
The application pixel oracle compares restored live and historical rows against
literal fresh reference grids at scales 1, 1.25 and 2. Set
`AUTOMEXIA_EXTREME_PREVIEW` to a private PNG path and inspect the fresh artifact.
`cargo bench -p rio-vt --bench vt_input --locked -- grid_extreme_replay_roundtrip_checked`
times extreme reflow, fictional native replay and visible snapshots, with exact
copied-input and bounded-history assertions outside each timed iteration. It is
separate from `grid_extreme_roundtrip_checked`, which cycles one-cell and large
viewports without replaying a frame from a different native live origin. Each
benchmark retains one terminal across iterations and validates original text.
This timing is not native PTY or compositor latency. Historical eviction at the configured
scrollback limit is distinct from corruption; no unbounded backup buffer is used.

The previous ASCII/constant-width fixtures missed spaces at a native history
seam, a cursor exactly at the new margin, and entirely blank soft-wrap fragments.
`resize_repaint` now covers these cases deterministically, including erased cells,
pre-existing selection and repeated width round trips. The existing application
pixel test also moves the suffix into history and compares its exact restored
column against a fresh literal reference at 100%, 125% and 200% scale. Set
`AUTOMEXIA_SEAM_PREVIEW` to a private PNG path to inspect that additional image.
Minimized zero-sized window events remain ignored by the application; a native
desktop minimize/restore campaign is separate from these process/grid checks.

Never record resolved checkout paths or distribution-specific personal data in
evidence. WSL validates the Windows ConPTY route, not a native Linux desktop.
Run the Unix tests from a native filesystem checkout on Linux/macOS. Tests must
not silently substitute Windows results for an unavailable native platform.

`cargo bench -p rio-vt --bench vt_input --locked -- grid_resize_snapshot`
measures repeated reflow plus visible snapshots for both policies with one
bounded retained terminal. Keep profile, host, history depth and input fixed
for comparisons; a development-profile smoke measurement is not a release
latency or performance-improvement claim. Native GUI resize/pane/tab gestures,
exact controlled pixels and accessibility checks remain separate gates.

`cargo bench -p rio-vt --bench vt_input --locked -- grid_seam_roundtrip_checked`
measures the native-policy seam round trip plus visible snapshots. Every iteration
checks exact copied text and bounded history outside the timed interval; a
corrupt faster run must fail. This is grid/snapshot timing, not WSL or GUI latency.

`cargo bench -p rio-vt --bench vt_input --locked -- grid_table_roundtrip_checked`
checks exact copied table text, snapshot dimensions and bounded history on every
iteration of a retained terminal. Those independent checks are outside the timed
grid/snapshot region. It is a correctness-guarded core benchmark, not native PTY
or compositor latency. Never report a crash-free timing loop as output fidelity.

`cargo bench -p rio-vt --bench vt_input --locked -- pane_close_prompt_repaint_snapshot`
measures a complete height shrink/grow, fragmented prompt repaint and visible
snapshot with cursor assertions and one retained terminal. It is core processing
cost, not shell input latency; the Windows post-resize compatibility interval is
an additional explicitly bounded 50 ms, not part of the parser benchmark.

`cargo test -p rio-vt --locked --lib resize_viewport` checks content identity,
not merely legal scroll offsets, through parser-created reflow. Unique input
colors identify repeated text; the first visible row and renderer-facing
snapshot must retain the anchor through narrow/wide and height transitions.
Cover selected content, unchanged live-follow siblings, no PTY input, inactive
normal buffers behind alternate screens, blank hard lines and actual history
eviction/absorption. Keep the original failing 120x9 reproduction. Compare the
scrolled `selection_resize_copy_snapshot_10k_*` Criterion case against a saved
same-host baseline; this remains core-path evidence, not live PTY redraw,
native desktop pixels or assistive-technology evidence.

The optional component host requires explicit all-feature testing:
`cargo test -p automexia-ecosystem-runtime --all-features --locked`.
Its cancellation regression follows a real all-feature readiness hang. Cover
interrupt-before-store-arming, shared-engine isolation, token reuse, worker
unwind and joined watchdog cleanup, in addition to deterministic fuel and
memory/table ceilings and interruption during component start functions.
Guard-removal mutations must fail for the expected runtime reason, not compilation
or timeout; restore the source before running the normal gates.
A default-feature workspace pass does not run that host.
Full QA now runs a separate required `component-host` nextest stage with all
features and a 15-minute outer ceiling, in addition to its per-test deadlines.
The no-retry default profile preserves the workspace CI JUnit instead of replacing
it with a package-only report. A CLI test verifies that this stage alone failing
fails QA; removing it or dropping its feature/profile flags fails that test.
Keep its activation-denied policy and the original failed run visible; see
[the interruption contract](adr/0044-invocation-local-component-interruption.md).

QA report and bundle announcements use repository-relative logical locations,
not the contributor's absolute checkout. A CLI regression exercises success and
failure results with real report/bundle writes and isolated external commands.

`cargo test -p rio-vt --locked --lib resize_selection` exercises selection made
once from parsed output across intermediate resize/scroll states, including
direction/edge retention, Unicode, cropping and eviction. The original regression
and the follow-up Vi-cursor/next-arrow regression both failed before their fixes.
The original selection regression
failed because selection became `None` on the first width change. The
`selection_resize_copy_snapshot_10k_*` cases in the existing `vt_input` benchmark
measure the combined core path with and without selection over 10,000 history
rows. Neither is native mouse, ConPTY repaint, renderer or accessibility proof.

On Windows with the existing `pty` feature,
`cargo test -p rio-vt --locked --test conformance_fixtures native_powershell_output_retains_selection_after_exit_and_reflow`
launches the fixed `rio-vt/tests/fixtures/selection-output.ps1` through ConPTY.
The child disables profiles and explicitly uses UTF-8; output is capped at
64 KiB with a 20-second deadline. Successful child exit and parsed completion
are independent requirements. Subsequent resizing checks retained output, not
live shell repaint, native pointer selection or window pixels.

`cargo test -p rio-vt --locked resize_stress_retained_hard_lines_spaces_and_styles_match_input_journal`
feeds fragmented VT/UTF-8 bytes into the parser, then checks 96 width/height
transitions plus top/bottom scrolling against exact expected logical lines and
foreground colors. It checks every intermediate state before any new PTY input.
Four intentional faults prove that the oracle rejects content, hard-break,
whitespace and style changes. This is parser/grid assurance, not a native
reproduction of window output loss or proof of selection/pixel correctness.

Full QA records `automexia-qa-content-v1` source identities before and after
execution. These include raw Git status and the bytes of changed/untracked
files, not just their names. A changed commit/content identity, unavailable
inventory, unsafe path or exceeded limit fails the required source-identity
gate even if every test command passes. Bounds: 1 MiB status, 8,192 paths,
16 MiB per changed file, 64 MiB total and a 20-second Git-status deadline.
The report contains digests, never source contents. Before/after identity is
not a substitute for an isolated runner and cannot prove absence of transient
edits that are completely reverted during a run. Do not edit the tested tree
while QA runs; repeat the affected validation after any late source change.

Release-metadata regressions run with `python tools/ci/test_public_distribution.py`
and `python tools/ci/test_release_trust.py`. The public tests invoke the real
preparation CLI and bundle verifier, recompute matching checksums around privacy
canaries, compare complete graph/hash/license preservation, exercise structural
limits and interrupted writes, and mutate scanner upload flags and signing
order. Synthetic metadata cannot establish fresh native package, Syft execution,
signing or hosted publication evidence. Keep those gates separate.

The current-source paste regression is
`cargo test -p automexia-terminal --bin automexia --locked context::paste::`.
It feeds bracketed-paste mode through the VT parser and checks actual channel
bytes, focus changes, terminal replacement/shutdown, empty/oversized input and
selection preservation. Pair it with application/layout/mouse tests for pointer
ownership. This does not test a native clipboard, physical mouse, IME or pixels;
manual native validation must include both panes, active selection, Shift mouse
override, overlays, and a clipboard read that fails or delays.

Automexia uses layered evidence for its public free-terminal behavior. This page
does not document or authorize unreleased advanced features or commercial
products.

A test proves only the exact behavior, artifact, platform, and revision it ran.
Cross-compilation is not native runtime evidence, and a retry does not erase the
first failure.

The provider-transient lifecycle regression retains all 64 cycles and 16 files
per cycle. Captured numeric operation checkpoints identify a stalled open,
publish, disable or shutdown phase without logging private paths or content.
It independently checks live file counts, empty roots after disable, and root
removal across repeated shutdown. A timeout is a failure even if later isolated
runs pass; retain its log and investigate the identified native filesystem phase.

## Fast documentation and policy gate

For documentation-only changes, run the repository's documentation hygiene,
coverage, link, confidentiality, and alignment checks, followed by:

```text
git diff --check
python tools/ci/validate_repository.py
```

Review the complete diff and verify that only documentation files authorized by
the task changed. Scan changed and untracked documentation for credentials,
machine-local paths, hostnames, account identifiers, and private product plans.

## Complete local gate

Compiler policy and its mutations run through repository validation and the
Python suite. Focused commands are `python tools/ci/rust_toolchain.py check-policy`
and `python tools/ci/test_rust_toolchain.py`. To verify a local compiler receipt,
set process-local `RUSTUP_TOOLCHAIN` to the repository pin, then run
`python tools/ci/rust_toolchain.py verify --kind development`. The `msrv` kind
instead verifies the workspace minimum. CI additionally runs the explicit
locked all-target/all-feature MSRV check. Neither check changes the global
rustup default. Keep exact-commit hosted results separate from local passes.
Compiler workflow probes require the explicit Python 3.12 bootstrap, not the
runner's default Python. `python tools/ci/test_free_security_tools.py --tool gitleaks`
also validates exact historical exemptions and detection of fresh findings in
those same repository paths.

For changes that affect Rust code or build behavior, start with:

```text
cargo fmt --all --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
python tools/ci/validate_repository.py
git diff --check
```

Use the narrower package and test owner first, then expand to the workspace.
The contributor command `cargo ready` is the final local gate when applicable.
Use the complete CI-equivalent command only when the change or policy requires
it.

GitHub's free hosted `CI` workflow is the automatic push and pull-request
authority. The local pre-push hook is intentionally dormant and exits without
running assurance. Invoke `cargo xtask assurance pre-push` manually when a
local profile is desired.

## Build cache and storage evidence

Cache and build-lifecycle changes must test exact path containment, link and
reparse-point rejection, zero/boundary/over-limit inventory, content identity,
corrupt and partial toolsets, atomic publication, concurrent leases, live and
dead process owners, current and dirty worktrees, grace periods, dry-run versus
apply, failure cleanup, and storage de-duplication. The independent oracles are
the generated storage tree and file digests, not cache self-reporting.

Run the focused owners first:

```text
python tools/ci/test_dev_cache.py
python tools/ci/test_github_free_assurance.py
python tools/ci/test_qa.py
cargo xtask cache status
cargo xtask cache gc --scope automatic --grace-hours 72
```

The cleanup command above is a dry run. Use `--apply` only after reviewing the
exact candidates. The current target, dirty worktrees, active leases, live
processes, required toolsets, recent entries, links, and broad paths must remain
protected. See [Development cache and build storage](DEVELOPMENT-CACHE.md).

## Evidence ladder

1. Unit tests for pure parsing, state, policy, layout, and limits.
2. Integration tests for crate, PTY, shell, persistence, and platform boundaries.
3. Property tests for invariants and hostile structured input.
4. Fuzzing for parsers, protocols, archives, configuration, and terminal input.
5. Deterministic concurrency tests for ordering, cancellation, saturation,
   stale generations, restart, and shutdown.
6. Renderer-neutral geometry, hierarchy, focus, semantic, and clipping tests.
7. Exact controlled raster comparisons for visible behavior.
8. Native workflow and accessibility evidence on each claimed platform.
9. Performance, resource, storage-growth, and cleanup measurements.
10. Packaged-artifact, upgrade, rollback, and uninstall verification.

A narrow pass never overrides a known real-workflow failure.

## Reported terminal regressions

Command discovery regression tests live beside the palette in
`apps/automexia-terminal/src/renderer/command_palette/navigation_tests.rs`.
They include persistent header Back while scrolled/searching, font/extension
parent return, compact/HiDPI geometry and independent legacy-score equivalence.
Run `cargo test -p automexia-terminal --locked renderer::command_palette::`
and `cargo test -p automexia-terminal --locked bindings::` for categorized
coverage, global query retention, Back/paging, pointer geometry, bounded wheel
state, non-executable navigation, all-platform chord tables and profile overrides.
Actual-configuration tests also disable splits, add custom chords, reject retired
punctuation, apply typed tombstones and replace the profile. Displayed labels for
every catalog action must match effective bindings, not a static platform guess.
Regressions cover missing Linux shortcuts, hardware Copy/Paste key precedence,
non-pane overrides, typed search direction/scope, mode/chain shadowing, repeated reload storage bounds and
independent left-arrow coordinates. Coordinate checks do not certify GPU pixels.

The repository gate in `tools/xtask/src/main.rs` additionally rejects missing
chords or mode guards in either Windows or Unix tables and restored punctuation
aliases. Its mutation test complements runtime table tests. Regenerate only
through `cargo xtask generate keybindings --version 1.3.1` when Automexia defaults
change; review the classic inventory and manifest diff and retain pinned bindings.

The Windows model benchmark measured browse/search/Back at 52.142 microseconds
(30 samples; no statistically detected change from the prior baseline).
Effective-label reload measured 112.53 microseconds and runs on configuration
changes, not per key/frame. Repeated reload tests retain stable label capacity.
Reproduce with `cargo test -p automexia-terminal --bin automexia --locked benchmark_palette_browse_search_and_back -- --ignored --nocapture`.
These are model timings, not physical keyboard or desktop-frame latency.
The native test snapshot adds a privacy-safe `palette_accessibility_summary`;
it is evidence plumbing, not a platform screen-reader implementation.

For native review, wait for application readiness, open the palette, visit all
six categories by keyboard and mouse, return by Back/Alt+Left, search from inside
a category, clear the query and verify restoration. Hold Enter during category
entry and verify no command runs. Resize and scale before pointer activation;
verify the visible row is the activated row, the scrollbar remains discoverable,
and the prior pane retains input focus after Escape. Test split/clone chords
with two different profiles/directories, verify independent child sessions, then
check shell Ctrl+R/D and explicit overrides. Capture controlled CPU/GPU frames
and current Narrator/NVDA, VoiceOver or Orca events on each claimed platform.
Source-model tests cannot replace those native or exact-pixel checks.

For pane-close regression coverage, run
`cargo test -p rio-vt --features pty --locked performer::workers::` and, on
Windows, `cargo test -p automexia-terminal --locked native_context_close_does_not_wait_for_conpty_grace -- --nocapture`.
The latter opens real idle CMD and PowerShell ConPTYs, captures exact synchronization
handle before close, checks Context removal below a conservative 250 ms guard,
then separately requires worker join and actual child exit. It is not a native
window-present latency measurement. Deterministic gates cover unfinished workers,
native TLS destruction, panic, capacity recovery and repeated cleanup. Preserve
the initial failing evidence; never infer child exit from bookkeeping alone.
On each desktop, also test WSL or native Unix shells, split/local-tab
close under output and resize, sibling responsiveness, late exits, and application
quit. Retain native process-tree and frame evidence before claiming that platform.

### Window dismissal and saturated native shutdown

On Windows, run `cargo test -p teletypewriter --locked --lib -- --nocapture`
and `cargo test -p teletypewriter --locked --test pty_lifecycle`.
The saturated native-pipe regression waits for the real 64 KiB ring to fill,
retires its consumer twice, and requires a producer completion acknowledgement
within 500 ms. Its failing baseline restores consumption before asserting, so
the test does not leak a blocked writer. The final-tail regression waits for
native EOF before reading, compares exact retained bytes through 1/7/14/64-byte
reads, and checks empty reads between chunks.

The real ConPTY regression runs a bounded profile-free PowerShell output
fixture, observes ring saturation, then exercises explicit shutdown and fallback
drop twice each. It captures the exact process handle before close and verifies
that it is signalled; explicit shutdown also checks the Job is empty. The
existing native grace/force and ten-second cleanup ceilings are unchanged.
Run `cargo bench -p teletypewriter --bench pty_io --locked` for native startup,
1 MiB output and clean-exit measurements with 30 samples per group. Compare on
the same host; these are process/pipe measurements, not native window-dismissal
latency.

`cargo nextest run -p teletypewriter --test conpty_handle_lifetime --locked`
runs a Windows-only, isolated-process resource regression. It launches the real
PowerShell fixture, resizes, waits for successful exit and fully joins teardown.
A bounded three-cycle stable handle count establishes native readiness; each
of twelve subsequent cycles must recover the exact baseline. Four missing-child
launches must also recover every handle. The test belongs to the serialized
native PTY group. This detects leaks on both successful and failed attachment,
not just slow visible window dismissal.

`cargo test -p rio-vt --lib --locked performer::tests::resize_worker`
includes a deterministic final-output race: hold the real terminal lock, deliver
literal final bytes, reach EOF, then release the lock. It checks exact parsed
text for zero reads, would-block and the compiled platform's native EOF, and
preserves unrelated error propagation. This is worker/lock evidence, separate
from native editor final-output/exit and live-resize tests.

The controlled `tests/integration/resize-stress-windows.ps1` harness separately
requires secondary and final windows to stop being visible within 500 ms while
retaining its process-tree exit ceiling. The contract checker and mutation suite
reject missing/reordered dismissal, a shared-service join before Quit dispatch,
missing saturation wakeups, EOF before pending bytes, or relaxed thresholds.
Source ordering and process tests do not replace running that desktop harness.
On each native desktop, check single/multiple windows, active/background panes,
confirmation Cancel/Close, explicit Quit, output storms and sibling focus.
Linux/macOS compositor timing, WSL teardown and assistive-technology delivery
remain separate platform evidence until those exact scenarios run.

Run the explicit Criterion model benchmark with
`cargo test -p automexia-terminal --locked benchmark_palette_browse_search_and_back -- --ignored --nocapture`.
It measures the actual palette browse/search/Back path with fixed public input;
reports are under `target/qa/palette-navigation-01/criterion`. This development
model measurement is not native rendering/input latency or a speedup claim.

[Terminal maintenance status](TERMINAL-MAINTENANCE-REQUIREMENTS.md) identifies
existing owners and known verification limits for reflow, rendering, input,
process exit, Unicode and graphics.
[Terminal interaction status](TERMINAL-INTERACTION-REQUIREMENTS.md) records
current palette, shortcut, preference, preview and accessibility ownership.

Treat these as regression inventories, not passing evidence. Reproduce the
reported path at the exact revision, retain its first failure, extend existing
tests, and update the matching feature-test reinforcement entries when code
changes. Source inspection alone cannot close a native workflow failure.
These documents introduce no new test commands or relaxed quality thresholds.

## Native platform ownership

Windows, Linux/BSD, and macOS claims require native runs on the named platform
for PTY/process behavior, windowing, input, clipboard, IME, accessibility,
packaging, and credential integration as applicable. Cross-compilation and
mock adapters are supporting evidence only. Unrun platforms remain explicit
external gates and are never reported as passing.

## Terminal conformance

Terminal tests cover:

- CSI, OSC, DCS, APC, hyperlinks, titles, and alternate-screen behavior;
- UTF-8, Unicode widths, combining marks, emoji, CJK, bidi, and controls;
- zero, boundary, maximum, oversized, fragmented, malformed, and repeated input;
- scrollback, reflow, selection, search, cursor, and viewport state;
- Sixel, Kitty, and iTerm2 image limits and lifecycle; and
- parser recovery after invalid or interrupted sequences.

Independent oracles compare exact bytes, cells, cursor state, modes, visible
rows, semantic events, and final images where appropriate.

## PTY and process lifecycle

PTY tests exercise real supported adapters with exact executable and argument
arrays. They cover launch, ordered input, resize storms, output storms,
interrupt, EOF, exit status, cancellation, close, child trees, and application
shutdown.

Assertions include forbidden side effects: no shell evaluation for structured
actions, no implicit Enter, no cross-session input, no stale publication, no
orphan child, and no leaked handle or worker.

Application and window teardown tests broadcast an idempotent shutdown request
to every active, background, split, pane-tab, and parked PTY before destructors
join workers. Native many-session tests record exact temporary-fixture process
identities before close, because a pseudoterminal host may reparent descendants;
parent-count sampling alone is insufficient. The owner and every recorded
identity must exit within the declared wall-clock ceiling. Repeated broadcasts
must not duplicate shutdown messages or extend that ceiling.

Run `cargo test -p automexia-terminal --bin automexia --locked parked_` for
parked-session exit and restoration regressions. These tests belong to the
desktop binary, not its services library; a zero-match run is not evidence.
The journal covers six mixed split/local-tab exit orders over repeated cycles,
exact shutdown recipients, connected silent survivors, weak-owner cleanup,
unknown and late exits, independent managers, undo/redo and alternating window
sizes/scales. Final-pane geometry includes configured scaled panel margins.
Invalid geometry must preserve undo and foreground selection. Zoom/unzoom
regressions require both layouts to retain the new extent and DPI. The restore
wrapper refreshes cell metrics and PTYs before visibility; model checks alone
do not prove native presentation, shell continuity or process-tree cleanup.

Unix PTY results do not prove ConPTY behavior, and Windows results do not prove
Unix process-group cleanup. Native claims name the operating system and
architecture that actually ran.

## Windows, tabs, panes, and input

Model and integration tests cover independent windows and PTYs, global tabs,
pane-local tabs, fresh and cloned splits, route isolation, focus movement,
divider resize, pointer routing, selection, clipboard, IME, search scope,
command navigation, and modal isolation.

Command-result navigation tests must resize before using Ctrl+Shift+Up/Down and
inspect every badge draw, not only the latest selected result. Raw shell bytes
must traverse VT lifecycle, reflow, visible snapshot, navigation, projection,
and drawing; result IDs must be unique, label rectangles pairwise disjoint, the
prior output boundary must own a shared prompt row, and prompt/route/PTY input
state must remain unchanged. Compare every badge rectangle with every co-located
prompt-context chip under the shared right reservation. Controlled rasters must
freeze both completion datetime and duration before the zero-tolerance diff.
The combined reflow-navigation-snapshot benchmark guards the terminal-owned
cost; native backend frames remain a separate visual gate.

Test every transition with one and multiple panes/tabs, rapid closure, stale
route IDs, shutdown, tiny through high-resolution viewports, and keyboard-only
operation.

## Configuration and persistence

Persistence tests cover:

- canonical round trip and supported predecessor formats;
- missing, corrupt, truncated, duplicate, oversized, and unknown input;
- read-only, permission-denied, disk-full, and interrupted writes;
- temporary-file creation, flush, atomic replacement, and last-known-good
  recovery;
- concurrent readers/writers and compare-and-swap where used;
- private permissions and link/replacement checks;
- migration, rollback, disable, uninstall, and restart; and
- storage-tree and digest comparison before and after cleanup.

Fixtures use fictional stable values or isolated temporary paths. Never snapshot
the real user profile, host, environment, Git identity, or shell history.

## Explicit local tools

Windows process regressions retain a live native process handle before cancellation
or before releasing an acknowledged parent to exit. They assert that same handle
is signaled immediately after capture; post-cleanup PID lookup is not an exact
identity oracle. The descendant regression runs four concurrent workers with
25 cycles each, both inherited-pipe and closed-pipe children. Failure-only later
observations remain diagnostic and never convert a failed assertion into a pass.

Directory handoff tests use `cargo test -p automexia-terminal --lib --locked amx_open_`.
Editor tests use `cargo test -p automexia-terminal --lib --locked amx_edit_`.
Run `amx_edit_benchmark_checked_uri_encoding` explicitly with `-- --ignored --nocapture`;
its checked URI timing excludes metadata, process and desktop latency. The real
post-build CLI tests cover configuration, disable/override, exact paths and
positions without launches or writes. Ten scenarios run through each available
shipped native shell. The guest suite also checks file symlinks and import isolation.
On an installed native desktop, manually open a disposable file at a known
line/column in both supported editions, check permission/UNC prompts, missing
registration and focus/close behavior. Do not mark preview-only execution as
actual editor-window evidence. Native Linux/macOS Rust and desktop checks must
be recorded separately from Windows-backed WSL. Preferences remain untouched
outside isolated fixtures; do not run this test using a real user configuration.
Windows local-tool cleanup additionally verifies empty job accounting and native
signals from bounded member handles pinned before termination. The process suite
includes 100 concurrent descendant cycles and an isolated 20-cycle exact handle-
recovery measurement. A later diagnostic wait does not erase an immediate failure;
leader exit and pipe EOF alone are insufficient. No native Unix equivalence or
interactive performance is inferred from these Windows fixtures.
Run the ignored `amx_open_benchmark_checked_guest_mapping` test explicitly with
`-- --ignored --nocapture` for checked pure conversion timing, not desktop latency.
The post-build native CLI harness verifies exact directory previews and no writes;
the guest runner additionally proves POSIX symlinks and isolated Python imports.
Neither opens a file manager. Manually check a disposable directory through the
real installed desktop association on every claimed platform, then close it.
Do not substitute Windows-backed WSL previews for native Linux Rust or macOS
execution, native desktop visibility, accessibility or handler-failure evidence.

```text
cargo test -p automexia-terminal --lib --locked amx_local_ -- --nocapture
cargo test -p automexia-terminal --lib --locked amx_process_ -- --nocapture
cargo test -p automexia-terminal --lib --locked amx_local_benchmark_checked_parsing -- --ignored --nocapture
python tools/ci/test_amx_guest_bridge.py
python tools/ci/check_amx_guest_native.py --binary target/debug/automexia.exe
```

Run the guest entrypoint inside the selected WSL distribution, supplying its
translated path to the built Windows executable. It requires installed Python
and ripgrep; missing prerequisites fail rather than becoming passing skips.
The tests cover exact guest process identities, EOF/cancellation/deadlines,
descendant retirement, default ignore/privacy rules and hostile Python import
paths. Tealdeer argument/cache policy uses an isolated client fixture; a real
installed tealdeer cache remains a separate native check.

The library tests exercise stream ceilings, malformed records, redacted errors,
pre-cancellation and actual native child cleanup. The ignored child fixture is
invoked only by its parent tests; run the benchmark by its exact name and require
one executed test. Its 500-file/500-match parsing workload checks output on each
iteration and measures parsing, not process, disk, shell or desktop latency.
Windows console cancellation and WSL supervision do not certify the native Unix
Rust adapter. Complete that adapter's lifecycle tests on each claimed native OS.

## Shell integration

The Google command's post-build smoke is part of `cargo ready`. To run it
separately after building the application, use
`python tools/ci/check_google_command_native.py --binary target/debug/automexia`
(append `.exe` on Windows). It prints the shells actually executed, checks the
real CLI's offline preview and lack of config writes, and sources each native
adapter twice with normal, collision, disabled and missing-executable cases.
All child runs are bounded; queries are fictional and no browser is opened.
Windows/WSL checks do not establish macOS or native Linux desktop behavior.
Interactive CMD DOSKEY, browser association and actual page loading remain
native manual gates; test quoted spaces and `&`, disabled integration, a prior
`amx` macro and a missing default browser without entering private data.

Run focused contracts with
`cargo test -p automexia-terminal --lib google_command_` and the correctness-
checked microbenchmark with
`cargo test -p automexia-terminal --lib google_command_benchmark_checked_encoding -- --ignored --nocapture`.
The benchmark checks exact encoded bytes on every operation; it measures URL
construction only, not shell startup, browser startup, PTY or network latency.
The reinforcement checker/mutation suite guards limits, encoding, privacy,
native capture, shared launch ownership and post-build readiness dispatch.

Supported-shell tests verify session-local provisioning, prompt boundaries,
status/duration/path/Git metadata, object-preserving listings, disable/remove
behavior, missing-resource fallback, quoting, Unicode, and startup cleanup.

The PowerShell test wrapper captures the real shell-identity stream from an
isolated child, asserts its user and path fields internally, and then discards
the stream so machine identity values do not enter local or hosted logs. CMD
identity fixtures use stable fictional values instead of live account or
executable-path data.

Repository-owned readiness status lines identify completion state, isolated
Cargo targets, and the debug smoke executable with stable logical labels rather
than contributor-specific managed-state or workspace paths.

The summarized workspace-test process is owned by a Unix process group or
Windows Job Object. It has a 30-minute deadline and a 16 MiB stdout ceiling;
exceeding either terminates the owned tree and retains only bounded failure
diagnostics. Compiler stderr remains live. The xtask tests use a real child to
prove success, pre-spawn zero-bound rejection, deadline cleanup, and output
overflow cleanup without relying on arbitrary sleeps as the result oracle.

The native shell remains the independent oracle for command editing, history,
completion, quoting, and pipeline objects.

The ConPTY history regression uses an isolated, non-persistent PSReadLine
history fixture and explicit Emacs mode. It waits for a fixture-owned prompt,
exercises Up Arrow, Ctrl+R, cancellation and Ctrl+D on an empty line, asserts
child exit and absence of a history file, and bounds captured output to 1 MiB.
Raw shell output is never included in its failure diagnostics. This native
adapter test does not substitute for graphical shortcut routing or other
shells' editing modes.

## Images

Image tests bound input bytes, dimensions, decoded pixels, frame counts, cache
size, and lifetime. Local preview tests cover regular files, links, replacement,
permissions, unsupported formats, malformed data, cancellation, stale results,
and cleanup.

Protocol images are tested through parser state, terminal state, visible
viewport, renderer data, and exact controlled pixels.

## OpenSSH interoperability and inventory

Public SSH tests use an authorized loopback fixture or ordinary manual system
OpenSSH. They verify that terminal input, paste, resize, search, selection, exit,
and cleanup behave like local sessions while OpenSSH keeps credential and
network ownership.

Inventory tests use only explicitly selected synthetic configuration files.
They cover includes, duplicates, invalid syntax, oversized files, links,
replacement, revocation, public metadata, last-known-good recovery, and absence
of credential persistence or passive network work.

## Extension contract runtime

Public extension-maintenance tests verify version negotiation, strict bounded
messages, capability denial, route/session isolation, cancellation, queue
saturation, stale generation rejection, disable, uninstall, worker restart, and
shutdown.

Optional code cannot obtain ambient filesystem, network, credential, process,
terminal-history, clipboard, or PTY authority. Extension failure must not break
the basic terminal.

## Accessibility and visual assurance

Visible changes require three separate layers:

1. renderer-neutral geometry, hierarchy, focus, relationships, clipping,
   z-order, contrast, reduced motion, and semantic assertions;
2. deterministic controlled raster goldens with exact comparison; and
3. native frames plus accessibility tree/event evidence on each claimed
   platform.

Native visual readiness must be coupled to a successfully presented matching
frame. A control consumed after draw-data construction is exercised on the next
forced frame; a dropped or skipped frame publishes no checkpoint. Before a
retained desktop capture, the driver establishes foreground ownership and
rejects detectable native dialog occlusion, positions the complete physical
client on the capture display, and rejects non-opaque client pixels. It then
requires two consecutive identical full-frame pixel digests inside a bounded
deadline. Controlled renderer comparisons hold a fixed unsent editor sentinel
and require frame-skip identity to include the physical surface extent; only a
successfully presented frame may be cached. Backend-to-backend equality is
evaluated only after each frame independently passes the automated checks.
Manual and independent visual review remains a separate release gate.

Cover small through high-resolution viewports, 100–300% scale,
light/dark/high-contrast themes, long and localized text, Unicode, IME, empty
and error states, modal stacking, keyboard and pointer input, focus restoration,
and motion enabled/disabled.

Automated accessibility checks do not replace current Narrator/NVDA, VoiceOver,
and Orca evidence for releases claiming those environments.

## Performance and resources

Measure the real owning path with same-host baselines and noise-aware
thresholds. Record latency distributions, throughput, allocations, CPU, memory,
GPU memory when observable, handles, threads, child processes, queues, caches,
storage growth, cancellation latency, long-session stability, and final cleanup.

Interactive benchmarks cover input-to-publication latency, PTY throughput,
resize coalescing, parser/state updates, snapshot publication, renderer enqueue,
search, large scrollback, image pressure, multiple panes, and shutdown.

## Fuzzing and property testing

Every parser and hostile structured-input boundary has a maintained corpus and
bounded fuzz target. Record engine, seed, corpus digest, duration, executions,
sanitizer, target, platform, and replayed crashes. A smoke run proves only that
campaign.

Property tests use independent invariants or reference behavior. Historical
failures stay in the seed corpus.

## Checker assurance

Run `python tools/ci/test_free_plan_contract.py` for compiler-selection and
free-plan workflow mutations. Its literal mutations reject every stable
workflow's drift, missing/nested/duplicate selector, global-default changes,
legacy override files, invalid UTF-8/TOML and the canonical pin's 16 KiB limit.
Both quality caches must derive their generation from the selected compiler;
`python tools/ci/test_public_distribution.py` additionally protects the Linux
release cache and cold-package boundary. Local checker passes are not evidence
that GitHub ran the changed workflows or that a Linux package works natively.

The explicitly invoked `cargo xtask assurance pre-push` profile includes the
`workflow-static-analysis` step for Actionlint and ShellCheck validation;
automatic local push hooks remain disabled. String-based contract mutations do not validate GitHub
expression context availability. On native Linux, the compiler suite also
executes the real cache initializer from each workflow with two fixture pins,
checks exact environment-file bytes and preserves an existing sentinel. On
other hosts that Bash-runner test is explicitly skipped, not reported as native
Linux evidence. Mutations reject missing, duplicate, commented, late, hard-coded
or destructive environment initialization and the invalid job-level expression.

S1 evidence fixtures use an explicit UTC test clock. Their positive baseline
must pass before a mutation is applied, so an expired fixture cannot hide the
missing check. `python tools/ci/test_s1_assurance.py` covers exact freshness and
future-skew boundaries, review ordering, and live-clock rejection of expired
evidence. Fixture clocks do not relax production release freshness or turn
synthetic results into native evidence.

Connection persistence regressions live beside the application stores and the
private `connections/persistence_support.rs` byte writer. Run
`cargo test -p automexia-terminal --locked --lib automexia::connections::`
and `cargo test -p automexia-terminal --locked --test connection_library`.
Repeat the library tests with `--no-default-features` and include the normal
all-feature CI gate. Literal JSON fixtures preserve byte/escaping/validation
contracts; write-sequence properties verify accepted-prefix integrity, exact
byte ceilings and capped capacity. The real JSON regression serializes a
half-budget-plus-one string followed by its closing quote, reproducing the
previous implicit capacity doubling. These checks do not establish allocator
RSS, power-loss durability, or unexecuted native filesystem behavior. SSH keeps
its extension-local writer; run
`cargo test -p automexia-devops-ssh --locked --lib bounded_json_writer`
for its literal JSON, UTF-8, all-or-nothing and capacity regressions, plus the
full SSH persistence suite for native recovery. The large-write SSH case tests
the writer contract, not a single field accepted by its metadata schema.

The repository, feature-assurance and reinforcement checkers share
`tools/ci/markdown_anchors.py`. Run `python tools/ci/test_markdown_anchors.py`
and both feature checker mutation suites when changing that parser. Literal
expected sets and real missing-reference failures are independent of the shared
implementation. Fenced examples do not supply heading anchors; generated
suffixes cannot collide with literal heading names. Heading slug spelling stays
compatible with the repository's ATX subset, not full Markdown-renderer
conformance (custom HTML anchors and link destinations are not normalized).
The real-validator fixture also counts one parse per referenced target and
rejects stale anchors after a subsequent invocation sees changed bytes.
Standalone execution and unittest discovery
remain supported; path scope and evidence policy stay with each checker.

Repository checkers are assurance code. Their mutation tests prove that missing
owners, deleted scenarios, weakened limits, reordered lifecycle steps, stale
paths, relaxed thresholds, fabricated evidence, and accidental private-data
publication fail closed.

A checker must validate semantics, not merely file existence or self-reported
counts.

## Packaging and release

Package verification covers identity, version, license notices, checksums, SBOM,
provenance, signatures, install, launch, upgrade, rollback, uninstall, startup
hooks, user-data retention, and final cleanup.

Controlled signing, notarization, store accounts, protected runners, hardware,
native assistive technology, and long-duration campaigns remain explicit
external gates when unavailable locally.

## Manual verification

Use [the public manual feature-testing guide](MANUAL-FEATURE-TESTING.md).
Record exact revision/package, environment, scenarios, first failure, redacted
evidence, cleanup, and reviewer. Do not convert a skipped or unavailable
scenario into a pass.

## Shared CPU raster contracts

Run `cargo test -p sugarloaf --locked --lib grid::cpu::tests` under default,
minimal and all-feature configurations. Independent literal pixels cover
transparent/opaque fast paths, channel order, partial alpha, repeated blending,
mask/color atlas bytes, every clipping edge/corner, empty/offscreen dimensions,
atlas offsets and boundary clipping, and untouched destination suffixes.
Actual-owner tests insert glyphs through the CPU atlas, write grid rows, paint
cursor/content/non-block-cursor order, apply cursor colors, and clear rows.
Repeated warmed updates and painting retain vector/atlas storage and glyph
identity; this is storage evidence, not global allocator instrumentation.

The existing `cargo bench -p sugarloaf --locked --bench ui_text` target includes
`cpu_grid/paint_cached_frame` and `cpu_grid/write_rows_and_paint`. They paint a
prepared 80-by-24 grid with mixed mask/color glyphs, boundary clipping and a
cursor. Atlas insertion and initial allocation are outside timing. Preserve a
same-host baseline, check nonempty raster output and do not overlap builds.
These are CPU pixel-buffer paths, not native window, GPU or accessibility tests.

## Shared text contracts

Run `cargo test -p automexia-terminal --locked --bin automexia renderer::responsive`
for the responsive fitting consumer's independent whitespace, marker-budget,
borrowed-storage, cluster-cut and actual rounded-font-width regressions. Run
the same command with renderer::text_fit for literal Unicode boundaries,
invalid geometry/measurements, source-view and probe limits, non-monotonic
widths, marker identity and exact fresh-owner CPU raster comparisons across
prepared regular/bold/italic faces, fractional sizes and scales. Tests pin the
literal resource ceilings independently of the implementation constants.
Literal probe traces preserve candidate order and retained-source identity during
allocation changes. Storage checks cover inline/spilled offsets, reused candidate
bytes, bounded amortized capacity growth and rebuilding a winner after overshoot.
These prove buffer contracts, not allocator call counts or total process memory.
Measured advance does not certify arbitrary glyph ink clipping or native UI.
`cargo bench -p automexia-renderer-benchmarks --locked --bench text_fit`
directly includes the private renderer helper rather than copying its algorithm
or adding a public runtime API for measurement. Its fixed-width oracle isolates
helper cost; its prepared-font case separately measures fitting, drawing and CPU
rasterization. Neither establishes native interactive latency. Save a same-host
baseline and do not overlap it with builds. The unpublished benchmark-only
package avoids Cargo's automatic application-binary build for benchmarks; it
does not change release profiles or expose private helpers as a product API.
Its development dependencies preserve the application's Windows/wasm WGPU
requirement; use --features wgpu for optional WGPU configurations elsewhere.
Architecture mutation tests reject runtime/build dependencies, product targets,
publication and inverse dependencies. The canonical performance inventory rejects
a missing or reclassified benchmark even if the aggregate target count agrees.
See [ADR 0047](adr/0047-private-renderer-benchmark-boundary.md).
Start a fresh baseline for this package. A preserved application-package
harness can have different transitive compilation fingerprints despite matching
helper source and release profile. Retain cross-package results as diagnostics,
not as a controlled product performance comparison.

Run the application binary tests with the `renderer::suggest` filter for
suggestion layout and label characterization. Literal cuts preserve whitespace
and ASCII-dot marker budgeting; literal highlighted/normal run draws provide
an independent exact CPU-frame oracle across prepared-font sizes and scales.
Generated-marker regressions reproduce removed-source highlight leakage and
reject nonfinite budgets without altering previously queued pixels. Additional
cases cover Prepend/control boundaries, literal source dots, borrowed contiguous
runs, the real-font retained-source ceiling and exact mixed-style fitting.
The literal pixel oracle draws separate known runs rather than calling the
fitter or partitioner under test. Full source values remain unchanged.
The same benchmark target directly
includes the private suggestion drawing owner and measures a matched label
plus description and CPU rasterization. Compare against the same-package
legacy baseline; these measurements do not certify ink clipping or native UI.

`cargo test -p sugarloaf --locked --lib text::` exercises bundled-font
measure/draw transitions against fresh renderer instances, including exact
CPU pixels after adjacent sizes and scale changes. Finalize the frame before
checking its base raster and reject an empty fixture. Cache tests force digest
collisions, exact entry/payload ceilings, one-byte-over rejection, spare vector
capacity, FIFO eviction, immutable hit storage and final release. Repeat under
minimal and all-feature graphs. Font-replacement cases alternate bundled
regular, italic and variable-weight faces across scales, compare complete CPU
rasters and widths with fresh owners, release stale runs and font caches, and
retain atlas allocations without initializing unused backends. The visible
fixture must differ before claiming a reload was tested. These tests do not
establish native window, GPU, display-server or screen-reader correctness;
interactive configuration reload on each claimed platform remains a separate
native scenario.

`cargo bench -p sugarloaf --locked --bench ui_text` measures real cached
measurement and draw-plus-CPU-raster paths with the bundled font. Save a
same-host baseline and compare without overlapping builds; it is not a GUI
latency, RSS or global allocator measurement. The benchmark requires nonempty
raster output before timing. Retained payload accounting excludes fixed bounded
map/queue bookkeeping and transient work.
The same target compares font-replacement-plus-raster with reconstructing a
fresh CPU owner. These are two correct lifecycle strategies, not the same
operation before and after a fix. Both alternate identical prepared fonts;
exclude font discovery and overlapping builds, and report timings accordingly.

Shared text characterization lives in
`automexia-extension-api/tests/text_boundaries.rs`. Run
`cargo test -p automexia-extension-api --locked` for literal presegmented
Unicode, whitespace, zero/one/exact/over-limit, borrowed-storage and long-cluster
contracts. Existing trimmed end/middle results remain byte-compatible.
Use `cargo bench -p automexia-extension-api --locked --bench text_compaction`
for same-host short/long ASCII and Unicode measurements; save a baseline before
editing and compare after. This is helper latency, not native UI latency or an
allocator/RSS measurement. Grapheme limits never replace upstream byte limits.
Register each new benchmark in the canonical feature matrix and retain the
assurance mutation that rejects replacing its performance evidence with an
unrelated reference. An adjusted inventory count alone is not evidence.

Suggestion projection tests in `automexia-ui-model/tests/suggestions_ui.rs`
preserve full bounded accessible values, whitespace, visible match ownership,
geometry and hit targets. They reject oversized byte fields, index lists and
candidate sets, including invalid rows beyond the visible slice. The application
`suggestions_broker` integration test submits original Unicode candidates through
validation/ranking and checks exact keyboard/pointer editor replacements without
execution. These source tests do not activate the preview or establish native
screen-reader evidence.

For Hub text-boundary regressions, run
`cargo test -p automexia-terminal --bin automexia --locked renderer::connection_hub::tests`.
The desktop binary owns these tests, not the application library. Literal
Unicode clusters verify truncation and wrapping independently; separate ASCII
fixtures retain whitespace, the extra ellipsis, zero/one/exact limits and empty
lines. Wrapped review text must concatenate to the original bytes. These checks
do not substitute for measured-width, clipping or native-renderer evidence.

## Documentation verification

Before handoff:

- load every public Markdown file;
- verify UTF-8, balanced code fences, one clear title, and valid relative links;
- scan for machine-local or confidential values;
- scan for advanced, commercial, pricing, market, and unreleased-plan language;
- verify privately archived originals remain available and ignored;
- confirm public indexes name only public baseline features; and
- inspect the complete documentation diff for accidental loss.

The publication boundary is defined by
[Private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Command Productivity Cp50 Research

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F2 Model And Golden Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F3 Application Composition

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F3 Catalog Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub F3 Private Profile Recipe And Preference Library

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Connection Hub M1 Read Only Product Activation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp20 Cp22 Quick Action Assurance

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp30 Pure Projection Compiler

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp31 Persistent Alias Publication And Activation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp32 Reviewed Devops Action Packs

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Cp33 Native Imports And Trusted Workspace Task Bridges

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### D7cp6 Accepted Source And Release Gate

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M13 Provider Aware Quick Actions

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M3 Direct Openssh Review Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M4 Ssh Routes And Trust Contract

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M5 Typed Openssh Tunnels And Native Release Evidence

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M6 Typed Automation And Multi Environment Workspaces

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M7 Provider Neutral Authentication And Capsule Isolation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M8 Aws Adapter Source Contracts

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### M9 Azure Adapter Source Contracts

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Nightly And Release Depth

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## Command-productivity CP1

CP1 assurance covers the native completion adapter lifecycle: bounded generated
artifacts, native-definition ownership, shell-specific parsing, provider-free
typing/startup paths, explicit refresh/remove, disabled fallback, hostile
metadata, exact executable arguments, filtered subprocess context, deadline and
output limits, repeated lifecycle cleanup, and native platform evidence.

Focused checks:

```text
python tools/ci/check_command_productivity_cp1.py
python tools/ci/test_command_productivity_cp1.py
python tools/ci/test_measure_completion_adapter.py
```

A passing source check does not replace supported-shell native runs or prove an
unpublished package. Record the exact revision, shell, platform, artifact, first
failure, performance sample, cleanup result, and remaining external gates.

## Verification plan

Command boundary style has focused parser/geometry/pixel coverage and an
allocation-free benchmark. The [command-marker assurance contract](COMMAND-RESULT-ASSURANCE.md#command-markers-versus-pane-dividers)
lists exact commands, native scenarios and limitations. Do not reuse older
full-width-rule captures as evidence for the current short inset marker.

Command-productivity evidence is phase-owned but cumulative. CP2.2 includes the
`quick_action_search_1024` boundary and verifies search, copy, editor handoff,
and insert-without-Enter without launch. CP3.0 checks deterministic shell
projection; CP3.1 checks transactional opt-in alias persistence; CP3.2 checks
reviewed static packs; and CP3.3 checks capability-separated native imports and
trusted workspace task bridges.

### Command-productivity CP5.0 research

Run `python tools/ci/test_command_productivity_cp50.py` with the CP5.0 checker.
The result proves the reviewed source decision only; native shell, transport,
privacy, performance, resource, accessibility, package, disable, uninstall,
and rollback evidence remain separate gates.

The complete alias scenario inventory is maintained in
[DEVOPS-ALIASES.md#verification-plan](DEVOPS-ALIASES.md#verification-plan).

### Session-launch D0

D0 is governed by ADR 0012. Tests preserve system OpenSSH ownership, exact
argument boundaries, no implicit Enter, route/session isolation, cancellation,
descendant cleanup, and ordinary manual SSH fallback.

## Wrapped command information

Run the actual library and binary targets; require a nonzero test count:

```text
cargo test -p automexia-terminal --lib --all-features --locked automexia::ui::command_info::tests
cargo test -p automexia-terminal --bin automexia --all-features --locked renderer::command_info::tests
cargo test -p automexia-terminal --bin automexia --all-features --locked context::paste::tests
cargo bench -p automexia-terminal --bench automexia_services --profile dev -- command_information
python tools/ci/check_feature_test_reinforcement.py
python tools/ci/test_feature_test_reinforcement.py
```

The parser-driven regression checks complete context and timestamps at 80, 24
and 12 columns, then restores the original width. Its independent row-origin
expectations, native cell/cursor/history invariants, real grid glyph emission,
metadata draw data and exact restored CPU pixels cover different failure modes.
The standalone CPU text fixture must render both layer partitions; queued glyphs
alone do not prove that a captured frame contains text.

Set `AUTOMEXIA_COMMAND_INFO_PREVIEW` to a temporary PNG output path before the
binary test to capture a fictional narrow-pane frame. Require a newly generated,
nonempty image and inspect it. Do not capture contributor shell metadata in
repository artifacts. Separate tests cover fractional-scale ink bounds,
three-row panes with no native history, queued wheel events, blank-padding
limits, source-row uniqueness, image texture cropping, and accepted/rejected
paste isolation. Re-run the existing live WSL resize and table-seam tests.

The benchmark checks every label's byte coverage and geometry during measured
packing, plus the allocation-free identity projection. Development-profile
layout timing is not a measurement of native PTY, GPU or compositor latency.

Native desktop checks remain separate: narrow/wide and minimize/restore with
multiple panes/tabs, mouse selection and copying, search, command jumps, wheel
and scrollbar drag, typing/paste return, IME, screen-reader navigation, inline
images, themes and reduced motion. Run the supported Windows, Linux and macOS
shell/backend combinations on actual desktops before claiming their native
visual or accessibility coverage. Finish with the full contributor readiness
and privacy gates; focused tests are not full-product certification.

## Focused core table view

Run `cargo test -p automexia-ui-model --test tables --locked`,
`cargo test -p automexia-terminal --lib --locked table_output`, and
`cargo test -p automexia-terminal --bin automexia --locked table_view`.
The parser test uses actual HT/tab stops and extreme VT resizing; the visual
test traverses parser, capture, model, draw stream and real CPU font rendering.
It asserts literal row edges, byte-exact source retention, exact restored pixels
and detection of a one-pixel separator shift. Set `AUTOMEXIA_CORE_TABLE_PREVIEW`
to a private output PNG path for a fresh fictional-data artifact and inspect it.
This isolated CPU surface does not certify native modal compositor ordering.

Run `cargo bench -p automexia-terminal --bench automexia_services -- core_table_view`
for correctness-checked 256-row capture and resize/scroll timing. Measurements
cover the bounded model, not PTY, clipboard, native input or compositor latency.
Also run the full application, VT, shortcut/checker mutation and contributor
readiness suites. Native validation must exercise opening from selection and
history, live output underneath, route closure, ordinary and modified key-up,
paste/IME containment, physical wheel/trackpad/scrollbar interaction, small and
large windows, multiple themes/scales and assistive-technology focus delivery.
