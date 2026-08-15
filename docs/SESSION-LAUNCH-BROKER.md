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
[OWASP injection prevention guidance](https://cheatsheetseries.owasp.org/cheatsheets/Injection_Prevention_Cheat_Sheet.html).

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
2. The verified principal is exactly the repository-owned
   `automexia.devops-ssh` publisher and build version.
3. Capability, decision, launch, executable resource, operation ID, session ID,
   and non-zero capsule revision all match.
4. The decision is not `Deny` and is not timestamped in the future.
5. Neither the exact extension nor session has been revoked.
6. The request is native, has no local shell profile, does not request inherited
   environment names, and contains no secret references.
7. The executable and operation grammar are explicitly supported.
8. The operation ID is not already active.

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

Windows obtains the system directory from `GetSystemDirectoryW`, then checks
its `OpenSSH` directory. Microsoft documents the in-box client under
[`%SystemRoot%\\System32\\OpenSSH`](https://learn.microsoft.com/en-us/windows-server/administration/openssh/openssh-overview).
Linux and macOS use reviewed fixed locations such as `/usr/bin`; optional
configured absolute paths cover administrator- or package-manager-installed
clients without trusting `PATH`.

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
the already validated core-owned safe default. A relative or invalid safe
default is denied. No failure changes the executable, shell, remote target, or
session.

## Lifecycle and audit contract

Authorization reserves an immutable operation lease containing operation ID,
session ID, capsule revision, and a monotonically changing nonce. Duplicate
operations fail. Completion and cancellation remove only an exact lease. A
stale lease cannot cancel a reused operation ID, and revoking one extension or
session cannot remove a sibling binding.

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

## Verification

Run the focused contract with:

```text
cargo test -p automexia-extension-api --lib --locked
cargo test -p automexia-terminal --bin automexia --locked context::launch_broker::tests
cargo xtask verify architecture
```

The suite covers hard production denial, exact principal/capability/decision
scope, future and denied decisions, broad-spawn rejection, shell/WSL rejection,
leading-dash and extra-argument rejection, literal native argument preservation,
fixed absolute resolution, unsupported tools, native file replacement,
vanished and relative cwd behavior, bounded environment validation, redaction
canaries, duplicates, revocation, cancellation, stale leases, and cross-scope
isolation. Native Windows uses file-index evidence; native Linux/macOS test
jobs compile and run the Unix device/inode path.

## Remaining activation gates

This phase is not complete as a shipped feature. Before removing the test-only
module gate, maintainers must:

1. accept ADR 0012 with the two protected-path approvals required by ADR 0003;
2. implement a visible, accessible capability decision UI and deterministic
   persisted/session grant policy if persistence is supported;
3. bind process, PTY, route, capsule, operation, and tunnels before publication
   through the existing application launch path;
4. add native Windows, Linux, and macOS spawn/cancel/teardown and hostile-argv
   evidence, including PID reuse and application close;
5. complete D4 OpenSSH inventory/security work and the redaction matrix;
6. pass the controlled 1/10/50-session performance and leak gates.

No roadmap or test result may describe managed SSH/session launch as available
until every activation gate passes.
