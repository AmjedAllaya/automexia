# Testing and verification

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
selection/search precedence, and reflow. Extension tests cover unavailable,
disconnected, busy, stale, malformed, and oversized inputs plus multi-window
session isolation. Shell tests cover syntax, idempotency, exit status, history
handlers, UTF-8 lambda handling, and uninstall behavior.

Run the primary suite with `cargo xtask test conformance`.

## Nightly and release depth

Nightly jobs fuzz VT, OSC metadata, configuration migration, semantic
classification, and label sanitization. Suitable pure crates run Miri and
ASan/TSan. Criterion tracks parser throughput, row rebuild, prompt layout, cache
access, and worker submission.

Performance tracking includes startup, sustained PTY throughput, resize/reflow
latency, idle/scrollback memory, and extension refresh latency. Results are
informational for the first 30-day baseline; afterward, regressions above 5%
latency or 10% memory need a recorded maintainer waiver.

Nightly builds unsigned installers for every artifact target. Stable release
requires WSL, real-GPU, clean-install, upgrade, uninstall, signature,
notarization, URL handler, terminfo, and migration smoke tests on controlled
hardware/self-hosted runners.
