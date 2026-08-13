# v0.4 implementation and readiness audit

Audit date: 2026-08-13

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
| Identity and coexistence | Central identity/path constants, Automexia executable/package/app IDs, environment variables, URL/desktop metadata, terminfo, identity allowlist, and migration/coexistence tests pass. Inherited private `rio-*`, `librio`, Sugarloaf, and engine type names remain intentionally attributed. Complete rebranding remains partial because XTGETTCAP `TN`/name still returns `rio`; the S0 roadmap item must close it. |
| Repository structure | The frontend lives at `apps/automexia-terminal`; brand, documentation, packaging, shell integration, conformance fixtures, integration tests, and `tools/xtask` use the planned v0.4 layout. The v0.5 engine/crate regrouping remains deliberately deferred. |
| Single-command workflow | `cargo ready`, `cargo dev`, `cargo automexia`, storage preflight, isolated verification targets, runtime launch copies, cleanup, and fail-fast automatic PowerShell/CMD/WSL or Bash/Zsh/terminfo provisioning are implemented and documented. Provisioning is source-aware, idempotent, tested in isolated homes, and occurs directly before every launch; verification-only commands remain non-mutating. The complete `cargo ready` gate passed. |
| Prompt and resize resilience | Generation-scoped OSC prompt metadata, terminal-owned context/path rows, stable `aid`, Unicode-safe full-path reflow, visibly separated cyan/violet/blue/lime path roles that preserve literal text, screen/line-erase repair, snapshot rebuilding, resize deduplication/coalescing, input/shutdown barriers, and transient ConPTY error handling pass deterministic and native Windows tests. Deterministic storms include 2,000 grid transitions; native Linux X11/Wayland and macOS GUI storms remain required on their hosts. |
| Session cloning | `SessionLaunchDescriptor`, `Ctrl+Alt+R`/`Ctrl+Alt+D` independent right/down clone actions, native bare shell controls, command-palette discovery, exact PowerShell/CMD/Unix/WSL shell/profile/user/distro/directory reconstruction, explicit failure without PowerShell fallback, shortcut compatibility, and route/PID isolation pass unit and native tests. |
| Window and tab scope | Ghostty-compatible `Ctrl+Shift+T` creates a window-level tab, `Ctrl+Alt+T` creates an independent PTY tab inside the selected split/session, `Ctrl+T` remains an additional window-tab alias, and `Ctrl+Shift+N` creates a separate OS window. Pane-local tab order, route lookup, selected styling, direct hit targets, and close isolation have deterministic regressions. |
| Ghostty-style keyboard defaults | Separate macOS and Windows/Linux/BSD tables implement the pinned defaults whose actions exist; Automexia extensions avoid those chords, shell controls remain native, and platform/collision/palette tests pass. This is a supported default subset, not a selectable exact Ghostty profile. The typed registry, versioned fixtures/profiles, atomic reload, consumption/fallthrough, sequences/tables/chains, generated tooling, and missing actions remain on the compatibility roadmap. |
| Shell history responsiveness | The ConPTY input writer wake-up regression, real PowerShell Up Arrow recall, and raw `Ctrl+R` reverse search pass. The latest native gate measured 55 ms shell/VT latency for Up Arrow (110 ms end to end) and 0 ms shell/VT latency for raw `Ctrl+R` (64 ms end to end), within the 1.5-second test budget. Bare `Ctrl+R` and `Ctrl+D` remain shell-owned. |
| Responsive UI | Extreme small/large/HiDPI layout, split ratios, tab/control collision, hidden hit targets, headerless palette containment, the allocation-free Find/Split Right/Split Down/Next Pane rail, DPI-independent vector action icons, prompt restoration, and 4K/8K-equivalent transitions are covered by renderer-neutral tests. A native run completed 590 resize/input operations without a blank surface, stale prompt, invalid grid, or crash. |
| Pane-local footer | Every usable pane has an independently outlined renderer-owned footer with pane/local-tab position, grid dimensions, selection and integration state, exact history offset, Find, and return-to-live. Layout reserves the strip outside PTY rows and scrollbar hit targets, exact-route action tests pass, narrow panes collapse optional labels, and panes below 112 logical pixels recover the full terminal height. |
| Context and colors | Live Git, Docker, Kubernetes, cloud, Terraform, environment, OS/WSL, production, and user roles use distinct anchors with centralized contrast correction on every pane's live and historical prompt rows. Provider mapping, stale-WSL clearing, initial publication-before-wake, default distinction, and custom-theme contrast tests pass. Provider discovery is periodic/cached rather than a guaranteed event stream; external tool/config access may delay or omit facts and must be represented truthfully. |
| PowerShell listings | The native formatting view keeps real `DirectoryInfo`/`FileInfo` objects and renders four metadata columns with the icon adjacent to the name. Name-only sensitive, configuration, log, source, documentation, test, build, asset, package, Git, tool, data, cache, infrastructure, and packaging categories use folder-shaped composite badges rather than stand-alone symbols; PowerShell 7 adds safe category colors while Windows PowerShell 5 preserves width without ANSI. Unicode, spaces, narrow views, sorting, filtering, piping, and the plain-listing opt-out pass the PowerShell contract suite. |
| Shell integration | Every PowerShell source parses; the Windows contract covers isolated automatic install/no-op/repair passes, PowerShell plus an in-process native CMD prompt, identity, cloning, and icon-aware listing smoke while preserving explicit `cmd /c` and built-in `dir`. A TTY-only compatibility layer gives Ubuntu/WSL eza 0.18.x the same composite folder badges without changing redirected output. Bash/Zsh syntax, isolated automatic install/no-op/repair passes, live eza output, ShellCheck, and integration jobs are part of the Unix local/CI gate; a font parser proves every category glyph exists in the bundled Symbols Nerd Font. |
| Correctness and policy | Locked metadata, rustfmt, all-target workspace check, warning-denied Clippy, workspace tests, conformance tests, migration tests, PTY tests, architecture, identity, provenance, package metadata, and `cargo deny` passed. |
| Coverage | LLVM coverage on the exact working tree passed at 45.38% global line coverage versus the 43.52% Windows baseline and 100.00% changed Automexia-owned executable lines. The checker now supports explicit `WORKTREE` mode and includes untracked Rust files. |
| Repository formats | TOML, YAML, JSON, XML, desktop metadata, Markdown documents and local links/anchors, and commit-pinned Actions passed repository validation. PowerShell and Unix shell validation have dedicated wrappers. |
| Performance safeguards | History interaction and resize delivery are fixed and measured on Windows. No-damage snapshots, prompt repaint, cache access, worker submission, bulk parser paths, row rebuild, prompt resize/reflow, bounded queues/caches, and context seeding have focused tests or Criterion benchmarks, but broader renderer/parser/startup claims still need optimized before/after datasets and the 30-day baseline. |
| Windows packaging | A real x86_64 release build produced a WiX MSI and portable ZIP. The ZIP executable reports `automexia 0.4.0`. This audit fixed package lookup under custom `CARGO_TARGET_DIR` and added Windows/Linux regression coverage for the resolved release path. |
| Contributor alignment | Contributor, conduct, security, support, governance, release, upstream, changelog, ownership, issue-form, PR-template, Dependabot, Release Drafter, DCO, protected-path-review, dependency-review, CodeQL, nightly, and release definitions are present and repository-validated. |

## Remaining source work

The following gaps are not covered by the implemented-results table and must
not be described as complete:

1. XTGETTCAP still returns the inherited terminal name `rio` for the `TN`/name
   capability. It must return Automexia's canonical terminal identity and gain
   an identity regression test.
2. Raw OSC overflow, APC/graphics payloads, and XTGETTCAP request buffers can
   grow beyond their optimized initial storage. They need explicit hard caps,
   discard-until-terminator recovery, diagnostics, boundary tests, and fuzz
   coverage so an untrusted PTY process cannot cause unbounded memory growth.
3. Runtime config reload applies `Config::default()` after a load/parse failure.
   It must instead keep the complete last known-good configuration and compiled
   bindings, surface diagnostics, and make no partial changes.
4. Ghostty compatibility beyond the current shortcut subset remains planned.
   The authoritative status and delivery order are in the
   [full compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md).

Items 1–3 are v0.4 correctness/security hardening and take priority over new
compatibility actions. Item 4 is not a v0.4 stable-release criterion.

The complete sequencing, cross-platform evidence, performance matrix, visual
review, hosted assurance, and stable acceptance gates are maintained in the
[stabilization roadmap](STABILIZATION-ROADMAP.md). The functional assessment is
therefore: correctness is mostly solved on tested Windows paths; global
performance proof and security assurance remain partial; stable delivery is
not complete.

## Test evidence from this audit

- Composite-folder/path follow-up: PowerShell/CMD formatter contracts, Bash and
  Zsh integration tests, focused ShellCheck/Perl syntax, a real WSL pseudo-TTY
  run against eza 0.18.2, installed-file SHA-256 comparison, bundled font cmap
  coverage, locked debug build/smoke, repository formats, warning-denied
  Clippy, `verify all`, and the expanded conformance suite passed.
- `cargo ready`: passed the complete local contributor gate and debug binary
  smoke test; its isolated artifacts were removed automatically.
- `cargo xtask test conformance`: passed all selected application, VT, backend,
  window, renderer, and PTY suites, including four pane-footer geometry/routing
  regressions and the DPI-stable terminal-boundary check.
- `cargo xtask test resize-stress`: passed deterministic parser/reflow and
  resize-ordering storms.
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
benchmark, unsigned-package, SBOM, checksum, attestation, notarization, and
clean package-install jobs. A Windows workstation cannot honestly certify the
macOS, Linux package-manager, ARM64-native, notarization, or hosted GitHub jobs.
Those jobs must pass on their declared runners before release.

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

Stable v0.4.0 remains blocked until every item above and every protected native
release job is complete. Unsigned artifacts from this audit are verification
outputs, not release candidates.
