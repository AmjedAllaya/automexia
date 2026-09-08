# ADR 0011: Restore Automexia keyboard defaults

Status: Accepted

The clone and displaced-shell-control defaults below are superseded in current
source by [ADR 0041](0041-shell-owned-history-and-eof-shortcuts.md). Other tab,
navigation and clipboard decisions remain unchanged. Current fresh-split and
non-shell clone chords are governed by
[ADR 0049](0049-grouped-command-discovery-and-pane-shortcuts.md).

## Context

ADR 0010 made a hand-maintained subset of Ghostty bindings the implicit
Automexia default. User evaluation found the established Automexia navigation
model faster and more memorable, especially its paired `R`/`D` clone and split
directions and its distinct window-tab and pane-local-tab chords.

## Decision

Automexia restores its original platform defaults. On Windows, Linux, and BSD:

- `Ctrl+T` creates a window-level tab;
- `Ctrl+Shift+T` creates a pane-local independent tab;
- `Ctrl+Shift+N` creates a separate OS window;
- `Ctrl+Shift+R` / `Ctrl+Shift+D` create fresh right/down splits;
- `Ctrl+R` / `Ctrl+D` clone the active session right/down;
- `Ctrl+Alt+R` / `Ctrl+Alt+D` send history-search/EOF controls to the PTY.
- `Alt`+Arrow focuses the nearest pane geometrically while `F6` /
  `Shift+F6` retains cyclic pane navigation;
- `Alt`+`PageDown` / `Alt`+`PageUp` moves between independent tabs inside the
  selected pane, while `Ctrl`+`Tab` remains window-level.

Windows additionally treats both `Ctrl+V` and `Ctrl+Shift+V` as clipboard
paste for every selected child session, including WSL and SSH. An explicit
`Ctrl+V = ReceiveChar` user binding restores terminal-application ownership.

macOS retains `Cmd+T`, `Cmd+Shift+T`, `Cmd+D`, and `Cmd+Shift+D` for its native
tab and fresh-split scopes; cloning and explicit shell passthrough use the same
Control chords as other platforms. macOS geometric focus uses
`Cmd`+`Alt`+Arrow and pane-local tabs use `Cmd`+`Alt`+`]` / `Cmd`+`Alt`+`[`.

User configuration remains authoritative. Newer actions such as configuration
reload, window close, and clear-screen remain available through configuration
and the command palette even when they do not receive a classic default chord.

Ghostty compatibility moves back to the roadmap as an explicit, versioned,
opt-in profile. It must never silently replace `automexia` defaults or rewrite
user configuration.

## Verification

Host-independent tests construct the Automexia macOS, Windows, and Linux/BSD tables,
assert the classic scopes and shell passthroughs, prove that Windows
`Ctrl+V` and `Ctrl+Shift+V` resolve to the shared Paste action, exercise the
`ReceiveChar` override, and
check command-palette labels. The architecture verifier prevents the clone,
fresh-split, and explicit shell-control chords from drifting apart.

## Consequences

Existing Automexia users regain the original muscle memory. Bare `Ctrl+R` and
`Ctrl+D` are application shortcuts again, so shell history search and EOF use
the documented `Ctrl+Alt` passthroughs unless a user overrides the defaults.
Ghostty migration remains possible later without coupling normal startup or
the v0.4 release to an incomplete compatibility profile.

On Windows, applications that use `Ctrl+V` as a terminal command must opt out
with `ReceiveChar`. Linux/BSD retains `Ctrl+Shift+V`, and macOS retains
`Cmd+V`, so their established terminal and platform conventions do not change.
