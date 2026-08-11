# ADR 0003: Extension capability and threading boundary

Status: accepted.

Built-in extensions declare least-privilege local-read capabilities. Discovery
uses a bounded worker queue and session-scoped cached snapshots. Rendering and
PTY processing never perform extension I/O or wait for worker completion. New
process or network capabilities require a security review, two protected-path
approvals, and a replacement ADR.
