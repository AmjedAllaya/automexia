# ADR 0007: Pane-local independent session tabs

Status: Accepted

Automexia distinguishes three navigation scopes: OS windows, window-level
workspace tabs, and split panes. A user working across several split sessions
also needs to open another terminal specifically inside the selected pane
without changing the surrounding layout or creating an unrelated workspace.

Each `ContextGridItem` therefore owns an ordered local tab stack. Exactly one
`Context` is active and rendered; every sibling retains an independent PTY,
route, terminal grid, history, input queue, launch descriptor, and extension
state. Creating a local tab resolves a fresh launch from the selected context,
including PowerShell/CMD profile, Unix shell, WSL distribution/user, and the
validated logical current directory. It never shares a PTY or silently falls
back to another shell.

Ghostty-compatible `Ctrl+Shift+T` creates a window-level tab,
`Ctrl+Alt+T` creates a local tab in the selected pane, and
`Ctrl+Shift+N` creates an OS window. `Ctrl+T` remains an additional
Automexia window-tab alias. When a pane owns more
than one local tab, the secondary chrome row becomes its local tab rail with
separate select, close, and add hit targets; otherwise the existing workspace
action rail remains visible. Top-row close targets only window-level tabs, and
local close targets only their exact pane tab.

All local PTYs track their pane's effective dimensions so switching never
reveals stale geometry. Route lookup searches active and inactive tabs.
Intentional close operations register exact route tombstones before dropping a
context, preventing delayed PTY shutdown events from closing a surviving
sibling, pane, workspace tab, or window.

Consequences: local tabs preserve layout while consuming one process and PTY
per tab. They are independent sessions rather than multiple views of one PTY.
Changes require binding, route-isolation, tab-order/close, responsive rail
geometry, hit-target, and full frontend regressions.
