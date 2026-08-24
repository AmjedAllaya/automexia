# Hosted CI and repository protection

The machine-readable source of truth is
[`repository-protection.json`](repository-protection.json). It fixes the exact
repository, merge settings, GitHub Actions permissions, security controls,
workflow/check names, review capacity, `main` ruleset, and release-tag ruleset.
[ADR 0031](../docs/adr/0031-versioned-hosted-ci-and-repository-protection.md)
owns the rationale and security boundary.

## Local contract

Run these before proposing a repository-policy change:

```text
python tools/ci/repository_protection.py check
python tools/ci/test_repository_protection.py
python tools/ci/test_pr_policy.py
```

The check rejects duplicate JSON keys, repository/check-name drift, bypass
actors, missing pull-request/signature/tag protections, broad or unpinned
Actions, workflow tokens that can write or approve, missing fail-closed security
requirements, and an absent or invalid repository-wide `CODEOWNERS` fallback.

The `Policy and repository contracts` CI check requires two distinct current
human approvals for protected release, signing, security, capability,
provenance, governance, packaging, workflow, CODEOWNERS, audited-base, terminal
context/process, extension contract/runtime, SSH extension, session-launch
fixture, and security-ADR paths. The author, bots, case-only duplicate logins,
dismissed reviews, and stale or superseded approvals do not count. Every
approval must identify the exact current pull-request head. Review input is
bounded to 4 MiB and 10,000 records.

## Remote audit and safe application

Authenticate an administrator with GitHub CLI, then run:

```text
python tools/ci/repository_protection.py audit
python tools/ci/repository_protection.py apply --confirm-repository AmjedAllaya/automexia-terminal
```

`audit` is read-only. `apply` changes only the exact reversible settings in the
contract and then audits again. It does not change repository visibility,
billing, plan, collaborators, credentials, secrets, releases, or Git history.
It enables public vulnerability reporting only when the repository is public,
and secret scanning/push protection only when GitHub exposes those controls.
Calls use typed arguments, a 45-second timeout, and a 2 MiB response limit.

Exit status 0 means every control passes. Status 1 means local or remote drift.
Status 2 means all available controls pass but one or more named external
prerequisites remain. JSON output is available with `audit --json` or
`apply ... --json` and contains no hostnames, credentials, workflow logs, or
repository content.

The apply command is idempotent. Run it after plan, visibility, collaborator,
or security-entitlement changes; do not hand-edit remote rules independently
of the versioned contract.

## Required protected behavior

The active target rules contain no bypass actors. `main` requires pull
requests, the exact CI/CodeQL check set, strict current-branch checks, resolved
conversations, current CODEOWNER review, last-push approval, stale-review
dismissal, signed commits, linear squash-only history, and no deletion or force
push. Release tags matching `v*` cannot be deleted or moved non-fast-forward.
Merged source branches are removed automatically, web commits require DCO
signoff, and workflow tokens are read-only and cannot approve pull requests.

A hosted success counts only when every required workflow is active and each
declared default-branch evidence workflow has a latest `main` run that executed
successfully on the exact current `main` commit no more than seven days ago. A
missing, stale, skipped-only, different-commit, or zero-step billing-rejected
run does not pass. The stable-release workflow must remain active but is not a
routine default-branch evidence workflow. Keep the pinned advanced CodeQL
workflow; do not also enable CodeQL default setup.

Fork pull-request workflows never receive secrets. Third-party Actions must be
both listed in the exact selected-Actions allowlist and pinned to a full commit
SHA. Immutable releases apply to new releases; existing releases are not
rewritten by this policy.

## Current external prerequisites

The authenticated 2026-08-24 audit applied every control exposed by the current
repository and left these gates open:

1. restore the GitHub Actions billing/spending state and rerun the exact
   protected commit;
2. either upgrade the private repository to a plan with rulesets or make a
   separate, explicit decision to publish it; this automation never chooses
   visibility;
3. invite at least two additional independent human reviewers, wait for access
   acceptance, and expand `CODEOWNERS` with eligible maintainers;
4. after public visibility, enable private vulnerability reporting; after
   entitlement or public visibility exposes them, enable secret scanning and
   push protection;
