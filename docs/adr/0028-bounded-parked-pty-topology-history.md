# ADR 0028: Bounded parked-PTY topology history

- Status: accepted
- Date: 2026-08-23
- Owners: Automexia maintainers
- Related: ADR 0006, ADR 0007, ADR 0026

## Context

Undoing a closed terminal topology cannot safely mean spawning a replacement
shell: that would lose process state, change identity, and risk running startup
logic twice. Keeping every closed PTY indefinitely would instead leak child
processes, handles, memory, scrollback, and renderer state. Windows, tabs,
pane-local tabs, and splits also have different ownership and restoration
boundaries.

## Decision

Undo parks the existing independent PTY topology in its owning
`ContextManager`; it never clones, shares, serializes, or relaunches a PTY. The
initial activated boundary covers closed top-level window tabs, including all
splits and pane-local tabs already contained inside that tab's `ContextGrid`.
The exact grid object, route IDs, PTYs, split tree, local-tab order, focus, and
original top-level index move together.

History is isolated per native window and is memory-only. It is bounded to
8 parked topologies, 5 minutes, and 250,000 retained scrollback lines across
parked entries. Oldest entries are destroyed on count, age, or scrollback
pressure. A managed child exit evicts its parked topology. Shutdown drops every
parked entry and joins/terminates through the existing `Context`/PTY owner.
Restore refuses when top-level capacity is full; the entry remains parked rather
than being duplicated. Redo reparks the exact restored route. Any later topology
mutation invalidates redo.

Closed individual splits, individual pane-local tabs, and whole native windows
are not activated by this first boundary. Their restoration needs distinct
Taffy-node, tab-position, or application/window ownership and native resource
evidence. The typed `undo`/`redo` actions therefore report `Adapted`, and user
references must state the top-level-tab scope until follow-up decisions extend
it.

## Alternatives considered

- Kill the PTY and recreate it on undo. Rejected because process/session state
  and identity cannot be restored safely.
- Persist parked sessions across application restarts. Rejected because PTYs
  and child processes are live resources, not serializable records.
- Keep an unbounded history. Rejected because terminal processes and scrollback
  are high-cost resources.
- Activate split/tab/window restoration under one generic transaction. Rejected
  for the initial delivery because each has a different owner and failure path.

## Verification

- Deterministic tests enforce count and TTL eviction; source invariants enforce
  aggregate scrollback bounds, exact route/index restoration, capacity refusal,
  child-exit eviction, redo invalidation, and drop-owned shutdown.
- Action support metadata and generated references expose the adapted scope.
- Native Windows process/handle/resource-cycle evidence is required before a
  release claim. Native Linux/BSD and macOS lifecycle evidence, and individual
  split/local-tab/native-window undo, remain explicit external or future gates.

## Consequences

Top-level tab closure can be undone without changing the live session, while
resource growth remains bounded and reversible. The limitation is visible and
fail-safe: unsupported topology kinds fall through instead of pretending that
a replacement shell is an undo. Extending the boundary requires native tests
and an ADR update for the additional owner.
