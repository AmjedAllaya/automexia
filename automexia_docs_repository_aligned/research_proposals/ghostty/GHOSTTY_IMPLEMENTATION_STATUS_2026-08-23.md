
# Ghostty Implementation Status — 2026-08-23

**Source:** supplied repository audit of the committed implementation baseline
**Audited committed baseline:**
`20ff7928ea2d1eac5d26f13c62d0cda9b86bc078`

## Source state

```text
G0  partial evidence/provenance qualification
G1  substantially implemented
G2  substantially implemented
G3  substantially implemented
G4  substantially implemented
G5  partial — native visual/accessibility/resource/platform/long-run evidence remains
G6  top-level tab parked-PTY undo/redo accepted + implemented under real ADR 0028
    user-visible parked count/list/clear controls and split/pane/window history
    remain future
```

The auditor inspected commit/source/contracts/tests but did not rerun the entire contributor/native suite during that audit.

Therefore distinguish:

```text
source-inspected state
≠
fresh native/release certification
```

## Architectural reclassification

- **TC — terminal capability:** binding/action/lifecycle behavior Automexia owns;
- **GM — Ghostty migration:** explicit profile/config import compatibility;
- **PS — persistent-session lifecycle:** Automexia-owned parking/restore semantics.

Keep G0-G6 as the stable traceability labels; this classification is an overlay.

## Release truth

Do not publish one blanket "Ghostty complete" status.

Track:
- profile fixture provenance;
- action mapping;
- migration;
- platform-native evidence;
- accessibility;
- performance;
- resource duration;
- session-history scope.
