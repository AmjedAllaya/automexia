# ADR 0050: Nonblocking session retirement and lightweight command navigation

Status: Accepted for current source; native desktop evidence remains platform-specific

## Decision and ownership

The application Router owns one injected `PtyWorkerRegistry`, shared by its
windows through ScreenServices and ContextManagerConfig. Ordinary and managed
PTY launches reserve capacity before native creation. Context destruction sends
its existing shutdown request and retires a lease; it never waits for the PTY
thread. All existing pane, local-tab and history-eviction paths use that owner.

The registry lives with the existing rio-vt worker mechanics. One lazily started
cleanup thread joins retired workers. It sleeps on a bounded channel when idle;
it does not create a thread per close. Acknowledgements are emitted after actual
joins, including native thread-local destruction. At most 256 reserved, active
and retiring workers exist per application. Saturation rejects a new launch
with an actionable error, preserving existing sessions and cleanup ownership.
The event loop reclaims acknowledgements at most every 50 ms while retirement
is pending; active sessions alone do not schedule cleanup polling.

Final application shutdown still broadcasts before destroying windows, then
waits for join acknowledgements against one ten-second budget. A timeout is
reported as failure, never interpreted as release of capacity. The cleanup
service itself is joined when drained. A stuck native primitive can prevent
complete final cleanup; this remains an explicit failure, not a success claim.
Native ConPTY Job Objects, Unix shutdown policy and managed-operation revocation
remain authoritative. Recent close acknowledgements are capped at 256; an older
unknown exit still resolves against live/parked route identity, never focus.

Confirmed window close now hides its native surface before dropping the route.
Explicit Quit and the final callback hide every remaining window before any
wait; the final callback destroys routes before preference/service cleanup.
This matters on Windows, where Window destruction posts a message that cannot
run while the final callback is waiting. Backends without visibility control
still destroy their surfaces before service waits. Native compositor timing is
separate evidence. Cancelled confirmation does not enter this path.

Route-level Quit only posts the existing application event. It no longer joins
the shared Connection Hub before the application receives that event. Router
retains shared-service shutdown authority; no service or PTY is detached to
make the visible window disappear faster.

ConPTY's existing reader enters irreversible discard-only draining on owned
shutdown or fallback PTY drop. The 64 KiB ring stays bounded and a saturated
producer wakes; final native output no longer waits for a retired VT consumer.
Live sessions retain normal backpressure and exact output. Separately, pipe EOF
is delivered only after already-buffered bytes, including the final short tail.
The existing two-second graceful and three-second forced Job budgets remain.
This follows the output-drain requirement in Microsoft's ClosePseudoConsole
documentation; no new thread, dependency or extension authority is introduced.

The existing palette owns a fixed header Back control, with an arrow-only
compact variant. Keyboard Back and parent selection restoration remain intact.
Rendering filters once per frame; pointer row hit testing filters once per hit.
Matching lowers the query once, avoids ASCII target copies and character vectors,
and skips matching allocations/sorting for empty browsing. Unicode contextual
lowercasing and stable score ties are preserved. No stale result cache is added.

Legacy shortcut dispatch normalizes the event once, borrows its physical or
logical trigger for each candidate, and clones only matched actions. Ordered
multi-match dispatch, ReceiveChar, modes, AltGr handling and user overrides are
unchanged; no additional binding index or cache competes with the existing table.

## Evidence and alternatives

The real Windows idle-CMD Context regression initially blocked for about two
seconds in native graceful shutdown. A separate native thread-local-destructor
gate disproved the assumption that Rust's finished hint makes a UI join safe.
[Rust JoinHandle](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)
distinguishes completion of the main function from stopping the native thread.
[ClosePseudoConsole](https://learn.microsoft.com/en-us/windows/console/closepseudoconsole)
also has platform-sensitive blocking and output-drain behavior. Neither native
shutdown nor its safety budgets were shortened to disguise the latency.

A per-close thread or unbounded queue was rejected for resource growth. Merely
dropping a JoinHandle would detach cleanup. A UI-owned finished-hint reaper was
rejected by the destructor regression. A new extension or dependency would add
authority and lifecycle overhead to fundamental terminal behavior. Standard
channels and the existing native adapters suffice; no dependency changes occur.

Back follows the existing modal input owner and
[combobox keyboard guidance](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/),
not a claim of web ARIA implementation. Shortcuts, user overrides, shell Ctrl+R/D,
command execution policy, Quick Action review and persistence remain unchanged.

## Validation and rollback

Tests cover blocked workers, native TLS cleanup, panic, capacity boundaries and
recovery, repeated lifecycle isolation, stale close bounds, real ConPTY context
retirement and exact pre-close process handles. Palette tests cover persistent
Back, child return, narrow/HiDPI geometry and an independent legacy score oracle.
The Criterion browse/search/back benchmark measures model cost, not GPU frames.
Native desktop pixels, physical layouts and screen-reader delivery on each
platform remain separate release evidence. Revert normally; no saved schema or
user keybinding migration is involved. This extends ADRs 0038 and 0049.
