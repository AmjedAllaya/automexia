
# Persistent Session History

**Repository-inspected current state:** accepted project ADR 0028 implements bounded top-level tab parked-PTY undo/redo. Broader split/pane/window history remains future work.

The important architecture correction is ownership:

```text
session lifecycle
≠
Ghostty compatibility
```

## Reattach semantics

Revealing/re-attaching an already-running PTY should not require a fresh credential merely to display its existing state.

Instead:
1. reattach through stable session/generation identity;
2. show context/authority state as valid/stale/expired/unknown;
3. require revalidation before **new** authority-bearing operations, renewal, reconnection, or privileged actions.

## Bounds

Keep:
- count/capacity;
- TTL/cleanup;
- memory/scrollback limits;
- deterministic park/undo/redo ordering;
- visible parked-session state.

## Future topology

Split/pane/window history needs:
- its own accepted extension to the real lifecycle ADR;
- native lifecycle evidence;
- resource/accessibility testing;
- clear ownership semantics.

Do not describe broader topology restoration as implemented until those gates pass.
