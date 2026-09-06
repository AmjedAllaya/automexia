# Terminal maintenance status

This page records existing implementation owners and known verification limits.
It does not publish implementation plans or announce additional features.
[Features](FEATURES.md) is the current availability reference; source support is
not the same as a tested public package.

## Current implementation and evidence

Application paths below are relative to `apps/automexia-terminal/src/`.

| Area | Existing owner | Current evidence and limitations |
|---|---|---|
| Retained content and resize | `rio-vt/src/crosswords/grid/resize.rs`, `rio-vt/src/selection.rs`, `context/renderable.rs` | Grid, selection, property, conformance and resize tests exist. They do not establish every native shell/display combination. |
| Command output decorations | `renderer/command_results.rs`, `renderer/devops_status.rs`, `rio-vt/src/crosswords/mod.rs` | Frame-local ownership, cross-overlay reservation and reflow/navigation regressions are covered by [command-result assurance](COMMAND-RESULT-ASSURANCE.md). |
| Input and paste targeting | `screen/mod.rs`, input routing and session messenger | Input is dispatched through the application session owner. Native clipboard and multi-pane evidence remain environment-specific. |
| Session exit | `context/mod.rs`, `teletypewriter/src/` | Broadcast shutdown, owned process trees and lifecycle tests are documented in [ADR 0038](adr/0038-owned-pty-trees-and-broadcast-shutdown.md). |
| Unicode and graphics | `rio-vt/src/`, `sugarloaf/src/`, `grid_emit.rs` | Existing parser/rendering suites cover declared fixtures. This does not imply universal terminal-protocol compatibility. |
| Responsiveness and resources | Session launch, PTY queues, snapshots and native presentation | Measurements and native evidence are scoped to their recorded host, build, scenario and duration. |

## Reported regression observations

The following user reports remain open pending reproduction against the exact
binary, configuration, shell and display environment. Existing tests and source
mechanisms do not invalidate a report; this documentation review did not run the
native reproductions or establish their causes.

| Observed symptom | Current owner or reference | Evidence limitation |
|---|---|---|
| Retained command output disappears during pane/window resizing | Grid/reflow and renderable snapshot owners above | Check intermediate widths; returning to the old size is not evidence that other sizes rendered correctly. |
| Red/green result highlights disappear on resize and return at the previous size | Command-result anchors, style/damage and renderer owners | Shell ANSI styles and application result accents are distinct; a specific root cause is not yet established. |
| Text overlaps or large unexpected blank gaps appear below `ls` and other output after resizing/moving | Grid, viewport, cell geometry and native presentation | Intentional output blank lines and ordinary unused viewport space are not themselves defects. |
| Context indicators, timestamps and command-output chrome collide or disappear/reappear on small panes | Context projection and renderer overlay owners | Current source coverage does not certify every narrow/fractional-scale combination. |
| Automexia-hosted WSL feels slower than the reference WSL terminal | Session launch, ConPTY, PTY queues and renderer | No same-host latency comparison was performed in this review; cold launch, warm launch and interactive echo are different measurements. |
| A pointer-associated paste reaches the previously selected pane | Application input routing and `screen/mod.rs` | Exact mouse button/modifiers and binding profile must be recorded; current left-click does not paste. See [mouse input](KEYBOARD.md#mouse-input). |

These are current reported limitations, not a universal statement that every
build/platform reproduces them or an announcement of additional capabilities.

## Native shell control keys

The published 0.4.0 defaults assign Ctrl+R and Ctrl+D to cloning actions,
displacing familiar shell controls. Current source removes those built-in
mappings while preserving explicit user overrides and clone palette actions;
see [Keyboard](KEYBOARD.md) and [ADR 0041](adr/0041-shell-owned-history-and-eof-shortcuts.md).
Platform-table and native Windows ConPTY/PSReadLine tests cover the correction.
Graphical key routing, other native shells/platforms, IME and accessibility
remain separate verification gates. No published artifact has been replaced.

## Verification boundaries

A successful unit test is not a native runtime result. Reflow, input, rendering,
clipboard and process cleanup evidence belongs to the exact package, platform,
shell and display environment that was tested. A reported native failure remains
open until reproduced and checked through its owning path.

[Testing](TESTING.md), [feature reinforcement](FEATURE-TEST-REINFORCEMENT.md)
and [readiness](READINESS-AUDIT.md) define the existing evidence requirements.
[Terminal interaction status](TERMINAL-INTERACTION-REQUIREMENTS.md) records the
current UI owners and limitations.
