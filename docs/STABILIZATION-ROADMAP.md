# Stabilization roadmap

## Scope

This roadmap tracks functional correctness, responsiveness, performance proof,
runtime security, cross-platform verification, and stable-release readiness.
It complements the [product roadmap](ROADMAP.md), the
[readiness audit](READINESS-AUDIT.md), and the separate
[Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md).

Status terms are evidence-based:

- **Complete locally** means the implementation and its focused tests pass on
  the available Windows workstation.
- **Partially complete** means the implementation is useful but still needs a
  source improvement, native-host evidence, measurement, or human review.
- **Planned** means source work remains.
- **External gate** means completion depends on credentials, hosted policy,
  another operating system, controlled hardware, or elapsed baseline time.

## Completed local correctness work

These items are regressions to preserve, not open implementation tasks.

| Area | Status | Preserved contract |
|---|---|---|
| OSC metadata snapshot architecture | Complete locally | Shell/prompt readiness and semantic metadata are present in renderer snapshots; the architecture gate passes. |
| PowerShell command submission | Complete locally | PSReadLine 2.0 delegate/script-block semantics are preserved instead of coercing the handler result to Boolean, so Enter does not hang. |
| PowerShell history controls | Complete and measured locally | Up Arrow measured 55 ms shell/VT latency and 110 ms end to end; raw `Ctrl+R` measured effectively 0 ms shell/VT latency and 64 ms end to end. Classic defaults expose it through `Ctrl+Alt+R` because bare `Ctrl+R` clones. |
| ConPTY mode 9001 input | Complete locally | Native tests inject full Win32 keyboard records with virtual key, scan code, modifiers, Unicode, and key-down/key-up state. |
| Resize delivery | Complete locally | Duplicate dimensions are discarded, adjacent resizes coalesce, and input/shutdown barriers preserve final-size ordering. |
| Prompt/path resize recovery | Complete on Windows | Deterministic storms, 2,000 grid transitions, and native Windows GUI storms preserve and restore prompt context without user input. |
| Stress snapshot publication | Complete locally | Test snapshots are staged and atomically replaced; readers cannot observe a deliberately truncated intermediate file. |
| Session cloning and tab isolation | Complete locally | Clones and pane-local tabs own independent PTYs, routes, PIDs, histories, input queues, directories, profiles, and WSL identity; close operations remain isolated. |
| PowerShell listing icons | Complete locally | Icons are adjacent to names while native `DirectoryInfo`/`FileInfo` values, filtering, sorting, pipelines, Unicode, spaces, and special names are preserved. |
| Stale shell identity removal | Complete for covered cases | PowerShell and CMD clear stale WSL facts; explicit session metadata, not window-title inference, owns shell identity. |
| Initial context publication | Complete for tested shells | Discovery publishes before waking the exact originating route, so initial context no longer requires a keystroke. |
| Link activation TOCTOU protection | Complete locally | Mouse-down latches the exact visible link text/range and release opens only that unchanged match. |
| Session/worker isolation | Complete for covered cases | Route/session identities isolate context, caches, worker results, input, PTYs, and terminal history. |
| Configuration migration safety | Complete for covered cases | Migration is narrow, atomic, repeatable, conflict-aware, and never copies logs, executable content, or unknown data. |
| v0.4 extension boundary | Complete for current scope | Only built-in reviewed extensions are enabled; downloaded/network-capable extension distribution remains outside v0.4. |
| Native test control surface | Complete | The automation control surface is feature-gated and absent from production builds. |
| Documentation correction | Complete | Architecture and testing docs now state that PowerShell formatting/colors are installed synchronously before the first editable prompt. |

## Partially complete areas

| Area | Current evidence | Remaining decision or evidence |
|---|---|---|
| Prompt/resize resilience across platforms | Deterministic engine/layout tests and native Windows GUI storms pass. | Run native Linux X11/Wayland and macOS GUI storms, including HiDPI, multiple panes, shell repaint, and rapid direction changes. |
| Docker/Kubernetes/cloud freshness | Discovery is asynchronous, session-scoped, cached, and refreshed automatically. | Keep bounded periodic refresh as the portable baseline. Add event/file watchers only where a reliable provider API exists, retain periodic reconciliation, expose truthful stale/unavailable state, and measure external CLI delays. Do not promise an instantaneous event stream that providers cannot supply. |
| Ghostty compatibility | The audited comparison and roadmap are documented; Ghostty bindings are no longer implicit defaults. | Implement a typed, versioned, explicit opt-in profile and missing action families before claiming compatibility. |
| Visual quality | Geometry, alignment, contrast, hit targets, narrow layouts, 4K/8K-equivalent sizes, and icon/font coverage have automated tests. | Perform recorded human review on Windows, macOS, Linux X11/Wayland, HiDPI, light/dark themes, and representative fallback fonts. “Premium” is a review outcome, not an automatable correctness claim. |
| Product identity | Normal identity and package checks pass. | Remove the XTGETTCAP `TN`/name response `rio`, return the canonical Automexia terminal identity, and extend the identity verifier to cover the protocol response. |
| Renderer and row-rebuild performance | No-damage frames return early; row/style/extras allocations are reused; dirty-row rebuilding is tested. | Record before/after frame-time, CPU, allocation, and memory results on representative small, split, 4K, and 8K workloads. |
| OSC/APC/DCS parser performance | Bulk slice scanning has parity tests and Criterion coverage. | Record optimized native baselines and compare throughput/latency before declaring a measured improvement. Parser speed work does not replace the control-string memory limits below. |
| Render-thread isolation | PTY parsing, context discovery, and extension work are off the render thread; queues/caches are bounded. | Keep architecture checks and add controlled saturation measurements proving input/render latency under worker and PTY pressure. |
| New-session startup | Equivalent context can seed immediately and the originating route wakes directly. | Benchmark process launch, shell integration, first prompt, synchronous PowerShell formatting, first context, and first editable input separately on cold and warm starts. |
| Build storage | `cargo ready` uses and removes an isolated verification target; `cargo storage` and `cargo purge` are available. | Document that arbitrary direct Cargo invocations own their persistent artifacts. Track target-size budgets and improve shared caches where safe, but do not claim Automexia can automatically clean artifacts created outside its workflow. |
| Overall performance assurance | History and resize interaction have strong Windows measurements. | Accumulate the required 30-day baseline; afterward require a waiver for regressions above 5% latency or 10% memory. Include startup, sustained PTY throughput, reflow, idle/scrollback memory, and context refresh. |
| Complete security assurance | Dependency policy and focused local protections pass. | Hosted CodeQL, fuzz, sanitizers, Miri, cross-platform jobs, signed artifacts, SBOMs, checksums, and provenance attestations must pass on their declared infrastructure. |
| Rendered-frame assurance | Layout and renderer-neutral JSON invariants are strong, but no automated test captures and compares the final painted WGPU frame. | Add deterministic offscreen/native frame capture, semantic and raster goldens, reviewable diffs, and controlled-runner evidence. Until then, a visually misplaced element can pass logical tests. |
| Test orchestration and evidence | Cargo runs the workspace tests successfully on three operating systems. | Add pinned Nextest profiles, explicit timeouts, shared-resource groups, child-process leak detection, JUnit output, and one redacted QA evidence bundle. Retain Cargo doctests separately. |
| Benchmark enforcement | Criterion cases exist, but the nightly workflow currently invokes `cargo bench --no-run`; it proves compilation only. | Execute benchmarks on named controlled hardware, retain reports, compare against an accepted baseline, and apply the 30-day ratchet below. |
| Property/concurrency assurance | Fixed-seed resize storms and ASan/TSan/Miri jobs cover important cases. | Add shrinking Proptest state machines and bounded Loom models for resize queues, snapshot publication, cache/worker ownership, and shutdown. Move broader models into pure v0.5 crates. |
| Accessibility | Contrast, keyboard actions, responsive scaling, and visible labels have focused tests, but there is no accessibility tree or assistive-technology contract. | Complete the v0.4 keyboard/focus/contrast/scaling baseline and recorded screen-reader smoke; design the renderer-independent AccessKit model for v0.5 before claiming full accessibility. |
| Native resource lifetime | PTY/PID isolation and process exit are tested. | Measure handles, threads, private bytes, working set, child processes, and GPU resources across repeated open/clone/close/resize cycles; run the Windows binary under Application Verifier on controlled infrastructure. |
| Test strength and supply chain | Changed Automexia-owned lines require 80% coverage; cargo-deny, dependency review, CodeQL, SBOMs, and attestations are defined. | Add an owned-code baseline, longer persisted fuzz corpora, then scoped mutation testing and maintainable cargo-vet adoption in v0.5. Do not add redundant advisory scanners without a distinct contract. |

## Required v0.4 source work

### S0 — protocol identity

1. Replace XTGETTCAP `TN`/name output `rio` with Automexia's canonical terminal
   name.
2. Add unit tests for both recognized capability names and unknown queries.
3. Extend `cargo xtask verify identity` so a future inherited protocol name
   fails the normal contributor gate.
4. Confirm terminfo names, `TERM_PROGRAM`, executable identity, and XTGETTCAP
   remain internally consistent.

Exit gate: no user- or application-observable product identity returns Rio
outside the explicit provenance/private-engine allowlist.

### S0 — bounded control strings

Treat all PTY output as untrusted, including output from local programs, SSH,
containers, multiplexers, and WSL.

1. Define reviewed hard limits for OSC raw accumulation, APC/graphics payloads,
   XTGETTCAP requests, and any equivalent DCS accumulation path.
2. On overflow, enter a bounded discard state until BEL/ST or the protocol's
   terminator; do not retain or repeatedly reallocate the rejected payload.
3. Recover parser state deterministically after termination, cancellation,
   malformed UTF-8, split input, and EOF.
4. Emit rate-limited diagnostics without echoing hostile payload contents.
5. Add exact-limit, limit-plus-one, unterminated, fragmented, cancellation,
   recovery, repeated-attack, and memory-bound tests.
6. Fuzz each accumulator and mixed control-string streams; assert bounded
   allocation, no panic, and correct parsing after recovery.
7. Benchmark normal short sequences to prove the defensive checks do not cause
   a meaningful hot-path regression.

Exit gate: no unterminated or oversized OSC/APC/DCS/XTGETTCAP stream can cause
unbounded memory growth, and valid text/control input following the discarded
sequence is processed correctly.

### S0 — atomic last-known-good reload

1. Parse and validate the complete candidate configuration before mutation.
2. Retain the current configuration, bindings, fonts, colors, global hotkeys,
   and per-window state after any read, parse, validation, or preparation error.
3. Prepare global registration changes before a single successful swap; never
   leave partially modified registrations.
4. Update all windows and palette metadata from the same accepted generation.
5. Add invalid, partially invalid, repeated, concurrent, font-failure,
   global-hotkey-failure, and recovery tests.

Exit gate: a failed reload changes no active behavior and reports actionable
diagnostics without recreating PTYs.

## Cross-platform and experiential gates

### S1 — native prompt and resize evidence

- Linux X11, Linux Wayland, and macOS run deterministic plus native GUI storms.
- Each job covers Bash/Zsh or platform shell editing, multiple panes/tabs,
  Unicode paths, output bursts, history navigation, minimize/restore, DPI or
  scale changes where supported, and extreme small/large dimensions.
- Inspect renderer-neutral snapshots and invariants rather than OCR alone.
- Preserve the final PTY/grid size, prompt generation, context, complete path,
  cursor validity, route isolation, and recovery after the viewport grows.

### S1 — native resource and hardware evidence

1. Extend the controlled Windows driver to record the process tree, handle
   count, thread count, private bytes, working set, child PTYs, and clean
   teardown before and after repeated tab/split/clone/close and resize cycles.
2. Establish stable idle and post-storm resource baselines; fail immediately
   on leaked child processes, monotonically growing handles/threads, or failure
   to release a pane's route, PTY, renderer snapshot, and GPU resources.
3. Add an administrator-only, controlled-runner profile that enables Windows
   Application Verifier for the exact staged `automexia.exe`. Run Basics,
   Heaps, Handles, Locks, and a separately bounded low-resource scenario.
   Always disable verifier state in a `finally` cleanup and upload its logs.
4. Record ETW through Windows Performance Recorder only for the performance
   profile or an automatic threshold failure. Store the ETL privately when it
   can contain paths/user data and publish a redacted summary.
5. Cover Intel, AMD, and NVIDIA on controlled Windows/Linux hardware where
   available, Windows RDP/software fallback, Linux X11/Wayland, and macOS
   Intel/Apple Silicon. Record GPU backend, adapter, driver, display, scale,
   power mode, and shell versions with every result.

Exit gate: native lifecycle tests leave no child process or material resource
growth, Application Verifier reports no enabled-layer failure, and every
published performance result identifies the hardware/software environment.

### S1 — visual acceptance

- Capture an approved screenshot matrix for default dark/light appearances on
  Windows, macOS, Linux X11, Linux Wayland, and HiDPI.
- Review icons at fallback-font boundaries, tab/pane selection, command palette,
  file listings, context identities, paths, footers, and smallest usable panes.
- File objective defects separately from aesthetic preferences. Automated tests
  remain authoritative for geometry, contrast, hit targets, and containment;
  maintainers own the aesthetic release decision.

### S1 — rendered-frame regression automation

1. Add an opt-in `visual-test-hooks` feature that is absent from production
   builds and can freeze clock/animation state, inject deterministic shell and
   DevOps facts, wait for font/glyph/GPU readiness, and capture the actual final
   offscreen or presented frame.
2. Keep renderer-state and raster contracts separate. Serialize layout/draw
   state with reviewed structured snapshots and exact geometry assertions;
   capture PNG output for what the user sees. The footer bottom, pane seams,
   focus border, cursor, hit target, and content viewport must agree in both.
3. Use bundled fonts, a fixed theme, deterministic text, fixed clocks, and
   named 1.0/1.25/1.5/2.0/3.0 scale profiles. Cover 300x200, compact, normal,
   portrait, ultrawide, split, 4K, and 8K-equivalent layouts plus command
   palette, window tabs, pane-local tabs, prompts, listings, and footers.
4. Use the existing `image` crate for the first repository-owned difference
   implementation. Compare invariant geometry exactly and pixels with reviewed
   masks/tolerances on one pinned software/offscreen PR environment; never
   require byte-identical PNGs across unrelated GPUs or font engines.
5. Use `insta` with serialization/redaction support for reviewable state
   snapshots. On failure, retain expected, actual, difference heatmap, state
   JSON, environment manifest, and logs as one artifact.
6. Require golden changes to be explicit, human-reviewed, and accompanied by
   the affected viewport/theme captures. A blanket snapshot acceptance command
   may not run in CI or during ordinary builds.
7. Run a small deterministic structural/offscreen set on every PR, the native
   OS/GPU/theme/DPI matrix nightly, and the approved representative matrix
   before release.

Exit gate: moving, clipping, overlapping, or incorrectly scaling a painted UI
element produces a focused state or image diff, including the previously
observed case where a logically valid pane footer was painted above the bottom.

### S1 — accessibility baseline

1. Inventory every custom-rendered interactive element and document its label,
   role, state, focus behavior, shortcut, pointer target, and keyboard-only
   equivalent. Decorative icons must not be the sole source of meaning.
2. Extend automated checks for focus visibility/order, hidden hit targets,
   default and customized theme contrast, non-color identity cues, 200% text
   scaling, reduced-motion behavior, and reflow without lost functionality.
3. Record Narrator and NVDA smoke evidence on Windows, VoiceOver on macOS, and
   AT-SPI/Orca on Linux for the semantics currently exposed by the platform.
   Document unsupported terminal-content semantics rather than claiming them.
4. Write an ADR for the v0.5 renderer-independent accessibility model and
   AccessKit/winit adapter. Runtime dependencies enter only after roles, text
   privacy, focus ownership, update frequency, platform fallbacks, and testing
   boundaries are reviewed.

Exit gate for v0.4: all functionality remains keyboard-operable, focus and
contrast/scaling contracts pass, and known screen-reader limitations are
published with native smoke evidence. Full accessibility remains a v0.5 gate.

### S1 — context freshness

- Record discovery age, source, availability, and last error per identity.
- Refresh immediately after prompt/cwd/profile changes and through the bounded
  steady reconciliation interval.
- Add reliable filesystem watchers for Git/Kubernetes/cloud config only when
  they reduce latency without making correctness depend on watcher delivery.
- Keep external CLI calls off the render/input path, bounded by timeout and
  session identity, and retain the previous truthful snapshot on transient
  provider failure.

## Verification infrastructure plan

### Planned tool and dependency manifest

| Item | Phase and form | Required contract |
|---|---|---|
| `cargo-nextest` | v0.4 S1 pinned CI/contributor tool | Profiles, JUnit, timeouts, leak checks, resource groups, and explicit flaky-result failure; Cargo doctests remain separate. |
| `insta` plus optional reviewer-side `cargo-insta` | v0.4 S1 development dependency/tool with structured serialization and redaction | Reviewable renderer/layout/accessibility snapshots; CI may check but never bulk-accept new goldens. |
| `proptest` | v0.4 S1 development dependency | Shrinking Unicode, viewport, DPI, prompt, pane/tab/session, and event-sequence state machines with persisted failures. |
| `loom` | v0.4 S1 `cfg(loom)`-scoped dependency, expanded in v0.5 | Finite models only for synchronization abstractions; never a normal product dependency or platform/GPU substitute. |
| Existing `image` crate | v0.4 S1 reused dependency | Decode/capture/difference implementation with exact geometry and controlled tolerance; add no image-diff crate until a measured need survives review. |
| Windows Application Verifier and WPT/WPR | v0.4 S1 controlled-host tools | Administrator-only native heap/handle/lock/resource/performance profiles with guaranteed cleanup and redacted artifacts. |
| AccessKit and its winit adapter | v0.5 reviewed runtime dependencies | Renderer-independent roles/text/focus/actions, privacy and update rules, native adapters, and assistive-technology tests. |
| `cargo-mutants` | v0.5 pinned weekly tool | Scoped Automexia-owned pure modules, survivor triage, time budgets, and informational rollout before any threshold. |
| `cargo-vet` | v0.5 pinned governance tool | Named audit owner, imported-audit trust, criteria/exemptions/renewal policy; complements existing dependency controls. |

Every added crate/tool must have a pinned version, license/source/advisory
review, lockfile or installer provenance, minimal enabled features, documented
host support, update owner, and `cargo xtask doctor` classification. Do not add
a tool merely because it is popular; it must close one of the contracts above.

### S1 — Nextest, JUnit, and deterministic ownership

1. Pin `cargo-nextest` in CI/tool bootstrap and commit
   `.config/nextest.toml` with local, CI, and deep-test profiles.
2. Define global and per-test slow/termination timeouts. Serialize native GUI,
   PTY/ConPTY, shell-profile, environment-mutating, and shared-file tests
   through named test groups rather than relying on accidental test order.
3. Configure child-process leak timeouts. If a retry is used to diagnose
   nondeterminism, mark a retry success as flaky and fail the CI job; retries
   must never silently convert an unreliable test into a pass.
4. Run unit/integration targets through Nextest and upload JUnit from Windows,
   Linux, and macOS. Continue `cargo test --doc` because Nextest does not own
   Cargo's documentation-test contract.
5. Teach `cargo xtask doctor` to distinguish required PR tools from optional
   native/deep tools and print exact installation guidance. Product launch
   commands must never install QA tools or mutate machine-wide verifier state.

Exit gate: every test has an owner, timeout, and report; mutually exclusive
native resources cannot run concurrently; hangs, leaks, and flaky retries are
visible failures rather than lost console output.

### S1 — one-command QA evidence

Add the proposed non-production commands:

```text
cargo xtask qa --full
cargo xtask qa --full --bundle
```

`--full` composes the applicable deterministic, visual, property, performance,
security, and native checks for the current host. `--bundle` writes a bounded
`target/qa/<run-id>/` report plus a ZIP containing:

- commit, dirty-worktree hash, toolchain, OS, shell, WSL, display, DPI, GPU,
  driver, backend, and power-profile identity;
- command lines, start/end times, durations, exit status, JUnit, coverage,
  benchmark comparisons, renderer state, screenshots/diffs, resource counts,
  and available sanitizer/AppVerifier summaries;
- an HTML index identifying passed, failed, skipped, unsupported, and
  externally required checks without calling skipped work successful.

Allowlist diagnostic fields and redact tokens, environment values, clipboard,
terminal contents outside explicit fixtures, user secrets, and private paths.
Cap logs/artifacts, use atomic writes, clean temporary capture state, and keep
the normal working tree unchanged. `cargo ready` remains the deterministic
contributor gate and `cargo automexia` remains the fast launch path.

Exit gate: a maintainer can reproduce a reported failure from one redacted
bundle and verify exactly which host-specific checks did or did not run.

### S1 — generated, property, model, and fuzz coverage

1. Add `proptest` as a development dependency for Unicode paths, dimensions,
   scale factors, split trees, tab/session lifecycle, prompt generations, and
   interleaved input/output/resize sequences. Assert complete pane tiling,
   footer-at-bottom geometry, cursor/grid validity, preserved history/path,
   route isolation, bounded state, and cleanup.
2. Persist minimized Proptest regressions. PR profiles use deterministic seeds
   and bounded case counts; nightly profiles expand seeds/cases and publish the
   smallest failing operation sequence.
3. Add `loom` only behind `cfg(loom)` for isolated resize coalescing, snapshot
   publication, context cache/worker queue, completion wake-up, and shutdown
   models. Do not pull window, PTY, filesystem, or GPU FFI into a Loom model.
4. Keep ASan/TSan and Miri because Loom does not replace runtime/native checks.
   During v0.5 extraction, move pure state machines into private crates so Loom
   and Miri cover more owned code without unsupported platform calls.
5. Seed fuzzers from conformance fixtures, persist minimized crashes, add weekly
   20-30 minute campaigns in addition to nightly smoke, and track corpus/edge
   coverage for parser accumulators, prompt reflow, configuration migration,
   semantic classification, and label/path sanitization.
6. Retain the non-decreasing whole-workspace coverage baseline and 80% changed
   Automexia-owned-line requirement. Add a separately recorded owned-code
   baseline and informational region/branch coverage before setting a ratchet.

Exit gate: generated failures are reproducible and minimized; concurrency
models cover reviewed finite state; fuzz corpora and owned coverage never
silently reset or regress.

## Performance proof plan

### S1 — measurement matrix

Measure optimized builds on named hardware and power settings:

- input-to-PTY and input-to-visible-response latency for ordinary typing,
  Enter, Up Arrow, `Ctrl+R`, paste, palette actions, and pane/tab shortcuts;
- cold/warm process launch, shell readiness, first editable prompt, first
  context snapshot, and first listing;
- parser throughput and CPU for plain text, mixed CSI/OSC/DCS/APC, Unicode,
  graphics, and adversarial-but-bounded streams;
- frame time, CPU, allocations, and GPU upload work for idle, scrolling,
  selection/search, prompt repaint, resize/reflow, multiple panes, and 4K/8K;
- idle, deep-scrollback, graphics, many-pane, and long-session memory;
- context discovery cold/warm latency and refresh cost with provider tools
  present, slow, missing, or disconnected;
- persistent and isolated build target growth for canonical workflows.

History/resize results already provide a Windows baseline; do not generalize
those numbers to unmeasured platforms or subsystems.

### S1 — executable benchmark pipeline

The current nightly `--no-run` command is only a compile check. Replace it with
a controlled execution and comparison pipeline:

1. Keep a fast benchmark compilation check where useful, but execute the
   selected Criterion suites in optimized builds on named, stable runners.
2. Preserve Criterion estimates/reports and a machine manifest. Compare a PR or
   nightly result with the accepted `main` baseline from the same runner class;
   never compare unrelated hardware as if it were a regression.
3. Add end-to-end probes for process spawn, first window, shell readiness, first
   editable prompt/context/listing, Up Arrow, reverse history, typing, paste,
   pane/tab actions, resize-settle, and context refresh.
4. Measure resource deltas across idle, deep scrollback, graphics, repeated
   pane lifecycle, sustained PTY throughput, and long resize/output storms.
5. Upload raw reports and a concise comparison artifact. Trigger WPR/ETW capture
   on controlled Windows only when a threshold fails or a maintainer requests
   diagnosis; do not collect private terminal data in routine traces.
6. Keep all results informational during the first complete 30-day baseline.

Exit gate: the workflow measures rather than merely compiles every benchmark
named in release claims, and each value is traceable to code, runner, power
mode, toolchain, and accepted baseline.

### S2 — enforcement

For the first 30 days, collect results without blocking merges unless a clear
regression or correctness failure appears. After the baseline period:

- latency regressions above 5% require a recorded maintainer waiver;
- memory regressions above 10% require a recorded maintainer waiver;
- benchmark noise must be controlled through repeated samples and documented
  machine/power/environment metadata;
- native nightly results inform the release gate; deterministic correctness
  tests remain mandatory on pull requests.

## v0.5 assurance maturation

- Move renderer-independent UI, keybinding, queue, cache, and lifecycle state
  into the planned private crates; expand Loom/Miri without platform FFI.
- Implement the reviewed AccessKit model and platform adapter, then test roles,
  names, states, actions, focus, text privacy, incremental updates, and native
  assistive technologies before claiming full accessibility.
- Run `cargo mutants` only against deterministic Automexia-owned pure modules,
  initially weekly and informational. Exclude generated, platform FFI, GPU, and
  timing-sensitive code; require reviewed time budgets and explicit survivor
  triage before using mutation score as a gate.
- Evaluate `cargo vet` only with a named audit owner, imported-audit trust
  policy, criteria, exemptions, renewal rules, and pull-request workflow.
  It complements cargo-deny/advisories/licenses/sources, CodeQL, dependency
  review, SBOMs, and attestations; it does not replace them.
- Consider unused-dependency analysis as an informational maintenance job with
  platform/build/feature false-positive review, not an automatic manifest
  rewriter.

## Tooling decisions and non-goals

| Decision | Reason |
|---|---|
| Use existing `image` plus structured snapshots before adding an image-diff crate | Keep the dependency surface small; exact geometry plus controlled tolerant pixel comparison covers the known failure class. |
| Keep Cargo doctests alongside Nextest | Nextest improves orchestration but does not replace Cargo's documentation-test behavior. |
| Keep ASan/TSan/Miri alongside Loom | Model checking and runtime instrumentation detect different bug classes and each has platform/model limitations. |
| Do not add `cargo audit` while cargo-deny owns the RustSec advisory gate | A second scanner would duplicate the current contract without closing a new gap. |
| Do not use Selenium/Playwright/Cypress or OCR as the primary native UI oracle | Automexia is a custom native WGPU UI; renderer state and captured frames are more direct and stable. |
| Do not compare PNGs byte-for-byte across unrelated GPUs/font engines | Native rasterization differences are expected; exact semantic geometry and controlled-runner image tolerances remain authoritative. |
| Do not hide flaky tests with retries | Diagnostic retries must still fail and report the test as flaky. |
| Do not put full QA in `cargo automexia` or ordinary startup | Fast launches and non-mutating contributor checks remain separate from expensive/native QA. |

## Release and delivery gates

Source completion does not satisfy these external requirements:

1. final brand sources, redistribution-rights evidence, and asset-manifest
   approval;
2. a private conduct-reporting contact;
3. Windows Authenticode and Apple Developer ID/notarization credentials;
4. protected `main`, required reviews/CODEOWNERS, DCO, squash-only merging,
   administrator applicability, and private vulnerability reporting;
5. successful hosted Windows, Linux, macOS, ARM64, CodeQL, dependency-review,
   fuzz, sanitizer, Miri, coverage, packaging, SBOM, checksum, and attestation
   jobs;
6. clean install/upgrade/uninstall, coexistence, migration, PTY, GPU, terminfo,
   desktop/URL-handler, signature, notarization, and Gatekeeper tests;
7. the completed 30-day performance baseline and controlled-hardware checklist;
8. reviewed, committed, pushed, and protected-branch-merged source. A local
   working tree or passing workstation build is not a delivered release.

The v0.4 assurance portion of those gates additionally requires successful
Nextest/JUnit execution with Cargo doctests retained, controlled rendered-frame
and native resource evidence, a redacted QA bundle, executed benchmark reports
for the complete 30-day baseline, the owned-code coverage record, and the
documented keyboard/focus/contrast/scaling plus screen-reader baseline. A
compile-only benchmark job or renderer-neutral JSON alone does not satisfy the
corresponding performance or visual gate.

Stable v0.4 remains blocked until all S0 work and applicable source gates pass,
then every external release gate is satisfied. S1 native/visual/performance
evidence may run in parallel but cannot be replaced by Windows-only results.

## Stable acceptance criteria

- all completed local contracts above remain covered by regressions;
- XTGETTCAP and every other product-observable identity are Automexia-owned;
- control-string memory remains bounded under hostile, fragmented, and
  unterminated input and parsing recovers afterward;
- invalid reloads retain the entire last known-good live state;
- native Windows, Linux X11/Wayland, and macOS prompt/resize suites pass;
- context freshness is truthful and never blocks rendering/input;
- the visual matrix receives maintainer approval on supported platforms;
- the performance baseline and regression policy are active;
- dependency/security policy and hosted assurance jobs pass;
- signed/notarized artifacts, SBOMs, checksums, and attestations validate;
- the release commit is reviewed, protected, reproducible, and traceable;
- deterministic renderer state and controlled rendered-frame captures agree,
  visual changes have reviewed expected/actual/diff evidence, and no footer,
  focus border, cursor, prompt, or overlay is misplaced after resize;
- Nextest/JUnit, timeout, shared-resource, child-process leak, Cargo doctest,
  Proptest, bounded Loom, extended fuzz, and owned-coverage contracts pass;
- repeated native lifecycle tests release PTYs, routes, processes, handles,
  threads, memory, and GPU resources, and Windows AppVerifier is clean;
- the redacted QA bundle identifies every pass, failure, skip, host limitation,
  environment, visual artifact, benchmark, and resource result;
- v0.4 keyboard/focus/contrast/scaling and recorded native screen-reader
  baselines pass without claiming the deferred v0.5 accessibility tree.
