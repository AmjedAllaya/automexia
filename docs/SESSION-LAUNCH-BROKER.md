# Exact-argument session-launch broker

Automexia contains a reviewable, test-only candidate for the v0.5
`session.launch` boundary. It is not a user-facing feature, is not present in
the production module graph, and cannot create a process or PTY. This page is
the exact contract and evidence reference for that candidate.

The governing decisions are [ADR 0003](adr/0003-extension-capability-and-threading.md)
and proposed [ADR 0012](adr/0012-first-party-ssh-and-session-launch-boundary.md).
ADR 0003 remains authoritative until ADR 0012 receives the required security
review and two protected-path approvals.

## Current activation state

- `apps/automexia-terminal/src/context/mod.rs` includes the broker only under
  `#[cfg(test)]`.
- `MANAGED_SESSION_LAUNCH_ENABLED` is a compile-time `false` constant guarded
  by a constant assertion.
- The production frontend has no broker construction, successful authorization,
  process spawn, PTY attachment, quick-connect UI, persistence, or SSH network
  path.
- Manual `ssh host` remains normal shell input and is unchanged.

This distinction is intentional: implementing and reviewing validation code is
allowed before the security decision; exposing process authority is not.

## Why exact argv and system OpenSSH

The candidate uses an executable identity plus an ordered vector of arguments.
It never creates a command string and never calls `sh -c`, `cmd /c`,
PowerShell evaluation, a batch file, or another command interpreter. Rust's
[`std::process::Command`](https://doc.rust-lang.org/stable/std/process/struct.Command.html)
passes `arg` values literally without shell expansion; its Windows guidance
also warns that `cmd.exe` and batch files use non-standard parsing and require
special care.

The eventual SSH process is the platform's system OpenSSH client. Automexia
does not implement SSH cryptography, host-key storage, key custody, agent
protocols, or configuration evaluation. This keeps OpenSSH authoritative and
follows the allowlist-plus-parameterization guidance in the
[OWASP OS-command injection guidance](https://cheatsheetseries.owasp.org/cheatsheets/OS_Command_Injection_Defense_Cheat_Sheet.html).

## Trust and data flow

The proposed flow is:

```text
verified first-party principal
  + typed CapabilityRequest
  + typed LaunchRequest
  + matching CapabilityDecision
  + exact session/capsule revision
  + core-owned cwd/environment
    -> application broker validates all scopes and limits
    -> resolver returns canonical executable + file identity
    -> operation lease binds extension/session/capsule/generation
    -> existing SessionLaunchDescriptor seam receives exact argv
```

The extension never receives or controls a PTY handle, process handle, route,
renderer object, terminal contents, inherited environment, agent protocol, or
resolved credential.

## Authorization contract

A request is rejected unless all of these conditions hold:

1. The broker is in the test-only review harness. The production/pending
   constructor always returns `PendingSecurityReview` before path resolution.
2. The verified principal exactly matches the broker's reviewed
   `automexia.devops-ssh` package policy: ID, publisher, workspace version,
   non-zero 32-byte digest, contract version, and either repository-reviewed
   or first-party-signed verification. Unverified packages and every mismatch
   fail before executable resolution. A real loader/attestation binding remains
   an activation gate.
3. Capability, decision, launch, executable resource, operation ID, session ID,
   and non-zero capsule revision all match.
4. The application has registered that exact session/capsule; closed session
   IDs cannot be reused and a rebind advances the capsule monotonically.
5. The decision is not `Deny`, future-dated, or expired. Its extension,
   capability, resource, operation, session, and capsule fields match exactly.
6. The operation ID is greater than the last successfully authorized operation
   in that session and is not already active.
7. The exact extension has not been revoked.
8. The request is native, has no local shell profile, does not request inherited
   environment names, and contains no secret references.
9. The executable and operation grammar are explicitly supported.

Broad `process.spawn`, wildcard executable paths, WSL shell creation,
third-party principals, and silent fallback are rejected. Recognizing the
names `ssh-add` and `ssh-keygen` does not grant their use: both remain
`UnsupportedOperation` until a shipped operation and argument grammar receive
separate review.

## Executable resolution and identity

Only these identifiers exist in the candidate policy:

- `ssh`
- `ssh-add`
- `ssh-keygen`

The resolver checks fixed absolute system locations and explicitly configured
absolute paths. It never searches `PATH` or the current working directory.
Configured paths must have the exact platform filename; `.cmd`, `.bat`, shell
scripts, renamed binaries, relative paths, and wildcard names are rejected.
An explicit configured path replaces the defaults for that executable ID: if
it is missing or changed, resolution fails instead of silently using another
system candidate.

Windows obtains the system directory from `GetSystemDirectoryW`, then checks
its `OpenSSH` directory. Microsoft documents the in-box client under
[`%SystemRoot%\\System32\\OpenSSH`](https://learn.microsoft.com/en-us/windows-server/administration/openssh/openssh-overview).
macOS checks `/usr/bin`, `/usr/local/bin`, and `/opt/homebrew/bin`; Linux
checks `/usr/bin`, `/bin`, and `/usr/local/bin`. Optional configured
absolute paths cover administrator-installed clients without trusting `PATH`.
WSL production launch is disabled and produces no candidates. Its future
contract requires the fixed Windows system-directory `wsl.exe`, `--exec`
without a shell, a verified distribution, and `/usr/bin/ssh`; those steps
require later native review and are not implemented by this model.

After canonicalization, the broker records native identity:

- Windows: volume serial, file index, size, and last-write time from
  `GetFileInformationByHandle`;
- Unix: device, inode, size, and nanosecond modification time;
- other compile targets: size and modification time as a conservative fallback.

The only conversion to the existing launch descriptor revalidates the
executable and working directory in the same operation, so a future caller
cannot omit those checks. A rename-and-replace attack is rejected even when the
new file has the same name and byte length. Activation still requires the
process owner to close the remaining check-to-spawn race with a platform-owned
handle or equivalent native primitive and native adversarial evidence.
[POSIX defines `fexecve`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/exec.html)
specifically so a verified file cannot be exchanged between inspection and
execution. Windows activation must likewise use an explicit application path
and prove the exact executable/handle strategy; Microsoft
[warns for `CreateProcessW`](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)
that leaving `lpApplicationName` null can execute an unintended binary when
paths contain spaces.

## Argument, environment, and cwd limits

The only successful review-harness grammar is currently one literal SSH
destination alias:

```text
ssh <destination-alias>
```

The alias is at most 512 UTF-8 bytes, contains no control or whitespace
characters, and cannot start with `-`. The request has one argument and a
32 KiB aggregate argument ceiling. No remote command or command-line option is
currently authorized.

Extension-selected inherited environment names are rejected. Core may supply
configuration-owned public overrides within these limits:

- 256 entries;
- 256 bytes per name;
- 16 KiB per value;
- 256 KiB total;
- no empty, duplicate, control-containing, NUL-containing, or `=`-containing
  name.

The inherited process environment—including an agent variable such as
`SSH_AUTH_SOCK`—remains application-owned and is never copied into the
extension request, diagnostic, or audit record.

The requested working directory must be absolute. A valid directory is
canonicalized. If an absolute requested directory vanished, the broker uses
the canonical core-owned safe default captured during authorization; conversion
does not accept a second caller-supplied fallback. A relative or invalid safe
default is denied. No failure changes the executable, shell, remote target, or
session.

## Lifecycle and audit contract

Authorization reserves an immutable operation lease containing operation ID,
session ID, capsule revision, and a monotonically changing nonce. Lease nonce
exhaustion fails closed. Duplicate active operations and replayed completed or
cancelled operation IDs fail. Completion and cancellation remove only an exact
lease. Rebind cancels only the old session revision, and revoking one extension
or session cannot remove a sibling binding. Session-close state is removed from
the active registry while one scalar high-water mark prevents ID reuse, avoiding
an unbounded revoked-session tombstone set.

The current lifecycle is a pure model: it owns no child, listener, PID, route,
or PTY. Actual graceful termination, bounded force termination, listener
closure, application-close cleanup, and PID-reuse evidence remain blocked
until process activation is approved.

Audit records contain only extension ID/version/publisher, decision,
operation kind, optional future public connection ID, operation/session ID,
timestamp, duration, and result class. They contain no argv, cwd, executable
path, environment value, terminal content, private username, process ID,
credential, or agent data. The broker does not infer a connection ID from a
destination argument.

## D0/D3 fixture contract

`tests/fixtures/session-launch/d0-d3-contract-v1.json` is the canonical local
decision and native-fixture baseline. For Windows, macOS, Linux, and WSL it
defines direct/explicit/user-port connections, encrypted-key prompts, agents,
certificates, new/known/changed host keys, ProxyJump, every forwarding type,
cancellation, exit classification, hostile output, offline behavior, shutdown
cleanup, and 1/10/50-session resource proof. Each row freezes its expected safe
outcome rather than only naming a case.

The same contract keeps process, PTY, network, provider, authentication,
key-custody, and renderer authority false. Strict host-key checking is
preserved; new trust is explicit, changed keys fail, agent/TCP/X11 forwarding
and remote commands default off, listener scope is loopback unless separately
confirmed, discovery runs no config command, and inherited environment,
secrets, and shell evaluation remain unavailable.

This matrix is a definition, not a claim that native sessions ran. F4/F5 own
the process/PTY implementation and execution evidence. The contract status
therefore remains
`local-contract-complete-protected-approval-pending`.


Run the focused contract with:
## Verification

```text
python tools/ci/check_session_launch_d0.py
python tools/ci/test_session_launch_d0.py
cargo test -p automexia-extension-api --lib --locked
cargo test -p automexia-terminal --bin automexia --locked context::launch_broker::tests
cargo xtask verify architecture
```

The suite covers contract mutation, hard production denial, exact package
digest/version/contract/verification matching, four-platform fixed roots,
principal/capability/decision scope, future/expired/denied decisions, capsule
registration/rebind, replay and nonce exhaustion, broad-spawn and shell/WSL
rejection, leading-dash and extra-argument rejection, literal native argv,
fail-closed resolution, file replacement, cwd/environment limits, redaction,
revocation, cancellation, stale leases, cross-scope isolation, and cleanup
after 1/10/50 pure lifecycle cycles.

## Remaining activation gates

This phase is not complete as a shipped feature. Before removing the test-only
module gate, maintainers must:

1. accept ADR 0012 with the two protected-path approvals required by ADR 0003;
2. bind the real package loader's digest/signature, publisher, exact compatible
   version/contract, and live revocation result to the frozen reviewed-package
   policy before constructing a principal;
3. implement a visible, accessible capability decision UI and deterministic
   persisted/session grant policy if persistence is supported;
4. bind process, PTY, route, capsule, operation, and tunnels before publication
   through the existing application launch path;
5. add native Windows, macOS, Linux, and WSL spawn/cancel/teardown and hostile-argv
   evidence, including PID reuse and application close;
6. close the executable check-to-spawn race with a reviewed native mechanism;
7. connect the completed disabled D4 inventory only through reviewed D5
   surfaces and complete the cross-surface redaction matrix;
8. pass the controlled 1/10/50-session process, PTY, renderer, performance,
   and leak gates. The pure broker lifecycle test is necessary but not a
   substitute for those native measurements.

No roadmap or test result may describe managed SSH/session launch as available
until every activation gate passes.
