
# Session Supervisor Future Expansion

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Broader supervisor research beyond currently accepted top-level history.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Decision
Separate frontend lifetime from managed terminal session lifetime. A session-host owns PTY/ConPTY, process, VT state, scrollback, context, and cleanup. The frontend attaches through authenticated/versioned local IPC.

Persistence is explicit and bounded rather than implicit.


## Session-history relationship

Detached-session survival does not automatically authorize "close surface but keep PTY alive" history semantics.

Top-level parked-tab undo/redo is already owned by real project ADR 0028. Any broader detached-session or split/pane/window restoration must extend the real lifecycle authority through a new repository-assigned decision and separate resource/security evidence.
