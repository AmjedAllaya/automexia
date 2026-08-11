# Required repository settings

Configure a `main` ruleset that applies to administrators and requires pull
requests, all CI/CodeQL jobs, resolved conversations, one approval, CODEOWNERS,
stale-approval dismissal, linear history, and signed commits/DCO status. Disable
force pushes and deletion; allow squash merges only and delete merged branches.

The `Policy and repository contracts` check requires two distinct approvals when
protected release, signing, security, capability, provenance, governance,
packaging, workflow, CODEOWNERS, or audited-base paths change.

Enable private vulnerability reporting, Dependabot security updates, grouped
weekly Cargo/Actions updates, secret scanning, push protection, dependency graph,
CodeQL default setup only if it does not duplicate this repository's pinned
workflow, and artifact attestations. Fork pull-request workflows must never
receive secrets.
