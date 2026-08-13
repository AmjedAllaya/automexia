# ADR 0008: Pane-local operational footer

Status: Accepted

Automexia needs persistent session controls that remain useful after global
shell, clock, and environment labels were removed from the header. A global
footer would repeat the same ambiguity: in a split or multi-cloud workspace it
could describe only the selected PTY while appearing to describe the window.

Each visible pane therefore owns a renderer-only 32 logical-pixel footer. It
reports pane and local-tab position, effective grid dimensions, selection,
shell-integration readiness, and live-versus-history state. Find and
return-to-live are direct, reversible actions routed to that pane. A passive
footer click focuses the pane but never reaches terminal selection or mouse
reporting.

Layout removes the footer reservation before computing terminal rows and
sending the PTY resize. Renderer text, image overlays, scrollbars and mouse
mapping continue to use the resulting grid dimensions; footer paint and hit
testing share a separate geometry. This prevents terminal output or a cursor
from being hidden behind status UI and prevents the scrollbar from crossing an
action. No escape sequence or footer label is written to terminal history.

The footer progressively removes optional labels and uses icon-only actions on
narrow panes. A pane below 112 logical pixels hides the footer and restores the
space to the PTY; growing it recreates the footer automatically. The behavior
is always enabled and adds no user-facing configuration.

Consequences: each normal pane loses a small fixed vertical strip but gains
session-local navigation and truthful viewport feedback. Changes require
reservation/DPI tests, responsive geometry and hit-target tests, scrollbar
boundary checks, multi-pane route tests, and the existing resize-storm gate.
