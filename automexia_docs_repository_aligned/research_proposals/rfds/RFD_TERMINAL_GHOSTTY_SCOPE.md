
# Terminal Ghostty Scope

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Integrate terminology into existing roadmap rather than new ADR.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


**Research note: Modern Terminal Compatibility and Ghostty Migration Scope.**


## Context

A previous implementation effort grouped terminal correctness, Ghostty migration behavior, and session lifecycle under a single "Ghostty Compatibility G0–G6" program. This creates product-identity, maintenance, and authority ambiguity.

## Decision

Automexia separates:

```text
Modern Terminal Compatibility
Ghostty Migration Compatibility
Persistent Session Supervisor
```

Ghostty is a version-pinned reference/migration source, not the specification for Automexia.

Automexia remains the default profile.

The Ghostty adapter depends on stable Automexia typed APIs. Core/session/renderer/security crates must not depend on Ghostty-specific types or runtime presence.

Ghostty source/Zig may be used by maintainer fixture tooling but are not normal Automexia runtime dependencies.

## Consequences

Useful binding/action/profile infrastructure remains reusable.

The project does not promise full Ghostty product parity.

New Ghostty upstream features are adopted only when they improve migration materially or correspond to independently desired Automexia capabilities.

Implementation of a compatibility mapping **does not authorize a new Automexia capability, lifecycle semantic, security authority, or persistence policy**.
