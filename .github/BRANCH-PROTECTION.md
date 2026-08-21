# Required repository settings

Configure a `main` ruleset that applies to administrators and requires pull
requests, all CI/CodeQL jobs, resolved conversations, one approval, CODEOWNERS,
stale-approval dismissal, linear history, and signed commits/DCO status. Disable
force pushes and deletion; allow squash merges only and delete merged branches.

The `Policy and repository contracts` check requires two distinct, current
human approvals when protected release, signing, security, capability,
provenance, governance, packaging, workflow, CODEOWNERS, audited-base, terminal
context/process, extension contract/runtime, SSH extension, session-launch
fixture, or security ADR paths change. The pull-request author, bot accounts,
case-only duplicate logins, dismissed reviews, and stale superseded reviews do
not count.

This CI check is defense in depth, not a substitute for server-side protection.
The repository ruleset must require the policy check and prevent a pull request
from merging changes to the workflow or checker that weakens its own review
requirement. If the hosting plan cannot enforce that ruleset or fewer than two
independent reviewers are available, capability activation remains blocked.

Enable private vulnerability reporting, Dependabot security updates, grouped
weekly Cargo/Actions updates, secret scanning, push protection, dependency graph,
CodeQL default setup only if it does not duplicate this repository's pinned
workflow, and artifact attestations. Fork pull-request workflows must never
receive secrets.
