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

On Windows, a running debug Automexia process owns its executable. To validate
another checkout or cache without closing that session, provide an alternate
Cargo target directory; `xtask` resolves both absolute and invocation-relative
`CARGO_TARGET_DIR` values for build, smoke, and launch consistently.

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

The DevOps status suite covers bounded WSL probe parsing, session-local
Docker/Kubernetes/cloud/Git/Terraform/user models, truthful badge visibility,
default Docker labeling, and the 100 ms pending/three-second steady refresh
cadence. Release smoke testing must additionally confirm a real WSL Docker
context appears and that switching a local context is reflected without a new
prompt or terminal restart.

These cover Windows-drive versus WSL title classification, custom chrome hit
targets and resize edges, responsive context layout, bundled Nerd icon
codepoints, explicit shell identity, resize-safe full-path three-row prompts, per-command
context snapshots, OSC command status/timing, and context/result survival
through shrink/grow reflow.

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
