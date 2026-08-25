# ADR 0025: Authenticated native-editor suggestion bridge

- Status: Accepted; source implementation is authorized, preview and stable activation remain disabled pending CP5.6 release evidence
- Date: 2026-08-23

## Context

CP1 already provides Automexia's complete supported completion path: each shell
owns its editor, native completers, candidate UI, quoting, insertion, history,
and Enter behavior. CP5.0 found no uniform mutation-free editor API across
PSReadLine, Readline, ZLE, Fish, CMD, and WSL, retained CP1, and required a new
security decision before an Automexia-rendered suggestion surface could exist.

Here, “editor” means the shell-owned editable command line. This record is
unrelated to the proposed
[Automation Studio](../AUTOMATION-STUDIO-ARCHITECTURE.md) file editor and grants
no document, webview, language-server, file-write, or script-run authority.

An app popup cannot safely infer the editable command from terminal cells or
OSC. It needs the active editor to deliberately provide a bounded buffer,
cursor, selection, replacement span, quote context, and generation. That data
is private and security-sensitive. A local endpoint also introduces
impersonation, replay, cross-pane publication, stale replacement, input-capture,
and per-keystroke amplification risks. The six stable threats and their
mutation owners are frozen in
[`cp51-bridge-threat-contract-v1.json`](../../tests/fixtures/command-productivity/cp51-bridge-threat-contract-v1.json).

Windows named pipes are not private under their default security descriptor.
The server must supply a current-user/current-logon-session descriptor, create
the first instance deliberately, set `PIPE_REJECT_REMOTE_CLIENTS`, and verify
the connected process and terminal session. Linux filesystem Unix sockets need
a private `0700` parent, a `0600` socket, and `SO_PEERCRED`; macOS uses the same
filesystem topology and `getpeereid`. TCP, UDP, abstract sockets, OSC, terminal
output, and terminal-grid inference are not acceptable transports.

The shell APIs also prevent a universal shortcut. Fish currently uses
`Ctrl+Space` to insert a space, while users and accessibility tools can bind the
same chord elsewhere. CP5 therefore cannot install a global `Ctrl+Space`
binding or replace Tab, Right, Enter, native prediction, or native completion.

## Decision

Automexia will add an optional preview feature with these owners:

- `automexia-devops::suggestions` owns capability-free versioned request,
  candidate, source, ranking, validation, and limit models. It performs no IO.
- `automexia-ui-model::suggestions` owns immutable listbox/option semantics,
  density, grapheme-safe display, accessible names, placement inputs, and
  hit-test geometry. It performs no IO and imports no application type.
- the desktop application composition root owns one bounded joined broker,
  endpoint lifecycle, per-route capability state, one-latest-generation slots,
  last-known-good memory cache, renderer snapshot publication, cancellation,
  and shutdown;
- thin Windows and Unix adapters own only endpoint creation, restrictive access,
  peer verification, framed reads/writes, and cancellation. Existing
  `windows-sys` and `libc` dependencies are sufficient; no new runtime
  dependency is approved;
- session-scoped shell adapters remain the sole owners of buffer capture,
  native completer/history queries, quoting, replacement-span calculation,
  acceptance, and insertion. They revalidate all state and replace once without
  Enter. Automexia never writes candidate text to the PTY.

The application creates a fresh 32-byte random capability for every route and
rotates it on session start, route rebind, kill, and teardown. The capability is
delivered only through inherited session setup, never argv, terminal output,
OSC, logs, diagnostics, or persistence. It is compared in constant time. Every
message binds schema, request, application generation, window, tab, pane,
session, shell/editor version, endpoint instance, capability, prompt and buffer
generations, cancellation, cursor byte and grapheme, selection, exact span,
quote/token context, cwd, mode, source revision, and reason. Replay tolerance is
zero.

This boundary does not claim to defend against a fully compromised process in
the same user session that can inspect Automexia or editor-process memory. That
is an operating-system account-compromise boundary; the bridge still minimizes
exposure, lifetime, replay, cross-route publication, and accidental disclosure.

Frames use a checked little-endian `u32` length followed by strict UTF-8 JSON.
The length is rejected before payload allocation; compressed and unknown-field
messages fail closed. Payloads are memory-only and are dropped after response.
Only public counters and redacted reason/error codes may enter observability.

The Windows server uses a private named pipe, `FILE_FLAG_FIRST_PIPE_INSTANCE`,
`PIPE_REJECT_REMOTE_CLIENTS`, an explicit current-user plus logon-SID ACL, and
client process/session verification. Unix uses a randomly named filesystem
socket below an application-owned `0700` runtime directory, sets and verifies
`0600`, rejects links and non-sockets, and checks the effective peer uid with
Linux `SO_PEERCRED` or macOS `getpeereid`. Endpoint names are non-secret; the
capability is the authentication secret. All handles, sockets, paths, clients,
workers, and relay processes have exact joined teardown owners.

PowerShell 7.2 with PSReadLine 2.2.2 or later may use its supported predictor
and editor contracts. Bash 5, Zsh 5.8, and Fish 3.6 or later may install only a
session-scoped function/widget plus one persistent signed Automexia helper; no
process is launched per key. WSL keeps the guest editor authoritative and may
use one session-resident signed host relay, with exact process-tree cleanup and
native evidence required. Windows PowerShell 5.1 and CMD remain truthful native
fallbacks with no Automexia popup.

No shortcut is installed by default. An explicit suggestion request can be
bound only after the shell adapter and Automexia binding registry both report
the chord free; otherwise it remains discoverable but unbound. Fish explicitly
reports the default `Ctrl+Space` collision. While the surface is active, only
the reviewed shell-advertised navigation/acceptance keys may be consumed. Enter
always dismisses the popup and is forwarded to the native editor for normal
submission; it never accepts or executes a suggestion.

Candidate sources and priority are fixed: native shell results; separately
opted-in in-memory shell history; nonrecursive cwd/executable results returned
by the shell; separately opted-in decayed candidate-ID counters; existing CP1
and cached public CP4 snapshots; then typed CP2/CP3 actions. Typing may never
perform provider/plugin processes, network, authentication, credential-store
access, secret expansion, clipboard reads, terminal-output reads, history-file
reads, recursive traversal, or AI calls.

Ranking is deterministic: exact prefix, native rank, word-boundary prefix,
case-insensitive prefix, opted-in frequency, then fuzzy score; ties use source
priority, normalized display, and request-local stable candidate ID. One pane
queues only its latest generation. Late, mismatched, cancelled, oversized, or
deadline-exceeded results are rejected before publication.

The immutable renderer-neutral surface belongs to exactly one pane. It is
clipped above terminal content and below modals, avoids the cursor, IME, tabs,
footer, dialogs, and siblings, and falls back to a compact noninteractive hint
when no safe rectangle exists. Each option exposes matched graphemes, kind icon
and text, bounded description, source, freshness, textual risk, position, set
size, and the full accessible insertion value. Color is redundant. Motion is
optional opacity only, at most 120 ms, and disabled under reduced motion.

The feature starts disabled behind a preview flag, with history and frequency
separately disabled. A runtime kill switch immediately closes the surface,
cancels work, rejects clients, rotates capabilities, and returns to CP1 without
restart or profile edits. Disable, reset, and uninstall remove only
Automexia-owned session artifacts and counters; native profiles, bindings,
history, completers, predictors, provider files, CP1 artifacts, and actions are
untouched.

The machine contract freezes 16 KiB buffers, 1 MiB frames, 512 candidates,
1 KiB per candidate, 512 KiB batches, 12 visible rows, one queued request per
pane, 64 active routes, and an 8 MiB cache. Local-source deadlines are 250 ms;
release targets are at most 50 ms warm-local p95, 8 ms render p95, and 50 ms
cancellation p95 on named controlled hardware.

## Alternatives

- Keep CP1 permanently and ship no CP5 surface: safe and remains the fallback,
  but does not provide one pane-aware cross-source view. It remains the default
  until CP5 passes every gate.
- Infer the buffer from terminal cells, OSC, prompts, or PTY output: rejected;
  rendering is not an editor protocol and would disclose unrelated output.
- Use localhost TCP/WebSocket: rejected because it broadens discovery, firewall,
  namespace, and remote-client exposure without providing better local identity.
- Use default named-pipe security or Linux abstract sockets: rejected because
  the endpoint would not have the required portable filesystem/access boundary.
- Embed Reedline or replace native editors: rejected because it creates a second
  editor owner and breaks shell semantics, configuration, IME, and accessibility.
- Adopt a new matcher/IPC dependency: rejected; CP5.0 measurements favored the
  in-tree deterministic matcher and existing platform APIs cover the transport.
- Spawn provider CLIs, plugins, or a helper per key: rejected for authority,
  latency, prompts, credentials, cancellation, and resource-amplification risk.
- Install `Ctrl+Space`, Tab, Right, or Enter globally: rejected because bindings
  are shell/user/platform dependent and Fish already gives `Ctrl+Space` meaning.

## Acceptance and required verification

The project owner explicitly accepted this ADR and the exact schema-1 machine
contract on 2026-08-25. Acceptance authorizes source implementation, not preview
or stable release. Preview activation remains disabled until its local gates
pass; stable activation remains forbidden until CP5.6 native, accessibility,
package, performance, resource, rollback, and longitudinal evidence is attached
to the exact release artifact.

Implementation must proceed in CP5.1 through CP5.6 order and satisfy the evidence
ladder in
[`CP51-CP56-IMPLEMENTATION-AUDIT.md`](../research/CP51-CP56-IMPLEMENTATION-AUDIT.md).
Required evidence includes protocol property/fuzz/mutation tests; native peer,
ACL/mode, replay, teardown, and route-isolation tests; redaction canaries;
deterministic ranking and saturation models; renderer-neutral goldens; native
PowerShell/Bash/Zsh/Fish/WSL insertion; keyboard collision, IME, focus, resize,
modal, accessibility, and multi-pane automation; latency/allocation/resource
baselines; 1,000 lifecycle cycles; a 30-day controlled soak; signed package
install/update/kill/disable/uninstall/rollback; and verified CP1 fallback.

## Implementation status

CP5.1's strict request/submission/replacement/authenticated-status codecs,
route validation, restrictive Windows and Unix endpoint sources, joined latest-
only service, and one bounded application read/publication/reply exchange are
implemented with property, fuzz, fragmentation, replay, supersession, exact-
limit, cleanup, and native Windows tests. CP5.2's six bounded sources and CP5.3's
deterministic ranking/insertion-safety model are implemented at their source/
local model boundaries.

CP5.4 now has a pane-owned UI model, controller, draw-only renderer, source-side
publication mailbox, exact candidate reconstruction, authenticated dismiss/no-
candidate replies, 64-route bound, 250 ms source deadline, 30-second UI response
deadline, and kill/route wakeup. It is not connected to a live screen because
runtime activation remains forbidden.

CP5.5 now includes the inert `automexia-suggestion-helper` package binary target,
bounded bootstrap/transport/session/endpoint runner, strict 2,176-byte shell
response envelope, and session-only PowerShell 7, Bash 5, Zsh 5.8, and Fish 3.6
request/response adapters. Each adapter revalidates generation/span/current
buffer, strict UTF-8, C0/C1 controls, and bidi controls before one native
replacement without Enter. Local native evidence covers Windows PowerShell
preview/anonymous-handle/status/cleanup and WSL Bash, Zsh, and Fish enable,
Unicode span, replacement, stale, hostile payload, and callback-return paths.
Fish uses fixed inherited fd 3/4, `/dev/fd/4`, a 2,176-byte read limit, and a
streaming 512-item/4-KiB-line native-completion loop.

CP5.6 still keeps `runtime_activation: false`. The launcher and restricted
handle inheritance, package signing/attestation, WSL host relay, interactive
PowerShell replacement, live screen composition, public preview/privacy/source
controls, native Linux/macOS endpoint churn, profile preservation, controlled
GPU/accessibility/resource evidence, installer update/uninstall/rollback, SBOM
attachment, named-hardware distributions, real endpoint lifecycle campaign, and
30-day soak remain release prerequisites. Hosted jobs now contain OS-specific
native bridge steps, but they count only after an exact remote run succeeds.
CP1 remains the default and complete fallback.
Primary references:
[Microsoft named-pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights),
[`CreateNamedPipe`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipew),
[`GetNamedPipeClientProcessId`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid),
[`GetNamedPipeClientSessionId`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientsessionid),
[Linux `unix(7)`](https://man7.org/linux/man-pages/man7/unix.7.html),
[macOS `getpeereid(3)`](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/getpeereid.3.html),
[PSReadLine](https://learn.microsoft.com/en-us/powershell/module/psreadline/about/about_psreadline),
[Bash programmable completion](https://www.gnu.org/software/bash/manual/html_node/Programmable-Completion.html),
[Zsh completion widgets](https://zsh.sourceforge.io/Doc/Release/Completion-Widgets.html),
[Fish interactive use](https://fishshell.com/docs/current/interactive.html), and
[WAI-ARIA listbox guidance](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/).
