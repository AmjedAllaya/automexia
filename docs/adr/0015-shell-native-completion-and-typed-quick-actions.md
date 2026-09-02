# ADR 0015: Shell-native completion and explicit insertion

- Status: Accepted for v0.5
- Scope: Public free-terminal boundary

## Context

Command editing, history, quoting, completion, and execution already have mature
owners in PowerShell, CMD, Bash, Zsh, and Fish. Reimplementing those semantics
from rendered terminal cells would be unreliable and unsafe.

## Decision

The native shell remains authoritative. Automexia may provide session-local
bounded integration, command navigation, static reviewed text for copy or
insertion, and ordinary search/palette UI.

Insertion and copy never add Enter. Structured actions do not evaluate a shell,
launch a process, read hidden history, or expand credentials. If integration is
missing, disabled, or unhealthy, the native shell continues normally.

Generated alias files require preview, collision review, explicit activation,
rollback, and removal.

## Review and activation gate

Listing, search, completion, preview, selection, copy, and insertion do not
execute a command or implicitly submit terminal input. Any persistent shell
integration or generated alias activation is a separate, explicit, reviewed,
reversible operation that owns only Automexia-managed files and preserves the
native shell as the final execution authority.

## Consequences

This design preserves native quoting, completion, accessibility, and user
configuration while keeping terminal input ownership simple. It limits
Automexia-specific behavior but provides a dependable fallback on every
supported shell.

Unreleased assistants, provider-specific actions, workflow products, and
commercial packages are outside this public ADR.

Alias projection and verification details are in [DEVOPS-ALIASES.md](../DEVOPS-ALIASES.md).
