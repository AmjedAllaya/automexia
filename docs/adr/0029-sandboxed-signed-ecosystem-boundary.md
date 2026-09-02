# Private planning boundary

Status: Not a public product decision

This compatibility path intentionally contains no public feature, architecture,
roadmap, or commercial details. Historical content is preserved only in ignored
private documentation.

Do not infer behavior from this filename or from source scaffolding. Public
behavior is limited to [the feature catalog](../FEATURES.md).

# ADR 0029: sandboxed and signed ecosystem boundary

Status: Accepted for source implementation; accepted for source implementation
does not imply distribution. Public activation remains denied.

## Context

Optional extensions need a stable public safety boundary without gaining
terminal, PTY, renderer, credential, provider, network, filesystem, process, or
configuration authority by installation alone.

## Decision

Keep extension code outside terminal hot paths and require a versioned manifest,
explicit capabilities, application-owned brokering, bounded messages and
resources, signed provenance, lifecycle isolation, and deterministic disable,
uninstall, rollback, and cleanup. Unknown capabilities and incompatible
versions fail closed. The terminal remains usable when the ecosystem is absent
or disabled.

The reviewed source-contract digest is
`fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4`.
It identifies the checked contract; it is not a release signature or activation
token.

## Consequences

Source packages and tests may exist while activation remains denied. Any future
distribution requires current native sandbox, signature, revocation, resource,
privacy, accessibility, packaging, recovery, and uninstall evidence for the
exact artifact. Commercial marketplace plans remain private.
