# ADR 0012: System OpenSSH and session ownership

Status: Accepted for the public free-terminal boundary

## Context

Users need ordinary command-line SSH without giving the terminal credential,
host-key, authentication, or network authority.

## Decision

Automexia hosts the system OpenSSH client as an ordinary terminal process.
OpenSSH and the operating system own configuration, credentials, agents,
host-key policy, authentication, proxy behavior, and networking.

Automexia owns one local session, PTY, focused route, ordered input, resize,
output, exit status, and descendant cleanup. Paste never adds Enter.

Any public local inventory reads only explicitly selected bounded files,
publishes public metadata, performs no passive connection or authentication
work, and preserves last-known-good state on invalid or replaced input.

## Consequences

Manual system OpenSSH remains available when optional inventory or shell
integration is disabled. Tests use an authorized loopback fixture and never
persist or publish real connection data.

Unreleased managed connection behavior and commercial services are private and
outside this ADR.
