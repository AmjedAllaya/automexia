# Testing

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

## Shell integration

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
