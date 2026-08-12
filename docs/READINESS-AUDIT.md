# v0.4 implementation and readiness audit

Audit date: 2026-08-12

This document reconciles the standalone-rebrand plan and the later prompt,
resize-storm, PowerShell-listing, responsive-layout, session-cloning, semantic
color, performance, build-workflow, packaging, and contributor-policy plans.
It separates implementation evidence from release prerequisites that source
code cannot satisfy.

## Implemented and locally verified

| Plan area | Evidence and result |
|---|---|
| Standalone source and history | The checkout builds without an overlay or bootstrap step. `origin` and `rio-upstream` are configured, Rio base `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2` is an ancestor, and local annotated tag `rio-base-0.5.20-7d595af` resolves to that base. |
| Identity and coexistence | Central identity/path constants, Automexia executable/package/app IDs, environment variables, URL/desktop metadata, terminfo, identity allowlist, and migration/coexistence tests pass. Inherited private `rio-*`, `librio`, Sugarloaf, and engine type names remain intentionally attributed. |
| Repository structure | The frontend lives at `apps/automexia-terminal`; brand, documentation, packaging, shell integration, conformance fixtures, integration tests, and `tools/xtask` use the planned v0.4 layout. The v0.5 engine/crate regrouping remains deliberately deferred. |
| Single-command workflow | `cargo ready`, `cargo dev`, `cargo automexia`, storage preflight, isolated verification targets, runtime launch copies, and cleanup are implemented and documented. The complete `cargo ready` gate passed. |
| Prompt and resize resilience | Generation-scoped OSC prompt metadata, terminal-owned context/path rows, stable `aid`, Unicode-safe full-path reflow, cross-shell semantic path colors that preserve literal text, stale-row repair, snapshot rebuilding, resize deduplication/coalescing, input/shutdown barriers, and transient ConPTY error handling pass deterministic and native tests. |
| Session cloning | `SessionLaunchDescriptor`, independent right/down clone actions, exact PowerShell/Unix/WSL shell/profile/user/distro/directory reconstruction, explicit failure without PowerShell fallback, shortcut compatibility, and route/PID isolation pass unit and native tests. |
| Window and tab scope | On Windows and Linux, `Ctrl+T` dispatches the independent-window action and `Ctrl+Shift+T` retains the current-window global-tab action; `Ctrl+Shift+N` remains a new-window alias. Binding conflict, user override, command-palette action, and shortcut-label regressions pass. |
| Shell history responsiveness | The ConPTY input writer wake-up regression, real PowerShell Up Arrow recall, and `Ctrl+R` reverse search pass. Native stress measured 930 ms shell/VT latency for Up and 1,048 ms for `Ctrl+R`, within the 1.5-second test budget. |
| Responsive UI | Extreme small/large/HiDPI layout, split ratios, tab/control collision, hidden hit targets, palette containment, prompt restoration, and 4K/8K-equivalent transitions are covered by renderer-neutral tests. A native run completed 590 resize/input operations without a blank surface, stale prompt, invalid grid, or crash. |
| Context and colors | Live Git, Docker, Kubernetes, cloud, Terraform, environment, OS/WSL, production, and user roles use distinct anchors with centralized contrast correction. Header/history parity, provider mapping, default distinction, and custom-theme contrast tests pass. |
| PowerShell listings | The native formatting view keeps real `DirectoryInfo`/`FileInfo` objects and renders four metadata columns with the icon adjacent to the name. Unicode, spaces, narrow views, sorting, filtering, piping, and the plain-listing opt-out pass the PowerShell contract suite. |
| Shell integration | Every PowerShell source parses; the Windows integration contract passes. Bash/Zsh syntax, ShellCheck, and integration jobs are part of the Unix local/CI gate. |
| Correctness and policy | Locked metadata, rustfmt, all-target workspace check, warning-denied Clippy, workspace tests, conformance tests, migration tests, PTY tests, architecture, identity, provenance, package metadata, and `cargo deny` passed. |
| Coverage | LLVM coverage on the exact working tree passed at 45.38% global line coverage versus the 43.52% Windows baseline and 100.00% changed Automexia-owned executable lines. The checker now supports explicit `WORKTREE` mode and includes untracked Rust files. |
| Repository formats | TOML, YAML, JSON, XML, desktop metadata, 44 Markdown documents and local links/anchors, and 59 commit-pinned Actions passed repository validation. PowerShell and Unix shell validation have dedicated wrappers. |
| Performance safeguards | No-damage snapshots, prompt repaint, cache access, worker submission, parser, row rebuild, prompt resize/reflow, bounded queues/caches, prompt-local history work, and resize coalescing have focused tests or Criterion benchmarks. Nightly workflow owns optimized measurements and the initial baseline period. |
| Windows packaging | A real x86_64 release build produced a WiX MSI and portable ZIP. The ZIP executable reports `automexia 0.4.0`. This audit fixed package lookup under custom `CARGO_TARGET_DIR` and added Windows/Linux regression coverage for the resolved release path. |
| Contributor alignment | Contributor, conduct, security, support, governance, release, upstream, changelog, ownership, issue-form, PR-template, Dependabot, Release Drafter, DCO, protected-path-review, dependency-review, CodeQL, nightly, and release definitions are present and repository-validated. |

## Test evidence from this audit

- `cargo ready`: passed the complete local contributor gate and debug binary
  smoke test; its isolated artifacts were removed automatically.
- `cargo xtask test conformance`: passed all selected application, VT, backend,
  window, renderer, and PTY suites.
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
