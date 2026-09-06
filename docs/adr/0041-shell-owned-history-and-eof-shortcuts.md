# ADR 0041: Shell-owned history and EOF shortcuts

Status: Accepted for current source; native release verification pending

## Decision

Bare Ctrl+R and Ctrl+D are no longer built-in clone commands on any platform.
Normal terminal input passes through the existing encoder to the selected PTY.
The shell owns interpretation: history search, character deletion and EOF depend
on its editor and current line. The old Ctrl+Alt passthrough workarounds are
removed so those chords use normal terminal encoding too.

Clone Active Session Right and Down remain command-palette actions and retain
their existing independent session/PTY owner. They are unbound by default.
Fresh splits, pane/tab navigation and explicit user bindings are unchanged.
No configuration is rewritten and no new shortcut is silently assigned.

This supersedes only the clone/control-key portion of ADR 0011. It does not
change the pinned Ghostty profile or introduce another dispatch authority.
Core input owns defaults; the application palette projects configured labels.
Optional extensions cannot own ubiquitous native shell controls.

## Verification and limitations

Regression tests cover all platform tables, normal and alternate-screen modes,
typed Automexia fallback, exact unbinds, user mappings, reset, clone discovery
and label refresh. The regression failed on the original Ctrl+R interception.
Real shell/PTY, keyboard-layout, IME, visual and accessibility evidence remains
separate from these source tests. Published 0.4.0 artifacts are unchanged.

Native behavior references: [Bash history editing](https://www.gnu.org/software/bash/manual/html_node/Commands-For-History.html)
and [PSReadLine key bindings](https://github.com/PowerShell/PSReadLine/blob/master/PSReadLine/KeyBindings.cs).

## Recovery

An explicit legacy `CloneSplitRight` or `CloneSplitDown` binding can restore
the old behavior deliberately. Removing that mapping restores shell ownership.
The palette labels an overlapping typed mapping as conditional rather than
promising an action that can depend on the current selection or topology.
