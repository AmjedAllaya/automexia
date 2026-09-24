# ADR 0068: safe Windows pipe buffer ownership

Status: accepted for current source; broader native release evidence remains external.

## Problem and placement

The Windows pipe ring was transferred between worker threads with a
non-atomic reference count and borrowed the entire storage mutably from both
endpoints. Pipe predicate locks did not make that private abstraction sound.
A zero-capacity ring also divided by zero during otherwise empty transfers.

The existing `teletypewriter::windows::spsc` module remains the core PTY
transport owner. Optional extensions cannot own this fundamental stream. No
new crate, dependency, capability, worker, persisted format or public API is
introduced.

## Decision

Share the fixed storage with `std::sync::Arc` and the existing pinned
`parking_lot::Mutex`. Each transfer borrows bytes only through a mutex guard;
compiler-derived Send and Sync replace manual unsafe implementations. Atomic
length queries remain advisory snapshots. Each endpoint keeps its own cursor,
and transfers preserve the existing contiguous partial-read/write contract.
Zero capacity accepts and returns zero bytes. Capacity never grows.

The outer pipe predicate mutex precedes the storage mutex. Storage operations
cannot acquire pipe locks, invoke callbacks, perform I/O, or wait for data.
They copy at most the fixed ring capacity and allocate nothing after creation.
Notifications and native pipe calls retain their existing owners outside the
storage guard. The existing native callers construct 65,536-byte rings.

A custom unsafe atomic ring was rejected because it adds aliasing, publication
and destruction proofs to a path already serialized by pipe predicate locks.
A new queue dependency adds no needed capability. An unbounded channel would
lose the transport's resource ceiling. Safe synchronization has a per-operation
cost, so the existing native PTY benchmark remains the performance oracle.

Official references: [Rust Arc thread safety](https://doc.rust-lang.org/std/sync/struct.Arc.html#thread-safety)
and the pinned [parking_lot mutex contract](https://docs.rs/parking_lot/0.12.5/parking_lot/type.Mutex.html).

## Evidence and compatibility

The module's regressions cover zero capacity, exact saturation, partial
transfers, wraparound, untouched destination suffixes, an independent FIFO
model, and bounded cross-thread acknowledgments followed by joined drops.
Native pipe and ConPTY lifecycle tests retain responsibility for byte delivery,
EOF, wakeups, process and resource cleanup. The existing `pty_io` benchmark
compares startup and sustained output with a same-host baseline. Neither unit
tests nor this benchmark certify desktop responsiveness or every native host.

No data migration or user action is needed. Rollback is source-only and must
retain safe thread ownership and the zero-capacity regression. The private
buffer remains an implementation detail; pipe readiness and shutdown contracts
are unchanged by this decision.
