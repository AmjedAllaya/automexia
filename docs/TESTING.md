# Testing and verification

## Complete local gate

Run the complete contributor gate and produce a smoke-tested debug executable
with one command:

```text
cargo ready
```

Use `cargo dev` to run that same gate and launch Automexia when it passes. Use
`cargo automexia` for a fast incremental build, version smoke, and launch when
the full gate has already passed. These commands are Cargo aliases backed by
`tools/xtask`, so they are identical on Windows, macOS, and Linux.
The launcher returns after a successful spawn, leaving Cargo available for the
next command while Automexia continues running.

The complete gate also parses every repository PowerShell source and executes
the PowerShell formatter/prompt contract on Windows. On Unix it syntax-checks
Bash and Zsh, runs ShellCheck, and executes both shell-integration suites.
`cargo xtask ci` runs the same non-launching gate; neither command leaves its
isolated exhaustive build artifacts behind.

## Build-artifact lifecycle and storage

The workflow has three deliberately separate artifact classes:

- `target/debug` is the persistent incremental application build used by
  `cargo automexia`; it makes ordinary edits and launches fast.
- `target/automexia-verification-v1-<pid>-<generation>` is the unique,
  non-incremental target used by
  `cargo xtask check`, `cargo ci`, `cargo ready`, and the verification phase
  of `cargo dev`. It is removed after success or ordinary failure, so
  all-target checks, Clippy, and tests cannot accumulate separate incremental
  graphs. Concurrent gates use different directories, so one run cannot clean
  another run's build products. Cargo's native `cargo check` command remains
  available for focused incremental diagnosis.
- `target/automexia-runtime` contains generation-specific launch copies. On
  Windows this lets the running process own its copy while Cargo's canonical
  `target/debug/automexia.exe` remains replaceable. Stale, unlocked copies are
  removed automatically before each launch.

Inspect the current target without changing it:

```text
cargo storage
```

The report shows the resolved target, filesystem capacity, total target size,
and its twelve largest direct children. The default warning is 12 GiB. To
recover all build space, first close every Automexia window and then run:

```text
cargo purge
```

`cargo purge` is the cross-platform alias for Cargo's built-in `clean` and
honors `CARGO_TARGET_DIR`. It removes rebuildable artifacts, never configuration
or source files.

An exhaustive verification gate needs at least 12 GiB free before it starts; a
persistent application build needs 4 GiB. This prevents a predictable build
from filling the volume halfway through. The following integer-GiB environment
variables exist for unusual build hosts:

- `AUTOMEXIA_VERIFY_MIN_FREE_GIB` (default `12`);
- `AUTOMEXIA_BUILD_MIN_FREE_GIB` (default `4`);
- `AUTOMEXIA_TARGET_WARN_GIB` (default `12`);
- `AUTOMEXIA_KEEP_VERIFY_TARGET=1` retains verification output for deliberate
  diagnosis instead of deleting it.

Keep the defaults on contributor machines. CI, nightly, and release workflows
set `CARGO_INCREMENTAL=0` and cache only downloaded Cargo registry/Git content,
not `target` build products.

To validate another checkout or filesystem, provide an absolute or
invocation-relative `CARGO_TARGET_DIR`; build, smoke, launch, storage preflight,
and cleanup all resolve the same directory consistently.

The local gate validates all checks that can run on the current host. GitHub CI
keeps separate native and cross-platform jobs for operating-system matrices,
coverage, CodeQL, dependency review, fuzzing, sanitizers, and controlled
hardware that one contributor machine cannot reproduce.

Cargo creates a test executable for each applicable library, binary, integration
target, and documentation target. Therefore, `test result: ok. 0 passed; 0
failed` is expected for a target that defines no tests; it does not mean the
workspace test suite was skipped. The command is successful only when every
target completes and `cargo ready` prints its final `PASS` line.
`cargo ready` summarizes successful harness output to keep the normal workflow
readable and prints the complete captured diagnostics automatically on failure.
Running `cargo test` directly retains Cargo's normal per-target output.

Known incompatible transitive dependency generations are maintained as an
exact, reasoned baseline in `deny.toml`. They do not print repetitive warnings.
Any newly introduced duplicate is denied, while advisories, banned crates,
licenses, and dependency sources continue to be checked independently.

## Every pull request

Every PR runs policy checks regardless of changed paths:

- Cargo metadata/lock consistency and `rustfmt --check`;
- TOML, YAML, JSON, XML, shell, PowerShell, documentation, and link validation;
- product identity, provenance/license, architecture graph, package metadata,
  and brand-manifest verification;
- warning-denied workspace Clippy and workspace tests on Windows, Linux, and
  macOS;
- Linux X11-only, Wayland-only, and combined checks;
- Windows MSVC x64 tests and ARM64 cross-check;
- macOS x64 and ARM64 compile checks;
- `cargo deny`, dependency review, CodeQL, and secret-safe fork permissions;
- LLVM coverage with a non-decreasing recorded global baseline and at least 80%
  line coverage on changed Automexia-owned lines.

An inherited engine file is exempt from the changed-line threshold only while
untouched. Any engine change requires a focused regression test.

To evaluate the exact local working tree, including uncommitted and untracked
Rust sources, generate coverage and select the checker's explicit worktree
mode:

```powershell
$env:BASE_SHA = "HEAD"
$env:HEAD_SHA = "WORKTREE"
$env:COVERAGE_PLATFORM = "windows-x86_64-msvc"
cargo llvm-cov --workspace --locked --lcov --output-path lcov.info
python tools/ci/check_coverage.py
```

For pull requests, CI continues to set `BASE_SHA` and `HEAD_SHA` to the exact
base and head commit IDs. The checker anchors Git and report paths to the
repository root, so invoking it from a wrapper or a different directory cannot
silently inspect the wrong checkout.

## Terminal conformance

Fixtures cover fragmented and malformed VT/CSI/OSC/DCS sequences, OSC 7, OSC
133, OSC 1337 user variables, title/prompt lifecycle, Unicode graphemes,
combining marks, emoji width, and cursor position. PTY suites cover ConPTY and
Unix lifecycle, resize, child exit, teardown, and throughput.

Renderer-neutral goldens cover prompt anchors, clipping, segment truncation,
selection/search precedence, stable OSC prompt identities, metadata-only
incremental snapshots, repeated command transitions, and shrink/grow reflow.
Extension tests cover unavailable,
disconnected, busy, stale, malformed, and oversized inputs plus multi-window
session isolation. Shell tests cover syntax, idempotency, exit status, history
handlers, monotonic prompt identities, UTF-8 lambda handling, and uninstall
behavior. The Windows contract additionally executes a generated native CMD
integration in-process, verifies that `cmd /c` is untouched, validates CMD
shell/user/executable and OSC 7/133 metadata, and runs the shared category-aware
listing helper against Unicode and sensitive/source fixtures.

The conformance suite is included in `cargo ready`. For focused diagnosis only,
run it directly with `cargo xtask test conformance`.

Prompt and resize resilience has a dedicated deterministic gate:

```text
cargo xtask test resize-stress
```

It feeds the real Automexia OSC 7/133/1337 byte stream through the parser at
every fragmentation boundary, performs 2,000 fixed-seed one-column through
8K-equivalent reflows, interleaves editing and command transitions, and checks
cursor bounds, row widths, prompt ordering, stable `aid` ownership, completed
history, and exact logical path/lambda counts after every resize. It also proves
that 1,000 queued PTY resizes collapse to the final size while input and
shutdown remain ordering barriers. A recording PTY sink verifies the exact
delivered resize/input/shutdown order, duplicate suppression, final size, and
retry behavior after an injected transient resize error.
The same suite feeds full-screen clear/home/line-erase repaint sequences used
by shell editors after SIGWINCH. It proves terminal-owned prompt rows repair at
the end of the PTY batch and that combining-mark/emoji paths survive a tiny
viewport, scrollback, and automatic restoration at a usable size.

On an interactive Windows machine with a working GPU, the native driver adds a
real-window storm and validates renderer-neutral JSON snapshots:

```text
cargo xtask test resize-stress --native-gui
```

The driver waits for one complete first prompt without sending input, executes
240 real window moves, restores a usable viewport, and waits for a matching
post-reflow snapshot before asserting. The opt-in `native-gui-test-hooks` build
feature is enabled only by that command. Product builds perform no snapshot I/O.
CI runs the deterministic gate on every pull request; nightly runs the native driver when the protected
`automexia-gpu` self-hosted runner is enabled.

Session cloning has its own deterministic gate:

```text
cargo xtask test session-clone
```

It covers action names, user overrides, Search/Vi exclusions, preservation of
bare shell control keys, PowerShell/pwsh, nested/direct CMD, and native Bash/Zsh descriptors, direct
and nested WSL descriptors, incomplete metadata, spaces/Unicode, environment
overrides, unknown/invalid logical directories, safe profile fallback, and
CreateProcess-compatible quoting. On a Windows GPU workstation,
the full native clone-plus-resize storm is:

```text
cargo xtask test session-clone --native-windows
```

That driver first executes a unique PowerShell command and requires both Up
Arrow recall and `Ctrl+R` reverse search to repaint through ConPTY within the
1.5-second native budget. The workspace's Windows-only PTY regression also
starts a clean real PowerShell process, negotiates Win32 input-record mode, and
checks both operations without the renderer so protocol and shell latency stay
separable. It then creates three independent PowerShell clones (four panes total),
verifies unique routes and ConPTY child PIDs, proves clone-only input/output
cannot contaminate the source, and interleaves the final clone plus active-pane
changes into 240 resize transitions before checking prompt/path restoration. A
controlled WSL runner additionally executes:

```text
cargo xtask test session-clone --native-wsl
```

Set `AUTOMEXIA_TEST_WSL_DISTRO` to the installed test distro. The suite proves
that distro, user, executable, profile, and Linux directory survive cloning and
that no PowerShell fallback is opened. These native hooks are feature-gated and
are absent from product builds.

For native liquid-hacker UI and prompt regressions, run:

```text
cargo test -p rio-vt semantic
cargo test -p automexia-terminal renderer::island::tests
cargo test -p automexia-terminal renderer::devops_status::tests
powershell -NoProfile -File tools/ci/test_shell_integration.ps1
```

The DevOps/runtime suites cover bounded WSL probe parsing, session-local
Docker/Kubernetes/cloud/Git/Terraform/user models, truthful badge visibility,
default Docker labeling, snapshot-publication-before-redraw ordering,
originating-route wake-up, strictly equivalent five-second snapshot reuse,
rejection of cross-path/unintegrated/stale reuse, and the 100 ms fallback/
three-second steady refresh cadence. Renderer tests additionally enforce every
semantic color anchor, independent AWS/Azure/GCP/unknown-cloud mapping,
pairwise default-theme distinction, shared live/history resolution, and
quantized 4.5:1 contrast on dark, light, low-contrast, and custom backgrounds.
Release smoke testing must additionally
confirm a real WSL Docker context appears at initial launch and after switching
shells without typing, opening a new prompt, or restarting the terminal.

The Windows shell integration test also waits for the automatic deferred style
event and proves that first-prompt deferral does not remove filesystem icons,
change native `ls` object semantics, or require user input. Its listing fixtures
cover the four native metadata columns, icon/name adjacency, directory suffixes,
spaces and Unicode, narrow-width truncation, and real `DirectoryInfo`/`FileInfo`
values after filtering and sorting. CMD coverage proves its interactive launcher
uses the existing PTY rather than a detached process, direct configured CMD
profiles receive integration once, nested CMD clones retain `%ComSpec%` and the
live directory, `ls`/`ll` keep icon/name adjacency, and built-in `dir` remains
unmodified.

These cover Windows-drive versus WSL title classification, custom chrome hit
targets and resize edges, the responsive workspace-action rail and its exact
Find/split/focus routing, bundled Nerd icon
codepoints, explicit shell identity, terminal-owned full-path three-row prompts,
per-command context snapshots, OSC command status/timing, and context/result
survival through shrink/grow reflow.

Responsive regressions exercise the supported 300×200 minimum, compact and
comfortable breakpoints, transient invalid dimensions, 4K/8K HiDPI logical
equivalence, very large grid counts, tab/control non-overlap, hidden-control
hit targets, adaptive palette row counts, overlay containment, Unicode-safe
label elision and split-layout underflow. Run the focused set with:

```text
cargo test -p automexia-terminal renderer::responsive::tests
cargo test -p automexia-terminal renderer::island::tests
cargo test -p automexia-terminal renderer::command_palette::tests
cargo test -p automexia-terminal layout::compute_tests
```

## Nightly and release depth

Nightly jobs fuzz VT, OSC metadata, configuration migration, semantic
classification, and label sanitization. Suitable pure crates run Miri and
ASan/TSan. Criterion tracks parser throughput, row rebuild, prompt layout, cache
access, and worker submission.

The renderer-neutral application-service benchmarks are available with:

```text
cargo bench -p automexia-terminal --bench automexia_services -- --noplot
```

They measure the route-scoped extension snapshot cache and non-blocking bounded
worker submission path. `rio-vt`'s `row_rebuild_full_snapshot` and
`prompt_layout_resize_reflow` cases cover full visible-row materialization and
repeated narrow/wide semantic-prompt reflow.

Performance tracking includes startup, sustained PTY throughput, resize/reflow
latency, idle/scrollback memory, and extension refresh latency. Results are
informational for the first 30-day baseline; afterward, regressions above 5%
latency or 10% memory need a recorded maintainer waiver.

For a focused optimized measurement of the most common unchanged-frame fast
path, run:

```text
cargo bench -p rio-vt --bench vt_input snapshot_visible_noop -- --noplot
```

The benchmark starts from a populated styled terminal and repeatedly requests
a no-damage snapshot. It protects the contract that cursor/UI-only activity
does not copy the resident style table or visible grid. Run the complete
`vt_input` benchmark before and after changes to parser, grid, or snapshot code;
record the machine, power mode, and median result in any performance waiver.

Shell-history repaint latency has a dedicated deep-scrollback benchmark:

```text
cargo bench -p rio-vt --bench vt_input history_navigation_repaint_deep_scrollback -- --noplot
```

It keeps 15,000 historical rows behind an active OSC 133 prompt and measures
the clear/repaint pattern emitted by PSReadLine, Readline, and ZLE for Up Arrow
and reverse-history search. Runtime must remain proportional to the live prompt
block, not the configured scrollback depth.

Nightly builds unsigned installers for every artifact target. Stable release
requires WSL, real-GPU, clean-install, upgrade, uninstall, signature,
notarization, URL handler, terminfo, and migration smoke tests on controlled
hardware/self-hosted runners.
