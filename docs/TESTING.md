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

## Build-artifact lifecycle and storage

The workflow has three deliberately separate artifact classes:

- `target/debug` is the persistent incremental application build used by
  `cargo automexia`; it makes ordinary edits and launches fast.
- `target/automexia-verification-v1` is the non-incremental target used by
  `cargo xtask check`, `cargo ci`, `cargo ready`, and the verification phase
  of `cargo dev`. It is removed after success or ordinary failure, so
  all-target checks, Clippy, and tests cannot accumulate separate incremental
  graphs. Cargo's native `cargo check` command remains available for focused
  incremental diagnosis.
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
behavior.

The conformance suite is included in `cargo ready`. For focused diagnosis only,
run it directly with `cargo xtask test conformance`.

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
three-second steady refresh cadence. Release smoke testing must additionally
confirm a real WSL Docker context appears at initial launch and after switching
shells without typing, opening a new prompt, or restarting the terminal.

The Windows shell integration test also waits for the automatic deferred style
event and proves that first-prompt deferral does not remove filesystem icons,
change native `ls` object semantics, or require user input.

These cover Windows-drive versus WSL title classification, custom chrome hit
targets and resize edges, responsive context layout, bundled Nerd icon
codepoints, explicit shell identity, resize-owned full-path three-row prompts,
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

Nightly builds unsigned installers for every artifact target. Stable release
requires WSL, real-GPU, clean-install, upgrade, uninstall, signature,
notarization, URL handler, terminfo, and migration smoke tests on controlled
hardware/self-hosted runners.
