# GitHub Free private production release setup

This configuration is designed specifically for a **private repository on GitHub Free**. It does not depend on private rulesets, protected environment reviewers, GitHub Code Security dependency review, or private artifact attestations.

## Release authorization

A stable release is initiated only by merging an internal branch named exactly `release/X.Y.Z` into `main`. The release workflow validates that the PR was merged, the head repository is this repository (not a fork), the branch uses exact stable SemVer, the merged commit is still the tip of `main`, the Cargo package version matches, no tag/release exists, and the release PR did not modify the release trust pipeline itself.

The workflow creates `vX.Y.Z` only after all build/package/final-verification gates succeed. Do **not** manually create the version tag.

## Optional review enforcement

Set repository variable `AUTOMEXIA_RELEASE_MIN_APPROVALS` to `1` or `2` for a team. Leave it `0` for a solo repository. The workflow counts distinct APPROVED reviews before proceeding. This is a useful control, but GitHub Free does not make it administrator-proof.

## Secrets

Repository Actions secrets are supported on GitHub Free private repositories. Configure Apple/Windows signing secrets only if you use those signing paths. A Free private repository cannot isolate them behind paid environment-review gates, so only trusted maintainers should have write access.

## Release procedure

1. Update the Cargo version to `X.Y.Z`.
2. Create `release/X.Y.Z` in this repository.
3. Push it and open a PR targeting `main`.
4. Wait for CI/security checks.
5. Obtain the configured number of approvals.
6. Merge the PR.
7. `Stable release` automatically starts.
8. The workflow builds, packages, verifies final artifacts, generates SBOM/checksum material, creates `vX.Y.Z`, and publishes the GitHub Release.

## Free-plan trust boundary

GitHub Free private repositories do not provide unbypassable branch/tag rulesets or protected-environment reviewers. Therefore the strongest practical free control is strict account/access hygiene: do not give write access to people who are not trusted to affect releases. For higher-assurance separation of duties, move the signer/release authority to a separate system or paid governance tier.
