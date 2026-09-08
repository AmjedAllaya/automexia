# CI and release assurance

This document is the implementation audit and operating guide for Automexia's
continuous-integration and release-assurance code. It describes source truth;
it does not turn unavailable hosted, signing, hardware, or human evidence into
a pass.

## Current status

Linux Early Access and stable releases have separate selectors. A merged
`release/linux/X.Y.Z` PR skips stable authorization before its review/signing
jobs, while `release/X.Y.Z` retains the complete stable-release contract. The
free-plan checker and its mutation suite reject removal or inversion of that
exclusion. This prevents an unrelated stable-lane failure on a Linux release;
it does not waive any test or signing requirement for either channel.

| Area | Source/local status | Remaining external evidence |
|---|---|---|
| Ordinary PR policy, Rust quality, dependency security | Implemented, mutation-tested, and passed in exact-commit run `33593759776` | Repeat on every changed commit; native and controlled evidence remains separate |
| Internal release validation | Implemented and mutation-tested | A real internal `release/X.Y.Z` PR must run it |
| Windows/MSVC coverage ratchet | Implemented with exact commit, platform, path, size, and changed-owned-line binding | The standard Windows runner must execute successfully within available Actions quota |
| Manual deep, S1, S2, and native OpenSSH workflows | Implemented as bounded opt-in/controlled workflow contracts | Their declared self-hosted runners, hardware, evidence manifests, and independent review remain required |
| Stable release graph and artifact trust | Implemented and mutation-tested through final package/signature/publication boundaries | Production signing identities, notarization, active S1/S2 evidence, governance audit, and a real release run remain required |
| Build and CI critical-path controls | Source-complete, mutation-tested, and exercised by exact-commit cold/warm run `33593793029`; local readiness avoids a duplicate compile, quality uses a versioned compiler cache, and native packages run beside quality while staying cold | GitHub eviction, quota, rate limiting, runner variance, and longitudinal evidence remain external |
| GitHub branch/ruleset enforcement | Locally versioned and remotely auditable | Private GitHub Free does not expose the required server-side branch/ruleset controls |

Authenticated inspection on 2026-08-31 found the then-latest `main` CI run stopped
before runner assignment with zero steps. GitHub reported an account payment or
Actions spending-limit prerequisite. This is classified as `external-billing`,
not a source-code test failure and not a passing run. The separate branch
protection API continues to report the private-plan upgrade/public-visibility
prerequisite. Source changes cannot legitimately hide either condition. Actions
execution subsequently became available: exact-commit PR run `33593759776` and
Linux rehearsal run `33593793029` passed on 2026-09-02. That later execution
evidence does not change the separate private-plan branch-protection limit.

## Workflow ownership

| Workflow | Trigger and owner |
|---|---|
| `.github/workflows/ci.yml` | PR/push policy, portable Rust quality, dependency security, Linux release validation, and release-only Windows coverage |
| `.github/workflows/nightly.yml` | Manual deep fuzz/Miri/sanitizer/package work plus opt-in controlled native/resource/benchmark evidence |
| `.github/workflows/f5-openssh-assurance.yml` | Manual exact-commit controlled OpenSSH evidence |
| `.github/workflows/s1-assurance.yml` | Manual controlled native/visual/resource/accessibility evidence validation |
| `.github/workflows/s2-assurance.yml` | Manual independently reviewed performance-baseline activation |
| `.github/workflows/release.yml` | Merged internal release PR authorization through signed immutable publication |

All workflows default to read-only contents. External Actions must use a
reviewed repository and a full lowercase 40-character commit. Only the final
publication job receives `contents: write`; signing credentials stay in their
dedicated jobs.

## Build performance and artifact isolation

`rust-toolchain.toml` is the stable compiler authority. The CI, Linux Early
Access, stable-release and manual-deep workflows select that exact version with
`RUSTUP_TOOLCHAIN`, which also controls Cargo's Rustup proxy. Installation steps
report `rustc --version` without changing the runner's global default. Reviewed
explicit nightly commands remain separate. The free-plan checker rejects
selection drift, duplicate or nested selectors, floating/invalid pins and a
legacy `rust-toolchain` file that would compete with the canonical file.
This follows [Rustup's override precedence](https://rust-lang.github.io/rustup/overrides.html).

`cargo ready` and `cargo xtask ci` run repository policy, metadata, formatting,
warning-denied all-target/all-feature Clippy, workspace tests, dependency
policy, shell checks, and smoke validation. They no longer run a standalone
workspace `cargo check` immediately before Clippy compiles the same graph.
`cargo xtask check` retains the explicit Cargo check for focused contributor
use. The local gate still uses a fresh bounded target and removes it on exit.

Ordinary and Linux-release quality jobs install sccache v0.16.0 through the
reviewed Mozilla Action pinned at commit
`fc920bf0ec8de6ee65d409111f7ec508035751ba`. The Action verifies its release
download; Automexia derives the `automexia-rust-<selected-version>-v1` cache
generation from `RUSTUP_TOOLCHAIN` and reports cache statistics. Cache misses, eviction, and service
limits degrade only performance. `cargo clean` remains between Clippy and
Nextest to bound the runner filesystem.

The setup step appends the cache generation to `GITHUB_ENV` before sccache
starts. A job-level `env` expression cannot reference the `env` context;
[GitHub's environment-file mechanism](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#setting-an-environment-variable)
shares the computed value with subsequent steps. Actionlint validates the actual
workflow syntax; native Linux Bash tests execute both initializer lines and
verify exact output while preserving preexisting environment-file content.

Release package jobs use the shared, lockfile-bound Cargo source cache but never
the compiler cache or `target`. They remain native cold builds of the exact
source. Package and quality jobs run concurrently after authorization; both
rehearsal and public assembly require quality plus every native architecture.
The architecture-matched nFPM v2.43.4 archive is SHA-256 checked before use.
The canonical design, baseline, threat model, test inventory, rollback, and
hosted evidence are in
[CI and build performance plan](CI-BUILD-PERFORMANCE-PLAN.md).

On exact implementation commit
`b38f0e884aa7dd1d7bd05adba6853218ffb98fb2`, credential-free rehearsal run
`33593793029` passed twice. The cold-cache attempt took 25m56s end to end and
reported 185 compiler-cache hits, 1,127 misses, and no errors. The warm attempt
took 15m01s, reported 1,308 hits, 4 misses, and no errors, and retained both
cache-free native package sets. Against pre-change run `33581137496` at 37m11s,
those bounded observations are 30.3% and 59.6% shorter. They are exact-run
evidence, not a guarantee for later hosted-runner or dependency conditions.
The independent ordinary PR quality/test job in run `33593759776` also passed
twice: 38m20s with 474 hits and 1,681 misses, then 13m44s with 2,153 hits and
2 misses. Both attempts reported no cache errors; the exact warm reduction was
64.2%.

## Checker and mutation ownership

- `.github/scripts/check_action_pins.py` bounds workflow count/bytes, rejects
  linked files, mutable refs, Docker actions, and unreviewed repositories.
  `tools/ci/test_action_pins.py` mutates every branch.
- `.github/scripts/check_free_plan_contract.py` freezes the seven-workflow
  inventory, excludes paid-only private features, and permits a standard hosted
  Windows runner only for `release-candidate-coverage`. It also freezes the
  checksum-pinned ShellCheck installation and explicit actionlint integration so
  workflow shell diagnostics cannot disappear on a contributor platform.
- `tools/ci/check_platform_coverage.py` parses workflow YAML and validates job
  graphs, permissions, timeouts, runners, evidence environments, signing,
  packaging, and final publication dependencies. Its mutation suite removes or
  weakens each critical edge.
- `tools/ci/check_coverage.py` accepts only bounded regular LCOV/baseline files,
  normalizes source identities against the checkout, rejects traversal and
  malformed records, and independently evaluates global and changed-owned-line
  thresholds. Its atomic summary contains counts, percentages, and at most 64
  sorted repository-relative uncovered line identities with an exact total and
  truncation flag, never source content.
- `.github/scripts/build_release_manifest.py` accepts exactly 11 package slots
  and, during finalization, exactly the named SPDX and CycloneDX documents. It
  rejects duplicates, extras, links, partial identities, resource excess, copy
  changes, and non-atomic manifest targets.
- `tools/ci/check_vendored_licenses.py` performs a bounded no-follow notice and
  incompatible-license scan with direct hard-link and limit mutations.
- `tools/ci/repository_protection.py`, `release_trust.py`, `stable_release.py`,
  `s1_assurance.py`, `performance_assurance.py`, and
  `github_free_assurance.py` remain the typed policy/evidence owners for their
  respective external boundaries.

The hosted policy job and explicitly invoked local assurance profile both run
actionlint 1.7.12 with an explicit checksum-pinned ShellCheck 0.11.0 path. The
local pre-push hook is dormant and does not invoke this profile. The full Python
policy layer also runs:

```text
python -m unittest discover -s tools/ci -p "test_*.py"
```

That semantic discovery command prevents new mutation suites from being left
out of a hand-maintained list. Platform-specific `.ps1`, shell, native GUI,
hardware, fuzz, sanitizer, and packaging tests remain explicit because their
environments are not portable Python unit-test environments.

The ordinary Rust job preserves complete all-feature Clippy, Nextest, and
documentation coverage within the standard private GitHub Free Linux runner.
That runner currently provides 2 CPUs, 8 GiB of memory, and 14 GB of storage.
The job therefore uses one Cargo build worker, one Nextest worker, disables
disposable dev/test debug information, and removes Clippy artifacts before the
separate all-feature test build. The free-plan checker and mutation suite reject
weakened limits, missing cleanup, and cleanup reordered outside the
Clippy-to-Nextest boundary.

This envelope was added after the exact 2026-09-01 hosted commit passed
all-feature Clippy but lost runner communication during the test build. GitHub's
failure annotation identified CPU, memory, network, or runner-process starvation
as the class of failure; it did not report a failed product assertion. The
bounded replacement keeps coverage intact instead of retrying an identical
resource-unbounded run.

The first bounded hosted run passed Clippy and artifact reclamation, then
exposed a separate Linux-native Ghostty migration regression: link rejection
failed only because its diagnostic said `non-symlink` while the security
contract required the explicit words `symbolic link`. The owning implementation
now emits one stable role-specific diagnostic for source, include, and
destination links. A platform-neutral assertion freezes the message, while the
real Unix symlink test remains the independent filesystem oracle; the rejection
itself was not removed or relaxed.

The successor hosted run passed the complete all-feature Linux workspace suite,
including that real symlink path, and the deterministic Loom model. It then
exposed a distinct test-graph defect in `cargo xtask test image-rendering`:
Sugarloaf's stand-alone test invocation inherited `rio-window` with workspace
defaults disabled, so neither Linux display backend was selected. The earlier
workspace build had hidden the missing edge through feature unification.
Sugarloaf now explicitly requests X11 and Wayland for its development-only
window dependency. The next hosted run passed that owner and exposed the same
hidden edge in the command's later stand-alone rio-backend test: its default
window bridge forwarded the platform features only to rio-vt, not to the
optional rio-window dependency. rio-backend now forwards X11 and Wayland to
both owners whenever that window bridge is active while preserving lean
headless consumers. The free-plan checker parses both manifest contracts, and
its mutation suite proves that removing any required edge fails before another
hosted run can be dispatched. Published dependency authority is unchanged.

That exact hosted run then passed the focused image/resource graph and
documentation tests before the Unix shell smoke exposed a separate
non-interactive Zsh failure. The runner's ambient `fpath` contained insecure
completion directories, so a plain `compinit -D` attempted to ask for a
decision through a terminal the CI process did not own. The harness now uses
Zsh's documented [`compinit -i` safe-ignore
mode](https://zsh.sourceforge.io/Doc/Release/Completion-System.html#Use-of-compinit),
which silently removes insecure entries rather than trusting them. A native
regression injects a world-writable completion directory, proves its canary is
never registered, runs without a controlling terminal, and mutation-checks that
removing `-i` reproduces the failure. Automexia's shipped Zsh adapter remains
native-first and never invokes `compinit`.

The same shell gate exposed a WSL-specific Bash alias-reload regression. Three
pre-fix 25-sample runs reported 91 ms, 66 ms, and 75 ms p95 against the 50 ms
budget because each reload launched separate metadata and byte-count processes
for every bounded path. The adapter now batches directory modes and obtains each
file's mode and size from one GNU/BSD `stat` invocation while preserving the
existing symlink, type, private-permission, size, digest, and tamper decisions.
Three repeated native WSL runs reported 38 ms, 37 ms, and 45 ms p95. The real
shell test remains the performance oracle, while the CP1 checker and mutation
suite prevent removal of the batching and portable combined-metadata contract.

## Coverage contract

The baseline in `.github/coverage-baseline.json` is platform-specific. Only a
Windows/MSVC report may compare against it. The workflow sets:

```text
BASE_SHA=<exact pull-request base commit>
HEAD_SHA=<exact pull-request head commit>
COVERAGE_PLATFORM=windows-x86_64-msvc
LCOV_FILE=target/coverage/lcov.info
COVERAGE_SUMMARY=target/coverage/summary.json
```

The global percentage may not fall below the recorded baseline. Changed
executable lines under `apps/automexia-terminal`, every Automexia-owned crate,
every first-party provider extension, `tools/xtask`, and the product identity
configuration require 80% coverage. Inherited Rio engine changes remain under
the global ratchet and require their focused behavior tests.

A native Windows x86_64 MSVC worktree run on 2026-08-31 using
`cargo-llvm-cov 0.6.21` measured 59.62% global line coverage against the
43.52% baseline and 100% coverage for seven changed owned executable lines. The
first run correctly failed on four unexercised `tools/xtask` lines; a direct
contract test now executes that owner inside the instrumented process. This
local result validates the current worktree but does not replace the workflow's
pinned `cargo-llvm-cov 0.6.18` run on the exact hosted release commit.

## Local verification

Run fast CI-policy evidence first:

```text
python .github/scripts/check_action_pins.py
python .github/scripts/check_free_plan_contract.py
python tools/ci/validate_repository.py
python -m unittest discover -s tools/ci -p "test_*.py"
python tools/ci/check_vendored_licenses.py
```

The Unix shell gate enumerates tracked and newly added non-ignored shell
sources with `git ls-files -co --exclude-standard`. Ignored caches, private
evidence, and archival worktrees cannot affect the current checkout's result,
while a newly added repository source is checked before it is committed.

Then run the contributor gate:

```text
cargo ready
```

Pinned security executables use an immutable content-addressed shared toolset.
Their mutable Cargo/Python state, downloads, staging, and process temporary data
use separate generated cache roots. When the assurance runner must invoke
repository readiness itself, the product build keeps the contributor's normal
Cargo home while retaining only the verified tool `PATH` and isolated temporary
directories. This prevents native dependency source roots from changing
underneath a persistent target. See
[Development cache and build storage](DEVELOPMENT-CACHE.md).

Repository readiness also owns a bounded workspace-test process tree. The
summarized Cargo test command runs in a Unix process group or Windows Job
Object, terminates after 30 minutes, and captures at most 16 MiB of stdout while
leaving compiler stderr visible. Real self-spawn tests prove success, deadline,
and overflow behavior; policy mutations fail when either platform wrapper, the
deadline, the ceiling, or those tests disappear.

### Current local evidence

The 2026-09-01 native Windows x86_64 MSVC audit of this worktree produced the
following bounded evidence:

- complete Python discovery passed 556 tests with seven platform-capability
  skips;
- all-feature Nextest passed 2,284 tests across 91 binaries with seven declared
  skips, and the separate workspace documentation-test command passed its
  applicable doctests;
- the finite Loom channel model, renderer-neutral image suite, Bash/Zsh/Fish
  source and install/repair/uninstall suite, repository validation, and full QA
  profile passed;
- `cargo ready` passed its clean isolated workspace check, warning-denied
  Clippy, unit/integration/documentation tests, dependency policy, application
  build, and version smoke, then removed 9.45 GiB of disposable artifacts; and
- the explicitly invoked local assurance profile passed Action pinning,
  actionlint, Zizmor, RustSec,
  Cargo Deny, Cargo Vet, changed-history and working-tree Gitleaks scans, two
  Semgrep rules over 620 tracked Rust files with zero findings, and three real
  scanner-canary mutations. This historical run does not mean the dormant
  pre-push hook runs automatically.

A later documentation-only protected push exposed an intermittent assurance
failure: one all-feature terminal test process remained responsive but made no
CPU or output progress for more than ten minutes. The exact all-feature binary
then passed sequentially and in 20 consecutive parallel repetitions, so no
individual product test is falsely blamed. The first stalled run remains the
regression seed; the readiness owner now fails closed at its hard deadline and
cannot retain unbounded output or leave descendants behind.
The interrupted legacy runner had in fact left its owned test tree alive; that
exact tree was removed before the passing bounded rerun, without touching any
unrelated application process.

The introduced-commit scan compares an established branch with its configured
upstream. A new branch with no upstream uses the fetched `origin/HEAD` merge
base. If neither exists, assurance stops with a fetch instruction; it never
passes a lone `HEAD` revision that would silently expand to all inherited
history.

This evidence covers that exact worktree on one Windows host. It does not
replace a successful hosted run on the exact release commit, Linux/macOS native
behavior, signing/notarization, controlled GPU/hardware, long fuzz/resource
campaigns, or human accessibility review. Those remain the explicit gates in
the status table above.

Release maintainers additionally follow `.github/FREE-PRIVATE-PRODUCTION-SETUP.md`,
`RELEASING.md`, and `docs/RELEASE-TRUST.md`. A local pass does not replace the
exact hosted commit, native OS, controlled hardware, signing/notarization,
assistive-technology, or human governance evidence declared there.

## Free-plan design and recovery

Routine PR jobs use Linux only. Internal release coverage uses standard Windows
hosted time because platform-matched coverage is more trustworthy than comparing
different compiler/OS reports. GitHub documents full Action SHAs as the only
immutable third-party Action reference and applies included-minute/storage limits
to private repositories. Keep paid overage disabled for a hard zero-cost ceiling:
quota exhaustion then blocks hosted evidence rather than charging.

When jobs finish with no runner and no steps, inspect the check annotation before
changing source. Restore the account's Actions eligibility or wait for quota,
then rerun the exact commit. Never weaken required checks, replace native proof
with cross-compilation, or classify an unexecuted job as passing.

Primary references:

- [GitHub workflow job dependencies](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-jobs)
- [GitHub secure use and full-SHA pinning](https://docs.github.com/en/actions/reference/security/secure-use)
- [GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions)
- [GitHub Actions limits](https://docs.github.com/en/actions/reference/limits)
- [Cargo continuous integration guidance](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [PyYAML 6.0.3 release](https://github.com/yaml/pyyaml/releases/tag/6.0.3)
