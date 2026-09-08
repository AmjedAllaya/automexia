# ADR 0038: Owned PTY trees and broadcast-first shutdown

Status: Accepted

## Context

Each terminal route owns an independent PTY and process tree. Ordinary Windows
ConPTY sessions previously lacked the Job Object ownership already used by
exact managed launches. Application teardown also dropped contexts one at a
time, so each worker could consume its shutdown deadline before the next
session received a request. Multiple panes therefore multiplied exit latency
and could leave descendants outside a verifiable owner boundary.

## Decision

Core terminal and PTY adapters remain the single owners of this behavior.
Optional extensions receive no process-lifecycle authority.

Confirmed child-exit readiness precedes obsolete queued input and resize work;
explicit host shutdown has priority. A single worker owner publishes final
available output, the authoritative exit status, close and render in order.
Transport errors reconcile an already arrived child event but never invent one.
The final available tail is drained across ordinary read batches, releasing the
terminal lease between batches. A separate exact 4 MiB ceiling and per-batch
cancellation checks prevent a surviving producer from draining indefinitely.
Reaching that ceiling emits a content-free warning. This is not a wait for
arbitrary future output; ordinary live-read and resize limits remain unchanged.
Independent boundary tests cover zero, read-batch edges, the byte ceiling and
cancellation. Real native fixtures retain all 1,024 final rows and their marker.

Every ordinary and exact Windows ConPTY process is created suspended, assigned
to a session-specific Job Object configured with kill-on-close, and resumed
only after assignment succeeds. Assignment or resume failure terminates the
child and closes its handles before returning an error. Existing Unix managed
sessions retain their process-group cleanup boundary.

Each context exposes one idempotent shutdown request. Closing a window or the
application broadcasts that request to active and background window tabs,
split panes, pane-local tabs, and parked topologies before route destruction or
worker joins begin. Context destruction remains a fallback for isolated paths.
The final event-loop callback repeats the broadcast safely before clearing
routes, and product quit actions return through that owned application path
instead of terminating the process directly.

Native lifecycle evidence captures the exact temporary-fixture process
identities before closing the application because ConPTY hosting may reparent a
shell. It verifies the owner and every captured descendant exit within a bounded
many-session wall-clock ceiling. Reports contain counts and timings, not command
lines or machine-local process identities.

## Consequences

All Windows terminal sessions have one enforceable descendant-cleanup owner.
Graceful budgets run concurrently across panes and windows, while forced Job
termination remains bounded per session. Launch now fails closed if Job
assignment cannot be established. Windows native evidence does not substitute
for native Unix process-group validation, which remains a platform-specific
release gate.

The feature belongs in core because PTY/process ownership and application
shutdown are mandatory terminal mechanisms. An extension would duplicate
authority and could not reliably contain descendants after core route teardown.
