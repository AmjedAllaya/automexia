
# Process, Session, and IPC Direction

**Current repository:** existing mature application/`ContextManager` PTY ownership.
**Candidate research:** separate session-host/extension-host processes.

Do not replace the current owner preemptively.

Apply useful principles now:
- one clear PTY/process owner;
- generation/stale-message rejection;
- bounded framing;
- cancellation;
- shutdown ownership;
- no secret-bearing diagnostics;
- authenticated local peer checks where separate processes exist.

If a separate `session-host` is later proposed, first produce a prototype measuring:
- crash isolation benefit;
- latency;
- memory/process overhead;
- detach/reattach value;
- packaging/startup cost;
- Windows/macOS/Linux behavior.

Persistent top-level tab history already exists under real ADR 0028; broader process-topology redesign is not required by that feature.
