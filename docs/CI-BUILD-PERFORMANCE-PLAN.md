# CI and build performance plan

Status: implemented; exact hosted cold/warm evidence recorded

Last reviewed: 2026-09-02

## Outcome and acceptance criteria

This plan reduces contributor compilation time and GitHub Actions critical-path
time without removing a test, weakening a trust boundary, increasing paid
GitHub usage, or allowing cached objects into release packages.

The change is complete only when all of the following are true:

1. `cargo ready` keeps repository, formatting, all-target/all-feature Clippy,
   workspace test, documentation test, shell, dependency, and smoke coverage,
   but does not perform a separate workspace `cargo check` immediately before
   the warning-denied Clippy compilation of the same graph.
2. Ordinary and Linux-release quality jobs use one checksum-verified,
   immutable compiler-cache implementation with an explicit cache generation,
   while retaining `cargo clean` between Clippy and Nextest.
3. Compiler-cache output is never consumed by the native package jobs. Public
   DEB, RPM, and portable archives remain cold, exact-source builds.
4. Cargo registry and Git source caches use one versioned Linux identity and
   are available to quality and native-package jobs. No `target` directory,
   credentials, runtime tokens, or generated executable is cached by
   `actions/cache`.
5. Native x64 and Arm64 package builds start after authorization in parallel
   with quality. Rehearsal and publication assembly still require both quality
   and every package job to pass.
6. nFPM is downloaded as the exact upstream v2.43.4 native archive and checked
   against a repository-pinned SHA-256 digest instead of being compiled from Go
   source on every runner.
7. Policy and mutation tests reject removal, duplication, reordering, version
   drift, cache leakage into packages, unpinned tools, missing downstream
   quality dependencies, and reintroduction of the redundant local compile.
8. A cold local evidence ladder passes, followed by a hosted run on the exact
   pushed commit. Hosted timings and compiler-cache statistics are recorded;
   a cache miss is not misreported as a performance win.

## Evidence ledger

| Area | Current status | Implemented contract | Evidence boundary |
|---|---|---|---|
| Free-runner resource bounds | Fully implemented | Single-job compilation, bounded debug artifacts, test threads, deadlines and lint cleanup | Hosted evidence below is scoped to the recorded runs. |
| Cargo source downloads | Fully implemented | Versioned, lockfile-bound registry/Git source cache shared across the Linux jobs | Source downloads are not compiled release objects. |
| Reuse across Clippy cleanup | Fully implemented | Content-addressed compiler cache in non-shipping quality jobs; mandatory lint cleanup retained | Cache misses and hosted eviction remain possible. |
| Native package trust | Fully implemented | Cold native build, validation, install, uninstall and upload for each architecture | No compiler-cache objects enter package jobs. |
| Critical-path scheduling | Fully implemented | Quality and package jobs start after authorization; consumers join every required result | Failures block assembly and publication. |
| nFPM provisioning | Fully implemented | Exact native v2.43.4 archives with per-architecture SHA-256 pins | Package filenames remain bound to the source-owned revision. |
| Local readiness | Fully implemented | Clippy compiles the graph once before tests; the separate focused Cargo check remains available | Readiness retains all other gates. |
| Performance observability | Fully implemented | Recorded job timestamps and compiler-cache statistics | Only the linked cold/warm measurements are claimed. |

The exact pre-change hosted rehearsal (`33581137496`) is the baseline: release
quality took 20m36s; native x64 and Arm64 package lifecycles took 16m05s and
15m37s after quality. The old dependency chain therefore placed roughly 36
minutes on the rehearsal critical path before aggregation.

## Implementation record

The implementation and exact-commit evidence now satisfy acceptance criteria
1–8:

- local readiness runs policy/metadata/formatting, Clippy, and workspace tests
  without the redundant standalone Cargo check; focused `cargo xtask check`
  retains it;
- ordinary and release quality jobs use the exact reviewed sccache Action,
  v0.16.0, and `automexia-rust-1.98-v1` generation, then report statistics;
- package jobs reject compiler-cache inputs and restore only the shared,
  versioned, lockfile-bound Cargo source cache;
- quality, x64 packages, and Arm64 packages start after authorization, while
  rehearsal and release assembly join all three results;
- nFPM v2.43.4 uses architecture-specific upstream archives and pinned digests;
  and
- action-pin, free-plan, repository-protection, public-distribution, xtask, and
  feature-reinforcement mutation owners cover the new boundaries.

The pre-host local verification completed on 2026-09-02 with these results:

| Evidence | Result |
|---|---|
| Focused action-pin, free-plan, distribution, repository-protection, feature-reinforcement, and xtask mutation tests | Passed |
| `cargo fmt --all -- --check` and `git diff --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo nextest run --workspace --locked --profile ci` | 2,279 passed; 8 explicitly skipped |
| `cargo test --workspace --doc --locked` | 64 passed; 3 explicitly ignored platform examples |
| `python3 tools/ci/qa.py --full` | Every locally executable stage passed; controlled native, elevated, long-campaign, and private-manifest gates remained explicitly external |

The exact implementation commit was
`b38f0e884aa7dd1d7bd05adba6853218ffb98fb2`. Pull request CI run
[`33593759776`](https://github.com/AmjedAllaya/automexia-terminal/actions/runs/33593759776)
passed policy, dependency security, and the complete Rust quality/test job on
that commit. Credential-free Linux rehearsal run
[`33593793029`](https://github.com/AmjedAllaya/automexia-terminal/actions/runs/33593793029)
then produced both native package sets without signing or publication:

| Evidence | Pre-change run `33581137496` | Optimized cold-cache attempt 1 | Optimized warm-cache attempt 2 |
|---|---:|---:|---:|
| Whole rehearsal, authorization start through retained evidence | 37m11s | 25m56s | 15m01s |
| Quality job | 20m36s | 25m38s | 12m05s |
| x64 cold native package lifecycle | 16m05s | 15m25s | 14m40s |
| Arm64 cold native package lifecycle | 15m37s | 15m39s | 14m03s |
| Compiler-cache report | Not present | 185 hits, 1,127 misses, 0 errors | 1,308 hits, 4 misses, 0 errors |

The optimized cold attempt reduced this exact rehearsal's measured critical
path by 30.3%; the warm attempt reduced it by 59.6%. The quality job itself was
52.9% faster warm than cold. These are bounded observations from the linked
GitHub-hosted runs, not universal promises: runner load, source-cache state,
cache eviction, quota, and dependency changes can alter later timings. Release
packages remained cold in both optimized attempts; their modest timing change
reflects source-download reuse and hosted-runner variance, not compiler-output
reuse.

The same commit's ordinary PR CI run passed twice as an independent workflow.
Its Rust quality/test job fell from 38m20s with 474 hits, 1,681 misses, and no
cache errors to 13m44s with 2,153 hits, 2 misses, and no cache errors: 64.2%
for those exact attempts. Repository policy and dependency security remained
separate read-only jobs and passed on both attempts.

### Development-cache lifecycle extension

The 2026-09-05 source extension separates Cargo final artifacts from
intermediates, shares immutable assurance tools by content identity, separates
mutable package-manager state, and gives verification, staging, temporary, and
QA benchmark data explicit cleanup owners. Cache collection is bounded and
dry-run by default; current, dirty, linked, leased, live, required, and recent
entries fail closed. This reduces duplicate per-worktree storage without
sharing mutable Cargo targets.

GitHub's free hosted `CI` workflow remains the automatic push and pull-request
authority. The local pre-push hook is now a non-blocking placeholder with the
manual assurance command commented for possible future reactivation. This
changes contributor latency, not hosted coverage. Exact implementation and
recovery commands are in [Development cache and build storage](DEVELOPMENT-CACHE.md).

## Placement decision

This is contributor and release infrastructure, so ownership stays in the
existing CI/assurance layer: `.github/workflows`, `.github/scripts`,
`tools/ci`, and the readiness orchestration in `tools/xtask`.

- Core terminal is rejected because no PTY, renderer, input, process, or user
  runtime behavior changes.
- Existing product extensions are rejected because build orchestration is not
  an optional product capability and must never ship in their dependency graph.
- A new extension is rejected because it would add packaging and lifecycle
  authority without a user-facing feature.

The compiler cache is an adopted build tool, not a Rust workspace dependency.
The selected `sccache` release and setup Action are Apache-2.0, maintained by
Mozilla, checksum-verify downloaded binaries, support the pinned Rust toolchain
and hosted Linux architectures, and remain outside application startup and
runtime authority. Direct `target` caching was rejected because Cargo documents
that internal build-directory layout is unstable, the repository already has a
real 14-GB runner-exhaustion regression, and GitHub warns that restored cache
contents are unsigned executable input.

## Trust, resource, and failure model

- GitHub cache scope prevents pull-request caches from overwriting the default
  branch scope. The compiler cache uses a dedicated Automexia/Rust generation
  so it can be invalidated without broad cache deletion.
- Compiler-cache data is accepted only by quality jobs. Native release binaries
  are rebuilt without `RUSTC_WRAPPER` or any `SCCACHE_*` environment.
- Cache misses, eviction, the free 10-GB repository ceiling, and service rate
  limits may reduce speed but must not change compiler or test results.
- No secrets are stored. The setup Action exposes only the ephemeral Actions
  cache runtime token already scoped to that job; workflow logs and docs must
  not print it.
- `cargo clean` remains between Clippy and Nextest to cap local runner disk.
- Quality and package jobs may run concurrently, but signing, rehearsal
  retention, activation, and publication cannot begin until quality and both
  native package variants pass.
- Tool downloads are full-SHA Action pinned, explicit-version pinned, and
  checksum verified. A checksum mismatch fails before compilation or packaging.
- Cancellation remains owned by GitHub concurrency and job deadlines. No new
  background service survives the ephemeral runner.

## Scenario and test inventory

The implementation must cover:

- compiler cache absent, cold, warm, evicted, rate-limited, and version-bumped;
- removal, duplication, mutable Action ref, wrong sccache release, missing
  wrapper, missing statistics, and accidental package-job cache use;
- source-cache pin, paths, schema generation, lockfile identity, shared use,
  target-path injection, and stale/static identity mutations;
- package/quality job order in both directions; quality failure; one
  architecture failure; manual unsigned rehearsal; authorized publication;
- x64/Arm64 nFPM archive mapping, wrong digest, wrong archive, missing checksum,
  download failure, and extraction failure;
- `cargo xtask check` retaining the full Cargo check while `cargo ready` uses
  all-target/all-feature Clippy as its single pre-test compilation owner;
- no test removal, no skipped doctests, no PTY/product behavior change, and no
  release artifact identity change.

## Evidence ladder and rollback

Run focused Python mutations and xtask unit tests first, followed by action-pin,
repository-protection, public-distribution, feature-reinforcement, repository
validation, formatting, Clippy, Nextest, doctests, full QA, and `cargo ready`.
Then push one DCO-signed feature commit and run ordinary CI plus a credential-free
Linux rehearsal on that exact commit.

Rollback is one coherent commit: remove the compiler-cache setup/environment,
restore the old source-cache keys and package dependency, restore `go install`,
and restore the standalone readiness check. Release package bytes and public
state require no migration because no cache is permitted in their build path.

## External evidence

The linked cold and warm attempts complete the exact-commit hosted evidence for
this change. GitHub service eviction, quota, rate limiting, runner contention,
and future dependency graphs remain external. Repeated longitudinal runs are
required before treating the bounded percentages above as a stable forecast.
