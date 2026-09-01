# v0.4 implementation and readiness audit

Audit date: 2026-08-24

This document reconciles the standalone-rebrand plan and the later prompt,
resize-storm, PowerShell-listing, responsive-layout, session-cloning, semantic
color, performance, build-workflow, packaging, contributor-policy, and Ghostty
keyboard-compatibility plans. It separates implementation evidence from
remaining source work and release prerequisites that source code cannot
satisfy.

## Implemented and locally verified

| Plan area | Evidence and result |
|---|---|
| Standalone source and history | The checkout builds without an overlay or bootstrap step. `origin` and `rio-upstream` are configured, Rio base `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2` is an ancestor, and the local and published remote annotated tag `rio-base-0.5.20-7d595af` resolves to that base. The stable-source policy now enforces exact remote `main`, tag, fork, clean-history, linear-history, and DCO provenance before publication. |
| Identity and coexistence | Central identity/path constants, Automexia executable/package/app IDs, environment variables, URL/desktop metadata, terminfo, identity allowlist, and migration/coexistence tests pass. Inherited private `rio-*`, `librio`, Sugarloaf, and engine type names remain intentionally attributed. XTGETTCAP `TN`/name now returns `automexia`; native error/dialog surfaces and the protocol response are enforced by identity regressions. |
| Repository structure | The frontend lives at `apps/automexia-terminal`; brand, documentation, packaging, shell integration, conformance fixtures, integration tests, and `tools/xtask` use the planned v0.4 layout. The release-critical provider-neutral API, runtime, DevOps, and UI-model code is extracted into four private crates. `automexia-app` extraction and inherited engine-directory regrouping remain deliberately deferred. |
| Single-command workflow | `cargo ready`, `cargo dev`, `cargo automexia`, storage preflight, isolated verification targets, runtime launch copies, and cleanup are implemented and documented. Launch now exposes a validated repository/package integration root only to the child session; it does not write profiles, run an installer, bypass execution policy, or provision WSL. Persistent integration is an explicit application subcommand. Doctor reports host-native, WSL-native, or mounted-drive workspace I/O, and heavy workflows reject WSL source/targets under `/mnt/<drive>` with a dual-checkout remedy. |
| Prompt and resize resilience | Generation-scoped OSC metadata, terminal-owned context/path rows, stable `aid`, Unicode-safe full-path reflow, immutable once-per-generation prompt snapshots, writer-fast-path ownership, hard-newline context boundaries, stale-cell cleanup, and final-resize repair pass deterministic and native Windows tests. PowerShell command acceptance now uses the documented `PSConsoleHostReadLine` extension boundary, leaves `AddToHistoryHandler` untouched, and clears only obsolete editor cells below the accepted line before command output starts. Deterministic storms include 2,000 grid transitions and restore the full path without a later PTY byte; incomplete prompt markers cannot claim subsequent command output; resize deduplication/coalescing, ordering barriers, and transient ConPTY errors are covered. Multiple WGPU and CPU native Windows storms passed without stale listing fragments; Linux X11/Wayland and macOS native GUI storms remain required on those hosts. |
| Session cloning | `SessionLaunchDescriptor`, classic `Ctrl+R`/`Ctrl+D` independent right/down clone actions, explicit `Ctrl+Alt+R`/`Ctrl+Alt+D` shell-control passthroughs, command-palette discovery, exact PowerShell/CMD/Unix/WSL shell/profile/user/distro/directory reconstruction, explicit failure without PowerShell fallback, and route/PID isolation pass unit and native tests. |
| Window and tab scope | Classic `Ctrl+T` creates a window-level tab, `Ctrl+Shift+T` creates an independent PTY tab inside the selected split/session, and `Ctrl+Shift+N` creates a separate OS window. Each multi-tab pane owns an internal 36 logical-pixel rail; sibling panes keep their geometry, tiny panes recover space without losing tabs, and PTY/grid/image/scrollbar/cursor/IME consumers share its DPI-safe content rectangle. `Alt`+Arrow geometric pane focus, `F6` cycling, pane-local `Alt`+`PageUp`/`PageDown`, global `Ctrl`+`Tab`, macOS equivalents, command-palette discovery, edge stopping, local wraparound, route isolation, pane-local tab order, selected styling, direct hit targets, native resize restoration, and close isolation have deterministic regressions. |
| Keyboard defaults | Automexia's original platform defaults are restored and command-palette labels match them. Ghostty compatibility remains a planned, explicit opt-in profile rather than an implicit default. Classic shortcut tables, user overrides, intentional compound actions, and palette-label uniqueness are regression tested. |
| Clipboard and keyboard selection | Bare `Ctrl+C` copies a non-empty terminal selection and otherwise preserves PTY ETX interrupt behavior. `Shift`+Arrow starts from the terminal insertion cursor (never an empty pointer-click anchor) or extends a real terminal-owned selection by cell/row; `Ctrl`+`Shift`+Left/Right moves by Unicode-aware word boundaries on every platform. A bare Arrow or non-empty text/paste/IME payload clears VT and renderer selection state before shell delivery, while search/Vi ownership and empty input remain unchanged. Motion is allocation-free, bounded by the grid/scrollback, reversible, wide-cell safe, pane-local, disabled during search/Vi ownership, and emits no PTY bytes; Windows consumes the paired ConPTY key release. Secondary/right click, including OS-mapped two-finger touchpad clicks, copies and clears an existing selection or pastes the system clipboard when no selection exists. Middle-click primary-selection paste remains available, primary/left click never pastes, application mouse reporting retains ownership, empty clipboard reads are ignored, and paste uses the established bracketed/control-filtering path. Unit, exhaustive small-grid, collision, override, architecture, and native PowerShell arrow/text snapshot tests cover the behavior; physical touchpad mapping remains host-level manual evidence. |
| Shell history responsiveness | Lost-wakeup regressions, real PowerShell Up recall, and raw `Ctrl+R` reverse search pass. Repeated native gates measured Up at 1.049-1.101 seconds and reverse search at 0.972-1.024 seconds, within the 1.5-second budget and consistent with the separate bare-ConPTY Windows PowerShell 5.1/PSReadLine 2.0 floor. Printable input stays below its independent 500 ms budget. `cargo xtask doctor` now reports this legacy stack and a non-mutating PowerShell 7/current-stable-PSReadLine recommendation. Classic users send history search with `Ctrl+Alt+R` because bare `Ctrl+R` clones. |
| Responsive UI | Extreme small/large/HiDPI layout, split ratios, tab/control collision, hidden hit targets, headerless palette containment, conditional pane-local tab-rail reservation, prompt restoration, and 4K/8K-equivalent transitions are covered by renderer-neutral tests. The removed workspace action shelf has no paint path or hit targets. Repeated native runs completed the full tab/CMD/history/four-pane/240-resize/multi-window-close scenario without a lost path/context, invalid grid, or crash. A bounded topmost `ClientToScreen`/`BitBlt` smoke now samples the actual composited Automexia client, rejects blank/single-color final WGPU frames, waits for secondary-window paint readiness before custom-close input, restores z-order in `finally`, and was visually reviewed from an explicitly retained private frame; element-level cross-platform raster goldens remain outstanding. |
| Pane-local footer | Every usable pane has an independently outlined renderer-owned, read-only footer whose minimal status line shows UTF-8, session-aware LF/CRLF, grid dimensions, and a live local clock. Pane/local-tab position, selection, and history offset appear only when relevant and space permits. Layout reserves the strip outside PTY rows and scrollbar hit targets; idle refresh, exact-route focus, shell convention, clock formatting, and responsive geometry have regressions, there are no footer action controls or hidden action targets, and panes below 112 logical pixels recover the full terminal height. Correct absolute geometry is covered structurally and the four-pane final-frame smoke confirms every footer remains bottom-anchored; focused raster-golden diffs remain planned. |
| Context and colors | Live Git, Docker, Kubernetes, cloud, Terraform, environment, OS/WSL, production, and user roles use distinct anchors with centralized contrast correction on every pane's live and historical prompt rows. Visible context uses the actual pane row height to enforce a content-aware minimum 1.22-row vertical rhythm after preceding output; absent context adds nothing, tiny rows clamp inside their reservation, and completion timing stays aligned. Provider mapping, stale-WSL clearing, initial publication-before-wake, default distinction, custom-theme contrast, and semantic-spacing boundary tests pass. Provider discovery is periodic/cached rather than a guaranteed event stream; external tool/config access may delay or omit facts and must be represented truthfully. |
| PowerShell listings | The native formatting view keeps real `DirectoryInfo`/`FileInfo` objects and renders four metadata columns with the icon adjacent to the name. Name-only sensitive, configuration, log, source, documentation, test, build, asset, package, Git, tool, data, cache, infrastructure, and packaging categories use folder-shaped composite badges rather than stand-alone symbols; PowerShell 7 adds safe category colors while Windows PowerShell 5 preserves width without ANSI. Unicode, spaces, narrow views, sorting, filtering, piping, and the plain-listing opt-out pass the PowerShell contract suite. |
| Image previews | Sixel, Kitty Graphics placements/placeholders, and iTerm2 inline images remain renderer-integrated. Local printed/typed raster paths support IO-free hover discovery, bounded background submission, click-to-pin, visible-path arrow browsing, Escape dismissal, selection/palette access, mouse-event ownership, strict predecode gates, bounded per-window queue/mailbox state, a 16-entry/32 MiB file-version cache, WSL validation, and responsive geometry. The renderer-free `automexia-image` suite covers every enabled codec, exact RGBA/straight alpha, malformed/mutated input, cache accounting/eviction, file-handle release, and no-sidecar storage. A required focused PR gate runs decoder/cache/frontend/Sugarloaf/VT/backend suites. The native Windows gate repeats 16 WGPU and CPU open/dismiss cycles with exact route-pixel/overlay/texture/byte state, zero active resources after dismissal, empty queue/mailbox checks, process-growth ceilings, transparent/opaque pixel sampling, and WGPU/CPU distribution comparison. The explicit-nightly decoder/tokenizer fuzz runner is time/RSS bounded and uses disposable storage; the recorded 544,609-execution WSL campaign passed. Controlled Linux/macOS visual/client evidence and extended sanitizer/fuzz soak remain release gates. |
| Shell integration | Every PowerShell source parses; the Windows contract covers session-only package sourcing, explicit isolated install/no-op/repair/uninstall, repeated sourcing, raw bounded WSL stdin, reparse safety, native CMD identity, accepted-input ownership, and icon-aware listing while preserving explicit `cmd /c`, built-in `dir`, and the user's PSReadLine predicate. Static/mutation gates reject automatic provisioning, execution-policy bypass, encoded/nested WSL commands, unbounded portable resources, and unsigned release scripts. Bash/Zsh/Fish source and persistent compatibility tests remain native-host owned. |
| Bounded configuration and migration | Normal config/theme loads no longer use unbounded panic-prone reads: config is capped at 4 MiB and themes at 1 MiB. Legacy import additionally rejects symlinked/non-regular config and bounds theme depth, entries, files, aggregate bytes, extension directories, markers, and aggregate extension state. Adversarial oversized, invalid-UTF-8, deep-tree, partial-storage, interrupted, repeated, and Unix symlink cases are present; native Unix CI owns the symlink execution. Network-style image paths now fail closed on every host. |
| Feature and platform assurance traceability | A schema-validated ledger covers all workspace members and required shell, package, workflow, and automation surfaces across correctness, security, performance, resource lifetime, storage hygiene, resilience, accessibility, visual quality, Windows, Linux, and macOS. It owns all nine controlled benchmark targets and the declared fuzz matrix. Mutation suites fail when native hosts, shells, X11/Wayland variants, alternate architectures, package validators, benchmark/fuzz ownership, or evidence paths/jobs/headings disappear. External Linux/macOS GPU/PTY/accessibility and representative-not-universal Linux distribution coverage remain explicit rather than reported as passed. |
| Correctness and policy | Locked metadata, rustfmt, all-target workspace check, warning-denied Clippy, workspace tests, conformance tests, migration tests, PTY tests, architecture, identity, provenance, package metadata, and `cargo deny` passed. |
| Coverage | LLVM coverage on the exact 2026-08-15 working tree passed at 49.61% global line coverage versus the 43.52% Windows baseline and 100.00% changed Automexia-owned executable lines. The checker supports explicit `WORKTREE` mode and includes untracked Rust files. |
| Repository formats | TOML, YAML, JSON, XML, desktop metadata, Markdown documents and local links/anchors, and commit-pinned Actions passed repository validation. PowerShell and Unix shell validation have dedicated wrappers. |
| Performance safeguards | Existing latency/resource ceilings remain. S2 source and automation are complete: no-follow/identity-stable bounded Criterion/native-memory normalization, repeated-sample/confidence quality, clean exact-commit/operator composition, a digest-bound 30-90-day independently reviewed builder, exact temporary waivers, protected activation validation, 90-day retention, and tagged-release enforcement at 5% latency/10% memory. Unknown metrics, runner drift, linked files, stale/future evidence, unsafe metadata, reviewer conflicts, and inactive baselines fail closed. The checked-in baseline is still collecting, so these gates do not claim the missing 30 controlled days. |
| Phase 0 assurance tooling | Pinned Nextest/JUnit/doctests, bounded redacted QA, viewport/DPI properties, Loom, and native stress remain. S1 adds a 512-case independent resize-queue model, bounded generation-aware atomic snapshot replacement, and an 8K/40-million-pixel visual comparator whose deterministic default rejects one changed channel in one pixel with no masks and emits atomic path-free evidence. Controlled native/golden/human review remains separate. |
| Phase 1 provider-neutral foundation | Four private crates, versioned/bounded validated contracts, generic renderer projection, exact adapter golden, injected exact-route wakes, registration-before-dispatch ordering, bounded cache/worker/coalescing, per-session capsules, stale cancellation/invalidation, last-truth error state, Loom models, Miri CI, architecture rules, and Criterion cases are implemented. No process/network/clipboard authority or managed SSH path was enabled. |
| Phase 2 exact-launch preparation | ADR 0012 is accepted by the project owner. D0/D3/M5 schema 7 preserves immutable schemas 1-6 and freezes exact M3-M5 route/tunnel/trust/lifecycle behavior and 23 scenarios. The broker remains hard disabled and the package unverified. One application runner owns bounded review with permanent pre-wrap exhaustion, exact requests, guarded PTY/route publication, cleanup, opaque per-tunnel receipts, and reconnect. F5.4 schema 2 binds private real manifests to the native OS/architecture, exact clean commit, exact application version/binary/package, all four fixed OpenSSH tools, the OpenSSH 10.5 security baseline, advisory review, and package provenance; its exact protected runner-group workflow uploads only a path-free summary. Synthetic evidence remains non-release evidence. ADR 0003 approvals/enforcement, attestation/revocation, actual status/SSH execution, real Windows/macOS/Linux descendant/resources/accessibility proof, and production activation remain open. |
| D4 OpenSSH inventory foundation | A private disabled-by-default package now provides exact canonical grants, immutable upper ceilings, bounded static concrete-alias parsing, lexical includes, race-resistant no-follow reads, symlink/reparse and Unix ownership/permission checks, redacted diagnostics, strict public-only metadata, serialization bounded before staging, durable atomic Unix 0700/0600 and protected current-user Windows storage, scanner-derived exact-file watching, burst coalescing, active obsolete-scan cancellation, periodic reconciliation, late-generation rejection, and last-known-good recovery. It has only filesystem.read authority and no renderer, PTY, process, launch, network, clipboard, or secret custody. The re-audited Windows suite passes 29 tests and the native Linux suite passes 33 tests, including the Unix-specific cases. The updated WSL-native libFuzzer campaign completed 20,517 executions in 11 seconds with no finding at 230 MiB peak RSS. After reducing cancellation polling to a bounded 64-line interval, the optimized Windows 10,000-alias benchmark measured 19.706-21.270 ms and Criterion classified the change within its noise threshold. The macOS library cross-check passes; native tests remain owned by macos-latest. Nightly compilation and controlled execution own the benchmark. D3 activation and D5 managed connections remain blocked. |
| Native resource evidence | Repeated full native Windows GUI/ConPTY storms passed after resource and frame sampling were added. The latest four-pane delta was 80 handles, 12 threads, 49,823,744 private bytes, 16,568,320 working-set bytes, and 10 descendants, within the tightened normal ceilings; its 1400x864 final frame presented in one bounded attempt after 43 ms, contained 144 sampled color buckets, and had a luminance spread of 224. AppVerifier/WPR are installed but this process is not elevated, so elevated-runner results remain external. |
| Windows packaging | A real x86_64 release build produced a WiX MSI and portable ZIP. The ZIP executable reports `automexia 0.4.0`. This audit fixed package lookup under custom `CARGO_TARGET_DIR` and added Windows/Linux regression coverage for the resolved release path. |
| Contributor alignment | Contributor, conduct, security, support, governance, release, upstream, changelog, ownership, issue-form, PR-template, Dependabot, Release Drafter, DCO, protected-path-review, dependency audit, pinned Actions static analysis, nightly, and release definitions are present and repository-validated. |

The 2026-08-15 post-hardening Windows-to-WSL decoder campaign compiled from a
fully staged WSL-native `/tmp` source, completed 228,879 executions in six
seconds with no crash or sanitizer finding, stayed within the 768 MiB RSS cap
at 410 MiB, and left no temporary campaign directory or generated Windows-tree
state.
## Closed source blockers and remaining product work

The 2026-08-14 audit closed the three previously open v0.4 source blockers:

1. XTGETTCAP `TN`/name returns `automexia`, with capability and identity-gate
   regressions.
2. OSC raw accumulation (1 MiB), APC/graphics payloads (96 KiB), and XTGETTCAP
   requests (4 KiB) have explicit caps. Oversized streams discard through their
   terminator; CAN/SUB cancels without dispatch; diagnostics are payload-free
   and exponentially rate-limited. Exact-limit, limit-plus-one, fragmented,
   repeated-attack, cancellation, bounded-memory, recovery, and dedicated
   nightly fuzz coverage are present. Synchronized-update storage no longer
   reserves its 2 MiB ceiling per pane, and large completed OSC/APC/synchronized
   sequences release high-water allocations while small normal buffers remain
   reusable.
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
| v0.4 S1 structured/rendered visuals | **Source fully implemented.** Exact `s1-standard-v1` test-only fixture state freezes public content, clock, and motion; the bounded comparator rejects one changed channel in one pixel; and the versioned policy requires five 8,352-frame theme/scale/viewport/surface/motion matrices with independent HTTPS human review. | Execute and approve the policy matrix on Windows, Linux X11/Wayland, and macOS Intel/Apple Silicon hosts. Retain native frames and reviewer decisions for the exact protected commit. |
| v0.4 S1 QA evidence | Bounded redacted HTML/JSON/ZIP runner implemented and passing locally; JUnit is included while private ETL and live-terminal PNGs are excluded | Retain bundles from every controlled release host. |
| v0.4 S1 property/model coverage | Existing viewport/Loom/runtime models plus independent resize-queue and generation-aware bounded snapshot-publication models are implemented and passing | Retain longer fuzz/corpus trends and add only reviewed finite models. |
| v0.4 S1/S2 performance/resources | S2 source/automation is fully implemented with strict sample quality, clean exact-source/operator binding, native memory composition, independent review, semantically mutation-checked manual activation, 90-day retention, and direct fail-closed release dependencies; status is collecting | Provision the declared runner/variables, run and independently review 30 consecutive complete same-runner days, activate through `stable-release`, and retain the exact summary; elevated WPR remains separate. Compile-only/local results do not count. |
| v0.4 S1 AppVerifier/native matrix | **Source fully implemented.** Native Windows WGPU/CPU GUI/resource storms pass locally; the exact-target wrapper separates Basics from bounded low-resource injection, rejects verifier errors/stops and oversized logs, and emits distinct redacted evidence. The strict 28-suite validator binds evidence to policy, protected commit, environment, freshness, and clean tracked state, including Linux NVIDIA and macOS Intel/Apple Silicon resource ownership. | Run elevated AppVerifier and WPR plus named Intel/AMD/NVIDIA/RDP, Linux X11/Wayland/NVIDIA, and macOS Intel/Apple Silicon suites; retain reviewed driver/adapter evidence. |
| v0.4 S1 accessibility | **v0.4 source policy fully implemented.** Keyboard/focus/contrast/scale/reduced-motion contracts, exact Narrator/NVDA/VoiceOver/Orca environment coverage, accessible scope requirements, privacy checks, and independent HTTPS review are release validated. | Execute and retain the assistive-technology matrix on the exact protected commit. The renderer-independent semantic tree remains the separate v0.5 ADR 0013 scope. |
| v0.5 Phase 1 foundation | Provider-neutral API/runtime/DevOps/UI-model crates, generic rendering, capsule/cache isolation, broader Loom models, and hosted Miri jobs are implemented | Retain cross-platform CI/Miri evidence; the complete platform accessibility tree, scoped mutation campaign, cargo-vet ownership, and managed SSH broker remain later gates. |

The authoritative steps, command contract, CI tiers, dependencies, exclusions,
and acceptance criteria are in the
[stabilization roadmap](STABILIZATION-ROADMAP.md#verification-infrastructure-plan).

## Test evidence from this audit

- `cargo xtask test image-rendering`: passed the complete codec/security/cache,
  frontend state-machine, Sugarloaf resource/draw-order, VT/backend graphics,
  and benchmark-compilation gate. The Windows `--native-gui` form passed real
  WGPU and CPU interaction, 16 resource lifecycle cycles per backend, strict
  process-growth limits, exact active/dismissed resource state, visible pixel
  thresholds, and controlled backend-equivalence comparison.

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
- The 2026-08-16 lazy control-buffer construction smoke measured
  `Processor::default` at 29.964-30.073 ns after removing its unconditional
  2 MiB synchronized-update reservation. This proves the focused harness and
  zero-capacity regression locally; it is not a controlled baseline ratchet.

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

The native Windows, Linux, and macOS PR job now runs locked all-feature Clippy,
Nextest, and doctests on each host; Linux and macOS additionally run the same
isolated Bash/Zsh install/repair/prompt contract. Linux separately checks
X11-only, Wayland-only, and combined frontend configurations. The workflows
also define Windows x86_64/ARM64, macOS x64/ARM64, Linux
X11/Wayland/combined, pinned Actions static analysis, dependency audit, fuzz, Miri, sanitizer,
benchmark compilation, unsigned-package, signed PowerShell resources, semantic
SBOM, checksum, attestation, Linux cold-build reproducibility, immutable
publication, notarization, and clean package-install jobs. The benchmark job's `--no-run`
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
4. The repository is private on GitHub Free. The 2026-08-26 authenticated audit
   passed squash-only merging, merged-branch cleanup, update-branch UX, web DCO
   signoff, selected full-SHA Actions, read-only non-approving workflow tokens,
   dependency graph/Dependabot alerts and security updates, and immutable future
   releases. The versioned contract, local checker, mutation tests, CODEOWNERS
   fallback, exact check set, no-bypass branch/tag rules, bounded audit/apply
   path, and authenticated stable-tag preflight are fully implemented locally.

   Hosted workflows are a real failure: controlled S1, S2, and F5 workflows are
   present on this branch but absent from remote `main`, which was 109 commits
   behind at audit time. They must reach `main` through reviewed integration.

   Five external gates remain: the current private Free plan rejects rulesets;
   only one human collaborator exists; Actions jobs are rejected before checkout
   with zero steps because of the account billing/spending state; private
   vulnerability reporting is unavailable while private; and Secret Protection
   is not entitled. An owner must restore billing, choose private plan upgrade or
   explicitly authorize public visibility, invite at least two independent
   reviewers and expand CODEOWNERS, rerun the exact protected commit, then rerun
   the apply/audit. None of those states is credited as passing source evidence.
5. Six existing post-fork merge commits violate the linear release-source
   contract, and downstream commit `0f3fec43ac` lacks a DCO trailer. Current PR
   policy enforces squash-only, DCO-signed new work; repairing already-published
   history requires an explicit coordinated legal/history process and is not
   automatic.
6. The required 30-day performance baseline and controlled-hardware release
   checklist are time- and infrastructure-dependent and cannot be declared
   complete by one local run.
7. Controlled Windows AppVerifier/WPR, cross-platform GPU/render capture, and
   native assistive-technology infrastructure have not been observed. Their
   v0.4 baseline jobs and redacted artifacts must run on the declared hosts.
8. The public Linux distribution implementation and passive public repository
   are complete at the source/setup boundary, but no package is published. A
   real minisign key, repository-scoped GitHub App installation, available
   Actions minutes, native x64/Arm64 package jobs, immutable post-upload check,
   landing-page live verification, protected preview, production deployment,
   and signed-out route smoke remain external gates. The website must stay
   `coming-soon` until all of them pass for one exact commit.

Stable v0.4.0 remains blocked until every item above and every protected native
release job is complete. Unsigned artifacts from this audit are verification
outputs, not release candidates.
