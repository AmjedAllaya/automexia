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

The clipboard path converts CRLF and lone CR to LF before review and removes
terminal control characters except Tab and LF. Multiline clipboard text opens a
preview for the captured pane. The preview owns its input and keeps the prepared
transaction until the user accepts or cancels it. Acceptance queues the same
text used by the preview. The context and PTY worker both check the observed
input destination. A recorded shell, prompt or input-mode change cancels the
transaction even when the same pane and PTY remain open.

An explicit **Bash batch** choice on the normal screen opens a second review
of the exact generated text for an empty Bash prompt. The application quotes
the normalized text into one printable ASCII line. The line contains no
control input, including on terminals without bracketed paste. When the user
submits it, Bash supplies a process-substitution path to the child Bash and
the interactive shell reports a child failure. The child owns fail-fast
behavior and terminal stdin remains available. Selecting the batch option
queues nothing. Confirming the second review uses the same guarded paste
transaction and size limit as ordinary paste.

[XTerm's control-sequence specification](https://www.invisible-island.net/xterm/ctlseqs/ctlseqs.html)
defines bracketed paste framing. The PTY worker reads the live mode before
encoding the transaction. Unbracketed paste encodes line breaks as CR. Embedded
newlines can execute commands when the receiving application lacks bracketed
paste support. Paste never appends an extra Enter, and literal key input keeps
its own byte contract.

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
