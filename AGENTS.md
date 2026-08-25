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

### 5. Produce an implementation plan before editing production code

The plan must cover:

1. requirements and acceptance criteria;
2. evidence-led full/partial/missing classification;
3. alternatives considered and the build/wrap/adopt decision;
4. crate, module, file, and authority ownership;
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
tests, coverage-guided fuzzing, duplicate-key tests, size/depth/count ceilings,
Unicode/control/bidi cases, path traversal and link cases, cancellation,
timeouts, redaction canaries, and exact capability/argv/environment assertions.
Seed corpora must include every historical failure. A passing fuzz smoke test
proves only that campaign; record engine, seed, corpus digest, duration,
executions, sanitizer, target, platform, and discovered/replayed crashes.

Concurrency tests must use deterministic models or controlled schedulers for
publish-before-wake, cancellation, saturation, stale generation rejection,
disconnect, restart, and shutdown. Then add repeated native stress for real
PTY/process/window behavior. Arbitrary sleeps, retry-until-pass, and timing-only
assertions are invalid evidence. Preserve and investigate the first failure of a
flaky run.

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

Follow [`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md). Update, as applicable:

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
- [ ] Architecture, trust, lifecycle, failure, UX, platform, and rollback plan written.
- [ ] Deterministic failing tests and limits defined before production changes.
- [ ] Affected reinforcement entries, scenario classes, interactions, independent oracles, and exit criteria updated.
- [ ] Real-path, one-pixel, negative-side-effect, checker-mutation, and native/platform evidence completed or explicitly external.
- [ ] Small coherent implementation preserves hot paths and unrelated work.
- [ ] Focused, domain, security, performance/resource, native, visual, and full gates run as applicable.
- [ ] Results re-audited; failures fixed and affected checks rerun.
- [ ] Guides, references, architecture/ADR, testing, roadmap/audit, navigation, and changelog updated as applicable.
- [ ] Final claims distinguish local evidence from external validation.
- [ ] Authorized changes are grouped, DCO-signed, pushed, and remote-verified.
