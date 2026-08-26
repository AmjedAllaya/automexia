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

The target `main` and release-tag rulesets are versioned in
`.github/repository-protection.json` and explained by ADR 0031. They have no
administrator bypass: `main` is pull-request-only with exact mandatory checks,
resolved conversations, current CODEOWNER and last-push approval, stale-review
dismissal, signed commits, linear squash history, no force pushes, and no
deletion. Protected paths additionally require two independent human approvals
bound to the exact pull-request head. Source branches are deleted after merge.

The current private GitHub Free repository cannot enforce those rulesets and
has only one human collaborator. That remote state is an explicit release and
capability-activation blocker, not active protection. Maintainers must use the
authenticated audit in `.github/BRANCH-PROTECTION.md` after plan, visibility,
or reviewer changes and must not claim enforcement from the local contract alone.
