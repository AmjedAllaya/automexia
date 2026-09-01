# CI and release assurance

This document is the implementation audit and operating guide for Automexia's
continuous-integration and release-assurance code. It describes source truth;
it does not turn unavailable hosted, signing, hardware, or human evidence into
a pass.

## Current status

| Area | Source/local status | Remaining external evidence |
|---|---|---|
| Ordinary PR policy, Rust quality, dependency security | Implemented and covered by repository validators plus complete Python mutation discovery | A current hosted run must execute on the exact commit |
| Internal release validation | Implemented and mutation-tested | A real internal `release/X.Y.Z` PR must run it |
| Windows/MSVC coverage ratchet | Implemented with exact commit, platform, path, size, and changed-owned-line binding | The standard Windows runner must execute successfully within available Actions quota |
| Manual deep, S1, S2, and native OpenSSH workflows | Implemented as bounded opt-in/controlled workflow contracts | Their declared self-hosted runners, hardware, evidence manifests, and independent review remain required |
| Stable release graph and artifact trust | Implemented and mutation-tested through final package/signature/publication boundaries | Production signing identities, notarization, active S1/S2 evidence, governance audit, and a real release run remain required |
| GitHub branch/ruleset enforcement | Locally versioned and remotely auditable | Private GitHub Free does not expose the required server-side branch/ruleset controls |

Authenticated inspection on 2026-08-31 found the latest `main` CI run stopped
before runner assignment with zero steps. GitHub reported an account payment or
Actions spending-limit prerequisite. This is classified as `external-billing`,
not a source-code test failure and not a passing run. The separate branch
protection API continues to report the private-plan upgrade/public-visibility
prerequisite. Source changes cannot legitimately hide either condition.

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

The hosted policy job and local pre-push profile both run actionlint 1.7.12 with
an explicit checksum-pinned ShellCheck 0.11.0 path. The full Python policy layer
also runs:

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

Pinned security tools use a repository-local Cargo home for reproducibility.
When the assurance runner must invoke repository readiness itself, the product
build keeps the contributor's normal Cargo home while retaining the isolated
tool `PATH` and temporary directories. This prevents native dependency source
roots from changing underneath a shared persistent target.

### Current local evidence

The 2026-08-31 native Windows x86_64 MSVC audit of this worktree produced the
following bounded evidence:

- complete Python discovery passed 534 tests with six platform-capability
  skips;
- all-feature Nextest passed 2,281 tests across 91 binaries with seven declared
  skips, and the separate workspace documentation-test command passed its
  applicable doctests;
- the finite Loom channel model, renderer-neutral image suite, Bash/Zsh/Fish
  source and install/repair/uninstall suite, repository validation, and full QA
  profile passed;
- `cargo ready` passed its clean isolated workspace check, warning-denied
  Clippy, unit/integration/documentation tests, dependency policy, application
  build, and version smoke, then removed 9.45 GiB of disposable artifacts; and
- the pre-push profile passed Action pinning, actionlint, Zizmor, RustSec,
  Cargo Deny, Cargo Vet, changed-history and working-tree Gitleaks scans, two
  Semgrep rules over 620 tracked Rust files with zero findings, and three real
  scanner-canary mutations.

The introduced-commit scan compares an established branch with its configured
upstream. A new branch with no upstream uses the fetched `origin/HEAD` merge
base. If neither exists, assurance stops with a fetch instruction; it never
passes a lone `HEAD` revision that would silently expand to all inherited
history.

This evidence covers that exact worktree on one Windows host. It does not claim
a successful hosted run, Linux/macOS native behavior, signing/notarization,
controlled GPU/hardware, long fuzz/resource campaigns, or human accessibility
review. Those remain the explicit gates in the status table above.

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
