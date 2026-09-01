# ADR 0033: Optional LLM Orchestration extension boundary

Status: proposed public summary; no LLM Orchestration implementation is active.

## Context

Some users want model-assisted workflow composition, while many need a fast,
predictable terminal with no model dependency, subscription, network request,
or data-sharing requirement.

## Proposed direction

- Keep LLMs out of the terminal core and out of first-party domain extensions.
- Offer orchestration only through a separately installed and enabled extension.
- Support user-selected local or remote providers behind one reviewed boundary.
- Share only user-selected context and keep credentials in approved custody.
- Convert model proposals into typed plans that pass through ordinary capability,
  policy, review, approval, and audit controls.
- Preserve useful non-LLM fallback workflows everywhere.

## Consequences

Automexia avoids coupling its identity and reliability to model availability,
while advanced users can still automate their own workflows. Integration may
feel less automatic than an LLM-first terminal, but authority and failure remain
clear and replaceable.

Detailed provider and workflow contracts remain local pending implementation
and publication review.
