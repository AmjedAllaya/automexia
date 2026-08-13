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
| PowerShell history controls | Complete and measured locally | Up Arrow measured 55 ms shell/VT latency and 110 ms end to end; raw `Ctrl+R` measured effectively 0 ms shell/VT latency and 64 ms end to end. Bare `Ctrl+R` remains shell-owned. |
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
| Ghostty compatibility | Every pinned default with an available equivalent action is mapped and collision-tested. | Implement the typed profile system and missing action families in the dedicated compatibility roadmap. |
| Visual quality | Geometry, alignment, contrast, hit targets, narrow layouts, 4K/8K-equivalent sizes, and icon/font coverage have automated tests. | Perform recorded human review on Windows, macOS, Linux X11/Wayland, HiDPI, light/dark themes, and representative fallback fonts. “Premium” is a review outcome, not an automatable correctness claim. |
| Product identity | Normal identity and package checks pass. | Remove the XTGETTCAP `TN`/name response `rio`, return the canonical Automexia terminal identity, and extend the identity verifier to cover the protocol response. |
| Renderer and row-rebuild performance | No-damage frames return early; row/style/extras allocations are reused; dirty-row rebuilding is tested. | Record before/after frame-time, CPU, allocation, and memory results on representative small, split, 4K, and 8K workloads. |
| OSC/APC/DCS parser performance | Bulk slice scanning has parity tests and Criterion coverage. | Record optimized native baselines and compare throughput/latency before declaring a measured improvement. Parser speed work does not replace the control-string memory limits below. |
| Render-thread isolation | PTY parsing, context discovery, and extension work are off the render thread; queues/caches are bounded. | Keep architecture checks and add controlled saturation measurements proving input/render latency under worker and PTY pressure. |
| New-session startup | Equivalent context can seed immediately and the originating route wakes directly. | Benchmark process launch, shell integration, first prompt, synchronous PowerShell formatting, first context, and first editable input separately on cold and warm starts. |
| Build storage | `cargo ready` uses and removes an isolated verification target; `cargo storage` and `cargo purge` are available. | Document that arbitrary direct Cargo invocations own their persistent artifacts. Track target-size budgets and improve shared caches where safe, but do not claim Automexia can automatically clean artifacts created outside its workflow. |
| Overall performance assurance | History and resize interaction have strong Windows measurements. | Accumulate the required 30-day baseline; afterward require a waiver for regressions above 5% latency or 10% memory. Include startup, sustained PTY throughput, reflow, idle/scrollback memory, and context refresh. |
| Complete security assurance | Dependency policy and focused local protections pass. | Hosted CodeQL, fuzz, sanitizers, Miri, cross-platform jobs, signed artifacts, SBOMs, checksums, and provenance attestations must pass on their declared infrastructure. |

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

### S1 — visual acceptance

- Capture an approved screenshot matrix for default dark/light appearances on
  Windows, macOS, Linux X11, Linux Wayland, and HiDPI.
- Review icons at fallback-font boundaries, tab/pane selection, command palette,
  file listings, context identities, paths, footers, and smallest usable panes.
- File objective defects separately from aesthetic preferences. Automated tests
  remain authoritative for geometry, contrast, hit targets, and containment;
  maintainers own the aesthetic release decision.

### S1 — context freshness

- Record discovery age, source, availability, and last error per identity.
- Refresh immediately after prompt/cwd/profile changes and through the bounded
  steady reconciliation interval.
- Add reliable filesystem watchers for Git/Kubernetes/cloud config only when
  they reduce latency without making correctness depend on watcher delivery.
- Keep external CLI calls off the render/input path, bounded by timeout and
  session identity, and retain the previous truthful snapshot on transient
  provider failure.

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

### S2 — enforcement

For the first 30 days, collect results without blocking merges unless a clear
regression or correctness failure appears. After the baseline period:

- latency regressions above 5% require a recorded maintainer waiver;
- memory regressions above 10% require a recorded maintainer waiver;
- benchmark noise must be controlled through repeated samples and documented
  machine/power/environment metadata;
- native nightly results inform the release gate; deterministic correctness
  tests remain mandatory on pull requests.

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
- signed/notarized artifacts, SBOMs, checksums, and attestations validate; and
- the release commit is reviewed, protected, reproducible, and traceable.
