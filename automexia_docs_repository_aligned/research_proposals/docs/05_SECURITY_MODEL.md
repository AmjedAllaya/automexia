
# Security Model Research Deltas

Integrate principles into the **existing project security model**, not a parallel policy authority.

Keep:
- least authority;
- exact executable identity;
- environment construction rather than ambient inheritance;
- opaque secret references;
- fail-closed validation;
- bounded parsers/IPC;
- redaction by construction;
- explicit generation/context;
- native security evidence.

Compatibility/profile code has no direct PTY/process/credential/network authority.

A future centralized Policy Engine/Context Guardian is a target concept only; extend existing brokers/owners incrementally until a shared service is justified.

Top-level parked PTY history is accepted/implemented under real ADR 0028. Showing/re-attaching an already-running PTY should not require a fresh credential merely to reveal it. Mark stale/risky authority visibly and revalidate before **new** authority-bearing operations, renewal, reconnection, or privileged action.
