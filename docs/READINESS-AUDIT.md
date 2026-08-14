# v0.4 implementation and readiness audit

Audit date: 2026-08-14

This document reconciles the standalone-rebrand plan and the later prompt,
resize-storm, PowerShell-listing, responsive-layout, session-cloning, semantic
color, performance, build-workflow, packaging, contributor-policy, and Ghostty
keyboard-compatibility plans. It separates implementation evidence from
remaining source work and release prerequisites that source code cannot
satisfy.

## Implemented and locally verified

| Plan area | Evidence and result |
|---|---|
| Standalone source and history | The checkout builds without an overlay or bootstrap step. `origin` and `rio-upstream` are configured, Rio base `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2` is an ancestor, and local annotated tag `rio-base-0.5.20-7d595af` resolves to that base. |
| Identity and coexistence | Central identity/path constants, Automexia executable/package/app IDs, environment variables, URL/desktop metadata, terminfo, identity allowlist, and migration/coexistence tests pass. Inherited private `rio-*`, `librio`, Sugarloaf, and engine type names remain intentionally attributed. XTGETTCAP `TN`/name now returns `automexia`; native error/dialog surfaces and the protocol response are enforced by identity regressions. |
| Repository structure | The frontend lives at `apps/automexia-terminal`; brand, documentation, packaging, shell integration, conformance fixtures, integration tests, and `tools/xtask` use the planned v0.4 layout. The release-critical provider-neutral API, runtime, DevOps, and UI-model code is extracted into four private crates. `automexia-app` extraction and inherited engine-directory regrouping remain deliberately deferred. |
| Single-command workflow | `cargo ready`, `cargo dev`, `cargo automexia`, storage preflight, isolated verification targets, runtime launch copies, cleanup, and fail-fast automatic PowerShell/CMD/WSL or Bash/Zsh/terminfo provisioning are implemented and documented. Provisioning is source-aware, idempotent, tested in isolated homes, and occurs directly before every launch; verification-only commands remain non-mutating. The complete `cargo ready` gate passed. |
| Prompt and resize resilience | Generation-scoped OSC metadata, terminal-owned context/path rows, stable `aid`, Unicode-safe full-path reflow, immutable once-per-generation prompt snapshots, writer-fast-path ownership, hard-newline context boundaries, stale-cell cleanup, and final-resize repair pass deterministic and native Windows tests. Deterministic storms include 2,000 grid transitions and restore the full path without a later PTY byte; incomplete prompt markers cannot claim subsequent command output; resize deduplication/coalescing, ordering barriers, and transient ConPTY errors are covered. Multiple consecutive full native Windows storms passed; Linux X11/Wayland and macOS native GUI storms remain required on those hosts. |
| Session cloning | `SessionLaunchDescriptor`, classic `Ctrl+R`/`Ctrl+D` independent right/down clone actions, explicit `Ctrl+Alt+R`/`Ctrl+Alt+D` shell-control passthroughs, command-palette discovery, exact PowerShell/CMD/Unix/WSL shell/profile/user/distro/directory reconstruction, explicit failure without PowerShell fallback, and route/PID isolation pass unit and native tests. |
| Window and tab scope | Classic `Ctrl+T` creates a window-level tab, `Ctrl+Shift+T` creates an independent PTY tab inside the selected split/session, and `Ctrl+Shift+N` creates a separate OS window. Pane-local tab order, route lookup, selected styling, direct hit targets, and close isolation have deterministic regressions. |
| Keyboard defaults | Automexia's original platform defaults are restored and command-palette labels match them. Ghostty compatibility remains a planned, explicit opt-in profile rather than an implicit default. Classic shortcut tables, user overrides, intentional compound actions, and palette-label uniqueness are regression tested. |
| Shell history responsiveness | Lost-wakeup regressions, real PowerShell Up recall, and raw `Ctrl+R` reverse search pass. Repeated native gates measured Up at 1.049-1.101 seconds and reverse search at 0.972-1.024 seconds, within the 1.5-second budget and consistent with the separate bare-ConPTY Windows PowerShell 5.1/PSReadLine 2.0 floor. Printable input stays below its independent 500 ms budget. `cargo xtask doctor` now reports this legacy stack and a non-mutating PowerShell 7/current-stable-PSReadLine recommendation. Classic users send history search with `Ctrl+Alt+R` because bare `Ctrl+R` clones. |
| Responsive UI | Extreme small/large/HiDPI layout, split ratios, tab/control collision, hidden hit targets, headerless palette containment, conditional pane-local tab-rail reservation, prompt restoration, and 4K/8K-equivalent transitions are covered by renderer-neutral tests. The removed workspace action shelf has no paint path or hit targets. Repeated native runs completed the full tab/CMD/history/four-pane/240-resize/multi-window-close scenario without a lost path/context, invalid grid, or crash. A bounded topmost `ClientToScreen`/`BitBlt` smoke now samples the actual composited Automexia client, rejects blank/single-color final WGPU frames, waits for secondary-window paint readiness before custom-close input, restores z-order in `finally`, and was visually reviewed from an explicitly retained private frame; element-level cross-platform raster goldens remain outstanding. |
| Pane-local footer | Every usable pane has an independently outlined renderer-owned, read-only footer whose minimal status line shows UTF-8, session-aware LF/CRLF, grid dimensions, and a live local clock. Pane/local-tab position, selection, and history offset appear only when relevant and space permits. Layout reserves the strip outside PTY rows and scrollbar hit targets; idle refresh, exact-route focus, shell convention, clock formatting, and responsive geometry have regressions, there are no footer action controls or hidden action targets, and panes below 112 logical pixels recover the full terminal height. Correct absolute geometry is covered structurally and the four-pane final-frame smoke confirms every footer remains bottom-anchored; focused raster-golden diffs remain planned. |
| Context and colors | Live Git, Docker, Kubernetes, cloud, Terraform, environment, OS/WSL, production, and user roles use distinct anchors with centralized contrast correction on every pane's live and historical prompt rows. Provider mapping, stale-WSL clearing, initial publication-before-wake, default distinction, and custom-theme contrast tests pass. Provider discovery is periodic/cached rather than a guaranteed event stream; external tool/config access may delay or omit facts and must be represented truthfully. |
| PowerShell listings | The native formatting view keeps real `DirectoryInfo`/`FileInfo` objects and renders four metadata columns with the icon adjacent to the name. Name-only sensitive, configuration, log, source, documentation, test, build, asset, package, Git, tool, data, cache, infrastructure, and packaging categories use folder-shaped composite badges rather than stand-alone symbols; PowerShell 7 adds safe category colors while Windows PowerShell 5 preserves width without ANSI. Unicode, spaces, narrow views, sorting, filtering, piping, and the plain-listing opt-out pass the PowerShell contract suite. |
| Shell integration | Every PowerShell source parses; the Windows contract covers isolated automatic install/no-op/repair passes, PowerShell plus an in-process native CMD prompt, identity, cloning, and icon-aware listing smoke while preserving explicit `cmd /c` and built-in `dir`. A TTY-only compatibility layer gives Ubuntu/WSL eza 0.18.x the same composite folder badges without changing redirected output. Bash/Zsh syntax, isolated automatic install/no-op/repair passes, live eza output, ShellCheck, and integration jobs are part of the Unix local/CI gate; a font parser proves every category glyph exists in the bundled Symbols Nerd Font. |
| Correctness and policy | Locked metadata, rustfmt, all-target workspace check, warning-denied Clippy, workspace tests, conformance tests, migration tests, PTY tests, architecture, identity, provenance, package metadata, and `cargo deny` passed. |
| Coverage | LLVM coverage on the exact working tree passed at 48.536% global line coverage versus the 43.52% Windows baseline and 100.00% changed Automexia-owned executable lines. The checker supports explicit `WORKTREE` mode and includes untracked Rust files. |
| Repository formats | TOML, YAML, JSON, XML, desktop metadata, Markdown documents and local links/anchors, and commit-pinned Actions passed repository validation. PowerShell and Unix shell validation have dedicated wrappers. |
| Performance safeguards | History interaction and resize delivery are fixed and measured on Windows. No-damage snapshots, prompt repaint, cache access, worker submission, parser/rebuild/reflow, and bounded queues/caches have focused tests or Criterion cases. Nightly keeps compile-only benchmark validation separate and now defines an opt-in named-runner job that executes and bundles Criterion evidence. No controlled benchmark run or 30-day baseline has yet been observed, so performance enforcement remains external. |
| Phase 0 assurance tooling | Pinned Nextest 0.9.137 profiles, no-silent-flaky policy, timeouts, leak detection, serialized native PTY ownership, JUnit, separate Cargo doctests, 512-case shrinking layout/DPI properties with a persisted minimized seed, a reviewed footer geometry snapshot, finite Loom channel readiness models, and `cargo qa --bundle` are implemented. The final local QA bundle passed all 17 required checks, capped every log, included redacted JUnit, excluded ETL and live-terminal PNGs, and passed a local-root/token-prefix scan. |
| Phase 1 provider-neutral foundation | Four private crates, versioned/bounded validated contracts, generic renderer projection, exact adapter golden, injected exact-route wakes, registration-before-dispatch ordering, bounded cache/worker/coalescing, per-session capsules, stale cancellation/invalidation, last-truth error state, Loom models, Miri CI, architecture rules, and Criterion cases are implemented. No process/network/clipboard authority or managed SSH path was enabled. |
| Native resource evidence | Repeated full native Windows GUI/ConPTY storms passed after resource and frame sampling were added. The latest four-pane delta was 80 handles, 12 threads, 49,823,744 private bytes, 16,568,320 working-set bytes, and 10 descendants, within the tightened normal ceilings; its 1400x864 final frame presented in one bounded attempt after 43 ms, contained 144 sampled color buckets, and had a luminance spread of 224. AppVerifier/WPR are installed but this process is not elevated, so elevated-runner results remain external. |
| Windows packaging | A real x86_64 release build produced a WiX MSI and portable ZIP. The ZIP executable reports `automexia 0.4.0`. This audit fixed package lookup under custom `CARGO_TARGET_DIR` and added Windows/Linux regression coverage for the resolved release path. |
| Contributor alignment | Contributor, conduct, security, support, governance, release, upstream, changelog, ownership, issue-form, PR-template, Dependabot, Release Drafter, DCO, protected-path-review, dependency-review, CodeQL, nightly, and release definitions are present and repository-validated. |

## Closed source blockers and remaining product work

The 2026-08-14 audit closed the three previously open v0.4 source blockers:

1. XTGETTCAP `TN`/name returns `automexia`, with capability and identity-gate
   regressions.
2. OSC raw accumulation (1 MiB), APC/graphics payloads (96 KiB), and XTGETTCAP
   requests (4 KiB) have explicit caps. Oversized streams discard through their
   terminator; CAN/SUB cancels without dispatch; diagnostics are payload-free
   and exponentially rate-limited. Exact-limit, limit-plus-one, fragmented,
   repeated-attack, cancellation, bounded-memory, recovery, and dedicated
   nightly fuzz coverage are present.
3. Runtime reload validates a complete candidate and keeps the last-known-good
   configuration after parse/theme/font/global-hotkey failure. Hotkey additions
   and removals have rollback tests, and no failed reload recreates PTYs.

The explicit, versioned Ghostty compatibility profile remains planned; it is
not an implicit v0.4 default or a v0.4 stable-release criterion. Its delivery
order is in the
[full compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md).

The complete cross-platform evidence, performance matrix, visual review,
hosted assurance, and stable acceptance gates are maintained in the
[stabilization roadmap](STABILIZATION-ROADMAP.md). The functional assessment is
therefore: the identified v0.4 source blockers are solved on the tested Windows
checkout; global performance proof and security/release assurance remain
partial, so stable delivery is not complete.
## Phase 0 assurance status

Source/tooling completion and remaining evidence are intentionally separate:

| Phase | Current status | Remaining completion evidence |
|---|---|---|
| v0.4 S1 Nextest/JUnit/doctests | Implemented and passing locally | Retain successful Windows, Linux, and macOS CI reports with no silent retry success, hang, leaked child, or order dependency. |
| v0.4 S1 structured/rendered visuals | Structured footer state snapshot plus topmost client-region Windows final-frame smoke implemented and reviewed | Add controlled expected/actual/diff goldens across the viewport/theme/DPI matrix and native Linux/macOS frame evidence. |
| v0.4 S1 QA evidence | Bounded redacted HTML/JSON/ZIP runner implemented and passing locally; JUnit is included while private ETL and live-terminal PNGs are excluded | Retain bundles from every controlled release host. |
| v0.4 S1 property/model coverage | Shrinking viewport/DPI properties, persisted regression, finite publish/wake models, and owned runtime cache/worker cancellation, coalescing, last-known-good, and shutdown models are implemented and passing | Expand pure resize-queue and atomic snapshot-replacement state machines. |
| v0.4 S1 performance/resources | Native Windows process sample passes; controlled Criterion and WPR jobs are defined | Named-runner reports plus a complete 30-day baseline; compile-only results do not count. |
| v0.4 S1 AppVerifier/native matrix | Safe exact-target wrappers and cleanup policy implemented; native Windows GUI/resource storm passes | Elevated clean heap/handle/lock run, Linux/macOS GPU storms, and reviewed adapter/driver records. |
| v0.4 S1 accessibility | Keyboard/focus/contrast/scaling inventory, manual matrix, limitations, and v0.5 ADR implemented | Narrator/NVDA, VoiceOver, and Orca smoke evidence; final semantic tree remains v0.5. |
| v0.5 Phase 1 foundation | Provider-neutral API/runtime/DevOps/UI-model crates, generic rendering, capsule/cache isolation, broader Loom models, and hosted Miri jobs are implemented | Retain cross-platform CI/Miri evidence; the complete platform accessibility tree, scoped mutation campaign, cargo-vet ownership, and managed SSH broker remain later gates. |

The authoritative steps, command contract, CI tiers, dependencies, exclusions,
and acceptance criteria are in the
[stabilization roadmap](STABILIZATION-ROADMAP.md#verification-infrastructure-plan).

## Test evidence from this audit

- Phase 0 QA evidence: `cargo qa --bundle` passed all 17 executed checks on
  commit `dc6edb4686`, including rustfmt, locked metadata, repository
  contracts/formats, PowerShell contracts, warning-denied Clippy, pinned
  Nextest/JUnit, Cargo doctests, resize stress, session cloning, finite Loom
  models, cargo-deny, native Windows GUI/ConPTY stress, LLVM coverage, and the
  coverage policy. The 97,768-byte portable ZIP independently passed its
  privacy check and contains no ETL, PNG, INFO, raw LCOV, unredacted local root,
  or token prefix.
- Native resource evidence: the real Windows GUI/ConPTY storm passed with four
  isolated panes, CMD/history/multi-window/resize coverage, bounded process
  growth, and a varied 1400x864 painted frame in one attempt. The atomic JSON
  report was inspected against every configured ceiling.
- Private visual review: the retained 1400x864 frame showed four distinct pane
  regions, active-pane highlighting, context rows, color-segmented full paths,
  prompt/cursor presence, and a footer attached to its pane boundary, with no
  blank surface, floating footer, duplicate prompt, or overlapping top chrome.
  The private PNG was not added to the portable bundle or repository.
- Local Criterion execution smoke: both `automexia_services` and `vt_input`
  executed successfully. Informational quick-mode samples measured context
  projection at 315.81-317.79 ns, responsive layout at 891.85-915.64 ns,
  bounded cache access at 65.605-66.816 ns, non-blocking worker submission at
  808.37-829.23 ns, no-damage snapshots at 42.516-43.162 ns, full row rebuild
  at 2.7171-2.7763 us, and prompt resize/reflow at 5.7175-5.7279 us. Plain VT
  parser-only throughput was 1.9533-2.0386 GiB/s. These uncontrolled laptop
  values prove harness execution only and do not satisfy the named-runner or
  30-day baseline gates.

- Composite-folder/path follow-up: PowerShell/CMD formatter contracts, Bash and
  Zsh integration tests, focused ShellCheck/Perl syntax, a real WSL pseudo-TTY
  run against eza 0.18.2, installed-file SHA-256 comparison, bundled font cmap
  coverage, locked debug build/smoke, repository formats, warning-denied
  Clippy, `verify all`, and the expanded conformance suite passed.
- `cargo ready`: passed the complete local contributor gate and debug binary
  smoke test; its isolated artifacts were removed automatically.
- `cargo xtask test conformance`: passed all selected application, VT, backend,
  window, renderer, and PTY suites, including four passive pane-footer geometry/routing
  regressions and the DPI-stable terminal-boundary check.
- `cargo xtask test resize-stress`: passed deterministic parser/reflow and
  resize-ordering storms.
- `cargo xtask test resize-stress --native-gui`: passed 19 deterministic cases
  plus the real four-pane Windows storm, exact-window painted-frame smoke,
  PowerShell history budgets, and isolated custom/native secondary-window close.
- `cargo xtask test session-clone`: passed deterministic launch, keybinding,
  quoting, and PTY isolation tests.
- `cargo xtask test session-clone --native-windows`: passed real GUI/ConPTY
  clone, history, and 590-operation resize/input stress.
- `cargo xtask test session-clone --native-wsl`: passed independent routes for
  `Ubuntu-24.04` while preserving WSL identity and directory state.
- `cargo llvm-cov --workspace --locked`: passed every workspace test under LLVM
  instrumentation; the threshold checker passed the values recorded above.
- `cargo xtask package --target x86_64-pc-windows-msvc`: produced the unsigned
  MSI and ZIP after the custom-target path fix; portable version smoke passed.

## Implemented in CI but requiring its native host

The workflows define Windows x86_64/ARM64, macOS x64/ARM64, Linux
X11/Wayland/combined, CodeQL, dependency review, fuzz, Miri, sanitizer,
benchmark compilation, unsigned-package, SBOM, checksum, attestation,
notarization, and clean package-install jobs. The benchmark job's `--no-run`
invocation is only a compile check and must not be reported as performance
measurement. A Windows workstation cannot honestly certify the macOS, Linux
package-manager, ARM64-native, notarization, or hosted GitHub jobs. Those jobs
must pass on their declared runners before release.

## External release blockers

These are not source defects and must not be bypassed:

1. `assets/brand/ASSET-MANIFEST.toml` correctly has `release.ready = false` and
   `rights_verified = false`. Editable logo/mark SVGs, light/dark/monochrome
   variants, and redistribution-rights proof are still missing. Existing raster,
   ICO, ICNS, and Linux PNG exports are valid only for development/nightly use.
2. `CONDUCT_CONTACT_REQUIRED` still marks the missing private conduct-reporting
   address.
3. Windows Authenticode and Apple Developer ID/notarization credentials are not
   configured. The locally built Windows executable is intentionally unsigned.
4. Public repository visibility, private vulnerability reporting, branch/ruleset
   protection, squash-only merging, review counts, CODEOWNERS enforcement, and
   administrator applicability require repository-owner configuration. The
   connected GitHub integration and anonymous API returned 404 during this
   audit, so hosted enforcement could not be verified.
5. The fork-point tag exists locally but was not observed on `origin`; publish
   it intentionally during repository administration rather than as a side
   effect of a code audit.
6. One existing downstream commit, `0f3fec43ac`, lacks a DCO trailer. Current
   PR policy enforces DCO for new work; repairing already-published history
   would require an explicit coordinated history rewrite and is not automatic.
7. The required 30-day performance baseline and controlled-hardware release
   checklist are time- and infrastructure-dependent and cannot be declared
   complete by one local run.
8. Controlled Windows AppVerifier/WPR, cross-platform GPU/render capture, and
   native assistive-technology infrastructure have not been observed. Their
   v0.4 baseline jobs and redacted artifacts must run on the declared hosts.

Stable v0.4.0 remains blocked until every item above and every protected native
release job is complete. Unsigned artifacts from this audit are verification
outputs, not release candidates.
