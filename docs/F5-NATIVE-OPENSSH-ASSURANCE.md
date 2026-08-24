# F5 native OpenSSH assurance

Status: **Fully implemented locally; controlled native results remain an
external release prerequisite.** This assurance path validates evidence; it
does not enable managed SSH, install OpenSSH, change a service or SSH file, or
grant process, PTY, network, listener, filesystem, or credential authority.

## Controlled workflow

Use `F5 controlled native OpenSSH assurance` only after the exact source commit
and release artifacts have completed protected review.

1. Configure `f5-openssh-release` as a protected environment with independent
   required reviewers and no self-review.
2. Set repository variable `AUTOMEXIA_F5_OPENSSH_RUNNER=1`.
3. In that environment, set `AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE`,
   `AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY`, and
   `AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE` to private absolute paths on the
   controlled host.
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
without persisted credentials, runs the mutation suite, and uploads only
`target/native-openssh/summary.json`. The path-free summary contains platform,
architecture, scenario/session counts, and binding flags; retention is 90 days.
The private manifest, paths, fixture, SSH configuration, destinations,
usernames, agent data, and terminal output must never be uploaded.

The workflow has no push or pull-request trigger. GitHub warns that self-hosted
runners are not isolated like hosted runners, so the controlled group must stay
restricted and ephemeral; see GitHub's official [secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use)
and [environment protection documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments).

## Direct validator use

For controlled debugging, define all four inputs and run the same validator:

```powershell
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE='C:\private\m5-windows.json'
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY='C:\private\automexia.exe'
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE='C:\private\automexia.msi'
$env:AUTOMEXIA_QA_NATIVE_OPENSSH_EXPECTED_COMMIT='<exact-lowercase-digest>'
python3 tools/ci/native_openssh_evidence.py --validate-environment
```

```sh
AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE=/private/m5-linux.json \
AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY=/private/automexia \
AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE=/private/automexia.tar.gz \
AUTOMEXIA_QA_NATIVE_OPENSSH_EXPECTED_COMMIT=<exact-lowercase-digest> \
python3 tools/ci/native_openssh_evidence.py --validate-environment
```

The validator reads one identity-stable bounded manifest snapshot. It requires
23 ordered passing scenarios, the executing native OS and normalized
architecture, the exact clean commit and schema-5 digest, fixed OpenSSH client
and server version strings, and fresh no-follow SHA-256 reads of the application
binary, package, and client. Release files are capped at 4 GiB. Symlinks,
reparse points, replacement or growth while reading, hash/version/host drift,
real zero-sentinel baselines, WSL, synthetic evidence, failed cleanup, resource
overflow, and redaction leaks fail closed without printing private paths.

The current OpenSSH security baseline is the official [OpenSSH 10.5 release](https://www.openbsd.org/openssh/releasenotes.html),
including agent session-binding/restricted-key behavior. The evidence contract
also observes the upstream post-quantum default and non-post-quantum warning; it
does not override system OpenSSH cryptographic policy.

## Local evidence on 2026-08-24

On Windows x86_64, the focused source suites passed 19 direct route/trust, 8
tunnel, 4 UI-model, 12 Connection Hub renderer/controller, and 1 strong
Allow-once tests. The native evidence suite passed 11 methods with one
symlink-privilege skip; schema-5 mutations passed 10; platform workflow
mutations passed 29; and repository-protection mutations passed 23.

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
