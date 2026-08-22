# Exact-argument session-launch broker

Automexia contains a production-compiled but hard-disabled candidate for the
v0.5 `session.launch` boundary. The application-owned runner, guarded native
executable seam, ContextManager PTY/route adapter, and approval surface are
locally implemented. The compile-time activation gate is false and the linked
extension principal is unverified, so no managed OpenSSH process or PTY is
reachable. This page is the exact contract and evidence reference.

The governing decisions are [ADR 0003](adr/0003-extension-capability-and-threading.md)
and accepted [ADR 0012](adr/0012-first-party-ssh-and-session-launch-boundary.md).
The project owner accepted ADR 0012 on 2026-08-22. ADR 0003's requirement for
two independent exact-head protected-path approvals remains unsatisfied and
continues to block production activation.

## Current activation state

- The broker and `ExternalToolRunner` are each declared once in the production
  module graph.
- `MANAGED_SESSION_LAUNCH_ENABLED` is a compile-time `false` constant guarded
  by a constant assertion.
- Router owns one runner and shares it with every window; shutdown reconciles
  active leases and the bounded redacted audit buffer.
- The linked `devops-ssh` candidate is `PackageVerification::Unverified`.
- ContextManager is the only owner that consumes a guarded executable and calls
  `create_exact_pty`; it inserts the Context before marking the lease published.
- The Connection Hub exposes deny, allow-once, and allow-session approval
  actions, but `execution_enabled` remains false and every attempt returns a
  fixed recovery state before executable or PTY work.
- Manual `ssh host` remains normal shell input and is unchanged.

This distinction is intentional: application ownership and UX can be reviewed
without weakening the protected gate or claiming a shipped connection path.

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

## Upstream M3 reviewed request

The nonactivated M3 model prepares the only direct request shape: canonical
executable ID `ssh`, exact `session.launch`, 17 ordered application-owned safety
options, and exactly one typed destination. A full current review binds profile/
source/capsule revisions, F2 plan, destination, executable identity, observation
content/generation/freshness, host trust, capability, and fingerprint into an
opaque launch value. Debug and presentation redact all identity-bearing values.

The runner accepts that opaque binding rather than rebuilding argv. It creates
typed capability/decision/launch values, reserves one exact route/session/
operation tuple, and submits exact ordered arguments to the broker, which
revalidates current native executable identity. The candidate principal remains
unverified and the protected gate denies before filesystem resolution. On a
future successful authorization, ContextManager alone consumes the guard and
publishes a new independent PTY route. Real loader attestation and product
controller consumption of a current observation remain activation prerequisites.

## Trust and data flow

The guarded flow, currently stopped by the first two activation checks, is:

```text
activation enabled + attested first-party principal
  + typed capability/launch/decision
  + exact operation/session/capsule/route
  + core-owned cwd/environment
    -> application broker validates scopes, policy, and limits
    -> resolver records canonical executable identity
    -> runner opens and re-compares the exact executable guard
    -> operation lease binds package/session/capsule/generation
    -> ContextManager consumes guard and creates the PTY
    -> Context insertion precedes lease publication and renderer wake
```

The extension never receives or controls a PTY handle, process handle, route,
renderer object, terminal contents, inherited environment, agent protocol, or
resolved credential.

### Frozen trust-boundary ledger

The active schema-3 contract retains nine boundaries: extension model,
application capability broker, future PTY/process owner, renderer/VT parser,
OpenSSH child, OpenSSH configuration, agent/keychain/hardware owner, remote
host, and future provider helper. Every row fixes accepted and returned data,
the applicable size/time ceiling, cancellation owner, log policy, and
fail-safe behavior. The application PTY/process owner now exists behind the
hard activation gate; the provider row remains specification-only. Neither may
gain production authority before its separate review and native evidence pass.

## Authorization contract

A request is rejected unless all of these conditions hold:

1. `MANAGED_SESSION_LAUNCH_ENABLED` is true. It is currently a const-asserted
   false value, so production returns `PendingSecurityReview` before path
   resolution. Tests use a narrowly scoped review harness.
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

The low-level PTY owner now has a separate guarded exact-spawn seam. It opens
only an absolute canonical executable and retains an opaque native identity and
replacement guard through process creation. Windows denies write/delete sharing,
passes a non-null application name to CreateProcessW, supplies an exact
application-owned environment, creates the child suspended, assigns it to a
kill-on-close Job Object, and only then resumes it. Unix executes the opened
descriptor with fexecve; macOS uses its guarded /dev/fd/<n> descriptor path.
POSIX defines fexecve so a verified file cannot be exchanged between inspection
and execution, and Microsoft warns that leaving the CreateProcessW application
name null can execute an unintended binary when paths contain spaces.

The application runner now opens and compares this guard with the broker's
reviewed identity. ContextManager atomically binds lease/session/capsule/route,
creates the PTY, inserts the Context, and publishes the lease afterward. This
closes the source-local check-to-spawn and publication handoff, not the product
gate; native Linux/macOS/OpenSSH adversarial execution remains external.

## Argument, environment, and cwd limits

The only authorized grammar is one literal SSH destination alias. Tests exercise
the successful grammar; production denies at the activation/principal gates:

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

The application runner owns active leases, a 50-operation ceiling, a
256-record FIFO audit ceiling, exact publish-before-complete state, session
revocation, and application shutdown reconciliation. Managed Context drop
cancels unreconciled leases; natural close records completion before route
removal. ContextManager owns the new PTY/route and never publishes a route whose
numeric ID differs from its session ID. The compile-time gate means this
lifecycle has no production child today. Graceful-then-forced cross-platform
child-tree teardown, listener/tunnel cleanup, PID-reuse evidence, durable audit
persistence, and controlled leak/resource proof remain activation gates.

Audit records contain only extension ID/version/publisher, decision,
operation kind, optional future public connection ID, operation/session ID,
timestamp, duration, and result class. They contain no argv, cwd, executable
path, environment value, terminal content, private username, process ID,
credential, or agent data. The broker does not infer a connection ID from a
destination argument.

## M3 managed direct-session contract

A managed direct request is now an opaque result of a fresh full-review equality
check, not a destination string that a screen can rebuild. The ordered argv is
exactly the 17 constants in `DIRECT_OPENSSH_MANAGED_OPTIONS` followed by one
typed destination. The broker independently verifies this count/order and binds
the current canonical executable's native file identity digest to the review.
Any profile, source, capsule, plan, trust, observation, executable, option, or
destination change requires a new review and approval.

The fixed options disable key addition, all configured forwarding, connection
multiplexing, escape-commandline, backgrounding, agent/X11/delegated-GSSAPI
forwarding, local/remote commands, proxy commands/jumps, stdin-null, and tunnels.
`StrictHostKeyChecking=ask` keeps first-use confirmation and changed-key rejection
inside OpenSSH's ordinary PTY. User OpenSSH configuration remains authoritative
for alias lookup and can itself contain helper directives such as `Match exec` or
`KnownHostsCommand`; activation therefore still requires native descendant-tree
ownership and cleanup proof. Discovery never executes those directives.

Automexia does not set `KexAlgorithms` or `WarnWeakCrypto`. This preserves the
system client's current algorithm policy and warnings. The dated upstream
baseline used for this review is [OpenSSH 10.5, released 2026-08-11](https://www.openbsd.org/openssh/releasenotes.html),
which includes security fixes for agent session binding/restricted keys and
forwarding cleanup. OpenSSH 10.0 made hybrid post-quantum
`mlkem768x25519-sha256` the default, and 10.1 enabled non-post-quantum warnings;
see the official [OpenSSH post-quantum guidance](https://www.openssh.org/pq.html).
Distribution backports and provenance matter, so a version string alone never
satisfies executable/package attestation.

Natural child exit, nonzero exit, unavailable status, cancellation, route close,
revocation, and shutdown produce distinct redacted results; route close is not
success. A successful or failed terminal outcome can queue a provider-neutral
receipt to the Connection Hub worker. The private atomic store is limited to 256
records and 2 MiB, retains one validated previous generation, and excludes argv,
destination, terminal text, credentials, paths, environment values, process IDs,
and executable identity. An opaque inventory/source revision may be retained for
reconnect. It must match current D4 inventory and still returns to a fresh
executable/host-trust review and explicit approval; no reconnect auto-runs.
## D0/D3 fixture contract

`tests/fixtures/session-launch/d0-d3-contract-v3.json` is the canonical
local decision and native-fixture baseline. The schema-1 fixture remains
unchanged as historical evidence and its exact hash is checked. For Windows,
macOS, Linux, and WSL it
defines direct/explicit/user-port connections, encrypted-key prompts, agents,
certificates, new/known/changed host keys, ProxyJump, every forwarding type,
cancellation, exit classification, hostile output, offline behavior, shutdown
cleanup, and 1/10/50-session resource proof. Each row freezes its expected
safe outcome rather than only naming a case.

Schema 3 retains schema 2's hermetic rules for how later native cases must run: an ephemeral
loopback OpenSSH server, no Internet or cloud account, a private per-case
workspace, disposable credentials, isolated `known_hosts` and agent state,
fixed recorded randomness, bounded readiness probes with no arbitrary sleeps,
and explicit readiness/connect/cancel/force-close/shutdown/case timeouts.
Cancellation must be exercised during DNS, connect, and authentication. Every
case must finish with no owned child, PTY, listener, tunnel, route, or temporary
secret file and must record platform/architecture, OpenSSH versions, fixture
hashes, duration, peak resource counts, lifecycle counts, and redaction-canary
results in a private bounded redacted artifact manifest.

The manual path is frozen separately: typing `ssh host` remains owned by the
interactive PowerShell, CMD, Bash, Zsh, or WSL shell. Managed launch is
additive, never downloads or installs OpenSSH during startup or launch, and a
missing client must yield redacted platform installation guidance without
substitution. Windows, macOS, and Linux require their scenario outcome before
managed activation; WSL managed launch remains denied until its own native
scenario outcomes pass.

The same contract keeps process, PTY, network, provider, authentication,
key-custody, and renderer authority false. Strict host-key checking is
preserved; new trust is explicit, changed keys fail, agent/TCP/X11 forwarding
and remote commands default off, listener scope is loopback unless separately
confirmed, discovery runs no config command, and inherited environment,
secrets, and shell evaluation remain unavailable.

This matrix is a definition, not a claim that native sessions ran. F4/F5 own
the process/PTY implementation and execution evidence. The contract status
therefore remains
`local-managed-session-source-complete-protected-activation-pending`.


Run the focused contract with:
## Verification

```text
python tools/ci/check_session_launch_d0.py
python tools/ci/test_session_launch_d0.py
cargo test -p automexia-extension-api --lib --locked
cargo test -p automexia-terminal --bin automexia --locked context::launch_broker::tests
cargo xtask verify architecture
```

The suite covers schema-1/schema-2 immutability, schema-3 contract mutation, hard
production denial, exact package digest source/size/version/contract/
verification matching, manual-path preservation, nine trust boundaries,
hermetic fixture and evidence rules, four-platform fixed roots,
principal/capability/decision scope, future/expired/denied decisions, capsule
registration/rebind, replay and nonce exhaustion, broad-spawn and shell/WSL
rejection, leading-dash and extra-argument rejection, literal native argv,
fail-closed resolution, file replacement, cwd/environment limits, redaction,
revocation, cancellation, stale leases, cross-scope isolation, and cleanup
after 1/10/50 pure lifecycle cycles.

## Remaining activation gates

This phase is not complete as a shipped feature. Before changing the activation
constant or verified-principal construction, maintainers must:

1. obtain the two independent exact-head protected-path approvals required by
   ADR 0003 and non-bypassable server-side enforcement;
2. pass inherited S0/v0.4, hosted CI, CodeQL, and required native jobs on that
   exact revision;
3. bind the real package loader's digest/signature, publisher, exact compatible
   version/contract, and live revocation result before constructing a verified
   principal;
4. add current executable and identity observation to the Connection Review and
   invalidate stale approval when either changes;
5. add native Windows, macOS, Linux, and separately gated WSL OpenSSH
   spawn/cancel/teardown/hostile-output evidence, including PID reuse and
   application close;
6. complete graceful-then-forced descendant cleanup, durable redacted completion
   audit, and listener/tunnel reconciliation;
7. finish controlled native pixels, keyboard/focus, screen-reader, redaction,
   and manual before/after `ssh` validation;
8. pass controlled 1/10/50-session process, PTY, renderer, latency, resource,
   and leak gates. Model and guarded-seam tests do not replace those native
   measurements.

No roadmap or test result may describe managed SSH/session launch as available
until every activation gate passes.
