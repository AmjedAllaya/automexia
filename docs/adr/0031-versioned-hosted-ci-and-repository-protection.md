# ADR 0031: Versioned hosted CI and repository protection

Status: Accepted

Date: 2026-08-24

## Context

Automexia already had extensive GitHub Actions workflows and a prose branch-
protection guide, but no machine-readable authority that tied workflow job
names, allowed Actions, reviews, rulesets, security settings, and hosted-run
evidence together. A local source check could therefore pass while the remote
repository remained weaker, unavailable, or blocked before checkout.

The repository is private on GitHub Free. Authenticated inspection on
2026-08-24 showed that branch rulesets and classic branch protection require a
plan upgrade or public visibility, only one human collaborator was available,
and hosted jobs were rejected with zero executed steps because of the account's
Actions billing or spending state. Those are external prerequisites, not
successful CI and not defects that source code may conceal.

## Decision

`.github/repository-protection.json` is the single versioned contract for
repository merge settings, GitHub Actions permissions, security controls,
hosted workflows, exact required checks, reviewer capacity, the `main` ruleset,
and the release-tag ruleset. `tools/ci/repository_protection.py` owns three
bounded operations:

- `check` validates the local contract, workflow names, full-SHA Action pins,
  the exact third-party allowlist, and a repository-wide `CODEOWNERS` fallback;
- `audit` reads authenticated GitHub state and reports each control as pass,
  fail, or external;
- `apply --confirm-repository AmjedAllaya/automexia-terminal` applies only the
  reversible controls that the current plan exposes, then performs the same
  audit.

The apply operation never changes visibility, billing, plans, collaborators,
credentials, secrets, release assets, or existing Git history. It requires an
exact repository confirmation and administrator access. Network calls use
typed `gh` argument arrays, a 45-second timeout, a 2 MiB response ceiling, and
redacted findings.

`main` has no bypass actor and requires pull requests, linear squash-only
history, signed commits, resolved conversations, current CODEOWNER review,
last-push approval, stale-review dismissal, and the exact CI and Actions-static-
analysis check set.
The repository policy check separately requires two distinct non-author,
non-bot approvals bound to the exact pull-request head for protected paths.
Release tags matching `v*` reject deletion and non-fast-forward updates.

Hosted evidence is valid only when every required workflow is active and every
declared default-branch evidence workflow has a latest `main` run that executed
successfully, belongs to the exact current `main` commit, and is at most seven
days old. A skipped-only run, stale run, different commit, missing run, or zero-
step billing rejection cannot pass. The stable-release workflow is required to
remain active but is not a routine default-branch evidence workflow. On the
GitHub-Free/private plan, pinned actionlint and offline zizmor own workflow
static analysis; CodeQL and private code-scanning uploads remain deliberately
absent.

The repository permits GitHub-owned Actions plus an exact list of required
third-party actions, with full commit-SHA enforcement. Default workflow tokens
are read-only and cannot approve pull requests. Dependency graph, Dependabot
alerts and security updates, and immutable future releases are required now.
Private vulnerability reporting becomes required when the repository is public;
secret scanning and push protection become required whenever the repository
plan exposes them.

## Alternatives

- Prose-only setup was rejected because it cannot detect remote drift or stale
  evidence.
- Classic branch protection was rejected as the target contract because one
  ruleset model can protect branches and release tags and can explicitly ban
  bypasses. It remains an unavailable fallback on the current private Free plan.
- Making the repository public automatically was rejected because visibility is
  a material product, disclosure, and security decision outside this change.
- Broadly allowing all verified Actions was rejected because publisher status
  is weaker than an exact, reviewed action allowlist plus full-SHA pinning.
- Treating payment rejection or plan limits as CI failure or success was
  rejected. They are separately reported external gates.

## 2026-08-26 stable-release binding amendment

Stable tag preflight now consumes the separate digest-pinned stable-release
source policy. Before packaging it requires an annotated local and remote tag at
the exact remote `main` head, the annotated published Rio fork tag at the audited
base, complete clean linear downstream history, and author-matching DCO trailers.
The release workflow consumes a minimally scoped repository audit credential
from repository secrets, and preflight accepts only an all-pass authenticated
audit. Exit 1 drift and exit 2 external prerequisites both block publication. This adds no
runtime authority and does not permit visibility, billing, plan, collaborator,
credential, security-entitlement, or history changes.

## Verification

Mutation tests cover duplicate JSON keys, identity drift, bypasses, missing
pull-request/signature rules, check-name drift, SHA pinning, workflow-token
authority, fail-closed security settings, CODEOWNERS syntax, hosted billing
classification, executed failures, empty job sets, stale/future runs, exact-head
binding, apply payload shape, and external-state classification. The repository
validator, PR policy tests, CI policy job, full QA command, and `cargo ready`
all invoke the contract or its tests.

Remote `audit` and `apply` are safe to repeat. Audit exit code 0 means all
controls pass, 1 means source or remote drift, and 2 means only explicit
external prerequisites remain.

## Consequences

Available repository settings can be restored deterministically and audited
without granting the automation broader authority. Required check renames,
workflow additions, Action publishers, review policy, and protection rules now
change in one reviewed contract with permanent regression coverage.

The repository cannot claim protected hosted delivery until an owner restores
Actions billing/spending, chooses private-plan upgrade or separately authorizes
public visibility, invites enough independent reviewers and expands CODEOWNERS,
and reruns the exact protected commit. Private vulnerability reporting and
secret scanning remain accurately external until their visibility/entitlement
conditions are met. Product capability activation and stable release continue
to fail closed while any of those gates remains.
