# Private planning boundary

Status: Not a public product decision

This compatibility path intentionally contains no public feature, architecture,
roadmap, or commercial details. Historical content is preserved only in ignored
private documentation.

Do not infer behavior from this filename or from source scaffolding. Public
behavior is limited to [the feature catalog](../FEATURES.md).

# ADR 0025: authenticated native editor suggestion bridge

Status: Accepted; source implementation is authorized. Preview and stable
activation remain disabled until required native release evidence passes.

## Context

Native shells own their editable command buffer, cursor, selection, quoting,
completion, and submission. A richer optional local suggestion source cannot
safely infer those values from terminal cells or transmit them through terminal
output. CP1 remains the documented complete fallback.

## Decision

Use one application-owned, session-scoped, authenticated OS-local endpoint with
strict framing, bounded payloads, explicit capability and peer checks, exact
route/generation binding, cancellation, memory-only buffer handling, and
metadata-allowlisted diagnostics.

On Windows, named-pipe creation rejects remote clients with
`PIPE_REJECT_REMOTE_CLIENTS` and verifies the client process and logon session.
On Linux, the filesystem Unix socket is user-private and verifies `SO_PEERCRED`.
On macOS, the user-private socket verifies `getpeereid`.

The native editor revalidates the buffer, cursor, selection, quote context,
replacement span, route, endpoint instance, and generations before a single
insert-without-Enter operation. The bridge never submits or executes commands.

## Alternatives

- Terminal-grid or terminal-output inference is rejected because it loses
  editor truth and creates spoofing and privacy risks.
- TCP/UDP is rejected because the capability is local and needs no network
  authority.
- A process per keystroke is rejected for latency, lifecycle, and resource risk.
- Replacing native editors is rejected; CP1 and native fallback remain the
  safe default.

## Acceptance and required verification

Acceptance requires protocol/property/fuzz/mutation coverage, native peer and
permission checks, replay and cross-route rejection, no payload persistence,
redaction canaries, deterministic ranking, stale-generation cancellation,
bounded latency and resources, visual/accessibility verification, exact package
identity, lifecycle cleanup, kill-switch, disable, uninstall, and rollback.
Cross-compilation and local unit tests alone do not authorize release activation.
