# ADR 0074: channel registration publication

Status: accepted for the registration race; broader channel accounting and native
platform evidence remain separately tracked.

## Problem and owner

Corcovado owns Evented channel readiness used by PTY input and Windows child-exit
notification. Registration previously inspected pending messages before exposing
its readiness handle. A sender could enqueue after that snapshot, find no handle,
and leave the receiver asleep despite an available message. Last-sender destruction
uses the same increment path and could lose the disconnect wakeup.

## Decision and alternatives

Keep the existing standard-library queue, native registration and AtomicLazyCell.
Publish both registration owners before inspecting pending. Place a sequentially
consistent fence between handle publication and the acquire snapshot. Pair it with
a sequentially consistent fence between the zero-to-one pending increment and the
acquire handle lookup. Sending into an already nonempty queue adds no fence.

This is an indispensable core polling mechanism, not an optional extension or a
new service. Replacing the queue or adding a mutex would change unrelated admission,
backpressure or lifecycle contracts. Reordering acquire/release operations alone
is insufficient across the independent handle and pending atomics. No dependency,
public API, native handle owner, worker, persistence, configuration or unsafe block
is added. Existing AtomicLazyCell release/acquire publication protects its value.

The ordering argument is a store/load handshake. If registration reads old pending
and the sender reads NONE or LOCK instead of the published SOME, each read precedes
the other thread's write in coherence order. The sequentially consistent fence
rules would require each fence to precede the other in their single total order.
That contradiction excludes both missed observations. This argument accompanies
the tests; model success alone is not a memory-ordering proof.

See [Rust fences](https://doc.rust-lang.org/std/sync/atomic/fn.fence.html),
[the atomic ordering rules](https://eel.is/c++draft/atomics.order), and
[Loom's limitations](https://docs.rs/loom/latest/loom/#limitations-and-caveats).
The existing dependency and native adapter remain authoritative.

## Evidence and limits

Per-receiver test checkpoints have finite waits and compile only in tests. Native
regressions force send, bounded send, try-send and final disconnect between the old
snapshot and handle publication, then require the exact poll token without later
traffic. Senders stay alive during message assertions so disconnect cannot mask a
missed message wakeup. Additional controls cover subsequent registration behavior.

An independent Loom model represents AtomicLazyCell's NONE/LOCK/SOME transitions.
Permanent mutation canaries reject the old snapshot order, either missing fence,
and acquire/release publication without fences. Both channel models are named in
CI and full QA. Native library and custom-evented tests, both model targets and
strict Clippy passed on Windows and Linux for the reviewed source. macOS/BSD,
native ARM and controlled channel-transition performance remain external gates.
The raw Registration polling benchmark is not a channel-send benchmark.

This decision does not fix unbounded queue admission, accepted-enqueue versus
readiness-error ambiguity, ignored readiness errors or decrement-before-increment
accounting. It makes no claim that the whole channel is correct. It changes no
terminal bytes, input policy or graphical surface. Rollback must retain a proven
publication handshake; restoring the earlier ordering restores the lost wakeup.
