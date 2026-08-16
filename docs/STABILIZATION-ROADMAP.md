# Stabilization roadmap

## Scope

This roadmap tracks functional correctness, responsiveness, performance proof,
runtime security, cross-platform verification, and stable-release readiness.
It complements the [product roadmap](ROADMAP.md), the
[readiness audit](READINESS-AUDIT.md), and the separate
[Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md). The v0.5
first-party SSH and multi-cloud work is specified here as executable stages and
uses the research and architecture in
[SSH, DevOps, and multi-cloud extension architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md).

Status terms are evidence-based:

- **Complete locally** means the implementation and its focused tests pass on
  the available Windows workstation.
- **Partially complete** means the implementation is useful but still needs a
  source improvement, native-host evidence, measurement, or human review.
- **Planned** means source work remains.
- **External gate** means completion depends on credentials, hosted policy,
  another operating system, controlled hardware, or elapsed baseline time.

## Phase 0 execution ledger

This ledger distinguishes source completion from evidence that requires a
controlled host, another operating system, credentials, or elapsed time. A
skipped external gate is never reported as a pass.

| Phase 0 step | Audit before this pass | Implemented or preserved now | Remaining gate |
|---|---|---|---|
| Protocol identity and bounded hostile control strings | Complete locally | Existing hard caps, cancellation/discard/recovery, non-payload diagnostics, architecture checks, seven fuzz targets, and exact-boundary regressions are preserved. | Hosted fuzz, sanitizer, Miri, and cross-platform hostile PTY evidence. |
| Terminal image protocols and quick look | Missing direct local filename UX; iTerm2 decode was less bounded than Kitty/Sixel | Sixel, Kitty placements/placeholders, and iTerm2 inline images are preserved. IO-free visible-path discovery, 100 ms plain hover, click-to-pin, wraparound arrow browsing, Escape dismissal, mouse-reporting ownership, selection/palette access, a 16-owner latest-generation queue, strict predecode dimension/magic/file-version gates, a 16-entry/32 MiB LRU, shared stable renderer identity, WSL validation, responsive overlay, and tested decoder limits are implemented. The required focused PR command exercises every enabled codec and renderer/protocol suite. Windows native automation drives real hover/click/key events, repeats exact WGPU/CPU resource lifecycle checks, enforces process-growth ceilings, rejects blank output, and compares backend capture distributions. | Controlled native visual/protocol evidence on Linux and macOS; retain extended fuzz/sanitizer soak and multiplexer/client compatibility evidence. |
| Reviewed local-only v0.4 extension boundary | Complete locally | Capability manifest remains filesystem/environment/terminal-output/UI-overlay only; architecture policy continues to reject network, process-spawn, and clipboard grants. | Hosted policy/security checks must pass before release. |
| Managed remote-session claims | Correctly absent | Documentation and release gates continue to forbid advertising managed remote sessions in v0.4. | v0.5 capability model plus hosted hostile-output and native evidence. |
| Provider status behavior | Complete locally for the provider-neutral contract | Versioned provider snapshots now carry source revision, observation time, and truthful current/refreshing/stale/disabled/unavailable/error state through the renderer and non-color accessibility labels. Session isolation, bounded cache, periodic refresh, and last-known-good error behavior remain enforced. | Retain controlled provider/CLI latency and failure evidence; no false instantaneous-live claim. |
| Deterministic test ownership | Partial | Pinned Nextest 0.9.137 profiles, JUnit, no silent flaky success, timeouts, leak policy, a serialized PTY group, separate Cargo doctests, QA-runner self-tests, and accurate QA-required/optional `doctor` classifications are now in PR CI. | Three-host CI evidence must be retained. |
| Property, state, and concurrency coverage | Partial | 512-case shrinking viewport/DPI properties, a persisted minimized regression, reviewed footer state snapshot, finite channel publish/wake models, and owned runtime models for cancellation, coalescing, exact-route publication, last-known-good preservation, and shutdown are now present. | Add pure resize-queue and atomic snapshot-replacement models; continue expanding only reviewed finite state spaces. |
| One-command evidence | Missing | `cargo qa` / `cargo xtask qa --full [--bundle]` now self-tests and enforces per-step deadlines, whole-process-tree cleanup, exact 2 MiB logs, 16 MiB files, a 64 MiB bundle, escaped-path/token redaction, allowlisted host identity, atomic JUnit/resource/coverage summaries, and explicit external skips. Private ETL, raw LCOV, and live-terminal PNG captures are excluded. | Run on every controlled release host and retain required evidence. |
| Windows native resources | Partial | Native GUI stress samples handles, threads, private bytes, working set, descendants, and final painted-frame variation against explicit ceilings; QA retains the existing atomic structured report, while controlled AppVerifier and WPR wrappers guarantee cleanup. | Execute on the elevated GPU runner and review AppVerifier/WPR output. |
| Benchmark execution | Compile-only | Compile-only coverage remains explicit; a separate named self-hosted job executes Criterion through the QA runner. | Complete and ratify the 30-day baseline before enforcement. |
| Accessibility | Partial | The v0.4 inventory, automated baseline, manual matrix, limitations, and v0.5 renderer-independent model ADR are now documented. | Recorded Narrator/NVDA, VoiceOver, and Orca smoke; v0.5 native semantic tree. |
| Rendered-frame evidence | Partial | Structured renderer-state snapshots are reviewable and CI-checked. The Windows native storm now uses a bounded, temporarily topmost `ClientToScreen`/`BitBlt` capture of the actual composited client region, with a five-second presentation deadline and a painted-readiness gate before custom-window clicks. It rejects blank/single-color final frames, restores z-order in `finally`, retains pixels only with explicit `-FrameCapture`, and excludes live-terminal PNGs from portable QA bundles. | Add pinned offscreen expected/actual/diff goldens plus Linux/macOS native frame evidence across viewport, theme, font, and DPI matrices. |

## Completed local correctness work

These items are regressions to preserve, not open implementation tasks.

| Area | Status | Preserved contract |
|---|---|---|
| OSC metadata snapshot architecture | Complete locally | Shell/prompt readiness and semantic metadata are present in renderer snapshots; the architecture gate passes. |
| PowerShell command submission | Complete locally | PSReadLine 2.0 delegate/script-block semantics are preserved instead of coercing the handler result to Boolean, so Enter does not hang. |
| PowerShell history controls | Complete and measured locally | Repeated full native gates measured Up at 1.049-1.101 seconds and raw `Ctrl+R` at 0.972-1.024 seconds; a bare-ConPTY probe shows the same roughly one-second Windows PowerShell 5.1/PSReadLine 2.0 floor, while ordinary Automexia input remains below its separate 500 ms budget. `cargo xtask doctor` reports this advisory. Classic defaults expose history through `Ctrl+Alt+R` because bare `Ctrl+R` clones. |
| ConPTY mode 9001 input | Complete locally | Native tests inject full Win32 keyboard records with virtual key, scan code, modifiers, Unicode, and key-down/key-up state. |
| Resize delivery | Complete locally | Duplicate dimensions are discarded, adjacent resizes coalesce, and input/shutdown barriers preserve final-size ordering. |
| Prompt/path resize recovery | Complete on Windows | Immutable generation snapshots, scalar/ASCII/Unicode `aid` propagation, hard-newline prompt-ownership boundaries, stale repaint removal, final-resize repair without later PTY input, deterministic 2,000-transition storms, and repeated native Windows GUI storms preserve and restore prompt context without claiming later command output. |
| Stress snapshot publication | Complete locally | Test snapshots are staged and atomically replaced; readers cannot observe a deliberately truncated intermediate file. |
| Session cloning and tab isolation | Complete locally | Clones and pane-local tabs own independent PTYs, routes, PIDs, histories, input queues, directories, profiles, and WSL identity; close operations remain isolated. Multi-tab controls are contained inside their owning pane, share one DPI-safe content geometry with the PTY/renderer/input stack, and do not reduce sibling panes. |
| PowerShell listing icons | Complete locally | Icons are adjacent to names while native `DirectoryInfo`/`FileInfo` values, filtering, sorting, pipelines, Unicode, spaces, and special names are preserved. |
| Stale shell identity removal | Complete for covered cases | PowerShell and CMD clear stale WSL facts; explicit session metadata, not window-title inference, owns shell identity. |
| Initial context publication | Complete for tested shells | Discovery publishes before waking the exact originating route, so initial context no longer requires a keystroke. |
| Link activation TOCTOU protection | Complete locally | Mouse-down latches the exact visible link text/range and release opens only that unchanged match. |
| Local image quick look | Complete locally at the source boundary | Plain hover discovers paths without IO and submits after 100 ms; click pins; arrows browse visible image paths; Escape dismisses; mouse-reporting children retain ownership unless Shift is held; selected-path/palette actions remain. Only local raster candidates reach the bounded decode/downscale worker; every enabled codec, straight alpha, malformed/mutated input, cache replacement, file-handle release, and no-sidecar behavior are tested. Work coalesces per window, file-version thumbnails stay within 32 MiB, and dismissal removes the route pixels, overlay, and matching GPU texture. Sixteen WGPU and CPU open/dismiss cycles assert exact active bytes/counts and zero remaining active resources, queue, or completion state, with process-growth ceilings and backend capture comparison. The explicit-nightly decoder/tokenizer fuzz runner is time/RSS bounded and uses disposable storage; the recorded 544,609-execution WSL campaign passed. Existing terminal image protocols remain independent. |
| Session/worker isolation | Complete for covered cases | Route/session identities isolate context, caches, worker results, input, PTYs, and terminal history. |
| Configuration migration safety | Complete for covered cases | Migration is narrow, atomic, repeatable, conflict-aware, and never copies logs, executable content, or unknown data. |
| v0.4 extension boundary | Complete for current scope | Only built-in reviewed extensions are enabled; downloaded/network-capable extension distribution remains outside v0.4. |
| Native test control surface | Complete | The automation control surface is feature-gated and absent from production builds. |
| Documentation correction | Complete | Architecture and testing docs now state that PowerShell formatting/colors are installed synchronously before the first editable prompt. |
| Protocol identity | Complete locally | XTGETTCAP `TN`/name returns `automexia`; capability tests and `verify identity` cover protocol and native dialog/error surfaces. |
| Bounded control strings | Complete locally | OSC, APC/graphics, and XTGETTCAP retain hard-capped payloads, discard oversized input through termination, cancel without dispatch on CAN/SUB, recover deterministically, and have exact-boundary, repeated-attack, memory-bound, recovery, and nightly-fuzz coverage. |
| Atomic runtime reload | Complete locally | Invalid config/theme/font/hotkey candidates preserve the complete last-known-good live state; hotkey registration replacement has addition/removal rollback regressions. |

## Partially complete areas

| Area | Current evidence | Remaining decision or evidence |
|---|---|---|
| Prompt/resize resilience across platforms | Deterministic engine/layout tests and native Windows GUI storms pass. | Run native Linux X11/Wayland and macOS GUI storms, including HiDPI, multiple panes, shell repaint, and rapid direction changes. |
| Docker/Kubernetes/cloud freshness | Discovery is asynchronous, session-scoped, bounded, cached, refreshed automatically, and projected with source revision, observation time, and truthful non-current states. | Keep bounded periodic refresh as the portable baseline. Add event/file watchers only where a reliable provider API exists, retain periodic reconciliation, and measure external CLI delays and failures. Do not promise an instantaneous event stream that providers cannot supply. |
| Ghostty compatibility | The audited comparison and roadmap are documented; Ghostty bindings are no longer implicit defaults. | Implement a typed, versioned, explicit opt-in profile and missing action families before claiming compatibility. |
| Visual quality | Geometry, alignment, contrast, hit targets, narrow layouts, 4K/8K-equivalent sizes, and icon/font coverage have automated tests. | Perform recorded human review on Windows, macOS, Linux X11/Wayland, HiDPI, light/dark themes, and representative fallback fonts. “Premium” is a review outcome, not an automatable correctness claim. |
| Renderer and row-rebuild performance | No-damage frames return early; row/style/extras allocations are reused; dirty-row rebuilding is tested. | Record before/after frame-time, CPU, allocation, and memory results on representative small, split, 4K, and 8K workloads. |
| OSC/APC/DCS parser performance | Bulk slice scanning has parity tests and Criterion coverage. | Record optimized native baselines and compare throughput/latency before declaring a measured improvement. Parser speed work does not replace the control-string memory limits below. |
| Render-thread isolation | PTY parsing, context discovery, and extension work are off the render thread; queues/caches are bounded. | Keep architecture checks and add controlled saturation measurements proving input/render latency under worker and PTY pressure. |
| New-session startup | Equivalent context can seed immediately and the originating route wakes directly. | Benchmark process launch, shell integration, first prompt, synchronous PowerShell formatting, first context, and first editable input separately on cold and warm starts. |
| Build storage | `cargo ready` uses and removes an isolated verification target; `cargo storage` and `cargo purge` are available. | Document that arbitrary direct Cargo invocations own their persistent artifacts. Track target-size budgets and improve shared caches where safe, but do not claim Automexia can automatically clean artifacts created outside its workflow. |
| Overall performance assurance | History and resize interaction have repeatable Windows measurements, and the shell/frontend latency boundary is now explicit. | Accumulate the required 30-day baseline; afterward require a waiver for regressions above 5% latency or 10% memory. Include startup, sustained PTY throughput, reflow, idle/scrollback memory, context refresh, and optimized PowerShell 7/current-PSReadLine measurements. |
| Complete security assurance | Dependency policy and focused local protections pass. | Hosted CodeQL, fuzz, sanitizers, Miri, cross-platform jobs, signed artifacts, SBOMs, checksums, and provenance attestations must pass on their declared infrastructure. |
| Rendered-frame assurance | Renderer-neutral JSON invariants and a topmost, client-region composited Windows capture now prove that a varied WGPU frame is painted after the full resize/clone/history/window lifecycle storm. The retained private frame was reviewed against the same four-pane snapshot. | Add pinned offscreen semantic/raster goldens, focused expected/actual/diff artifacts, and Linux/macOS controlled-runner evidence. The Windows smoke rejects blank frames but does not replace element-level image diffs. |
| Test orchestration and evidence | Pinned Nextest local/CI/deep profiles, explicit timeout/leak/flaky policy, serialized PTY ownership, JUnit, separate Cargo doctests, and a self-tested QA runner with subprocess deadlines, process-tree cleanup, privacy/size ceilings, allowlisted host identity, and structured native/coverage evidence are implemented. | Retain successful reports from Windows, Linux, and macOS and investigate every retry as a failure. |
| Benchmark enforcement | Criterion compilation remains a distinct nightly check, and a gated named-runner job now executes and retains Criterion through `cargo xtask qa --full --bundle`. | Execute it for 30 complete days, approve the baseline, then apply the ratchet below. |
| Property/concurrency assurance | Fixed-seed storms, shrinking viewport/DPI properties with persisted regressions, reviewed structured footer snapshots, finite channel readiness models, and owned runtime models for cache/worker cancellation, coalescing, exact-route publication, last-known-good preservation, and shutdown cover the first bounded invariants. ASan/TSan/Miri jobs remain hosted. | Add pure resize-queue and atomic snapshot-replacement models, then continue expanding only reviewed finite state spaces. |
| Accessibility | Contrast, keyboard actions, responsive scaling, visible labels, the custom-surface inventory, manual matrix, and v0.5 renderer-independent model ADR are documented and tested where automatable. | Record screen-reader smoke on all supported hosts; the native semantic tree remains a v0.5 gate. |
| Native resource lifetime | Native Windows stress records handles, threads, private bytes, working set, and descendant processes with ceilings; AppVerifier Basics and WPR wrappers target only staged `automexia.exe`, refuse conflicting state, and clean up in `finally`. | Execute and review these profiles on the elevated controlled GPU runner; retain only bounded/redacted reports and private trace manifests. |
| Test strength and supply chain | Changed Automexia-owned lines require 80% coverage; cargo-deny, dependency review, CodeQL, SBOMs, and attestations are defined. | Add an owned-code baseline, longer persisted fuzz corpora, then scoped mutation testing and maintainable cargo-vet adoption in v0.5. Do not add redundant advisory scanners without a distinct contract. |

## Completed v0.4 S0 source work (preserve)

The following source gates passed locally on 2026-08-14. They remain mandatory
regression contracts in `cargo ready`; hosted/native assurance is tracked under
S1 and the external release gates.

### S0 — protocol identity

- XTGETTCAP `TN` and `name` return `automexia`; recognized and unknown
  capability requests have unit coverage.
- `cargo xtask verify identity` covers the protocol response, native close/quit
  dialogs, initialization/config errors, terminfo, `TERM_PROGRAM`, executable,
  package, desktop, and bundle identities.
- Inherited Rio names remain only in the explicit provenance/private-engine
  boundary.

Local exit: passed. No covered user- or application-observable identity returns
Rio outside that boundary.

### S0 — bounded control strings

All PTY output remains untrusted, including local programs, SSH, containers,
multiplexers, and WSL.

- Raw OSC retention is capped at 1 MiB, APC/graphics retention at 96 KiB, and
  XTGETTCAP requests at 4 KiB. Sixel is streamed through its dimension-bounded
  decoder rather than retained as one raw DCS; synchronized updates retain the
  existing 2 MiB cap.
- Limit overflow retains no additional bytes and consumes through BEL/ST or the
  applicable terminator. CAN/SUB clears OSC, APC, DCS/XTGETTCAP, and Sixel state
  without dispatching a partial payload.
- Diagnostics never echo hostile payloads and use exponential rate limiting.
- The deterministic suite covers exact limit, limit plus one, unterminated
  state, fragmentation, cancellation, valid recovery, repeated attacks, and
  retained-memory bounds. The dedicated `control_string_bounds` fuzz target is
  routed through the nightly matrix.
- Normal short streams retain the inline/bulk fast paths and the existing VT
  Criterion cases. Executed controlled-hardware comparisons remain an S1
  performance-evidence gate, not an open memory-safety implementation task.

Local correctness/security exit: passed with 106 performer regressions. Hosted
fuzz corpora, sanitizers, cross-platform native jobs, and performance baselines
must still pass before stable release.

### S0 — atomic last-known-good reload

- Read/parse/theme/platform validation produces a complete candidate before any
  live mutation. Config, theme, and missing-path failures retain the active
  generation and report diagnostics.
- A replacement font library is prepared before config/window mutation; a
  missing requested family retains the current fonts and live windows.
- Global-hotkey triggers parse all-or-nothing. Additions register before
  removals; failed additions and removals roll back in reverse order, retain the
  logical last-known-good set, and report any operating-system rollback error.
- Only after every fallible preparation succeeds are the font library, config,
  bindings, palette metadata, and windows updated. PTYs are never recreated.
- Focused tests cover every load-error class, valid value preservation,
  deterministic missing-font rejection, duplicate/invalid hotkeys, failed and
  partially failed additions, failed removals, rollback, and recovery. Reload
  events are serialized by the application event loop, so two candidates cannot
  mutate live state concurrently.

Local exit: passed. An operating system that rejects both a hotkey operation and
its compensating rollback is reported as an explicit degraded external state;
no desktop hotkey API offers a true atomic transaction.
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
| System OpenSSH client | v0.5.0 external runtime dependency for `devops-ssh` | Platform-detected and version-reported; exact-argv PTY launch; Automexia never silently downloads or substitutes an SSH engine. |
| Mocked deterministic SSH server/fixtures | v0.5.0 test-only infrastructure | Hermetic host-key/auth/jump/tunnel/failure/control-string cases for PRs; controlled native OpenSSH/server evidence remains a release gate. |
| AWS/Azure/Google/Kubernetes/OpenShift official CLIs | v0.5.1 optional provider runtime dependencies | Detected lazily, invoked visibly or through exact reviewed argv, never installed at terminal startup, and absent tools degrade only their extension. |
| `keyring-rs`, `secrecy`, and `zeroize` | Deferred until a secret-reference/custody ADR proves a need | Defense in depth only; no dependency may turn Automexia into a plaintext or general-purpose credential vault. |
| Cedar or OPA adapter | v0.5.1/v0.6 policy evaluation after ADR | Local typed policy and enterprise integration; provider IAM/RBAC/remote policy remains authoritative. |

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

Current source status: implemented. `cargo xtask doctor` probes Cargo
subcommands through Cargo, distinguishes the pinned QA-required Nextest runner
from optional review/deep tools, and prints its exact install command without
installing or mutating the host.

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
  benchmark comparisons, renderer state, resource counts, and available
  sanitizer/AppVerifier summaries;
- an HTML index identifying passed, failed, skipped, unsupported, and
  externally required checks without calling skipped work successful.

Allowlist diagnostic fields and redact tokens, environment values, clipboard,
terminal contents outside explicit fixtures, user secrets, and private paths.
Cap logs/artifacts, exclude live-terminal PNGs and private ETL from portable
bundles, use atomic writes, clean temporary capture state, and keep
the normal working tree unchanged. `cargo ready` remains the deterministic
contributor gate and `cargo automexia` remains the fast launch path.

Exit gate: a maintainer can reproduce a reported failure from one redacted
bundle and verify exactly which host-specific checks did or did not run.

Current source status: implemented and self-tested. Every child has a recorded
start/end/duration/deadline; timeout terminates the Windows process tree or
POSIX process group. The portable ZIP has per-log, per-file, and total ceilings,
contains a bundle manifest, and retains path-free coverage plus native resource
summaries while excluding raw/private captures.

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
- v0.5.0 OpenSSH detection, bounded config indexing, quick-connect search,
  session/process creation, first remote prompt, jump/tunnel startup,
  cancellation, and teardown at 1/10/50 concurrent sessions;
- v0.5.1 capsule creation/rebind, provider CLI/config refresh, interactive login,
  expiry recovery, Kubernetes exec-plugin decisions, cloud-native remote
  transport startup, and mixed-provider 10/50/100-session cache/memory cost;
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

## Early DevOps and SSH delivery track

This track is the implementation authority for the first-party SSH and
multi-cloud work described in the [product roadmap](ROADMAP.md) and the
[consolidated architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md). The goal is
to ship useful, production-quality SSH in v0.5.0 rather than waiting for the
v0.6 public extension platform, while preserving Automexia as a generic
terminal.

### Ordering and parallelism

| Stage | May start | Must finish before exposure | User-visible result |
|---|---|---|---|
| D0 design and threat model | During late v0.4 | Before a process-capable build | Reviewed contracts and replacement ADR; no SSH UI. |
| D1 private contract extraction | After stable seams are identified | Before SSH implementation depends on them | No behavior change. |
| D2 generic context/status adapter | In parallel with D1 after types compile | Before provider extensions | Existing context with no renderer provider branches. |
| D3 exact-argv session launch | After D0/D1 | Before quick connect | Internal reviewed first-party capability only. |
| D4 OpenSSH index and connection model | After D1; parser work may overlap D3 | Before SSH palette/host UI | Safe host inventory with no key custody. |
| D5 production SSH UX | After D3/D4 and v0.4 control-string closure | v0.5.0 release | Responsive Connection Hub, reviewed quick connect, jumps, tunnels, agent/certificate visibility, and capability lifecycle. |
| D6 multi-cloud capsules/providers | After D2/D3 prove isolation | v0.5.1 release | Provider-native Hub journeys and isolated AWS/Azure/GCP/Kubernetes/OpenShift sessions; organization adapters remain independent. |
| D7 direct provider APIs/public SDK | After D6 measurement and policy | v0.6 or later | Optional inventory and sandboxed ecosystem. |

D0, pure D1 types, a non-executable D4 parser, fixtures, and tests may be built
while v0.4 assurance is running. The S0 bounded-control-string source gate now
passes locally, but D3/D5 exposure still requires hosted fuzz/sanitizer and
native security evidence. Remote feature work never weakens or bypasses the
remaining v0.4 native, packaging, hosted, signing, SBOM,
provenance, visual, accessibility, or performance gates.

## Command productivity delivery track

This parallel track closes the previously implicit autocomplete and reusable
DevOps-alias requirements. Its full product, architecture, security, performance,
data, shell-adapter, test, and acceptance contract is
[Command Productivity](COMMAND-PRODUCTIVITY.md), governed by
[ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md). It does
not authorize shell evaluation, provider access, or arbitrary process launch.
CP2/CP3 implementation details and exit evidence are canonical in
[DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md).

| Stage | Dependency | Required work | Exit evidence |
|---|---|---|---|
| CP0 decisions/threats | May run beside D5 design | Accept shell/editor ownership, typed schema, precedence, trust, privacy, ceilings, native support matrix, and terminal-grid-inference prohibition. | ADR accepted; hostile fixtures, precedence/collision matrix, and architecture ratchets reviewed. |
| CP1 native completion | CP0 | Idempotent PowerShell/Bash/Zsh/Fish/CMD/WSL adapters; read-only doctor health; official provider generator discovery/cache; clean uninstall and disabled-integration behavior. | Native cursor/history/quoting/exit-status/profile, collision, startup, update/uninstall, and shell-disabled tests pass on supported hosts. |
| CP2 typed Quick Actions | CP0; D3 not required for insert/copy | Bounded versioned store; atomic user-only writes; last-known-good reload; exact-file watcher; layered scopes; CRUD/import/export; placeholder review; accessible search; insert/copy only. | Persistence/recovery/concurrent-window/isolation, Unicode/hostile input, deterministic ordering, leak, accessibility, and Windows/Linux/macOS tests pass. |
| CP3 aliases and static packs | CP1/CP2 | Reversible opt-in aliases/functions/abbreviations plus Git, Docker, Kubernetes/OpenShift, Helm, Terraform/OpenTofu, AWS, Azure, GCP, and SSH packs; no default short alias. | Native definitions win; projections round-trip/remove cleanly; completion follows aliases; risk and tool/version states are truthful. |
| CP4 capsule-aware actions | D3 activation plus D5/D6 | Bind cached public capsule/target context with freshness; explicit refresh; reviewed exact launch only through the broker. | Mixed-pane/session isolation, offline/stale/revocation/cancellation, production-risk, provider-native, audit-redaction, and resource tests pass. |
| CP5 Shell Completion and Suggestions | CP1 plus a separately reviewed bridge ADR/threat fixture; CP2/CP3 for action candidates; CP4/D6 for provider context | CP5.0 baseline/dependency evaluation; CP5.1 authenticated local buffer/cursor/span/generation bridge; CP5.2 local-only source broker; CP5.3 deterministic bounded ranking; CP5.4 pane-owned accessible UI; CP5.5 per-shell activation; CP5.6 preview/rollback gate. Native completion remains default. | No grid/history-file/remote-output inference, network/auth/secret work, or per-keystroke process. Protocol/fuzz/security, exact insertion, IME/grapheme, accessibility, responsive layout, native shell/OS, cancellation, latency, memory/handle/socket/storage leak, 30-day baseline, disable and uninstall evidence proves a measurable benefit. Not a v0.5.0 blocker. |
| CP6 ecosystem packs | v0.6 capability/sandbox/signing gates | Signed third-party packs and separate opt-in AI tools only through provenance, capability, quota, revocation, and privacy policy. | Malicious-package/sandbox/supply-chain/data-flow gates pass; no ambient terminal, secret, provider, or process authority. |

#### CP5 activation ledger (planned)

| CP5 checkpoint | Required implementation | Blocking proof |
|---|---|---|
| CP5.0 research | Native shell/version baselines and a written adopt/reject report for PSReadLine APIs, Reedline design patterns, Nucleo matching, Carapace, and in-tree alternatives | No new dependency or runtime path before license/advisory/size/startup/latency/privacy comparison is reviewed |
| CP5.1 bridge | Private Windows named pipe / Unix domain socket, restrictive endpoint, session capability, version negotiation, bounded messages, generation/span validation, memory-only private payloads | Impersonation/replay/cross-session, downgrade, malformed-frame, cleanup, crash-redaction, and native-fallback tests pass |
| CP5.2 broker | Shell-native, opt-in history/frequency, cwd/executable, cached public provider, and typed-action sources with deterministic precedence | Zero implicit network/provider/auth/secret/history-file/remote-output access; bounded queue/cache and stale cancellation pass |
| CP5.3 match/insert | Explainable stable ranking, Unicode-safe display, shell-returned insertion values and spans | Property/hostile Unicode, quoting, replacement, rapid typing and Criterion gates meet frozen limits |
| CP5.4 UI | Renderer-neutral pane-owned listbox projection, source/freshness/risk labels, responsive placement, modal/IME/cursor avoidance | Tiny-to-8K/100–300% goldens, input/cursor stability, screen-reader/high-contrast/reduced-motion tests pass |
| CP5.5 shells | Version-gated PowerShell, Bash, Zsh, Fish and WSL/remote opt-in; truthful CMD fallback | Native framework/profile/binding preservation, install/update/remove, independent-pane failure and unsupported-version tests pass |
| CP5.6 release | Preview flag, kill switch, reset/disable UX, last-known-good fallback, rollback and uninstall | Windows/Linux/macOS native CI, controlled NVDA/Narrator/VoiceOver/Orca, fuzz/leak/storm campaigns and 30-day performance baseline are linked |

Failure at any checkpoint leaves CP1 native completion unchanged; a visually
working popup is not evidence that the bridge, privacy, insertion, accessibility,
or lifecycle gates are satisfied.

#### CP0 implementation ledger

| CP0 step | Audit before implementation | Implemented evidence | Remaining gate |
|---|---|---|---|
| Decision and terminology | Partial: ADR 0015 was proposed and the canonical roadmap defined intended ownership | ADR 0015 accepted; shell-native completion, typed Quick Action, alias projection, provider pack, insert, and D3 exact-launch terms are fixed | Reopen only on a mandatory threat-model trigger |
| Shell/editor/platform compatibility | Partial prose without a machine authority | Compatibility baseline plus schema-1 fixture covers PowerShell/Bash/Zsh/Fish/CMD, Windows/Linux/macOS/WSL, native fallback, provider support, versions, profiles, conflicts, uninstall, and bounded read-only discovery that never invokes definitions | Satisfied by the CP1 ledger below |
| Threat/privacy model | General security bullets only | Threat model and schema-1 fixture define seven boundaries, 16 threats, assets, controls, verification, residual risks, and review triggers | Each later CP phase adds runtime evidence for activated boundaries |
| Precedence and ceilings | Defined in prose but not mutation-tested | Exact precedence, native-wins policy, execution modes, 11 compatibility cases, 14 hard ceilings, nested schemas, and canonical fixture fingerprints are machine validated | Changes require ADR/threat/schema review |
| Architecture nonactivation | Documented but not enforced | Policy uses bounded no-symlink reads, normalizes shell-hook whitespace, scans every runtime workspace crate, rejects all premature command-productivity runtime markers/provider hooks, and retains the terminal-grid inference prohibition | CP1 now narrows activation to its separate exact allowlist |
| CI and regression ownership | Missing | Repository validation, architecture verification, PR policy, seventeen policy tests with an 11-case hostile corpus, shell contracts, and feature-assurance traceability own CP0 | Hosted CI must pass on pushed commit |

CP0 result: satisfied at the source/policy boundary. It grants no runtime
capability and does not claim CP1 completion, Fish support, Quick Action
persistence, generated aliases, provider execution, or a custom completion UI.

#### CP1 implementation ledger

| CP1 step | Audit before implementation | Implemented evidence | Remaining gate |
|---|---|---|---|
| Shell ownership and adapters | PowerShell/Bash/Zsh existed only for prompt/listing; Fish and managed completion were absent | First-class guarded PowerShell/Bash/Zsh/Fish adapters preserve PSReadLine/Readline/ZLE/Fish ownership; CMD explicitly falls back; terminal-grid inference remains forbidden | Hosted native macOS shell job must pass on the pushed commit |
| Provider discovery and refresh | Provider contracts were documentation only | `cargo xtask completion doctor/refresh/remove/enable/disable`; exact executable and argv; explicit-only Docker/Kubernetes/OpenShift/Helm generation; truthful external/manual/unsupported states for all 11 providers | New provider contracts require ADR/threat/allowlist review |
| Security and persistence | No runtime artifact boundary | 750 ms process deadline, null stdin, POSIX process-group/Windows Job-Object descendant termination including early leader exit with inherited pipes, native-image-only Windows refresh, stable executable revalidation, 1 MiB/256 KiB capture, UTF-8/control validation, absolute fixed paths, private directories, same-directory atomic files, bounded artifact/digest/metadata/provenance health validation, file/parent link checks, and PowerShell explicit override | Controlled hostile-provider campaigns remain nightly evidence, not startup work |
| Install/update/uninstall | No Fish or generated-artifact lifecycle | Source-fingerprinted Windows/Unix/WSL installation, platform-canonical roots including macOS Application Support, bounded atomic profile blocks, stale-owned-block repair, exact-file nonrecursive uninstall, all-target preflight, surrounding-content preservation, disable/native fallback, and repair tests | Remote/container installation remains explicit and out of scope |
| Native correctness | Bash/Zsh/PowerShell prompt tests existed | Bash/Zsh/Fish native collision/digest/disable/relative-root tests, Windows PowerShell/CMD integration, WSL and simulated-Darwin install/repair/uninstall, provider timeout/overflow/kill/leader-exit tests, doctor integrity tests, and CP1 policy mutations are PR-owned | Hosted Windows/macOS/Linux jobs provide final platform evidence |
| Machine enforcement and documentation | CP0 rejected all activation | Schema-1 CP1 fixture/checker limits activation to 12 reviewed files and zero network/secret/grid/startup capability; CLI, shell, compatibility, threat, testing, feature, and roadmap docs are updated | CP2 must define and pass its own activation gate |

CP1 result: implemented at the source and locally available native-host
boundaries. It intentionally adds no Quick Action store, generated alias,
provider authentication/network call, custom candidate UI, or exact launch.
CP2 is next. Stable release claims still require the pushed hosted Windows,
Linux, and macOS checks plus the existing controlled release gates.

CP0-CP3 may proceed alongside D5 without delaying safe system-OpenSSH work. CP4
must not precede D6 capsule isolation. CP5 and CP6 cannot be pulled into the
release merely because a candidate UI or pack parses locally. A completed stage
records exact commands/results, native environments, resource ceilings,
benchmark comparison, redaction proof, external gaps, and rollback/uninstall
behavior.

### Implementation-agent protocol

An AI or human implementation agent follows these rules:

1. Execute D0 through D7 in dependency order. Parallelize only tasks explicitly
   allowed by the table above and never merge an activated feature whose
   prerequisite exit gate is open.
2. Before each stage, read this complete track, the linked architecture, current
   accepted/proposed ADRs, relevant source modules/tests, and current working
   tree. A CP stage additionally requires the complete
   [Command Productivity](COMMAND-PRODUCTIVITY.md) contract. Preserve unrelated
   user changes and do not mix their ownership into the stage.
3. Keep extraction, behavior migration, and new capability activation as
   separate reviewable changes. First prove equivalence, then change contracts,
   then add behavior. Do not rewrite provider discovery while moving it.
4. Implement the smallest vertical slice that closes a numbered step with its
   unit/property/integration/native tests and documentation. A compiling type or
   visible UI without denial, failure, cancellation, cleanup, accessibility,
   redaction, and performance behavior does not complete a step.
5. Do not add a dependency, executable, endpoint, filesystem root, environment
   variable, IPC message, persisted field, or capability outside those listed
   for the current stage. If required, stop activation, update the threat model
   and ADR, and obtain the prescribed review first.
6. Treat every external file, CLI output, remote byte, label, path, manifest,
   provider response, and extension message as untrusted and bounded. Never use
   real user secrets in fixtures or diagnostics.
7. Run the narrowest focused tests during development, then the architecture,
   identity, conformance, security, and QA gates assigned to the stage. Native
   claims require the declared OS/tool/hardware; a mock or unavailable provider
   is reported as such and never recorded as a pass.
8. Update roadmap status only with evidence: changed paths, contract satisfied,
   exact commands/results, native environments, benchmark comparison, redaction
   evidence, known limitations, skipped external gates, and rollback behavior.
9. Do not mark v0.5.0 or v0.5.1 complete while a required exit criterion in this
   document is partial. Do not compensate for a blocker by broadening core,
   weakening host-key/authentication policy, storing credentials, enabling
   shell evaluation, or granting direct network/process authority.

### D0 — decision, threat model, and compatibility baseline

1. Accept a replacement ADR for ADR 0003 that authorizes only the first-party
   `session.launch` capability described below. Keep direct extension network,
   raw secret access, downloaded extensions, arbitrary executable launch, and a
   public SDK denied.
2. Record the trust boundaries: extension model code, application capability
   broker, PTY/process owner, renderer/VT parser, OpenSSH child, OpenSSH
   configuration, agent/keychain/hardware, remote host, and later provider
   helper. For each boundary list data accepted, data returned, size/time limit,
   cancellation owner, log policy, and failure behavior.
3. Preserve the manual baseline on every supported OS: a user typing
   `ssh host` in PowerShell, CMD, Bash, Zsh, or WSL continues to use that
   shell's normal executable lookup and behavior. Managed quick connect is an
   additional path, never a replacement.
4. Capture native fixtures before refactoring: independent sessions, first-use
   host-key prompt, changed-key failure, agent success/failure, encrypted key
   prompt, certificate authentication, ProxyJump, local/remote/dynamic forward,
   cancellation during DNS/connect/authentication, remote exit, remote hostile
   control strings, and child cleanup.
5. Define platform support explicitly. v0.5.0 requires the supported Windows
   OpenSSH client and system/user-installed OpenSSH on macOS/Linux. Missing or
   unsupported clients produce installation guidance; Automexia does not
   download a binary during launch or application startup.

Exit gate: the ADR and threat model are accepted, the manual SSH baseline is
recorded, and no proposed API requires provider-specific code or secret values
inside the renderer, VT parser, or generic terminal model.

### D1 — minimal private-crate extraction and stable types

Perform behavior-preserving moves in this order:

1. Move the pure types now in
   `apps/automexia-terminal/src/automexia/api.rs` into a private
   `automexia-extension-api` crate. It has no dependency on winit, WGPU,
   Sugarloaf, PTY/ConPTY, the renderer, or provider SDKs.
2. Move bounded queue, cache, generation, completion, and cancellation state
   from `automexia/runtime.rs` into `automexia-extension-runtime`. Keep the
   application callback that wakes an exact route behind an injected trait;
   never import frontend event types into the runtime crate.
3. Move current DevOps models/detection/semantics behind
   `automexia-devops`. First preserve the existing `DevOpsSnapshot` behavior
   through an adapter; split it into provider packages only after equivalence
   tests pass.
4. Move renderer-independent segment layout/accessibility data into
   `automexia-ui-model`. GPU paint remains in
   `apps/automexia-terminal/src/renderer`.
5. Extend the existing `SessionLaunchDescriptor` seam in
   `context/launch.rs`; do not create a parallel session/process owner. The
   application remains the only component allowed to attach a launched process
   to a route and PTY.
6. Add versioned, serializable types with explicit size-checked constructors:
   `ExtensionId`, `SessionId`, `OperationId`, `ExecutableId`,
   `LaunchRequest`, `EnvironmentCapsule`, `ContextContribution`,
   `StatusSegment`, `Freshness`, `CapabilityRequest`,
   `CapabilityDecision`, `SecretReference`, and `PublicDiagnostic`.
7. Make secret-bearing strings impossible in these public models. Capsules and
   connection records contain public identifiers and opaque references only.
   Redacted `Debug` implementations are mandatory for any secret-adjacent
   internal wrapper.

Required tests:

- dependency/architecture checks reject frontend/renderer/PTY/provider imports
  from the API and model crates;
- behavior-equivalence tests compare every existing DevOps snapshot and segment
  before and after the adapter;
- serialization tests pin schema version, unknown-field behavior, maximum
  lengths/counts, Unicode, and forward-compatible rejection;
- Loom covers queue submission, coalescing, publish-before-wake, cancellation,
  disable-while-running, shutdown, and last-known-good snapshot ownership;
- Miri covers pure lifecycle and policy state; no platform FFI enters the model.

Exit gate: the core behaves identically with existing extensions, disabling all
extensions leaves a complete terminal, and the renderer consumes generic model
types without depending on a provider implementation.

### D2 — generic status and Environment Capsule plumbing

1. Replace renderer-owned AWS/Azure/GCP/Kubernetes conditionals with a generic
   `ContextContribution -> StatusSegment` projection. Core owns ordering,
   width, ellipsis, semantic color role, contrast correction, accessibility,
   hit testing, and details routing; extensions provide bounded labels, icon
   tokens, semantic roles, freshness, timestamps, and a typed details action.
2. Keep historical prompt context immutable. A prompt row retains the exact
   contribution snapshot used by its command even after live context refreshes
   or the session is explicitly rebound.
3. Attach one immutable Environment Capsule reference to every new session.
   Cloning copies only non-secret intent, assigns a new capsule/session ID,
   starts a new PTY, and schedules fresh discovery. It does not share cached
   tokens, plugin results, process memory, jobs, terminal cells, or operations.
4. Add an explicit rebind transition: validate the new capsule, cancel work for
   the old revision, atomically swap live intent, invalidate affected cache
   keys, publish `refreshing`, and launch a new shell/session when environment
   variables cannot safely change in place. Never mutate another pane.
5. Cache keys include extension ID, session ID, capsule revision, provider
   identity reference, source revision, and request kind. A result with a stale
   key is discarded before publication.
6. Preserve the last truthful snapshot after timeout/error and mark its
   freshness. Empty/default state must not replace a known production identity.

Exit gate: two panes can display different providers, identities, clusters, and
risk classifications without sharing results; provider removal from renderer
code is enforced by architecture tests.

#### Phase 1 execution ledger (D1-D2, 2026-08-14)

| Obligation | Status | Implemented evidence |
|---|---|---|
| D1.1 typed API extraction | Complete | `automexia-extension-api` is private and renderer/window/GPU/PTY/provider independent; the frontend API file is a compatibility re-export only. |
| D1.2 runtime extraction | Complete | `automexia-extension-runtime` owns generation, cancellation, bounded FIFO cache, bounded non-blocking worker, coalescing, exact-route wake trait, registration latch, and rebind planning without frontend event types. |
| D1.3 local DevOps extraction | Complete | Existing models, detection, sanitization, and semantics moved behind `LocalContextProvider`; the frontend contains only a compatibility facade. No network, clipboard, or process capability was added. |
| D1.4 UI-model extraction | Complete | Generic projection, priority/deduplication, responsive layout, grapheme-safe compaction, icon policy, semantic color/contrast, accessibility summaries, hit testing, and details routing live in `automexia-ui-model`; GPU paint remains in the frontend renderer adapter. |
| D1.5 single launch owner | Complete | `SessionLaunchDescriptor` remains the sole launch source. Its versioned view exports exact argument elements, public environment names, and non-secret capsule intent; the application remains the only route/PTY/process owner. |
| D1.6-D1.7 versioning, bounds, and redaction | Complete | Required identifiers and schemas use version 1, size-checked constructors, validated deserialization, unknown-field rejection, collection ceilings, opaque secret references, and redacted secret-adjacent `Debug`. Environment values are absent from serialization. |
| D1 verification | Complete locally | Exact adapter goldens, schema/Unicode/forward-version/adversarial-size tests, dependency graph enforcement, cancellation/coalescing/publish-before-wake/disable/shutdown Loom models, and pure-state Miri jobs are present. Hosted Miri remains CI evidence, not a Windows-native claim. |
| D2.1 generic rendering | Complete | Renderer provider branches and provider types were removed. It consumes generic contributions and delegates model policy to `automexia-ui-model`. |
| D2.2 immutable history | Complete | Prompt snapshots clone the generic projected segments once per prompt generation; live refresh cannot rewrite historical prompt context. |
| D2.3 session capsule isolation | Complete | Every live/dead/cloned context owns a capsule matching its route/session ID. A clone receives a new capsule and independent PTY; no operation/cache identity is shared. |
| D2.4 safe rebind transition | Complete at the provider-neutral boundary | `plan_rebind` validates monotonic revisions and requires a new session for shell/cwd/distro/user changes. The application cancels old-revision work and invalidates only the affected session cache. Managed environment-changing relaunch remains inactive until the Phase 2 broker supplies authority and UI. |
| D2.5 complete cache identity | Complete | Keys contain extension, session, capsule revision, provider identity, source revision, and request kind. Results require an already-registered exact operation and current capsule before publication. |
| D2.6 truthful failure state | Complete | Provider failure catches unwinds, retains the last known snapshot and original observation timestamp, publishes explicit error freshness, and wakes only after publication. |

Phase 1 exit result: satisfied at the source boundary. Two-pane/session isolation,
provider-neutral rendering, bounded lifecycle behavior, accessible generic UI
policy, and behavior equivalence are automated. Phase 2 remains blocked from
activation until its replacement ADR, exact executable broker, SSH security
model, and native cross-platform evidence pass; Phase 1 does not weaken those
gates.
### D3 — exact-argv first-party session launch

1. Add a generic application-owned capability broker. The extension submits a
   typed `LaunchRequest`; the broker returns an operation/session ID or a typed
   denial. The extension never receives a PTY handle, process handle, inherited
   environment, agent protocol, or renderer object.
2. Resolve an `ExecutableId` through platform policy to one canonical absolute
   path. Search approved system locations and configured absolute paths; never
   search the current directory. Record file identity/metadata and revalidate
   immediately before spawn so a path swap cannot silently change authority.
3. For v0.5.0, authorize only reviewed first-party executable IDs needed by the
   extension, initially OpenSSH `ssh`, `ssh-add`, `ssh-keygen`, and only the
   subset actually used by a shipped operation. An extension manifest cannot
   use wildcard executable paths.
4. Carry arguments as an ordered vector. Reject NUL, values above reviewed size
   limits, ambiguous leading-dash destination data, unsupported encoding, and
   a request that asks for shell evaluation. Preserve exact Windows and Unix
   process argument semantics through platform-native tests.
5. Core builds the child environment from the normal trusted session launch
   environment. The extension may request only allowlisted public deltas; it
   never reads the inherited environment. Agent variables such as
   `SSH_AUTH_SOCK` are inherited by the child through core policy, not copied
   into an extension snapshot or diagnostic.
6. Validate the working directory through the existing launch path. Use a safe
   default if the requested directory no longer exists; never fall back to a
   different shell or remote target.
7. Bind the process, PTY, route, capsule, operation, and optional tunnels before
   publication. Cancellation sends the platform-appropriate graceful signal,
   waits for a bounded interval, terminates the owned process tree if needed,
   closes owned listeners, records a redacted result, and cannot target a reused
   PID or another session.
8. Audit only extension ID/version/publisher, capability decision, operation
   kind, public connection ID, session ID, timestamp, duration, and result
   class. Do not audit argv wholesale, environment, terminal contents,
   usernames when policy classifies them as private, or credential/agent data.

Exit gate: property and native tests prove command/argument injection is not
possible, denial and revocation are deterministic, and launch/cancel/teardown
cannot affect another route or leave a child/listener behind.

#### Phase 2 execution ledger (D3 review boundary, 2026-08-15)

ADR 0012 remains proposed, so this ledger separates reviewed source preparation
from activation. The broker module is reachable only through `#[cfg(test)]`;
the release binary contains no managed-launch success path.

| Obligation | Status | Implemented evidence |
|---|---|---|
| D3.1 application capability broker | Complete as a non-activated review model | `LaunchRequest`, `CapabilityRequest`, and expiring `CapabilityDecision` carry matching operation, session, and capsule scope. The application registers exact active capsules, rebind is monotonic, session IDs cannot be reused, and production/pending mode denies before resolution. |
| D3.2 canonical executable resolver | Complete for path/identity review; native atomic execution pending | Only fixed platform locations or one explicitly configured absolute exact filename are considered; `PATH` and cwd are never searched and a broken configured override fails closed. Windows records volume/file index and Unix records device/inode, with size/time metadata and descriptor-time revalidation. Activation must still eliminate the remaining check-to-spawn race with a reviewed native primitive. |
| D3.3 exact first-party grant | Partial by design | The review harness matches the exact repository ID/publisher/version tuple and only the reviewed one-argument `ssh` operation; `ssh-add` and `ssh-keygen` stay denied. The real D4 package does not exist yet, so digest/signature/compatibility/revocation verification is not falsely claimed. DevOps manifests declare no session/process/network authority. |
| D3.4 exact argv | Complete for the pure/native command boundary; real spawn evidence pending | One bounded ordered destination argument is preserved as one native `Command::arg`; leading-dash, whitespace/control, extra arguments, shell executables, unsupported IDs, and oversized input are denied. The code contains no spawn or shell-evaluation call. Native process-level adversarial evidence remains an activation gate. |
| D3.5 trusted environment | Complete at the restrictive boundary | Extension-selected inherited environment and secret references are denied. Only bounded, duplicate-free, core-owned public overrides enter the prepared descriptor; audit/debug redaction canaries exclude values and destinations. Public extension deltas remain an empty allowlist until separately reviewed. |
| D3.6 working directory | Complete in the review model | Requested cwd must be absolute and canonical. The canonical core-owned safe default is captured during authorization and cannot be replaced by a conversion caller; relative/invalid input is denied and no shell, remote target, or session fallback occurs. Native directory-identity/handle evidence remains coupled to activation. |
| D3.7 process/PTY/route/tunnel lifecycle | Partial by design | Registered session/capsule state, monotonic per-session operation IDs, non-wrapping leases, duplicate/replay rejection, exact completion/cancellation, rebind, revocation, stale-lease isolation, sibling preservation, and zero retained state after 1/10/50 pure cycles are tested. No process, PID, PTY, route, or listener exists yet; graceful/forced teardown and native leak evidence require approved activation. |
| D3.8 redacted audit model | Complete for authorization | Records contain only the approved identity, decision, operation class, optional future public connection ID, operation/session IDs, time/duration, and result class. They contain no argv, cwd/path, environment value, terminal content, username, PID, secret, or agent data. Process completion audit persistence remains coupled to D3.7 activation. |

Phase 2 result: the safe review boundary and its deterministic tests are
implemented, but D3 is not complete as a product capability. The exit gate
remains blocked on ADR acceptance, package signature/digest identity,
capability UI/grant policy, atomic native executable launch, real native
spawn/cancel/teardown evidence on Windows/Linux/macOS, D4-to-D5 activation
integration, and controlled 1/10/50-session performance/leak results. See the
[exact broker contract](SESSION-LAUNCH-BROKER.md).

### D4 — safe OpenSSH inventory and persistence

1. Add `devops-ssh` as a separate manifest and package. During v0.5.0 it is a
   repository-reviewed compiled-in or first-party signed extension; it is not a
   downloaded third-party package and has no direct-network capability.
2. Read user/system OpenSSH configuration only through exact filesystem grants.
   Use a bounded static parser for inventory. Suggested initial ceilings are
   1 MiB per file, 8 MiB total, 128 included files, include depth 8, 10,000
   indexed aliases, and 4 KiB per displayed value; finalize them through review
   and tests rather than silently increasing them.
3. Resolve include paths with canonical-path, symlink, cycle, ownership, and
   permission checks. An include outside already granted roots is excluded and
   reported until separately approved. Parsing failure retains the previous
   index and reports source plus line without echoing sensitive content.
4. Parse only enough non-executable syntax to index concrete aliases and public
   hints. Skip wildcard-only entries as connectable inventory records. Never
   evaluate `Match exec`, `ProxyCommand`, `LocalCommand`, shell expansion,
   command substitution, or `ssh -G` during background discovery. Actual
   OpenSSH execution remains the authority for complete precedence/semantics.
5. Store Automexia-owned metadata under a versioned extension directory, for
   example `extensions/devops-ssh/connections.v1.json`, using atomic
   last-known-good writes and user-only platform permissions. Records may hold
   connection ID, display name, tags, favorite, recent-use time, source, host
   alias, public host/port/user hints, jump references, transport, capsule
   template, and opaque identity reference. Key bytes, passphrases, tokens, and
   resolved secret values are forbidden by schema and tests.
6. Watch only known configuration/metadata files. Debounce changes, retain
   periodic reconciliation, coalesce identical refreshes, cancel obsolete
   generations, and publish freshness/source/error. Indexing never runs in
   renderer/input/PTY code and never starts a network connection.
7. Prefer existing `ssh-agent`, OS agents/keychains, FIDO2/PIV/PKCS#11 devices,
   encrypted key files referenced by OpenSSH, and short-lived certificates.
   `keyring-rs`, `secrecy`, and `zeroize` may enter only after a separate
   dependency and custody review; none justifies a new Automexia vault.

Exit gate: malicious, recursive, oversized, changing, and permission-denied
configuration stays bounded; disabling/removing the extension removes its UI
and state without touching OpenSSH files or user keys.

#### D4 implementation ledger (2026-08-15)

| Item | Status | Evidence |
|---|---|---|
| D4.1 private package and authority | Complete | automexia-devops-ssh is a private disabled-by-default workspace package. Its manifest declares only filesystem.read; architecture verification rejects process, network, launch, clipboard, terminal, environment, or UI authority and forbidden dependencies. |
| D4.2 exact grants and ceilings | Complete, re-audited | InventoryGrant requires a directory root, canonicalizes exact entries, and rejects more than 128 entries before collecting them. InventoryLimits may lower but cannot raise the fixed 1 MiB/file, 8 MiB total, 128 files, depth 8, 10,000 aliases, 4 KiB/value, and 16 KiB/line security ceilings. |
| D4.3 includes and recovery | Complete for the nonactivated inventory | Includes expand lexically only inside the grant, reject symlinks/dynamic tokens/unsafe Unix ownership or permissions, bound cycles and changes, and use public labels plus line numbers. RefreshCoordinator retains last-known-good state on any failure. |
| D4.4 static non-executable syntax | Complete | Only concrete Host records and public hints are indexed. Wildcards/negation and Match are excluded; executable directives are diagnostics only. The package contains no process, socket, DNS, or ssh -G path. |
| D4.5 private metadata | Complete, re-audited | Strict schema 1 stores public Automexia metadata only in connections.v1.json. Same-directory synchronized replacement, Unix 0700/0600, protected current-user Windows DACL, race-resistant no-follow bounded reads, serialization bounded before staging, malformed/secret-field rejection, and exact removal are tested. |
| D4.6 refresh and watches | Complete for package scope, re-audited | WatchPlan can be derived only from scanner-observed canonical files plus the exact owned metadata file; notify uses nonrecursive exact watches, event bursts coalesce, newer requests actively cancel obsolete scans, late generations are discarded, and periodic reconciliation is mandatory. No production renderer/input/PTY wiring exists. |
| D4.7 credential custody | Complete as a boundary | Records retain only an opaque identity kind. OpenSSH, agents, keychains, hardware providers, certificates, and encrypted files remain external owners; no vault or secret dependency was added. |

D4 is complete as a nonactivated package boundary. It does not make D3 or D5
shipped: ADR 0012 is still proposed, no capability UI or managed OpenSSH launch
exists, and controlled native lifecycle/parallel-session evidence remains
required before connection activation.
The re-audit also makes nightly compilation and controlled execution of the
10,000-alias benchmark mandatory; UI/accessibility remain not applicable until
D5 connects this package to a product surface.

### D5.1 — production SSH UX and connection lifecycle

The implementation-ready product contract is
[Connection Hub](CONNECTION-HUB.md). Its D5.0 contract/golden slice must close
before this section draws production UI; D5.1 read-only discovery must close
before D5.2 activates reviewed OpenSSH launch. The Hub is a window-level modal
over renderer-neutral models and never resizes the PTY or becomes a credential
vault.

Implement in this order so every slice is independently testable:

1. Build the read-only Connection Hub over D4 records: passive/no-process first
   run, explicit bounded local scan, virtualized quick-connect search over
   alias/display name/tag/favorite/recent/provider/context, deterministic
   filter/group projections, stale-while-revalidate status, and responsive
   wide/medium/narrow layouts. Every result shows transport and enough public
   destination intent to avoid connecting to the wrong environment. Production
   risk uses text and accessibility state, not color alone.
2. Destination choice: new pane, pane-local tab, workspace tab, or OS window.
   All choices create a fresh PTY/route/capsule; no mode attaches another view
   to an existing PTY.
3. Basic OpenSSH launch using a selected config alias. Reject untrusted
   option-like aliases; never concatenate user data into option strings.
4. Config-defined ProxyJump support, followed by an explicit reviewed `-J`
   request model. Show the jump chain. Do not rewrite ProxyCommand or execute it
   during preview/indexing; OpenSSH may execute user-configured behavior only
   after the user starts the connection.
5. Local, remote, and dynamic tunnels as typed models, not free-form options.
   Validate host/port ranges; local/dynamic listeners bind loopback by default.
   Show owner, endpoints, health, start time, and stop action. Session-owned
   tunnels close with the session. A persistent background tunnel requires a
   separate explicit mode, bounded restart policy, and visible lifetime.
6. Agent and certificate public status using bounded official commands where
   available. Do not parse private keys. Agent forwarding is off by default,
   visible when enabled, scoped to one connection, and may be denied by policy.
7. Preserve OpenSSH's strict host-key interaction in the PTY. Any Automexia
   helper must show the full fingerprint/provenance, block a changed key, and
   never delete, replace, or accept a `known_hosts` entry silently.
8. Add SFTP only after v0.5.0 if required. It needs separate read/write,
   overwrite, symlink, permission, path-traversal, transfer-size, cancellation,
   resume, and partial-file contracts. It is not a blocker for secure SSH.
9. Keep a native Rust SSH engine, embedded web UI, RDP/VNC/Telnet, shared
   sessions, and Termix embedding outside v0.5.0. Termix remains a UX reference
   or future optional metadata bridge.

The activation slice also implements every provider-neutral authentication
state, the full Connection Review, allow-once/exact persisted capability
approval and revocation, unchanged-route one-click reconnect, platform-specific
agent setup, external-custody recovery text, keyboard/focus behavior, and the
renderer-neutral interaction/accessibility goldens required by the
[Hub contract](CONNECTION-HUB.md#verification-plan).

Exit gate for v0.5.0: quick connect, jumps, tunnels, authentication prompts,
host-key behavior, cancellation, offline failure, remote exit, and cleanup pass
on supported Windows/macOS/Linux OpenSSH clients; the terminal remains fully
functional with `devops-ssh` disabled.

### D5.2 — SSH security, native, and performance verification

Add deterministic and native coverage for:

- exact argv with Unicode, whitespace, quotes, leading dashes, long values,
  metacharacters, config aliases, jump chains, and every forward type;
- first use, accepted host, changed host key, revoked/expired certificate,
  missing agent, locked agent, encrypted key prompt, FIDO/PIV interaction where
  controlled hardware is available, and agent forwarding policy;
- DNS failure, timeout, unreachable host, proxy failure, authentication failure,
  server disconnect, user cancellation at every phase, and application close;
- 1/10/50 parallel SSH sessions and tunnels with exact route/process/listener
  ownership, no PID reuse race, no leaked handles/tasks/sockets, and bounded
  queue/cache growth;
- hostile remote OSC/APC/DCS/XTGETTCAP/graphics streams, paste/mouse modes,
  high-throughput output, resize/reflow, scrollback, Unicode, and multiplexer
  nesting under the same parser limits as local output;
- config include cycles, symlink swaps, permission changes, atomic replacements,
  oversized files/labels/counts, malformed encodings, watcher storms, extension
  disable during refresh, and last-known-good recovery;
- redaction canaries proving keys, passphrases, tokens, agent messages,
  environment values, terminal contents, and private paths never reach logs,
  snapshots, crash/QA bundles, telemetry, clipboard history, or AI surfaces;
- cold/warm extension activation, config indexing, palette search, session
  creation, connect start, first remote prompt, tunnel startup, cancellation,
  idle CPU, and memory at 1/10/50 sessions while a provider worker is slow.

Add these results to `cargo xtask qa --full --bundle` with private native logs
kept out of public artifacts and a redacted summary identifying every skipped
hardware/provider case. A mocked SSH server is useful for deterministic PR
tests, but release evidence also requires real system OpenSSH clients and
controlled native servers; neither layer replaces the other.

### D6 — v0.5.1 multi-cloud and orchestrator delivery

The provider-neutral state, review, discovery, and UI model from D5 is reused;
providers do not create separate connection dialogs. Exact AWS, Azure, Google
Cloud, Kubernetes, OpenShift, Teleport, and OpenBao journeys and the D6.0-D6.5
gates are in [Connection Hub](CONNECTION-HUB.md#delivery-phases-and-exit-gates).

Implement provider extensions only after the D1-D3 contracts are stable:

1. Split `devops-context`, `devops-kubernetes`, `devops-openshift`,
   `devops-aws`, `devops-azure`, `devops-gcp`, and
   `devops-infrastructure` into independently enabled manifests. The DevOps
   Pack is only a meta-package; disabling one provider revokes only that
   provider's capabilities and cancels its operations.
2. Build a non-secret Environment Capsule per PTY. Pin public identity/profile,
   account/subscription/project, tenant/organization, region/zone,
   kubeconfig/context/cluster/namespace, infrastructure directory/backend/
   workspace, remote transport, risk, provenance, source revision, freshness,
   and policy references. Never include credentials or full inherited
   environment.
3. AWS: set only session intent such as `AWS_PROFILE`, `AWS_REGION`, and
   `AWS_DEFAULT_REGION`; use IAM Identity Center/federation and short-lived STS
   roles; run required `aws sso login --profile ...` visibly; keep access keys
   out of capsules; use supported EKS exec authentication and prefer Session
   Manager for eligible hosts.
4. Azure: isolate identities with an identity-scoped `AZURE_CONFIG_DIR` where
   required, pass `--subscription` explicitly for extension-launched sensitive
   operations, use Entra/MFA or workload identity, use supported AKS/
   `kubelogin` flows, and prefer Azure Bastion. Do not use a hidden global
   `az account set` to change other panes.
5. Google Cloud: set `CLOUDSDK_ACTIVE_CONFIG_NAME` and a reviewed config root
   policy per session, use Workforce/Workload Identity Federation, supported
   GKE authentication, and prefer IAP plus OS Login. Avoid service-account keys
   and never import them into Automexia.
6. Kubernetes: create a per-session `KUBECONFIG` overlay or explicit source
   list; pin context and namespace; align Helm/tool variables; treat kubeconfig
   as executable-capable untrusted input; deny or confirm unfamiliar exec
   credential plugins through an executable allowlist; keep `ExecCredential`
   results in memory or official caches and out of logs/config.
7. OpenShift: build on the Kubernetes contract; pin API endpoint/context/project;
   run `oc login` or `oc login --web` visibly; review loopback callbacks; and
   use the same exec-plugin/certificate isolation. OpenShift domain actions get
   their own later network grants.
8. Infrastructure: pin tool, working directory, public backend identifier,
   workspace, provider set, state/lock expectations, and production risk.
   OpenTofu/Terraform workspaces are context, not a credential or authorization
   boundary. Never inject `TF_WORKSPACE` into an unrelated directory.
9. Provider-native remote transport adapters construct exact official CLI argv
   for AWS SSM, Azure Bastion, and GCP IAP/OS Login and connect their interactive
   stream to a normal PTY. They do not expose tokens to extensions or core UI.
10. Add direct provider SDK/API access only for lazy inventory that cannot be
    served by local config or official CLI state. Run SDKs outside renderer/VT
    code in a bounded extension host over user-scoped named pipe/Unix socket,
    with peer checks, typed schemas, deadlines, output caps, redaction,
    cancellation, and no general TCP control port.

Exit gate for v0.5.1: concurrent development/staging/production sessions across
AWS, Azure, GCP, Kubernetes, and OpenShift preserve exact identity and capsule
isolation; expired/offline/slow providers remain truthful and cannot affect
terminal latency or another pane.

### D7 — deferred ecosystem and AI gates

Third-party downloads, a public SDK, direct arbitrary network, a native SSH
engine, and AI command execution wait for the v0.6 sandbox, signing, revocation,
quota, migration, and capability UX. AI extensions receive no ambient PTY
environment, SSH agent, cloud cache, terminal history, capsule, connection, or
production authority. Every tool call names a structured operation, exact
environment/session, risk, and capability; read permission never implies
command permission and destructive work requires policy plus explicit review.

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
| Use system OpenSSH for the first managed SSH release | It preserves mature config, known-host, agent, certificate, hardware-key, jump, forwarding, and organization policy while keeping protocol code out of core. |
| Do not background-evaluate OpenSSH executable configuration | `Match exec`, `ProxyCommand`, `LocalCommand`, command substitution, and `ssh -G` can cross the process boundary; inventory parsing remains static and actual execution occurs only after user intent. |
| Do not embed Termix or an Electron/Node SSH stack | Termix is a useful UX/reference or optional future metadata bridge, but embedding it duplicates the native renderer/session model and expands the web/credential attack surface. |
| Defer a Rust-native SSH engine and structured SFTP | System OpenSSH is the compatibility/recovery authority; a later isolated transport must justify protocol, file-write, key, and resource risk independently. |
| Prefer provider CLI/config before provider SDK/API | Official CLI paths maximize authentication compatibility and minimize dependency/binary cost; direct APIs remain lazy, permissioned, and isolated. |
| Treat local policy as defense in depth | Cedar/OPA may gate local operations, but IAM, Entra, Google IAM, Kubernetes/OpenShift RBAC, SSH CA, network, and remote policy remain authoritative. |

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

The S0 source gates pass locally, but stable v0.4 remains blocked until every
applicable S1, hosted, native, and external release gate is satisfied. S1
native/visual/performance
evidence may run in parallel but cannot be replaced by Windows-only results.

The v0.5.0 DevOps/SSH release additionally requires D0-D5.2 completion: accepted
replacement capability ADR, generic extension/session/status contracts,
provider-neutral renderer, exact-argv launch validation, safe bounded OpenSSH
inventory, production quick-connect/jump/tunnel/host-key behavior, secret
redaction, and controlled Windows/macOS/Linux native SSH evidence. A mocked
server alone, a Windows-only run, or manual `ssh` success does not satisfy the
managed-extension gate.

The v0.5.1 multi-cloud release additionally requires D6 completion: per-PTY
capsule isolation, official provider authentication and expiry/cancellation
paths, Kubernetes/OpenShift exec-plugin controls, cloud-native remote
transports, truthful offline/stale state, provider-specific native evidence,
and performance/resource results under concurrent mixed-provider sessions.
Direct provider SDK/API inventory is not a v0.5.1 blocker unless included in
release claims.

Any v0.5.x release that includes command productivity additionally requires the
corresponding CP stage exit gates. A release must not claim managed autocomplete
or persistent aliases from the existing prompt/listing shell integration alone.
CP1-CP3 require native shell-disabled and install/update/uninstall evidence,
bounded atomic state and recovery, collision/precedence proof, review-before-
insert behavior, secret-negative tests, and performance/resource results on
Windows, Linux, and macOS. CP4 exact launch additionally requires accepted D3
activation and D6 capsule isolation. CP5/CP6 are never implicit blockers unless
their UI/ecosystem capability is included in the release claim.

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
  baselines pass without claiming the deferred v0.5 accessibility tree;
- shipped completion and Quick Action stages preserve shell/editor ownership,
  keep startup and keystrokes offline and secret-free, leave native definitions
  intact, insert without implicit execution, recover last-known-good bounded
  state, and uninstall without profile or generated-file residue.
