# CI and build performance plan

Status: source implemented; exact hosted timing evidence pending

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

| Area | Status before this work | Evidence | Required action |
|---|---|---|---|
| Free-runner CPU, memory, debug-info, and test-thread bounds | Fully implemented | CI and Linux Early Access jobs pin single-job compilation, stripped test/debug artifacts, single-thread Nextest, deadlines, and lint cleanup | Preserve unchanged |
| Cargo source downloads | Partially implemented | CI and release quality cache registry/Git sources, but use isolated keys; native package jobs do not restore them | Introduce one versioned, lockfile-bound source-cache contract across the Linux jobs |
| Reuse across the mandatory Clippy cleanup | Not implemented | `cargo clean` intentionally deletes the full lint target before Nextest | Add a content-addressed compiler cache only to non-shipping quality jobs |
| Native release package build trust | Fully implemented | Each architecture builds, packages, validates, installs, uninstalls, and uploads exact artifacts natively | Preserve cold builds and prohibit compiler-cache environment/action use in package jobs |
| Release critical-path scheduling | Partially implemented | Architectures run in parallel, but both wait for the complete quality job | Start quality and package jobs after authorization; make every consumer depend on both |
| nFPM provisioning | Not implemented for performance | Every package runner compiles `nfpm@v2.43.4` with Go | Adopt the signed-release project’s native archive with exact per-architecture SHA-256 pins |
| Local readiness compilation | Partially implemented | `cargo ready` runs all-target/all-feature `cargo check`, then Clippy over the same graph, then tests | Keep `cargo xtask check`; make `cargo ready` compile the graph once through Clippy before tests |
| Performance observability | Partially implemented | GitHub records step durations and the earlier rehearsal records wall time, but no compiler-cache statistics exist | Emit sccache statistics and compare exact cold/warm hosted runs |

The exact pre-change hosted rehearsal (`33581137496`) is the baseline: release
quality took 20m36s; native x64 and Arm64 package lifecycles took 16m05s and
15m37s after quality. The old dependency chain therefore placed roughly 36
minutes on the rehearsal critical path before aggregation.

## Implementation record

The source implementation now satisfies acceptance criteria 1–7:

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

Acceptance criterion 8 remains open until exact hosted cold/warm runs complete.
The pre-host local verification completed on 2026-09-02 with these results:

| Evidence | Result |
|---|---|
| Focused action-pin, free-plan, distribution, repository-protection, feature-reinforcement, and xtask mutation tests | Passed |
| `cargo fmt --all -- --check` and `git diff --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo nextest run --workspace --locked --profile ci` | 2,279 passed; 8 explicitly skipped |
| `cargo test --workspace --doc --locked` | 64 passed; 3 explicitly ignored platform examples |
| `python3 tools/ci/qa.py --full` | Every locally executable stage passed; controlled native, elevated, long-campaign, and private-manifest gates remained explicitly external |

The final contributor-gate receipt and hosted run URLs belong to the signed
commit/PR evidence. Source shape or a cache hit by itself is not performance
evidence, and no percentage improvement is claimed here.

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

The first run can prove correctness and cold behavior. A second exact-policy run
after a trusted default-branch cache exists is required to measure warm-cache
benefit. GitHub service eviction, quota, and rate limiting remain external. No
performance percentage is claimed until exact hosted job timings and sccache
hit/miss statistics exist.
