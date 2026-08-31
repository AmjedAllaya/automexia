# F5 native OpenSSH assurance

Status: **Fully implemented locally; controlled native results remain an
external release prerequisite.** This assurance path validates evidence; it
does not enable managed SSH, install OpenSSH, change a service or SSH file, or
grant process, PTY, network, listener, filesystem, or credential authority.

## Controlled workflow

Use `F5 controlled native OpenSSH assurance` only after the exact source commit
and release artifacts have completed independent review recorded outside the
workflow.

1. On GitHub Free/private, keep write and self-hosted-runner access restricted
   to release-authorized maintainers. Record independent review before manual
   dispatch; private environment reviewers are unavailable on this plan.
2. Set repository variable `AUTOMEXIA_F5_OPENSSH_RUNNER=1`.
3. Store the five private evidence paths as repository secrets:
   `AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE`,
   `AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY`,
   `AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE`,
   `AUTOMEXIA_QA_NATIVE_OPENSSH_ADVISORY_REVIEW`, and
   `AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE_PROVENANCE`. Repository variables are
   rejected for these paths. The runner resolves the secret values locally;
   they must never be printed or uploaded.
4. Provision one clean ephemeral JIT runner for the job in the restricted
   `automexia-openssh` group. Its label must be
   `automexia-openssh-<platform>-<architecture>`, where the platform is
   `windows`, `macos`, or `linux` and the architecture is `x86_64` or
   `aarch64`.
5. Provide fixed system `ssh`, `ssh-add`, `ssh-keygen`, and `sshd` tools, the
   exact application artifacts, and a private loopback-only fixture. Do not
   place ambient credentials on the runner.
6. Dispatch the workflow manually with the reviewed lowercase commit digest and
   matching host tuple.

The workflow has read-only repository permission, checks out the exact digest
without persisted credentials, runs the D0 checker and both D0/native mutation
suites in fail-closed platform shells, and uploads only
`target/native-openssh/summary.json`. The path-free summary contains platform,
architecture, scenario/session counts, and binding flags; retention is 90 days.
The private manifest, paths, fixture, SSH configuration, destinations,
usernames, agent data, and terminal output must never be uploaded.

The workflow has no push or pull-request trigger. GitHub warns that self-hosted
runners are not isolated like hosted runners, so the controlled group must stay
restricted and ephemeral; see GitHub's official [secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use)
and [environment protection documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments).

## Direct validator use

For controlled debugging, define all six inputs and run the same validator:

```powershell
$evidenceRoot = Join-Path $PWD 'controlled-evidence'
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE=(Join-Path $evidenceRoot 'm5-windows.json')
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY=(Join-Path $evidenceRoot 'automexia.exe')
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE=(Join-Path $evidenceRoot 'automexia.msi')
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_ADVISORY_REVIEW=(Join-Path $evidenceRoot 'openssh-advisory-review.json')
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE_PROVENANCE=(Join-Path $evidenceRoot 'openssh-package-provenance.json')
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_EXPECTED_COMMIT='<exact-lowercase-digest>'
python3 tools/ci/native_openssh_evidence.py --validate-environment
```

```sh
evidence_root=./controlled-evidence
AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE="$evidence_root/m5-linux.json" \
AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY="$evidence_root/automexia" \
AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE="$evidence_root/automexia.tar.gz" \
AUTOMEXIA_QA_NATIVE_OPENSSH_ADVISORY_REVIEW="$evidence_root/openssh-advisory-review.json" \
AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE_PROVENANCE="$evidence_root/openssh-package-provenance.json" \
AUTOMEXIA_QA_NATIVE_OPENSSH_EXPECTED_COMMIT=<exact-lowercase-digest> \
python3 tools/ci/native_openssh_evidence.py --validate-environment
```

The validator reads one identity-stable bounded manifest snapshot. It requires
23 ordered passing scenarios, the executing native OS and normalized
architecture, the exact clean commit and schema-7 digest, the exact
`automexia --version` result, fixed OpenSSH client/server version strings, and
fresh no-follow SHA-256 reads of the application binary/package, `ssh`, `sshd`,
`ssh-add`, `ssh-keygen`, advisory review, and package provenance. Release files
are capped at 4 GiB. Symlinks,
reparse points, replacement or growth while reading, hash/version/host drift,
real zero-sentinel baselines, WSL, synthetic evidence, failed cleanup, resource
overflow, and redaction leaks fail closed without printing private paths.

The current upstream security baseline is the official
[OpenSSH 10.5 release](https://www.openbsd.org/openssh/releasenotes.html) from
2026-08-11. Evidence must affirm the 10.5 `ssh-agent` locked-agent/session-bind
fix and pending remote-forward cleanup fix, plus the existing restricted-key
session binding, post-quantum default, and weak-crypto warning checks. The
validator deliberately does not compare a vendor version string numerically:
supported operating systems may backport fixes. Instead, the exact tool hashes,
reviewed advisory artifact, and package-provenance artifact must support each
affirmation. Automexia does not override system OpenSSH cryptographic policy.

## Last completed full local evidence on 2026-08-24

On Windows x86_64, the focused source suites passed 19 direct route/trust, 8
tunnel, 4 UI-model, 12 Connection Hub renderer/controller, and 1 strong
Allow-once tests. That run predates schema 7 and is retained as historical
evidence rather than evidence for the current contract.

The required full gates passed: Rustfmt, warning-denied workspace Clippy,
Nextest with 2,070 passed and 7 skipped, and documentation tests with 64 passed
and 3 ignored. Full QA passed its repository, architecture, shell, S1/S2,
research benchmark, resize, session-clone, Loom, dependency, test, and doctest
steps. Its redacted report is under
`target/qa/20260824T200813Z-10160/report.html`. `cargo ready` independently
passed cold workspace checks, Clippy, unit/integration/documentation tests,
dependency policy, debug build, and version smoke, then removed its 8.58 GiB
isolated target.

The local prerequisite probe found OpenSSH for Windows 9.5p2 client tools but
no fixed `sshd`, so no real server, tunnel, native resource, visual, or
accessibility result is claimed from this host.

## Current schema-7 local evidence on 2026-08-27

The schema-7 contract checker and all 10 mutation cases pass. The schema-2
native-evidence validator passes 12 tests with one Windows symlink-privilege
skip, and all 35 F5 workflow mutation cases pass. These results prove the local
contract, validator, and workflow behavior only; they do not replace the
controlled native matrix listed below. Focused source evidence passed 79
connectivity tests, 32 broker lifecycle tests, 4 review-worker tests, 7
application composition tests, and 4 UI-model tests. The real planning benchmark
measured `direct_openssh_prepare_selected` at 7.5300-7.5767 microseconds on this
uncontrolled Windows host; it is not a named-hardware release baseline.

Warning-denied workspace Clippy passed. CI-profile Nextest passed 2,270 tests
with 7 declared skips; documentation tests passed 64 with 3 ignored. Full QA
passed after its first run exposed and recovered a 20-KiB free-space condition.
`cargo ready` then passed its cold isolated workspace check, Clippy, all
workspace and documentation tests, dependency policy, application build, and
`automexia 0.4.0` smoke on a short native C: target; the 9.44-GiB verification
tree and 4.5-GiB temporary target were removed afterward. The first deep-path
attempt failed with Windows linker `LNK1104`; it was investigated rather than
retried in place or treated as passing.

## Remaining release evidence

- [ ] Real loopback OpenSSH cases on controlled Windows, macOS, and Linux for
  host keys, encrypted keys, agents, certificates, jumps, tunnels,
  cancellation, offline behavior, hostile output, exit status, and cleanup.
- [ ] Native 1/10/50 CPU, memory, process, handle/descriptor, PTY, listener,
  tunnel, task, route, cache, log, storage, and zero-leak evidence.
- [ ] Before/after manual SSH, enable, disable, uninstall, and generic-terminal
  baselines plus controlled visual and screen-reader review.
- [ ] ADR 0003 protected approvals/enforcement and real package
  attestation/revocation. WSL remains separately denied.

To disable collection, unset `AUTOMEXIA_F5_OPENSSH_RUNNER` or remove runner
access. Delete only private runner evidence according to the release retention
policy; never change user SSH files or services. Managed SSH remains unavailable
because `MANAGED_SESSION_LAUNCH_ENABLED` is false and the package is unverified.
