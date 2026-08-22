# ADR 0003: Extension capability and threading boundary

Status: accepted.

Built-in extensions declare least-privilege local-read capabilities. Discovery
uses a bounded worker queue and session-scoped cached snapshots. Rendering and
PTY processing never perform extension I/O or wait for worker completion. New
process or network capabilities require a security review, two protected-path
approvals, and a replacement ADR.

[ADR 0012](0012-first-party-ssh-and-session-launch-boundary.md) is the accepted
replacement only for a narrowly scoped, application-owned first-party
`session.launch` capability. This ADR still requires two independent protected-
path approvals bound to the exact head plus non-bypassable server enforcement,
attestation, and native evidence before activation. Until those gates pass, the
local-read-only runtime boundary remains authoritative.
