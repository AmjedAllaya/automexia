# ADR 0034: Situation-aware production operations boundary

Status: proposed public summary; not an accepted implementation decision.

## Context

DevOps and SRE users benefit from recommendations grounded in their current
environment, but provider access and production mutations carry authority that
does not belong in the terminal core or completion hot path.

## Proposed decision

- Place situation-aware behavior in the independently enabled DevOps/SRE
  extension.
- Keep the core provider-neutral and reuse its context, completion, diagnostic,
  review, and session primitives.
- Collect bounded evidence only through application-owned capability brokers.
- Rank and explain suggestions with deterministic, inspectable policy.
- Keep recommendation, review, approval, execution, observation, and
  verification as distinct steps.
- Never make LLM output an authority for production action.

## Consequences

Users receive useful incident guidance without turning Automexia into an
autonomous operations platform. Advanced provider features can be installed,
disabled, and replaced independently. Initial support will be deliberately
narrower than a monolithic observability suite.

Detailed internal contracts and execution recipes remain local until this
decision is accepted alongside implementation and tests.
