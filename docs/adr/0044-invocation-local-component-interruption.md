# ADR 0044: Invocation-local component interruption

Status: Accepted for the existing disabled source host; native validation pending.

## Decision

Keep cancellation in `automexia-ecosystem-runtime`, the existing optional
component host. Do not add a terminal worker, extension, executor or dependency.
The public execution entry remains denied, and the release permit remains
unconstructible by ordinary callers. No activation, imports, capabilities,
fuel ceilings or memory/table limits change.

Each invocation owns its absolute deadline and completion state. A cancellation
token may be shared or reused, but completion of one call cannot disarm another.
The watchdog and guest worker remain joined owners. Worker spawn failure wakes
and joins the watchdog; a completion drop guard also wakes it during unwind.

The pinned [Wasmtime Store contract](https://docs.rs/wasmtime/48.0.1/wasmtime/struct.Store.html)
sets an epoch deadline relative to the engine's current epoch. A one-shot tick
before store arming is therefore insufficient. Check persistent cancellation
and the absolute deadline before guest instantiation and at return. Install a
store-local epoch callback: a call's own interruption traps, whereas another
call's shared-engine tick renews the healthy store's epoch deadline.

## Failure and evidence

The all-feature readiness run exposed an infinite guest surviving its requested
deadline. That run was stopped and recorded as failed, not retried into success.
Deterministic tests reproduce interruption before store arming with finite
emergency fuel, isolate cancelled and healthy stores on one real engine, reuse
cancellation tokens across real calls, and join the watchdog after worker unwind.
An infinite core start function also checks interruption during instantiation,
before the exported function is reached. Actual guard-removal mutations proved
that the entry and healthy-store regression oracles reject their intended faults.
Default-feature workspace QA cannot replace this explicit all-feature coverage.

These checks do not authorize public execution or establish every platform's
native sandbox/resource assurance. OS scheduling, compilation before execution,
and non-preemptible host work are not hard real-time guarantees. Existing
protected native/release gates remain required. Reverting the fix requires
retaining the reproducer and recording the restored cancellation defect; there
is no persistence migration.
