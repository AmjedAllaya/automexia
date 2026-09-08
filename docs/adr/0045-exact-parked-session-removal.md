# ADR 0045: Exact parked-session removal and restoration

Status: Accepted for current source; native restoration evidence remains partial.

## Decision and ownership

Keep parked lifetime and undo in the core application's existing ContextManager
and ContextGrid owners. An exit removes its exact local tab or pane; only the
final route removes the parked entry. Reuse current tab removal and Taffy tree
collapse rather than introducing another topology or process registry. Optional
extensions cannot own fundamental session cleanup or window geometry.

The old whole-grid removal dropped healthy sibling Contexts. Rust's
[destructor contract](https://doc.rust-lang.org/std/ops/trait.Drop.html)
also drops owned fields; keeping a route list after such removal would not keep
those sessions alive. Preserve their actual owners, not copies of their IDs.
Existing Context/PTY teardown still owns shutdown and worker cleanup.

Separate model removal from renderer cleanup. Parked image overlays have already
been cleared; removing one parked route neither submits input nor resizes the
remaining PTYs. On undo, reconcile the model with the owning window's current
size, scale and margins, then refresh font metrics and PTY sizes before making
it visible. Do not rely on a caller's conditional margin refresh.
Rebuild saved zoom styles at the new size and scale so a later unzoom cannot
reinstate pre-park geometry. Keep the same focused route and split proportions.

## Limits and failure behavior

Existing history count, retained-row and expiry budgets remain unchanged. No
new thread, dependency, capability, persistence or launch path is introduced.
Unknown and repeated exits are no-ops. An unexpected removal failure preserves
siblings and emits a redacted error. Invalid restore dimensions preserve the
undo entry and foreground selection. Existing protected activation gates stay
disabled. Reverting this change must not restore whole-grid exit destruction.

## Evidence and limitations

The desktop-binary regression failed on unintended sibling shutdown before the
fix. A second mixed-topology regression failed on stale restore dimensions.
Real message channels independently detect shutdown, disconnection, resize and
input; weak references detect terminal-owner removal. Test all six three-route
exit orders, repeated cycles, unchanged surviving selection, independent
managers, final/unknown/late exits, undo/redo, size/scale changes and configured
panel margins. These model-path checks do not prove native shell continuity,
presented pixels, accessibility, idle-expiry scheduling, OS descendant cleanup
or end-to-end latency; those remain separate validation requirements.
