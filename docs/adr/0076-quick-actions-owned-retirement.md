# ADR 0076: Quick Actions owned retirement

Status: accepted for current source; native evidence is platform-specific.

## Problem and placement

The application Quick Actions adapter joined its native search worker when the
last runtime clone was destroyed. Filesystem reconciliation, workspace lookup or
a completion callback could therefore block a window or application teardown.

The adapter now adopts the existing capability-free
`automexia-extension-runtime::BoundedWorker` lifetime described in
[ADR 0073](0073-bounded-extension-worker-retirement.md). The application remains
the sole search, provider, workspace trust and publication authority. A separate
extension, cleanup service or duplicated join registry would add a second owner.
No dependency, persisted data or public configuration format changes.

## Decision

One capacity-one kickoff transfers the existing monitor, index and search loop
to the bounded worker. The established per-route coalescing map, route and
workspace cache limits, watcher cadence, provider identities and search behavior
remain in the application adapter. This consumes the shared runtime's existing
owner allowance and one cleanup service; it introduces no additional reaper.

`request_shutdown` closes admission, invalidates route results and workspace
permissions, wakes the search condition and requests retirement. Final runtime
destruction uses the same nonblocking request. A worker-owned drain guard keeps
queued completion callbacks and their captured destructors off the shutdown
caller, including when the kickoff is cancelled before its handler starts.
Already-drained requests, the filesystem monitor and workspace cache also remain
owned by the worker through destruction.

`shutdown_timeout` shares one caller budget with actual native join
acknowledgement. A false result retains cleanup ownership and is not successful
shutdown. The adapter never starts a replacement after retirement.
`shutdown_status` exposes the shared owner's running, retiring, complete or
cleanup-failed state. Cancellation does not claim to interrupt a blocked native
filesystem operation or arbitrary in-process callback.

The existing pending-state mutex now serializes the final request/provider
identity check and result publication with route removal, provider replacement,
new requests and shutdown. Filesystem/search work and completion callbacks run
outside that mutex. Publication precedes the wake; an already-issued wake after
cancellation cannot retrieve a cancelled result. Cancelling one route preserves
its siblings.

## Evidence and compatibility

A real-worker regression first reproduced blocking final-owner destruction.
Focused regressions cover blocked callbacks, cancelled queued routes, queued
callback destruction, actual native thread-local destruction, bounded timeout
and later acknowledgement, callback re-entry, sibling isolation and repeated
open/search/shutdown cycles. Existing provider-generation, route-capacity,
workspace-revocation and latest-query tests remain applicable.

Native Windows and Linux tests exercise this application owner and its provider
publication integration. Native macOS and BSD watcher behavior and native desktop
frame latency require their own evidence; a cross-compile does not establish
those claims. No throughput or latency improvement beyond the tested
blocked-cleanup boundary is claimed.

There is no data migration. A rollback must preserve asynchronous cleanup
ownership and fail-closed cancellation; restoring the previous synchronous join
would restore the defect.
