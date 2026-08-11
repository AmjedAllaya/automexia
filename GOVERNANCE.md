# Governance

Automexia Terminal uses a maintainer-led, review-first model. Maintainers are
responsible for roadmap decisions, releases, security response, upstream ports,
and enforcing the architecture and capability boundaries.

Ordinary changes require one approving review plus applicable CODEOWNERS.
Changes to release workflows, signing, security policy, capability declarations,
provenance, branch policy, or the audited engine base require two approvals.
Maintainers with a conflict of interest must recuse themselves.

Decisions that alter dependency boundaries, persistence, threading,
security/capabilities, or public behavior require an ADR. If consensus cannot be
reached, the repository owner records the decision and rationale in that ADR.

The `main` ruleset applies to administrators: pull requests only, mandatory
checks, resolved conversations, stale-approval dismissal, linear squash history,
no force pushes, and no branch deletion. Source branches are deleted after merge.
