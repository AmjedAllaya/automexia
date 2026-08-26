
# Future Session-History Deltas Beyond Real ADR 0028

**Type:** RFD — not a project ADR
**Existing authority:** accepted project ADR 0028

Repository audit says ADR 0028 already owns bounded top-level tab parked-PTY undo/redo.

Do not duplicate or supersede it from this research pack.

Potential future work only:
- split history;
- pane history;
- native-window topology history;
- broader supervisor persistence.

Future expansion should preserve:
- stable identity/generation;
- bounded capacity/TTL/resource use;
- visible parked state;
- deterministic cleanup;
- native lifecycle evidence;
- stale/risk display;
- revalidation before new authority-bearing operations.

Read/reveal of an already-running PTY should not be blocked solely because the credential that originally created the connection later expired.
