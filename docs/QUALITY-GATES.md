# Performance, correctness and release gates

Professional terminal architecture is not only folder structure. The dependency rules need measurable gates so latency, terminal compatibility and extension behavior cannot silently regress.

## Latency classes

### Class A — terminal/render hot path

Examples: PTY read/parse, row rebuild, damage, glyph cache lookup, draw submission.

Rules:
- no extension filesystem/network/process IO;
- no unbounded queues;
- avoid allocation in repeated per-cell/per-glyph loops after warm-up;
- avoid contended global locks;
- prefer immutable snapshots/generation checks between subsystems.

### Class B — interactive application path

Examples: command palette, tab/split routing, settings changes, extension activation.

Rules:
- synchronous work must stay small and deterministic;
- expensive work is submitted to a worker;
- failures must report/degrade rather than block terminal startup.

### Class C — background services

Examples: local context discovery, update checks, future marketplace metadata.

Rules:
- bounded input/output sizes;
- bounded/coalescing queues;
- cancellation/time budgets where work can be long-running;
- no direct mutation of terminal/render internals;
- publish validated models/snapshots.

## v0.3 enforced architecture gates

`verify_architecture.py` mechanically checks:
- one compiled Automexia extension namespace;
- async bounded extension worker;
- non-blocking renderer submission;
- no filesystem discovery in the DevOps render adapter;
- capability declarations and least privilege for the built-in DevOps extension;
- namespaced persisted state plus legacy-state migration;
- exact audited engine pin for fresh stable bootstrap;
- removal of obsolete v0.2 extension source files;
- generic semantic-prompt anchors derived only from blank OSC-133 Prompt rows; no fixed extension toolbar or extra PTY resize lane.

`verify_warning_cleanup.py` protects all reported warning cleanups, including Win32 naming, Sugarloaf irrefutable patterns, Windows-only unused values, DevOps public-surface cleanup and removal of obsolete text-icon constants.

`verify_devops_extension.py` protects the first-party DevOps contract: native/default-enabled prompt-context UI, error/warning/success/info/debug semantics, Windows-aware local config sources, no empty-context card, no custom CLI/process/network execution, no private-use default icons, bounded prompt history/labels, and no fixed top-strip HUD placement.

`verify_visual_system.py` protects the unified Automexia named-color palette, scrollable segmented vector-icon prompt context line, user/session identity model, Windows PowerShell launch normalization, WSL identity propagation and PowerShell/Bash/Zsh prompt/editor integration.

`verify_regressions.py` protects idempotency, mixed-source upgrades, transactional rollback, applied-tree source-quality checks, semantic live-prompt/reflow call sites and legacy ownership/session migrations.

`verify_source_quality.py` lexically validates generated Rust/PowerShell/Python structure and enforces prompt retry/lifecycle, non-blocking WSL, shell portability and release-script contracts. `verify_shell_behavior.py` additionally executes Bash lifecycle/status/idempotency tests when a Bash runtime is available and otherwise reports an explicit skip while static shell checks remain enforced.

## Fast Windows development gate

`BOOTSTRAP-WINDOWS.ps1` owns generated-source normalization through the checkout-pinned `rustfmt`. `DEV-WINDOWS.ps1` runs package/source architecture checks, a non-mutating rustfmt verification, and `cargo check`. With `-Run` it then starts the dev-profile terminal. It intentionally skips the test and release profiles so normal iteration can reuse Cargo's incremental dev artifacts.

## Windows release gate

The full release path must run, in order:

```text
package integrity
→ patcher fixture/idempotency checks
→ architecture regressions
→ warning-clean verification on the applied checkout
→ architecture verification on the applied checkout
→ DevOps extension verification on the applied checkout
→ source-quality + shell-behavior gates
→ generated-source normalization completed during bootstrap/direct apply
→ cargo metadata
→ FORMAT-WINDOWS.ps1 -CheckOnly (cargo fmt --all -- --check)
→ cargo check -p rioterm --all-targets
→ cargo clippy (Automexia-touched crates) --all-targets -- -D warnings
→ terminal-critical cargo tests
→ release build
→ release binary --version smoke test
→ distributable copy
```

A release artifact should not be produced when any earlier gate fails.

## Gates to add before v1.0

### Terminal conformance
- VT/control-sequence regression fixtures;
- Unicode/grapheme/width corpus;
- keyboard protocol matrix;
- OSC/CSI/DCS protocol tests;
- Windows ConPTY lifecycle and resize tests;
- image protocol cases where supported.

### Renderer correctness
- deterministic render-model/golden tests where possible;
- application-chrome layout tests proving navigation/title, extension status and terminal grid never overlap;
- default extension UI must remain understandable without Nerd Font/private-use glyph coverage;
- status labels derived from repository/context data must have explicit width/length bounds;
- damage-region invariants;
- selection/search/cursor precedence tests;
- glyph atlas stress/eviction tests;
- backend feature-parity checklist.

### Performance
- startup time;
- sustained PTY throughput;
- input-to-present latency;
- resize/reflow latency;
- memory at idle and under large scrollback;
- glyph cache hit rate and atlas pressure;
- extension-worker queue/drop/refresh timing.

Track baselines in CI and require an explicit review for material regressions rather than relying on subjective “feels fast” testing.

### Reliability/fault isolation
- extension worker unavailable/disconnected must not prevent shell startup;
- malformed extension/context data fails closed;
- corrupt config has a recoverable path;
- renderer/backend errors produce diagnostics rather than silent blank windows where practical;
- third-party extension traps/crashes must be isolated by the future sandbox host.
