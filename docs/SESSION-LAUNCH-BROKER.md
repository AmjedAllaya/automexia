# Session launch safety

This public document covers the launch boundary used by ordinary local terminal
sessions and manually invoked system tools. It does not describe unreleased
managed connection products or services.

## Ownership

The application resolves a typed executable and exact argument array, creates
one session owner, and hands process/PTY lifetime to that owner. Optional code
cannot receive a PTY or process handle or become a second lifecycle owner.

## Authorization contract

Before launch, validate the executable form, working directory, bounded
environment changes, route, session identity, and current configuration
generation. Structured launches never use command-string concatenation,
`sh -c`, `cmd /c`, PowerShell expression evaluation, or implicit Enter.

## Lifecycle

The session owner publishes state before waking the renderer, coalesces resize,
keeps input ordered, rejects stale work, reports exit, terminates owned
descendants on close, and joins workers during shutdown.

A failure returns a redacted actionable error and leaves existing sessions
unchanged. Optional integration failure must not block the basic local shell.

## Remote interoperability

When a user manually runs system OpenSSH, Automexia owns only the local terminal
session. OpenSSH and the operating system own configuration, credentials,
host-key policy, agents, authentication, and network behavior.

Unreleased launch brokers, provider integrations, and commercial workflows are
private and are not public commitments.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Upstream M5 Reviewed Tunnel Request

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## D0 and ADR 0012

D0 preserves one application-owned session/PTY and system OpenSSH authority for
networking, authentication, credentials, host trust, and proxy behavior. Review
or inventory paths cannot launch a session, send Enter, or weaken the manual
system-SSH fallback.
