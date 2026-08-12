# ADR 0006: Prompt-owned context and workspace actions

Status: Accepted

Automexia no longer renders the selected session's OS, Git, Docker, cloud,
environment, user, shell, or local clock in global chrome. Every visible pane
already owns generation-scoped live and historical operational context beside
its commands. Repeating selected-pane facts in the header consumed scarce
space, could be mistaken for the identity of another split, and added no new
information.

The reserved secondary chrome row instead exposes four frequent, reversible
workspace actions: Find, Split Right, Split Down, and Next Pane. One responsive
geometry function drives drawing and hit-testing. Comfortable layouts include
text labels; compact and minimal layouts retain vector icons; viewports below
260 logical pixels hide the rail while keeping all actions available through
keyboard shortcuts and the command palette. The fixed-size geometry uses no
per-frame heap allocation.

Operational discovery remains asynchronous and session-scoped. Its cached
facts are rendered only on each pane's semantic prompt rows, with the selected
pane identified by its four-sided accent border. This preserves multi-cloud and
multi-environment visibility without presenting one pane's identity as a
window-wide truth.

Consequences: the responsive chrome reservation and terminal grid geometry do
not change, while the former global status renderer becomes a metadata-refresh
path only. Changes to rail geometry or actions require Island hit-target,
responsive extreme-size, screen-routing, prompt-context, and architecture
verification.
