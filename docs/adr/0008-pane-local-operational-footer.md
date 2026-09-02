# ADR 0008: Pane-local operational footer

Status: Accepted

Automexia needs persistent session status that remains useful after global
shell, clock, and environment labels were removed from the header. A global
footer would repeat the same ambiguity: in a split or multi-session workspace it
could describe only the selected PTY while appearing to describe the window.

Each visible pane therefore owns a renderer-only 32 logical-pixel footer. Its
normal, minimal status line reports UTF-8, a session-aware LF/CRLF convention,
effective grid dimensions, and local time. Pane and local-tab position,
selection, and history offset appear only when relevant and space permits. The
footer is read-only: it contains no Find, return-to-live, icon-only, or hidden
action targets. A footer click focuses the pane but never reaches terminal
selection, mouse reporting, search, or scroll state. The existing focused-route
maintenance tick requests a redraw for the clock, without a new worker or PTY
traffic.

Layout removes the footer reservation before computing terminal rows and
sending the PTY resize. Renderer text, image overlays, scrollbars and mouse
mapping continue to use the resulting grid dimensions; footer paint and passive
pane routing share a separate geometry. This prevents terminal output or a
cursor from being hidden behind status UI and prevents the scrollbar from
crossing the footer. No escape sequence or footer label is written to terminal
history.

Footer geometry is anchored to pane bounds. The first and last pane absorb the
outer horizontal terminal margins, so a single-pane footer connects to both
window sides and split neighbors meet at the divider without a gutter or
overlap. The vertical top-chrome/root offset is retained because pane rectangles
are root-relative; removing it would paint the footer inside terminal content.
The active segment continues the pane-focus accent along its top and sides. This
keeps one session visually continuous while preserving truthful per-pane
ownership as soon as the workspace is divided.

The footer progressively removes optional status labels on narrow panes. A pane
below 112 logical pixels hides the footer and restores the space to the PTY;
growing it recreates the footer automatically. The behavior is always enabled
and adds no user-facing configuration.

Consequences: each normal pane loses a small fixed vertical strip but gains
truthful session-local viewport feedback. Changes require reservation/DPI tests,
responsive geometry, edge-connection, split-seam tiling and no-action-target tests, scrollbar boundary checks,
passive multi-pane route tests, session-convention and clock-format tests, and
the existing resize-storm gate.
