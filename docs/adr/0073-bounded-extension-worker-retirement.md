# ADR 0073: bounded extension worker retirement

Status: accepted for current source; native evidence is platform-specific.

## Problem and placement

Worker registration callbacks previously ran while holding the worker mutex.
Shutdown could wait for queue capacity or join work on its caller, and a finished
Rust function did not establish completion of native thread-local destruction.

Extend the existing capability-free `automexia-extension-runtime::BoundedWorker`.
This mechanism serves current application consumers, so a new extension, crate
or process service would duplicate ownership. Provider authority, domain request
identity, cancellation and result publication remain with each consumer. The
PTY-specific registry retains its own lifecycle; its dependency direction and
process responsibilities do not fit a shared extension worker.

## Decision

Registration runs after bounded admission and outside runtime locks. A lease
counts admitted callbacks until they return or unwind. A retired generation
cannot be replaced until its actual native join and all registrations finish.
Retirement during registration returns `Unavailable`; the consumer must retire
its domain registration and reject obsolete results before publication.

Each owner lazily starts one reusable cleanup service and at most one worker.
The cleanup service owns the native worker handle through a one-slot mailbox.
A process-wide atomic limit admits at most 64 cleanup owners. A dropped owner
with blocked work retains its admission until the service exits. This fixed
extra thread per active owner is an explicit resource cost; no startup service,
new dependency, capability or persisted schema is introduced.

`request_shutdown` sets cancellation and attempts a nonblocking control send;
a full queue already wakes the worker. Idle workers block on their channel.
Only registration waits poll, with a ten-millisecond interval. A running handler
must honor its own operation bounds and cancellation.

`shutdown_timeout` waits outside the slot mutex for actual join and registration
completion. A false result retains the generation and refuses replacement.
`shutdown_status` distinguishes running, retiring, complete and cleanup failed.
The compatibility `shutdown` wrapper has a two-second acknowledgement budget.
The application checks that result during background shutdown, while review
runtime destruction invalidates its request and only requests retirement.

Last-owner destruction does not join on the caller. It closes the cleanup
mailbox while the service continues owning the worker until actual termination.
Blocked native work can therefore outlive the public owner, with its bounded
capacity still charged. This is incomplete cleanup, never proof of success.

Recoverable panic payload destruction runs inside the worker's native join
boundary, including thread-local cleanup it creates. Destructor failure marks
cleanup failed and permanently refuses restart. Two disposal attempts bound
recovery; recursively panicking destructors beyond that resume ordinary Rust
unwinding. Trusted in-process handlers and destructors can still block or abort;
this primitive does not sandbox arbitrary Rust code or foreign exceptions.

## Alternatives and evidence

A synchronous join on an input/close caller violates responsiveness. Spawning a
new join helper per retirement permits unbounded accumulation. A detached worker
with immediate replacement loses ownership and admits stale work. Reusing the
standard channel and native join preserves a small explicit lifetime boundary.
See Rust's [join contract](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join),
[nonblocking send](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send),
[unwind boundary](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) and
[resumed unwind](https://doc.rust-lang.org/std/panic/fn.resume_unwind.html).

Regression tests gate full queues, blocked handlers, callback re-entry/unwind,
registration beyond native join, stale replacement, service reuse, owner limits,
last-owner destruction, panic payloads and actual native thread-local cleanup.
The application review test holds a real handler behind a channel while dropping
its owner. Architecture mutations require actual join before acknowledgement and
reject the former finished-function shortcut. Native Windows and Linux runs do
not certify macOS, BSD, WASM or desktop responsiveness. No speedup is claimed.

Rollback reverts the runtime and application adaptations together. No data or
configuration migration is required. Preserve regression coverage and explicit
incomplete-cleanup reporting through any replacement.
