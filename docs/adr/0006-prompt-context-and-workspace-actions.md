# ADR 0006: Prompt-owned context and workspace actions

Status: Accepted

Automexia no longer renders the selected session's OS, Git, Docker, cloud,
environment, user, shell, or local clock in global chrome. Every visible pane
already owns generation-scoped live and historical operational context beside
its commands. Repeating selected-pane facts in the header consumed scarce
space, could be mistaken for the identity of another split, and added no new
information.

ADR 0007's local-tab rail belongs inside each pane that owns multiple sessions;
it is not a secondary window-wide chrome row. A single-tab pane does not render
or reserve an empty workspace-action shelf. Find, split, and pane-focus commands
remain available through keyboard shortcuts and the command palette, avoiding
duplicated controls and hidden hit targets. One responsive geometry function
drives pane-local tab drawing, hit-testing, and terminal-grid reservation;
panes below 96 logical pixels fold that rail away without losing tab state.

Operational discovery remains asynchronous and session-scoped. Its cached
facts are rendered only on each pane's semantic prompt rows, with the selected
pane identified by its four-sided accent border. This preserves multi-session and
multi-environment visibility without presenting one pane's identity as a
window-wide truth.

Consequences: the former global status renderer remains a metadata-refresh path
only. Single-tab panes retain all of their space, while each multi-local-tab
pane owns and reserves only its scoped controls. Changes to
chrome reservation require Island hidden-hit-target, local-tab geometry,
responsive extreme-size, prompt-context, and architecture verification.
