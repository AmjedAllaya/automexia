# GitHub Free private governance

GitHub Free private repositories do not provide the repository rulesets/environment-review model used by the Enterprise edition. This tree therefore makes no false claim of server-enforced branch or tag governance.

The stable release ceremony is `release/X.Y.Z` -> pull request -> `main` -> merge -> automatic release pipeline. Set `AUTOMEXIA_RELEASE_MIN_APPROVALS` to require workflow-verified approvals when the repository has multiple maintainers.

For strongest free operation, restrict write access to trusted maintainers, require PRs by team convention, enable 2FA/passkeys, enable Dependabot alerts/security updates, and keep signing keys outside ordinary build jobs whenever practical.
