# ADR 0003: Extension capability and threading boundary

Status: accepted.

Built-in extensions declare least-privilege local-read capabilities. Discovery
uses a bounded worker queue and session-scoped cached snapshots. Rendering and
PTY processing never perform extension I/O or wait for worker completion. New
process or network capabilities require a security review, two protected-path
approvals, and a replacement ADR.

[ADR 0012](0012-first-party-ssh-and-session-launch-boundary.md) is the proposed
replacement only for a narrowly scoped, application-owned first-party
`session.launch` capability after v0.4 hostile-output and release gates pass.
Until ADR 0012 is accepted and its prerequisites are implemented, this ADR's
local-read-only runtime boundary remains authoritative.
