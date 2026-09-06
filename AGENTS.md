# Automexia AI contributor workflow

This file is the repository operating contract for AI coding agents. It applies
to planning, implementation, review, testing, documentation, and delivery work
under this directory. Product purpose, audience, values, and public messaging
must follow [`docs/PRODUCT-VISION.md`](docs/PRODUCT-VISION.md). Human contributor
policy remains authoritative in [`CONTRIBUTING.md`](CONTRIBUTING.md); security,
architecture, and release work must also follow `SECURITY.md`,
`docs/ARCHITECTURE.md`, and `RELEASING.md`.

User and system instructions take precedence. Never weaken a quality gate,
security boundary, or test merely to make a change pass.

## Definition of done

A change is complete only when all applicable statements are supported by
evidence:

1. The requested outcome, scope, non-goals, and acceptance criteria are clear.
2. Existing behavior, ownership, tests, documentation, and local modifications
   were inspected before editing.
3. Fully implemented work is preserved; partially implemented work is extended;
   missing work is implemented without duplicating another authority.
4. Relevant current standards, official documentation, maintained tools, and
   open-source alternatives were evaluated using primary sources.
5. The design fits Automexia's architecture and trust boundaries.
6. Tests were designed before or with the production change and reproduce the
   failure or contract being changed.
7. The implementation is bounded, cancellable where asynchronous, observable,
   maintainable, and reversible where practical.
8. Applicable correctness, security, performance, resource, storage,
   resilience, accessibility, visual, and native-platform checks pass.
9. User, contributor, architecture, reference, roadmap, and release
   documentation accurately describe the resulting state.
10. External or unavailable validation is reported honestly and is not silently
    treated as passing.
11. Authorized changes are grouped coherently, DCO-signed, pushed without
    rewriting shared history, and verified on the remote.

A focused test passing is not proof that the full change is complete. A
cross-compile is not a native runtime test. Retrying a flaky test does not erase
the first failure; investigate it and record the cause.

## Non-negotiable project boundaries

- Preserve user work. Start with `git status`, never overwrite unrelated dirty
  files, and never use destructive Git or filesystem commands without explicit
  authorization and exact target verification.
- Keep one clear owner for each session, PTY, process, renderer snapshot,
  persisted record, and UI surface. Do not introduce competing sources of truth.
- Keep filesystem, provider, network, authentication, database, and extension
  work off input, PTY, resize, renderer, and startup hot paths.
- Treat terminal output, shell metadata, imported files, paths, provider output,
  completions, and AI-generated content as untrusted input.
- Launch processes with typed executables and exact argument arrays. Avoid shell
  evaluation, command-string concatenation, implicit Enter, `sh -c`, `cmd /c`,
  and PowerShell expression evaluation for structured actions.
- Keep credentials in platform or external credential stores. Persist opaque
  references, never secret material, whenever possible.
- Never persist, commit, publish, or quote confidential, private, or
  machine-local information in any repository artifact. This applies to source
  code, tests, fixtures, snapshots, goldens, examples, comments, documentation,
  screenshots, recordings, logs, reports, benchmarks, generated files, change
  fragments, commit messages, pull requests, and handoff text.
- Forbidden local information includes real usernames, personal names that are
  not intentionally public project metadata, machine or device names, hostnames,
  home/profile directories, absolute checkout/workspace/project-folder paths,
  environment-variable values, internal domains or IP addresses, account or
  tenant identifiers, cluster or project names, credentials, tokens, cookies,
  private history, and copied shell/provider output that can identify a person,
  computer, organization, or environment.
- Never derive persistent examples or test data from live values such as
  `USERNAME`, `USER`, `COMPUTERNAME`, `HOSTNAME`, `HOME`, `USERPROFILE`, `PWD`,
  the current working directory, Git configuration, shell metadata, provider
  configuration, or command output. Tests that need host paths must create
  isolated temporary paths at runtime and must not snapshot the real values.
- Use clearly fictional, stable placeholders such as `alice`, `devbox`,
  `example.invalid`, documented test-network addresses, and repository-relative
  paths. Redact sensitive values before showing command output or diagnostics;
  never repeat a discovered value merely to explain that it was removed.
- Before handoff, committing, or pushing, scan every changed, staged, untracked,
  generated, and newly referenced artifact for secrets and local identifiers.
  Treat any verified leak as a blocking failure, remove it without weakening the
  behavior or test, re-run the scan, and keep scan reports fully redacted.
- Repository-owned test and readiness success output must use stable logical
  labels for managed roots and generated targets, never contributor-specific
  absolute paths. Failure diagnostics must redact private path prefixes while
  preserving the failing operation and repository-relative owner.
- Bound input bytes, decoded dimensions, recursion, file counts, queues, cache
  size, history, concurrency, time, retries, logs, and persisted storage.
- Preserve pane, tab, route, session, and generation isolation. Cancel obsolete
  work and reject stale results before publishing them.
- Publish state before waking the renderer. Expensive refreshes must be
  generation-aware, cancellable, and capable of a safe last-known-good result.
- Respect the build/wrap/adopt boundary documented in
  `docs/BUILD-WRAP-ADOPT-ARCHITECTURE.md`.
- Dependency, capability, unsafe-code, persistence, protocol, threading,
  security, and public-behavior changes require explicit review and normally an
  ADR.

## End-to-end workflow

### 1. Establish scope and authority

Before editing, write a short working statement containing:

- the user-visible or contributor-visible outcome;
- measurable acceptance criteria;
- authoritative roadmap phase, specification, ADR, implementation, tests, and
  documentation, if they exist;
- in-scope and out-of-scope behavior;
- actions the user authorized, especially external writes, deletion, Git
  operations, release, migration, or credential changes;
- external prerequisites or validation that cannot be completed locally.

Ask the user only when a missing choice would materially change behavior,
security, compatibility, data, or scope. Otherwise make a conservative,
documented assumption and continue.

### 2. Inspect the repository before proposing a solution

At minimum:

```text
git status --short
git branch --show-current
git log -n 10 --oneline
rg --files
rg "relevant symbol or behavior"
```

Read the smallest complete set of authoritative files. Inspect:

- implementation owners and all important call sites;
- public configuration, shortcuts, protocols, schemas, CLI, and UI contracts;
- existing unit, integration, conformance, property, fuzz, benchmark, native,
  packaging, and policy tests;
- architecture checks and contributor commands;
- recent commits touching the same subsystem;
- platform adapters and feature gates;
- the current worktree, including changes that must remain untouched.

Do not infer implementation status from roadmap prose alone. Verify it in source
and tests.

### 3. Build an evidence ledger

Classify each requested item before implementation:

| Status | Required evidence | Action |
|---|---|---|
| Fully implemented | Source owner, contract tests, applicable platform evidence, and current docs agree | Preserve; skip after recording evidence |
| Partially implemented | Some behavior or evidence exists, but a stated contract, boundary, platform, or failure case is missing | Extend the existing owner |
| Not implemented | No authoritative source and test owner exists | Design and implement |
| External prerequisite | Signing, hardware, account, elevated runner, long campaign, or human assessment is required | Automate what is possible and report the remaining gate |

For every row, record the source owner, tests, benchmark or resource evidence,
security/UX/platform implications, missing proof, and exit criteria. This avoids
duplicate implementations and false completion claims.

### 4. Research current practice and reusable technology

Research is part of design, not a justification added afterward. Begin with
project ADRs and existing dependencies, then use current primary sources:

- official specifications and platform documentation;
- the dependency's official documentation, source repository, releases, and
  security advisories;
- maintained reference implementations;
- peer-reviewed or vendor-authored technical material where relevant.

External research is mandatory for new dependencies, security-sensitive work,
accessibility and interaction patterns, OS integration, protocols, packaging,
cryptography, authentication, sandboxing, and unfamiliar technology. A purely
mechanical repository change does not require irrelevant broad browsing, but it
still requires verification against project conventions and official tool docs.

For every serious candidate, evaluate:

- build, wrap, or adopt;
- license and redistribution compatibility;
- maintenance activity, ownership, release cadence, advisories, and provenance;
- required features, transitive dependencies, binary size, build time, startup
  impact, MSRV, and supported platforms;
- thread, cancellation, resource, storage, network, and offline behavior;
- unsafe code and authority/capability footprint;
- accessibility and localization support for UI components;
- migration, rollback, disable, uninstall, replacement, and failure behavior;
- benchmark quality and native-platform evidence.

Prefer the standard library and current project infrastructure when they satisfy
the contract. Do not rebuild mature protocol, parser, authentication, keychain,
vault, database, editor, or accessibility primitives without a documented reason.
Do not add a dependency merely because it appears in a future roadmap.

If current research cannot be completed because network or source access is
unavailable, record the limitation and do not invent results.

### 4.1 Decide feature placement before implementation

Before editing production code for any feature, analyze where that feature
belongs and record one explicit decision: core terminal, an existing extension,
or a new extension. Do not start implementation until this ownership decision is
supported by the current architecture, call sites, trust boundaries, lifecycle,
and tests.

Use these placement rules:

- Put behavior in the core terminal only when it is fundamental to terminal
  operation or must be shared by every installation, such as PTY/process
  ownership, terminal state, input, rendering, panes/tabs, windowing, clipboard,
  common configuration, or a stable capability-broker boundary. Keep optional
  product and provider authority out of core hot paths.
- Extend an existing extension when the feature belongs to that extension's
  cohesive domain and can remain behind its established capability, dependency,
  lifecycle, persistence, enable/disable, and uninstall boundaries.
- Create a new extension only when the feature is optional, cohesive, and has a
  distinct authority or dependency footprint that does not fit an existing
  extension. Define its public contract, capability manifest, limits, lifecycle,
  failure isolation, feature gate, disable/uninstall behavior, tests, and
  packaging ownership before implementation.
- Split cross-cutting features at trust boundaries: keep only capability-free
  shared contracts or indispensable terminal mechanisms in the appropriate core
  owner; keep provider, network, authentication, filesystem, process, or other
  optional authority in an extension; and route privileged activation through
  the application-owned broker rather than duplicating authority.

For the chosen placement, identify the existing authoritative owner and all
important consumers, reject duplicate sources of truth, and document why the
other two options are inferior. Include dependency direction, startup and hot-
path impact, security/capability surface, persistence, failure containment,
cross-platform behavior, test ownership, packaging, rollback, and future
replacement cost. A new or materially changed package/capability boundary
normally requires an ADR and architecture-checker coverage.

### 5. Produce an implementation plan before editing production code

The plan must cover:

1. requirements and acceptance criteria;
2. evidence-led full/partial/missing classification;
3. alternatives considered and the build/wrap/adopt decision;
4. the recorded core/existing-extension/new-extension placement decision, plus
   crate, module, file, authority ownership, and rejected alternatives;
5. data flow, state model, threads/tasks, cancellation, cleanup, and shutdown;
6. trust boundaries, capability checks, redaction, limits, persistence,
   migration, rollback, and fail-safe behavior;
7. UI hierarchy, density, input model, focus, responsive behavior,
   accessibility, contrast, reduced motion, and error/empty/loading states;
8. Windows, Linux/BSD, macOS, shell, architecture, and feature differences;
9. tests to write first and the benchmark/native/manual/full-gate ladder;
10. documentation, ADR, roadmap/audit, and changelog ownership;
11. coherent commit groups and rollback strategy;
12. externally blocked evidence and the exact way to obtain it.

Use SOLID, DRY, and KISS to clarify ownership, not to create abstraction for its
own sake. Prefer pure models with thin platform, shell, provider, persistence,
and renderer adapters. Keep mechanical moves separate from behavior changes.

### 6. Threat-model failure and resource behavior

Before choosing APIs or limits, consider:

- malformed, fragmented, oversized, hostile, Unicode, combining, bidi, and
  control-character input;
- missing, stale, corrupt, partial, read-only, symlinked, permission-denied,
  disk-full, offline, timed-out, cancelled, and interrupted state;
- rapid input, resize storms, output storms, multiple panes/tabs/windows,
  generation replacement, full queues, worker restart, shutdown, and child
  process trees;
- command injection, path traversal, TOCTOU, secret disclosure, privilege
  escalation, untrusted remote output, and silent fallback;
- tiny through 8K viewports, high scale factors, localization, long text, IME,
  keyboard-only use, screen readers, focus restoration, and modal stacking;
- repeated operation and long-session growth in memory, handles, threads,
  processes, caches, logs, targets, temporary files, and persisted data.

Define enforceable limits and cleanup ownership. Fail closed at trust boundaries,
return redacted actionable errors, and ensure optional feature degradation never
breaks the terminal's core input/output path.

### 7. Design tests first

Use TDD for behavior and regression work:

1. add the smallest deterministic test that fails for the real reason;
2. add boundary and negative cases before broad implementation;
3. characterize behavior before refactoring legacy ownership;
4. implement only enough to satisfy the next contract;
5. keep the test as a permanent regression guard.

Select tests proportionate to the change:

- unit tests for pure state, parsing, policy, layout, and limits;
- integration and conformance tests for crate, shell, provider, protocol, and
  persistence boundaries;
- property tests and fuzz targets for parsers and hostile structured input;
- deterministic concurrency/model tests for ordering, cancellation, saturation,
  stale generations, restart, and shutdown;
- renderer-neutral snapshots/goldens for UI before native GUI automation;
- native tests for ConPTY, Unix PTY, windowing, GPU, clipboard, IME,
  accessibility, packaging, signing, and OS credential integration;
- benchmarks for hot paths, latency, throughput, allocation, memory, startup,
  sustained operation, and cleanup;
- repeated lifecycle tests for resource and storage leak detection;
- disabled, uninstall, rollback, import, migration, and recovery behavior.

Avoid arbitrary sleeps, timing-only assertions, OCR when renderer-neutral state
exists, and tests that merely duplicate implementation. Use bounded readiness,
fixed seeds, explicit fake clocks/processes/filesystems, and invariant assertions.
Claim only the operating systems and architectures that actually ran natively.

#### Comment test intent strategically

Test names and assertions should make the observable behavior clear. Add concise
comments only where they preserve reasoning that is not obvious from the code:

- why a fixture has a particular shape, boundary value, ordering, or identity;
- which real user path, historical failure, threat, or authority boundary a
  mutation reproduces;
- which dependency or host observation is mocked and which independent oracle
  still proves the result;
- why cleanup, redaction, forbidden-side-effect, atomicity, concurrency, or
  resource assertions are essential;
- why a native setup phase must occur in that order or retain a sentinel.

Prefer one comment on a shared helper or scenario block over repeating the same
explanation in every test. Do not narrate syntax, restate a test name or
assertion, add mechanical `Arrange`/`Act`/`Assert` labels, or use comments to
compensate for unclear names and oversized helpers. Exact scanner, parser, and
golden fixtures may intentionally remain uncommented when comments would change
the bytes under test. Preserve existing comments unless their contract changes;
when behavior changes, update nearby comments in the same patch and review them
for stale claims, confidential data, and machine-local identifiers.

### 7.1 Apply the anti-escape assurance protocol

Tests reduce risk but cannot prove that arbitrary software has no defects. Never
promise that no issue can escape. Make the strongest bounded claim supported by
the exact fixtures, environments, artifacts, repetitions, and review that ran.
Unexecuted native platforms, hardware, accounts, assistive technologies, signing,
long campaigns, and human review remain explicit gates.

The canonical per-feature reinforcement plan is
[`docs/FEATURE-TEST-REINFORCEMENT.md`](docs/FEATURE-TEST-REINFORCEMENT.md), and
its machine-enforced mirror is
[`tests/assurance/feature-test-reinforcement-v1.json`](tests/assurance/feature-test-reinforcement-v1.json).
For every behavior, fix, refactor, or dependency change:

1. identify every affected feature-matrix entry and reinforcement-plan section;
2. update its risk flags, interactions, needed tests, independent oracles,
   checker reinforcements, evidence owners, and exit criteria;
3. run the contract checker and its mutation suite;
4. leave a feature classified as partial or external until every applicable
   exit criterion has current evidence.

Do not use file names, command labels, mocked return values, test counts, or line
coverage as substitutes for behavior. Before production editing, create a
scenario inventory that covers, where applicable:

- zero, one, boundary-minus-one, boundary, boundary-plus-one, large, maximum,
  over-limit, repeated, fragmented, malformed, cancelled, stale, and concurrent
  inputs;
- every state transition, failure transition, retry, disable, rollback,
  migration, recovery, uninstall, shutdown, and restart;
- every supported shell, process adapter, platform, architecture, renderer,
  theme, scale, viewport, input method, credential backend, provider, and
  protocol variation;
- all pairwise interactions with overlays, panes, tabs, routes, generations,
  selection, scrollback, clipboard, IME, resize, focus, and lifecycle ownership;
- output or data that is shorter than, equal to, and longer than every visible
  viewport, queue, cache, parser, persistence, and transport boundary.

For each reported regression, first add a test that reproduces the user's real
path and fails for the observed reason. A synthetic unit test may isolate the
cause, but it cannot replace the real-path regression. Native shell behavior
must launch the real supported shell or PTY adapter with exact executable and
argument arrays; renderer regressions must traverse parser, terminal state,
visible viewport, layout, draw data, and pixels. Test hooks may freeze clocks,
motion, accounts, and public fixture data, but may not inject the internal state
whose production creation is under test.

Every high-risk assertion needs an independent oracle chosen outside the code
path under test. Depending on the feature, compare:

- exact bytes, typed state, protocol frames, grid cells, accessibility events,
  persisted round trips, process trees, handles, files, network attempts,
  capability decisions, redacted logs, or final artifact identities;
- forbidden side effects as well as intended effects, including no PTY input,
  implicit Enter, shell evaluation, secret persistence, global configuration
  mutation, stale publication, orphan process, leaked handle, or unexpected
  network access;
- a second implementation, reference parser, metamorphic relation, model, or
  pre-change characterization when an exact expected value is impractical.

Visible changes require three distinct layers:

1. renderer-neutral geometry, hierarchy, focus, semantic, clipping, z-order,
   contrast, and reduced-motion assertions;
2. deterministic controlled raster goldens using exact RGBA comparison;
3. native frames and accessibility tree/event evidence on each claimed
   OS/display/renderer environment.

Row-anchored terminal chrome needs an additional ownership audit. A semantic
prompt row can validly own its command completion while also carrying the
preceding command's output boundary. After every visible snapshot, reflow, and
display-offset change, normalize overlay projections into a frame-local map
scoped by route and pane: paint each stable result identity at most once and
give each display row at most one badge owner. Prefer the truthful preceding
output boundary over a source-row fallback, reject invalid or stale geometry,
and resolve malformed collisions deterministically without changing terminal
cells, scrollback, PTY bytes, focus, or command input. Do not render a raw
post-reflow anchor list directly.

Regressions for row-anchored chrome must traverse raw supported-shell bytes,
VT parsing, multiple adjacent command lifecycles, scrollback, alternating
narrow/wide resize, visible snapshot publication, previous/next command
navigation in both directions, projection, draw rectangles, and controlled
pixels. Assert unique result identities, non-intersecting badge rectangles,
non-intersection with every co-located overlay from another owner,
stable source/boundary metadata, pane and route isolation, no PTY input, and
the restored live prompt after every transition. Cover source eviction,
silent/output commands, repeated resize/navigation, split panes, scale and
responsive label fallbacks. Benchmark the combined reflow-navigation-snapshot
path over deep bounded history and ensure projection work remains bounded by
the visible frame. A single-result test or a hook that reports only the latest
badge is insufficient evidence for multi-result paint isolation.

When independent UI contributors can paint the same row, they must share an
explicit renderer-neutral reservation or exclusion contract. Measure and expose
every contributor's issued rectangles in controlled native hooks, then compare
them pairwise across owners after resize, scale, scroll, navigation, focus, and
extension enable/disable transitions. Testing only each contributor against
itself cannot detect cross-overlay collisions. Native pointer automation must
wait for the renderer-owned hit target before pressing; posting a move and click
back-to-back is not deterministic evidence on a scheduled desktop event loop.

Before exact raster comparison, freeze every visible volatile value, including
wall clock, completion duration, animation phase, cursor blink, discovery data,
and fixture output. Prove that the control hook is absent from product builds,
then run the zero-tolerance comparator and inspect both native frames. Stable
geometry with a changing label is not a deterministic pixel oracle.

Native readiness must describe a frame that the compositor can actually show.
When a test control is consumed after overlay or draw-data construction, defer
its snapshot to a new forced frame and publish readiness only after that frame
has presented successfully. A model snapshot written before present can race a
partially rasterized CPU or GPU surface and is not visual evidence. Retained
desktop captures must first establish foreground ownership, reject detectable
native dialogs or occluders, position the entire physical client area on the
capture display, reject non-opaque or uninitialized client pixels, and then
obtain two consecutive exact full-frame pixel digests within a bounded
deadline. Freeze live editor bytes as well as clocks and animation before
comparing renderers; an active cursor or partially typed command is volatile
visual state. Matching captures from two renderers do not prove correctness if
both captured the same dialog, stale frame, or occluder; inspect each retained
native frame independently.

Frame-skip caches are presentation state, not content-only state. Their identity
must include the physical surface width and height so a content-preserving
resize still repaints every newly exposed pixel. Record a reusable frame only
after native presentation succeeds; acquisition or presentation failure must
leave identical content eligible for retry. Test same-content extent changes,
failed-present retry semantics, initial native sizing, and cross-backend full-
client equality. A later input event that happens to repaint an incomplete
surface does not make the resize path correct.

The repository's deterministic visual policy has zero channel tolerance, zero
changed-pixel ratio, and no default masks. A change to one channel in one pixel
must fail with the first changed coordinate, changed bounds, counts, and diff
artifact. Do not hide nondeterminism with a broad tolerance. Instead freeze the
source, use environment-specific baselines, or classify the native comparison as
separate human-reviewed evidence. Any intentional pixel change requires a
reviewed baseline update tied to the feature and exact environment. Exercise
tiny through 8K viewports, 100–300% scale, light/dark/high-contrast themes, long
and localized text, Unicode/combining/bidi input, empty/loading/error states,
keyboard/pointer/IME use, focus restoration, modal stacking, and animation both
enabled and reduced.

Accessibility assurance must test role, name, value, state, relationships, focus
order, focus restoration, live announcements, keyboard-only operation,
pointer-target behavior, high contrast, reduced motion, and 200%/400% text or
scale as applicable. Automated tree/event checks are necessary but do not replace
current Narrator and NVDA evidence on Windows, VoiceOver on macOS, and Orca on
X11/Wayland for any release claim covering those environments.

Security and parser boundaries require table-driven negative cases, property
tests, coverage-guided fuzzing, repeated object-member cases,
size/depth/count ceilings, Unicode/control/bidi cases, path traversal and link
cases, cancellation, bounded-deadline behavior, sensitive-value removal
canaries, and exact capability decisions, argument vectors, and subprocess
contexts.
Seed corpora must include every historical failure. A passing fuzz smoke test
proves only that campaign; record engine, seed, corpus digest, duration,
executions, sanitizer, target, platform, and discovered/replayed crashes.

Concurrency tests must use deterministic models or controlled schedulers for
publish-before-wake, cancellation, saturation, stale generation rejection,
disconnect, restart, and shutdown. Then add repeated native stress for real
PTY/process/window behavior. Arbitrary sleeps, retry-until-pass, and timing-only
assertions are invalid evidence. Preserve and investigate the first failure of a
flaky run.

Multi-session shutdown must broadcast an idempotent termination request to all
active, background, split, pane-tab, parked, and top-level-window PTY owners
before any sequential drop or worker join begins. Test one and many sessions,
window-close and application-quit paths, repeated requests, and the final event-
loop callback. On platforms where a pseudoterminal host can reparent children,
native cleanup evidence must capture exact platform-owned process identities
before closing the owner rather than relying only on parent traversal. Assert
that the application and every captured identity exit, that no orphan remains,
and that total many-session wall-clock shutdown stays within a declared ceiling.

Persistence tests must cover canonical round trip, every supported predecessor,
corruption, truncation, duplicate keys, read-only/disk-full/interrupted writes,
atomic replacement, permissions, concurrent readers/writers, rollback, disable,
uninstall, and restart. Compare storage trees and digests before and after while
redacting private content.

Performance and resource evidence must measure the real owning path with
same-host baselines and noise-aware thresholds. Include latency distributions,
throughput, allocations, CPU, memory, GPU memory where observable, handles,
threads, child processes, queue/cache/storage growth, long-session stability,
cancellation, and final cleanup. Benchmark shortcuts or isolated helpers do not
prove interactive latency. A late performance fix requires rerunning correctness,
resource, and visual gates affected by it.

Checker and test infrastructure are production assurance code. Each checker
must have mutation tests proving that deletion, weakening, reordering,
duplication, stale paths, count adjustment, threshold relaxation, mask addition,
missing platform scope, and fabricated evidence fail closed. A checker must
validate semantic scenarios and required owners, not merely file existence,
non-empty arrays, self-reported counts, or matching prose. Test the test fixture
by introducing the smallest fault it is supposed to detect.

For a feature to move to fully implemented, re-audit the complete path and
record:

- focused unit/integration/property/fuzz/model results;
- real native workflow results for every platform being claimed;
- exact visual and accessibility results for visible behavior;
- performance/resource/storage measurements and cleanup;
- security, negative, rollback, disable, and recovery evidence;
- packaged-artifact identity and external prerequisites;
- checker mutation results and the exact commit under test.

Run these repository-owned reinforcement gates before the broader evidence
ladder:

```text
python tools/ci/check_feature_test_reinforcement.py
python tools/ci/test_feature_test_reinforcement.py
python tools/ci/s1_assurance.py check-policy
python tools/ci/test_s1_assurance.py
python tools/ci/validate_repository.py
```

A narrow pass cannot override a known real-workflow failure. If source, tests,
documentation, and observed behavior disagree, mark the feature partial, preserve
the failing reproduction, fix the owning path, and rerun every affected layer
before claiming completion.

### 8. Implement in small coherent increments

- Make the smallest production change for the next failing test.
- Reuse bounded workers, snapshots, atomic persistence, cancellation, and wake
  mechanisms already owned by the project.
- Validate and normalize at ingress; keep internal types precise.
- Make process, thread, task, file, cache, and temporary resource ownership
  explicit and join/terminate descendants during cleanup.
- Comment non-obvious invariants and decisions, not line-by-line mechanics.
- Do not mix a directory move, dependency upgrade, global reformat, and behavior
  change in one patch.
- Delete code or files only after proving there are no callers, build owners,
  generated owners, packaging references, tests, or provenance requirements.
- Format and run focused tests after each meaningful increment.
- Use `apply_patch` for intentional edits and preserve unrelated dirty files.

### 9. Execute the evidence ladder

Start fast and expand according to risk:

```text
cargo fmt --all --check
cargo test -p <owner> --locked <focused-test>
cargo clippy -p <owner> --all-targets --all-features --locked -- -D warnings
python tools/ci/validate_repository.py
git diff --check
```

Then run applicable domain gates, for example:

```text
cargo xtask test conformance
cargo xtask test resize-stress
cargo xtask verify architecture
cargo xtask verify identity
cargo xtask package --check
```

Use native Windows GUI validation for renderer/window changes and native OS jobs
for platform behavior. Cross-compilation is useful evidence, but it does not
replace native runtime verification.

For performance work, compile and run the relevant Criterion or repository
benchmark, compare against a same-host baseline, account for noise, and measure
latency, throughput, allocation/resource growth, and cleanup. Remove disposable
test targets and temporary artifacts after recording results.

Before handoff, run the contributor gate:

```text
cargo ready
```

Use `cargo xtask ci` when the task or policy specifically requires the complete
CI-equivalent suite. Do not use `cargo dev` merely as a test command because it
intentionally launches the application. Use `cargo automexia` only for deliberate
manual UI validation.

### 9.1 Preserve build-cache ownership and push policy

- Keep final Cargo artifacts, intermediate compiler data, per-gate verification
  targets, immutable assurance tools, mutable downloads/runtime state, staging,
  process temporary data, and QA benchmark targets under their documented
  separate owners. Never share a mutable Cargo target across worktrees or
  operating systems.
- Before adding a cache, define its content identity, byte/file/directory
  ceilings, producer, consumers, lease or process ownership, atomic publication,
  corruption behavior, cleanup scope, grace period, rollback, and proof that it
  cannot affect release authority.
- Inspect with `cargo xtask cache status`. Preview `cargo xtask cache gc` before
  applying cleanup. Never broaden an exact generated candidate into a cache
  root, worktree root, linked path, dirty worktree, live process, or leased
  directory.
- GitHub's free hosted `CI` workflow is the automatic push and pull-request
  authority. Keep the local pre-push hook dormant and non-blocking until the user
  explicitly authorizes reactivation. Preserve the manual
  `cargo xtask assurance pre-push` path and update generator, mutation tests,
  resource budgets, and documentation together if this policy changes.

### 10. Perform visual and manual verification

For visible or interactive changes, exercise the real workflow and inspect a
rendered artifact. Cover relevant combinations of:

- tiny, normal, ultrawide, 4K/8K-equivalent, split, and pane-tab layouts;
- 100–300% scale, Unicode, long paths, light/dark/custom themes;
- keyboard, pointer, selection, IME, focus loss/restore, overlays, modals,
  confirmation screens, clipping, cursor placement, and z-order;
- alignment, density, hierarchy, contrast, icon consistency, and redundant
  meaning that does not rely on color alone.

Update renderer-neutral goldens and capture screenshots when useful. Never claim
that UI “looks correct” without inspecting the rendered result. Record external
screen-reader, controlled-GPU, signing, notarization, or elevated testing as an
outstanding gate when it cannot run locally.

### 11. Re-audit before declaring completion

Re-read the request and evidence ledger. Search for obsolete paths, duplicate
authorities, unreachable code, TODO placeholders, hard-coded identifiers,
unbounded operations, secret-bearing logs, silent fallback, and stale docs.
Review the complete diff and diff stat. Re-run every check affected by a late fix.

Errors and reports must be actionable but redacted: do not expose credentials,
tokens, environment values, private history, personal paths, or remote host data.

### 12. Document the resulting truth

Follow [`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md).

Publish only current implementation, observable limitations, tests, and release
status. Do not publish business plans or future feature names, designs, or
delivery plans, including free or open-source features. During merges, review
newly restored documentation and checker requirements for the same boundary.
Keep current security, native, and release evidence requirements intact.

Update, as applicable:

- user guides for setup, workflow, failure, recovery, disable/uninstall, and
  migration;
- CLI/configuration/shortcut/protocol/reference documentation;
- architecture and an ADR for material boundaries or decisions;
- testing documentation with exact local, CI, native, benchmark, and external
  evidence;
- the roadmap and phase audit when delivery status changes;
- the product vision and brand guidance when purpose, audience, value, or public
  messaging changes;
- feature and assurance matrices when a new capability or guarantee ships;
- `docs/index.md` navigation;
- one fragment under `changes/` unless policy explicitly exempts the change.

Keep shipped behavior, planned behavior, and external prerequisites visibly
separate. Explain why the chosen approach fits Automexia and why serious
alternatives were rejected. Commands must be copyable and platform-specific
limitations explicit.

### 13. Commit and push safely

Only commit or push when authorized. Then:

1. inspect `git status`, the full diff, and the staged diff;
2. stage only intended files; never sweep in unrelated work or generated output;
3. group commits by coherent behavior, tests, and supporting documentation;
4. use conventional, meaningful messages and DCO sign-off:

   ```text
   git commit -s -m "type(scope): describe the completed outcome"
   ```

5. verify no secrets, local paths, build outputs, or large accidental artifacts
   are included;
6. push the feature branch without force-pushing protected/shared history;
7. verify the remote branch and commit, then report the SHA and remaining
   worktree state.

If one Git transport fails, diagnose it and use another configured, secure
transport when available. Never disable TLS verification, expose credentials,
rewrite shared history, or push directly to protected `main` to bypass policy.
Pushing a branch does not imply merge, release, or deployment authorization.

## Dependency adoption checklist

Before adding or materially changing a dependency, verify and document:

- official source, exact version, enabled features, and why existing code is
  insufficient;
- license, advisories, repository/source policy, bans, and duplicate versions;
- maintenance and provenance;
- Rust/toolchain/MSRV and Windows, Linux/BSD, macOS, and architecture support;
- build time, binary size, startup, CPU, memory, storage, network, and background
  process impact;
- unsafe code, trust boundary, permissions, and data exposure;
- cancellation, timeouts, limits, cleanup, offline behavior, and version drift;
- migration, rollback, disable, uninstall, and replacement path;
- deterministic fake-based tests plus native integration evidence;
- lockfile, `cargo deny`, SBOM/provenance, doctor, configuration, and contributor
  documentation updates.

## Required final handoff

Lead with the outcome and include:

1. what changed and where;
2. which acceptance criteria are proven;
3. exact tests and exit results;
4. benchmark/resource/security/visual/native evidence, when applicable;
5. documentation and migration implications;
6. commit SHA, branch, and push verification when authorized;
7. honest external prerequisites, limitations, or manual follow-up.

Do not use absolute claims such as “100% secure,” “all operating systems work,”
or “no issues exist.” State the exact platforms, configurations, fixtures,
durations, and evidence actually exercised.

## Compact execution checklist

- [ ] Scope, authority, acceptance criteria, non-goals, and prerequisites recorded.
- [ ] Worktree, owners, callers, contracts, tests, docs, and recent history inspected.
- [ ] Every item classified as full, partial, missing, or external with evidence.
- [ ] Current primary-source research and build/wrap/adopt analysis completed.
- [ ] Core, existing-extension, or new-extension placement is decided and
  justified before production editing.
- [ ] Architecture, trust, lifecycle, failure, UX, platform, and rollback plan written.
- [ ] Deterministic failing tests and limits defined before production changes.
- [ ] Affected reinforcement entries, scenario classes, interactions, independent oracles, and exit criteria updated.
- [ ] Real-path, one-pixel, negative-side-effect, checker-mutation, and native/platform evidence completed or explicitly external.
- [ ] Small coherent implementation preserves hot paths and unrelated work.
- [ ] Focused, domain, security, performance/resource, native, visual, and full gates run as applicable.
- [ ] Results re-audited; failures fixed and affected checks rerun.
- [ ] Changed, staged, untracked, and generated artifacts contain no secrets,
  machine names, usernames, profile paths, absolute workspace/project paths, or
  other private environment values; redacted scans verify this claim.
- [ ] Guides, references, architecture/ADR, testing, roadmap/audit, navigation, and changelog updated as applicable.
- [ ] Final claims distinguish local evidence from external validation.
- [ ] Authorized changes are grouped, DCO-signed, pushed, and remote-verified.
