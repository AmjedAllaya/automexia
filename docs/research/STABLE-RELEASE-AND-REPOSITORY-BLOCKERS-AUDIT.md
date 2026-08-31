# Stable-release and repository blocker completion audit

Status: repository-owned enforcement fully implemented; release evidence and
account prerequisites remain partial/external

Audit date: 2026-08-27

## Outcome, authority, and scope

Section 12 now has one fail-closed source of truth for release-source
provenance: `tests/assurance/stable-release-policy-v1.json`, enforced by
`tools/ci/stable_release.py`. A stable tag must be annotated, identify the exact
remote `main` head, exist on the remote, descend from the exact audited Rio fork
commit, preserve the annotated remote fork tag, use complete clean history, have
linear downstream history, and contain an author-matching DCO trailer on every
downstream commit. The protected tag preflight also runs the authenticated
repository audit and accepts only its all-pass exit status.

The accepted repository authority remains ADR 0031 and
`.github/repository-protection.json`. S1, S2, package trust, signing,
notarization, and controlled native evidence retain their existing independent
owners. This change does not acquire credentials, assert legal rights, choose
repository visibility or billing, invite reviewers, rewrite shared history, or
manufacture native or elapsed evidence.

## Evidence ledger

| Requirement | Classification | Implemented evidence | Remaining exit evidence |
|---|---|---|---|
| Exact release source and tag | **Fully done in source** | Versioned policy, exact schema and external owner/evidence bindings, immutable stable-tag grammar, annotated local/remote tag checks, exact expected commit and remote `main` equality, complete-history and clean-tracked-source checks | Exercise on the eventual protected stable tag |
| Fork provenance | **Fully done** | Local and remote annotated `rio-base-0.5.20-7d595af` must peel to `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`; the missing tag was published and remotely verified on 2026-08-26 | Preserve the immutable tag |
| Downstream history/DCO | **Fully enforced; historical evidence blocked** | One bounded 16 MiB/30-second batch traversal covers at most 100,000 commits, rejects merges, and requires an author-email-matching `Signed-off-by` trailer | Resolve the six post-fork merge commits and commit `0f3fec43ac` through an explicitly coordinated legal/history process; no automatic rewrite is authorized |
| Local repository-protection contract | **Fully done** | Two rulesets, fourteen exact checks, selected full-SHA Actions, least-authority workflow token, security settings, ownership fallback, mutation tests | Preserve contract and mutations |
| Hosted repository state | **Partially done / external** | Authenticated audit/apply path is bounded and idempotent; all available settings pass | Activate controlled workflows on protected `main`; restore Actions billing; obtain ruleset-capable plan or separately approve public visibility; add two reviewers; enable visibility/entitlement-dependent security controls |
| Stable tag repository audit | **Fully done in source** | GitHub-Free/private preflight requires a repository audit credential and `repository_protection.py audit --json`; any fail or external result blocks publication | Configure the repository secret, record the independent release review, and make the live audit all-pass |
| Brand assets and redistribution rights | **External prerequisite** | Manifest and package checks correctly fail closed | Supply editable SVG/variants and legal rights evidence; independent reviewer sets final status |
| Private conduct-reporting contact | **External prerequisite** | Release preflight rejects the marker | Governance owner supplies a private monitored address |
| Windows signing | **Fully done in workflow / external credentials** | Exact Azure Artifact Signing or PFX backend, timestamp, subject, executable/MSI/PowerShell verification, package smoke | Provision verified identity/profile or certificate secrets and retain signed native evidence |
| Apple signing and notarization | **Fully done in workflow / external credentials and host** | Developer ID signing, hardened runtime, notarytool acceptance, stapling, Gatekeeper and mounted-DMG validation | Provision Apple credentials and retain macOS controlled evidence |
| S1 assurance | **Fully done in source / external execution** | Strict 28-suite current-commit policy, five 8,352-frame visual matrices, mutation tests, exact controlled-runner activation/timeout/fail-closed semantics, one exact 90-day path-free summary, and non-bypassable release dependency | Complete and independently review Windows/Linux/macOS native, visual, resource, and assistive-technology manifests |
| S2 performance ratchet | **Fully done in source / collecting** | Exact 5% latency/10% memory ratchet, controlled builder, independent review, semantically checked serialized protected activation, exact 90-day summary identity, and non-bypassable controlled release job/dependency | Complete thirty consecutive comparable controlled-runner days and activate the baseline |
| Final packages, SBOM, checksums, provenance | **Fully done in source / external execution** | Signed platform packages, semantic CycloneDX/SPDX, complete checksums, attestations, immutable publication, clean install/upgrade/uninstall and hardware trust jobs | Run successfully on the exact protected release commit |

## Live repository evidence

The read-only authenticated audit on 2026-08-26 returned pass for repository
merge/DCO/cleanup settings, Actions permissions and exact allowlist, workflow-
token authority, Dependabot alerts/security updates, and immutable releases.
It returned a real failure for hosted workflows because the S1, S2, and F5
controlled workflow files exist on the implementation branch but not on remote
`main`. At audit time `main` was 109 commits behind the branch. It separately
reported the private-Free ruleset limit, one-human reviewer capacity, Actions
billing rejection, unavailable private vulnerability reporting, and unavailable
Secret Protection as external prerequisites.

These results are not collapsed into a pass. The controlled workflows must
reach `main` through a reviewed pull request; direct or force push to `main` is
not an acceptable workaround.

## Threat, failure, and resource model

The checker accepts only bounded UTF-8 JSON with duplicate-key rejection, a
pinned policy digest, a bounded tag/remote grammar, full lowercase commit IDs,
at most 100,000 downstream commits, 16 MiB per Git response, and 30-second typed
Git operations. It never evaluates a shell command or reads credentials. Errors
name the failed invariant without printing paths, repository content, tokens, or
commit messages. Missing, malformed, shallow, dirty, lightweight, stale,
different-head, unpushed, non-linear, or unsigned-off source fails closed.

The authenticated remote audit remains isolated in the protected GitHub
environment. Its token is not forwarded to builds or packages. Existing S1/S2
jobs retain their own runner, evidence, freshness, review, privacy, and cleanup
boundaries.

## Current-practice research and adopt decision

- GitHub documents that rulesets are available for public repositories on Free
  and for private repositories on Pro, Team, or Enterprise Cloud. The project
  therefore keeps the current plan condition external instead of claiming local
  emulation: <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets>.
- GitHub documents secret scanning and push protection as visibility/license-
  dependent security controls. The audit enables them when exposed and never
  treats absence as compliance: <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-security-and-analysis-settings-for-your-repository>.
- Microsoft requires an Artifact Signing account, verified identity,
  certificate profile, and signer role. The existing OIDC workflow wraps the
  maintained official integration instead of introducing private key custody:
  <https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations>.
- Apple requires Developer ID signing and notarization for direct distribution;
  current tooling uses `notarytool`, validates acceptance, and staples the
  ticket: <https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution>.

No new runtime dependency was adopted. The implementation wraps the existing
Git executable with exact argument arrays and reuses the existing authenticated
GitHub audit, release workflow, GitHub-Free manual-governance contract, and QA
registries.

## Verification and rollback

Mutation coverage proves duplicate-policy rejection, immutable blocker
identities, exact valid source, lightweight-tag rejection, remote-main drift,
missing remote fork provenance, non-linear history, missing DCO, dirty tracked
source, and removal of the authenticated repository audit. Repository validation
and full QA invoke both policy and mutations.

Rollback is a normal reviewed revert of the policy/checker/workflow change. A
rollback must not remove the older package, S1, S2, repository-protection, or
release-trust gates. The published fork tag is provenance and must not be moved
or deleted.
