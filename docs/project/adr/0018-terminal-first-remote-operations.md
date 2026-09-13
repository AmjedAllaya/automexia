# ADR 0018: Terminal-first remote interoperability

Status: Accepted for the public free-terminal boundary

## Context

A terminal should remain useful with ordinary system remote-access tools and
must not require a proprietary account, credential vault, or graphical
connection manager.

## Decision

Automexia supports remote work through normal terminal sessions and the system
OpenSSH client. OpenSSH and operating-system facilities retain ownership of
configuration, credentials, host-key policy, agents, authentication, and
network behavior.

Automexia owns only terminal input/output, resize, selection, search, tabs,
panes, exit reporting, and cleanup. Public inventory reads only explicitly
reviewed bounded local files and performs no passive connection or login work.

## Consequences

Users retain familiar, portable command-line workflows and can disable all
optional integration without losing remote terminal use. Advanced remote
products and services are outside this public ADR and remain private until an
explicit publication decision.
