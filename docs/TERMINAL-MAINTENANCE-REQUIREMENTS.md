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

## Native shell control keys

The current Automexia defaults assign Ctrl+R and Ctrl+D to cloning actions,
as documented in [Keyboard](KEYBOARD.md). They can displace familiar shell
controls. Effective behavior depends on the selected binding profile, user
overrides, input mode and legacy fallback.

This is a current compatibility limitation, not a claim that native shell
bindings have already been restored. The binding tables remain authoritative.

## Verification boundaries

A successful unit test is not a native runtime result. Reflow, input, rendering,
clipboard and process cleanup evidence belongs to the exact package, platform,
shell and display environment that was tested. A reported native failure remains
open until reproduced and checked through its owning path.

[Testing](TESTING.md), [feature reinforcement](FEATURE-TEST-REINFORCEMENT.md)
and [readiness](READINESS-AUDIT.md) define the existing evidence requirements.
[Terminal interaction status](TERMINAL-INTERACTION-REQUIREMENTS.md) records the
current UI owners and limitations.
