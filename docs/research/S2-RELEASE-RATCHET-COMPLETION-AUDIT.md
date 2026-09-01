# S2 release-ratchet completion audit

Status: **source and automation fully implemented; controlled evidence
collecting**. Audited 2026-08-27 against the current repository. The roadmap
implementation badge is **Fully done**; release evidence remains incomplete
because the first active baseline needs elapsed native evidence and independent
human review that cannot be created by a source change.

## Outcome and authority

S2 must prevent a stable release when a comparable candidate regresses by more
than 5% latency or 10% memory, while refusing to compare incompatible or weak
evidence. It must remain inactive until at least 30 consecutive controlled
Windows-runner days are complete and independently accepted.

The authoritative owners are:

- `tests/assurance/performance-ratchet-policy-v1.json` for immutable limits,
  metric classification, evidence quality, activation, and waiver policy;
- `tools/ci/performance_assurance.py` for bounded collection, composition,
  baseline construction, validation, and evaluation;
- `tools/ci/test_performance_assurance.py` for deterministic policy and hostile
  input regression coverage;
- `tests/fixtures/performance/s2-baseline-v1.json` for the intentionally
  inactive repository state;
- `.github/workflows/nightly.yml` for daily evidence, `release.yml` for stable
  tag enforcement, and `s2-assurance.yml` for independently reviewed
  activation validation;
- `docs/STABILIZATION-ROADMAP.md`, `docs/ROADMAP.md`, and this audit for status.

No product runtime, telemetry, network upload, or terminal hot path is added.
S2 is CI/release assurance only. Source implementation, tests, documentation,
and workflow registration are in scope. Fabricating elapsed days, changing
repository/environment settings, provisioning hardware, or approving the
baseline on another person's behalf are out of scope.

## Evidence ledger

| Requirement | Audited status | Evidence and resulting action |
|---|---|---|
| Exact 5% latency and 10% memory policy | Fully implemented | Digest-frozen policy and mutation tests; preserved. |
| Classified Criterion and native memory collection | Partially implemented, now source complete | Added repeated-sample validation, exact 95% confidence, interval-width limits, bounded no-follow traversal, operator identity, privacy-safe runner metadata, and clean exact-commit binding. |
| Comparable daily candidate composition | Partially implemented, now source complete | Exact timestamp/commit/runner/toolchain/profile/operator identity is enforced; stale, future, dirty-source, linked, oversized, duplicate, non-finite, and unclassified evidence fails closed. |
| 30-90 consecutive-day active baseline | Partially implemented, now source complete | Every day carries its measurement time, operator, and canonical evidence digest. Acceptance must follow the final measurement within seven days and use a reviewer independent of every collector. |
| Exact, temporary waiver | Partially implemented, now source complete | Waivers remain commit/metric/baseline/maximum bound and at most 30 days; strict HTTPS URLs and an approver independent of the candidate operator are now required. |
| Candidate release evaluation | Partially implemented, now source complete | Evaluation requires the exact expected commit, fresh evidence, an active non-future baseline, and emits bounded sample/interval information. |
| Activation review automation | Not implemented, now source complete | The registered `S2 controlled activation` workflow validates a clean exact-source active baseline on the GitHub-Free Ubuntu runner and retains one exact digest summary for 90 days. Repository validation and hostile mutations reject non-manual triggers, private-environment or secret use, concurrent cancellation, runner drift, missing exact-commit binding, removed policy/mutation checks, fail-open steps, unbounded time, redirected summaries, and weakened retention. |
| Stable-tag dependency | Partially enforced, now source complete | Semantic workflow validation requires native GUI, native WSL, S1, and S2 jobs as direct preflight dependencies and requires an explicit `success` result for every gate. The release S2 job must retain its exact controlled-runner activation, bounded timeout, native/QA workload, classified evidence, exact-commit binding, `--require-active`, fail-closed steps, and 90-day evidence identity. |
| First active baseline | External prerequisite | The checked-in fixture remains `collecting`; 30 real consecutive complete days, controlled runner variables, independent manual review, and an HTTPS review record do not yet exist. |

## Reuse and current practice

S2 wraps the repository's existing Criterion output and Windows native resource
report. It does not replace Criterion's estimator, invent a benchmark engine,
or add a dependency. Criterion documents repeated statistical samples,
confidence intervals, and the possibility of environmental noise; S2 therefore
requires the repository's exact 95% confidence setting, 50-10,000 finite
samples, and bounded confidence widths before accepting latency evidence:
<https://bheisler.github.io/criterion.rs/book/user_guide/command_line_output.html>.

GitHub Actions artifacts remain transport/retention evidence, not a trust
authority. Nightly and activation summaries use the existing pinned upload
action with 90-day retention. GitHub's artifact API and retention behavior are
documented at <https://docs.github.com/en/rest/actions/artifacts> and
<https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts>.
GitHub Free/private cannot use protected-environment reviewers. Independent
activation therefore requires a recorded review before manual dispatch, with
repository write access restricted to release-authorized maintainers. GitHub
documents the plan limits for environments at
<https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments>.

Build/wrap/adopt conclusion: **wrap** the maintained benchmark output and native
resource owner with a small standard-library-only validator. This keeps runner,
filesystem, process, and release authority in existing owners, avoids runtime
impact, and gives the project an auditable rollback path.

## Trust, limits, and lifecycle

All evidence is untrusted. Reads are bounded before decoding, reject duplicate
JSON keys, reject symlinks and special files, require one-link regular files,
and verify file identity/size did not change during the read. Criterion
discovery uses bounded `scandir` traversal instead of an unbounded recursive
glob: depth 16, 4,096 entries, 512 result files, and 1 MiB per sample document.
Outputs are same-directory atomic replacements and reject linked destinations
or linked parent directories.

Candidate evidence is at most 24 hours old with at most five minutes of future
clock skew. Every command that produces or enforces trusted evidence verifies a
clean tracked worktree and an exact 40-hex source commit through typed Git argv
with bounded timeouts. Runner fields are ASCII allowlisted public labels; path-
like, control, token, and secret-like values are rejected.
Only allowlisted schema fields reach reports.

An active baseline contains 30-90 strictly consecutive UTC dates from one exact
runner/toolchain/profile identity. Each day includes the source commit,
measurement timestamp, operator, canonical evidence SHA-256, and every required
metric. Acceptance must be later than the final sample, no more than seven days
later, HTTPS linked, and made by someone who did not collect any included day.

Evaluation rejects an inactive, stale, future-reviewed, incompatible, or
malformed baseline. A waiver never changes the baseline: it names one metric,
one candidate commit, one accepted-baseline digest, one maximum regression, one
independent approver, and an expiry no more than 30 days after approval.
Unknown metrics and wildcard scope fail closed.

Collection is finite and synchronous; no worker or background process is left
behind. Re-running publication atomically replaces the same owned output.
Removing the S2 workflow or collector loses no terminal/user data. Rolling back
an active baseline must be an explicit reviewed policy/release change; it must
not be simulated with an environment fallback or a permissive waiver.

## Operator flow

1. Configure the controlled Windows runner and public metadata variables named
   by `.github/workflows/nightly.yml`; never put secrets or paths in them.
2. Enable `AUTOMEXIA_WINDOWS_PERFORMANCE_RUNNER=1`. Keep the same hardware,
   power mode, toolchain, profile, and named runner for the campaign.
3. Let the nightly workflow collect the native memory and nine Criterion
   families. Investigate every missing or failed date; do not duplicate or
   synthesize a day.
4. After 30-90 complete consecutive UTC dates, build the candidate baseline
   from a clean checkout of the exact proposed source commit:

   ```text
   python tools/ci/performance_assurance.py build-baseline `
     --evidence <normalized-day-files> `
     --accepted-by <independent-reviewer> `
     --accepted-at-utc <UTC-timestamp> `
     --review-url <HTTPS-review-record> `
     --expected-source-commit <40-hex-protected-commit> `
     --output tests/fixtures/performance/s2-baseline-v1.json
   ```

5. Review the baseline diff and canonical daily digests through a pull request.
   The collector/operator must not approve it.
6. Record independent review of the exact commit, then dispatch `S2 controlled
   activation` manually. Its bounded summary must pass and remain attached for
   90 days. On GitHub Free/private this is a documented human control, not a
   server-enforced reviewer gate.
7. Merge only through normal protected-branch review. Stable tags then run
   `evaluate --require-active --expected-commit <tag-commit>` on fresh exact-
   commit candidate evidence.

If any day, identity, review, or validation is missing, leave the fixture in
`collecting`, repair the controlled campaign, and restart the consecutive window
where required. Do not lower the policy or pre-create an `active` document.

## Test and release exit criteria

Source implementation is complete only when all of these remain true:

- policy and schema mutation tests reject weakened thresholds, quality limits,
  identity fields, review independence, or required claims;
- hostile-input tests cover duplicate/oversized/linked files, bounded traversal,
  invalid sample counts/intervals, unsafe runner metadata, dirty/mismatched
  source, stale/future candidates, future baselines, and malformed waivers;
- nightly, release, activation, repository-protection, and architecture checks
  keep the exact operator/source/review/enforcement arguments;
- repository validation, full QA, formatting, warning-denied Clippy, Nextest,
  doctests, and `cargo ready` pass for the resulting commit.

Release evidence becomes complete only after the external campaign proves:

- 30-90 consecutive complete UTC dates on the declared controlled Windows
  runner with every required latency and memory claim;
- identical public runner/toolchain/profile identity and independently
  traceable operator/digest per date;
- protected-environment approval by a reviewer who collected none of the days;
- successful activation validation and protected merge for the exact source;
- a subsequent controlled stable-tag evaluation against that active baseline.

## Current external evidence

The 2026-08-24 remote audit found no configured repository variables and no S2
artifact to accept. The recent nightly runs examined failed before a controlled
runner executed (`runnerName` was absent), and controlled jobs were skipped.
Consequently the honest count is **zero accepted S2 baseline days**. This is not
a source defect and is not treated as passing. Runner provisioning, repository
variables, elapsed time, and independent review remain the exact external gates.
