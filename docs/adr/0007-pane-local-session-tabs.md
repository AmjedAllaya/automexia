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

On Windows, a WSL clone checks distribution availability on one bounded
background worker per window. The check owns its client process, output limits,
deadline and cancellation. It publishes the result before waking the window.
A changed source route or launch descriptor discards the result; it cannot
open a tab in another pane. A closed window cancels its check without joining
the worker on the interface thread. The existing PTY owner creates the session
only after the check succeeds.
An app-wide query slot retains any unconfirmed process cleanup across window
closure. Later requests retry that exact owner before another query can start.

Classic Automexia `Ctrl+T` creates a window-level tab,
`Ctrl+Shift+T` creates a local tab in the selected pane, and
`Ctrl+Shift+N` creates an OS window. When a pane owns more than one local tab,
a 36 logical-pixel rail is rendered inside that pane's top edge with separate
select, close, and add hit targets. Every eligible pane can expose its own rail;
sibling panes are neither shifted nor resized. The rail is omitted for one tab
and for a pane below 96 logical pixels, returning its space to the terminal
without discarding tab state. Top-row close targets only
window-level tabs, and local close targets only their exact pane tab.

All local PTYs track their pane's effective dimensions so switching never
reveals stale geometry. Route lookup searches active and inactive tabs.
Intentional close operations register exact route tombstones before dropping a
context, preventing delayed PTY shutdown events from closing a surviving
sibling, pane, workspace tab, or window.

Consequences: local tabs preserve layout while consuming one process and PTY
per tab. They are independent sessions rather than multiple views of one PTY.
Changes require binding, route-isolation, tab-order/close, per-pane PTY sizing,
responsive rail geometry, HiDPI hit-target, and full frontend regressions.
