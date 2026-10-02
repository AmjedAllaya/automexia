# ADR 0003: Extension capability and threading boundary

Status: accepted.

Built-in extensions declare least-privilege local-read capabilities. Discovery
uses a bounded worker queue and session-scoped cached snapshots. Rendering and
PTY processing never perform extension I/O or wait for worker completion. New
process, network, or extension-initiated session-launch capabilities require a
security review, two protected-path approvals, and an exact replacement ADR.

[ADR 0012](0012-first-party-ssh-and-session-launch-boundary.md) accepts manual
system OpenSSH as an ordinary terminal process and defines local session
ownership. It does not authorize extension `session.launch` or unreleased
managed connection behavior. Any application-owned adapter must first satisfy
the replacement ADR and exact-head approval requirements, non-bypassable grant
enforcement, attestation, and native evidence. Until then, the local-read-only
runtime boundary remains authoritative.
