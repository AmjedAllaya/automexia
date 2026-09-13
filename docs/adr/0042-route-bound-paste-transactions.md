# ADR 0042: Route-bound paste transactions

Status: Accepted for current source; native GUI validation outstanding.

## Decision

Paste remains an application-owned terminal input operation. The existing
context manager captures route and terminal identity before reading the
clipboard; delivery consumes that target and rejects replacement, shutdown or
disconnection without redirecting to current focus. Existing clipboard providers
remain synchronous. This change does not introduce a worker or another session,
PTY, credential or persistence authority.

The terminal input owner prepares at most 1 MiB of UTF-8 text and sends one
queue message containing the entire transaction. Oversized text is rejected
before changing scroll or selection, with a fixed, content-free warning.
This bounds application conversion and individual queue messages, not OS-owned
clipboard allocation or aggregate input queue growth.

[XTerm's control-sequence specification](https://www.invisible-island.net/xterm/ctlseqs/ctlseqs.html)
defines bracketed paste framing. Existing ESC/ETX filtering is retained inside
the frame. Unbracketed clipboard paste retains CRLF/LF normalization; literal
input remains byte-identical. No Enter is appended. Embedded newlines can still
execute commands in a shell without bracketed-paste support.

Secondary pointer gestures select the hit-tested terminal pane before resolving
selection and clipboard actions. Target selection survives this focus change
so right-click can copy it instead of accidentally pasting. Pane rails, footers,
gaps and active search/palette ownership cannot become paste destinations.
Host-consumed secondary presses also consume their releases after mode/modifier
changes. Terminal mouse reporting retains its Shift override.

## Placement and alternatives

Extend host context, layout, screen and existing diagnostic owners. An optional
extension cannot own essential input for every shell. A new core clipboard
provider or process broker would duplicate existing authority. No dependency,
protocol negotiation, stored configuration or global clipboard mutation is added.
Reversion is a source-only rollback with no migration, but must retain the exact
byte and pane-isolation regression rather than restore unsafe three-part writes.

## Evidence and limitations

Tests enable mode 2004 through the real VT parser, capture actual input-channel
messages, switch focus between capture and delivery, and assert exact framing
and sibling silence. Negative tests cover absent/replaced/closing destinations,
disconnection, empty/oversized text, Unicode boundaries, raw input and no unwanted
selection changes. Layout and per-button tests cover chrome exclusion and release
ownership independently of the clipboard provider.

These are renderer-neutral tests, not native clipboard, IME, accessibility,
pixel or shell execution evidence. Those supported-platform gates remain open.
The published Linux 0.4.0 prerelease predates this change.
