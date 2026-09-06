# ADR 0040: Owner-authorized Linux Early Access releases

Status: Accepted (2026-09-06, explicit repository-owner instruction)

## Scope and decision

Linux Early Access publication uses a solo-maintainer authorization model.
The repository owner explicitly removed the second-person approval and distinct
merger requirement. Only `AmjedAllaya` may author, merge, trigger, or rerun a
publishing release PR. An approval count is not a substitute for this identity
check. This decision does not relax independent native evidence review or the
separate stable-release workflow, nor change public-archive branch/tag rules.

The owner is the existing contributor release infrastructure:
`tools/ci/public_distribution.py` and `.github/workflows/linux-early-access.yml`.
Terminal core and extensions are inappropriate because this is repository
publication authority, with no product process, renderer, persistence, or hot-
path behavior. Reuse Python's standard JSON/parser infrastructure; no dependency,
service, new extension, or paid GitHub feature is added.

## Acceptance and evidence ledger

| Contract | Initial evidence | Required verification |
| --- | --- | --- |
| Native package/quality pipeline | Fully implemented; merged candidate passed hosted CI and native x64/Arm64 unsigned rehearsal | Preserve all quality/package joins and credential-free rehearsal isolation |
| Solo authorization | Not implemented; Linux workflow required an impossible self-approval | Owner-only merged-event, actor, rerun-actor, source/target, exact commit and current-main checks |
| Artifact trust | Fully implemented source contract; real signing/publication external | Keep Minisign, scoped App, immutable release, exact assets, checksums and attestations |
| Documentation | Partially implemented; current instructions still required two people | Describe the accepted policy and residual risk without product plans or confidential data |

## Implementation and verification plan

1. Add failing tests using realistic merged-event structures. Cover author,
   merger, sender, original/rerun actors, forks, repository substitution, wrong
   base/branch/version, unmerged/closed state, missing/type-confused fields,
   stale head/main, malformed/oversized/duplicate/deep JSON, and redacted errors.
2. Validate the bounded runner event before emitting publication authority.
   Require an internal `release/linux/X.Y.Z` PR merged into `main`, the pinned
   source repository, valid head/merge hashes, exact checked-out merge identity,
   current main identity, and the pinned human owner for all actor roles.
3. Keep manual dispatch isolated. Use typed CLI arguments, no evaluation of
   event strings, and return only a validated version. A malformed event, API
   failure, or stale rerun stops before signing or GitHub App token creation.
4. Mutate workflow wiring to prove removal, duplication, failure suppression,
   actor substitution and publication-before-authorization fail closed. Keep
   stable/team approval tests and all existing distribution mutations.
5. Run focused Python and workflow security checks, reinforcement/mutation
   gates, full QA, contributor readiness, and privacy scans. No UI benchmark is
   applicable; event size, execution deadline and no-write tests bound this path.
6. Update release instructions, roadmap, assurance ledger and change fragment;
   DCO-sign and push one coherent authorization commit. Merge only its exact
   green PR head as the owner, then verify the real signed publication. Local
   mocks do not constitute release evidence.

## Trust, lifecycle and rollback

The GitHub event and actor contexts are inputs, not shell commands. Validation
has bounded file bytes and fails with stable redacted diagnostics. It owns no
long-lived tasks or stored credentials. Existing job deadlines, cleanup, source
read-only permissions, and post-quality signing/publication remain unchanged.

One compromised owner account can now authorize a release; independent review
would reduce that risk but is deliberately not required for this channel. Keep
write access limited, protect account credentials, and never share signing keys.
Archive protection still prevents replacement or deletion of published assets.
Reverting this ADR's code commit restores the prior two-person policy without
rewriting published tags; failed immutable verification requires a new version.

## Primary sources

- [GitHub merged pull-request events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-pull_request-workflow-when-a-pull-request-merges)
- [GitHub actor and triggering-actor contexts](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#github-context)
- [GitHub pull-request approvals](https://docs.github.com/en/pull-requests/how-tos/review-pull-requests/approving-a-pull-request-with-required-reviews)

GitHub distinguishes the original actor from the actor rerunning a workflow;
both must be checked. A PR author cannot provide their own required approval.
The accepted owner-only policy encodes that choice explicitly rather than
manufacturing an approval or removing package verification.
