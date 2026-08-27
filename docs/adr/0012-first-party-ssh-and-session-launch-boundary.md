# ADR 0012: First-party SSH and scoped session-launch boundary

- Status: Accepted by the project owner on 2026-08-22; production activation remains blocked by ADR 0003's two independent exact-head approvals and the evidence below
- Date: 2026-08-13
- Replaces: ADR 0003 only for the narrowly scoped first-party capability below

## Context

Automexia is a generic terminal, but its DevOps distribution needs managed SSH
earlier than the v0.6 public extension ecosystem. Manual `ssh host` already
works through the user's shell and PTY. Saved hosts, quick connect, jump hosts,
tunnels, agent/certificate state, and cloud-native transports require a
reviewed way for an extension to request a new interactive process/session.

ADR 0003 intentionally limits v0.4 extensions to reviewed local reads and says
that process or network authority requires a replacement ADR. Adding an SSH
protocol engine, credential vault, provider SDKs, or Electron/Termix application
to core would expand the most trusted boundary and contradict the standalone
terminal architecture. Waiting for a complete third-party sandbox would delay a
useful first-party extension unnecessarily.

Remote output also crosses the existing untrusted PTY parser. The S0 bounded
OSC/APC/DCS/XTGETTCAP gate must therefore close before managed remote sessions
are exposed.

## Decision

1. v0.4 keeps ADR 0003 unchanged. Design, pure types, static fixtures, and
   non-activated tests may proceed, but no managed process capability is
   exposed before v0.4's hostile-output and release gates pass.
2. v0.5.0 may add a generic, application-owned `session.launch` capability.
   The extension submits a versioned `LaunchRequest`; the application remains
   the only component that resolves the executable, constructs trusted child
   state, creates a PTY/route/session, starts/cancels the process, and tears down
   owned resources.
3. A request contains extension/publisher/version, operation/session/capsule
   scope, an `ExecutableId`, ordered argv, allowlisted public environment
   deltas, validated working-directory reference, interactive-PTY intent,
   reason, and risk. It contains no command string, PTY/process handle, inherited
   environment, key, passphrase, token, agent message, or recovered secret.
4. The application maps `ExecutableId` to a canonical absolute path, never
   searches the current directory, revalidates file identity before spawn,
   bounds arguments/environment, rejects NUL and option-confused destination
   data, and never invokes `sh -c`, `cmd /c`, PowerShell evaluation, or an
   equivalent shell parser for extension input.
5. v0.5.0 grants this capability only to repository-reviewed or first-party
   signed operations in `devops-ssh`, and only for exact OpenSSH executable
   identities actually used by shipped features. Wildcard executable grants,
   arbitrary child processes, third-party process access, and background
   package downloads remain denied.
6. The first SSH engine is the system OpenSSH client running in a normal
   Automexia PTY. OpenSSH owns complete config semantics, `known_hosts`,
   agents, encrypted key files, hardware tokens, SSH certificates, jump hosts,
   authentication prompts, forwarding, and the remote network connection.
7. `devops-ssh` may statically index a bounded, non-executable subset of
   granted OpenSSH config for UI. It never background-evaluates `Match exec`,
   `ProxyCommand`, `LocalCommand`, command substitution, shell expansion, or
   `ssh -G`. Actual user-started OpenSSH execution remains authoritative.
8. The extension has no direct-network and no raw-secret capability in v0.5.0.
   It stores only versioned public connection metadata and opaque references.
   Strict host-key checking remains enabled and agent forwarding is off by
   default.
9. Every managed PTY owns an immutable, non-secret Environment Capsule.
   Capsules and extension caches are scoped to an exact session/revision.
   Clones copy intent into a new session and perform fresh resolution; rebinding
   one pane never mutates another.
10. v0.5.1 provider extensions may reuse the exact-argv capability for official
    AWS, Azure, Google Cloud, Kubernetes, and OpenShift CLI flows after separate
    executable/argument/policy review. Direct SDK networking, browser callbacks,
    secret handles, an out-of-process provider host, SFTP writes, and a native
    SSH engine each require their own reviewed capability extension.
11. Third-party and AI extensions receive no process, network, secret, agent,
    terminal-history, inherited-environment, capsule, or production authority
    from this ADR. Their sandbox/distribution model remains v0.6 or later.

## Required acceptance evidence

- S0 bounded control strings and all inherited v0.4 release gates pass.
- Architecture checks keep the API/model crates provider-, renderer-, PTY-, and
  GPU-independent and keep process-to-PTY attachment application-owned.
- Property and native tests prove exact argv on Windows/macOS/Linux with
  whitespace, Unicode, leading dashes, metacharacters, long values, and hostile
  aliases.
- Capability denial, allow-once/persisted grant if supported, manifest/publisher
  change, revocation, cancellation, PID reuse, session close, and application
  close are deterministic and leak no child/listener.
- First-use/changed host keys, agents, encrypted keys, certificates, hardware
  keys where available, ProxyJump, every tunnel type, offline/failure states,
  and hostile remote output have deterministic plus native evidence.
- Redaction canaries prove secrets are absent from config, extension state,
  logs, renderer snapshots, diagnostics, crash/QA bundles, telemetry, clipboard
  history, and AI surfaces.
- Input, rendering, VT parsing, and resize latency remain within established
  ratchets while indexing, connecting, cancelling, and running 1/10/50 managed
  sessions.

## Non-activated implementation evidence

As of 2026-08-22, the active local D0/D3/M5 contract is
`tests/fixtures/session-launch/d0-d3-contract-v5.json`; schemas 1-4 remain
byte-for-byte hash-checked history. Schema 5 retains their manual/fixture,
package/grant/audit/trust-boundary, and four-platform rules and adds the exact
M5 tunnel/lifecycle plus 23-scenario source/build/fixture/security/resource
native-evidence ratchet.

It also records nine trust boundaries with accepted/returned data, limits,
cancellation, logging, and failure ownership plus a hermetic fixture protocol:
loopback-only server, isolated disposable authentication state, bounded probes
and lifecycle timeouts, DNS/connect/auth cancellation, hostile argument corpus,
platform activation semantics, cleanup invariants, evidence metadata, private
artifact policy, and nine redaction surfaces. Dedicated checks reject drift.

The production-compiled broker remains a compile-time hard denial. Its review
harness binds a verified principal to an exact reviewed package policy including
digest, version, contract version, and repository-reviewed or first-party-signed
proof. It separates Windows, macOS, Linux, and disabled WSL resolution, retains
native file-identity revalidation, and keeps exact argv, bounded environment/cwd,
expiring decision, session/capsule, replay, revocation, lifecycle, and
redacted-audit controls. The linked first-party candidate is deliberately
unverified until a real loader supplies attestation and revocation evidence.

As of 2026-08-22, one application-owned ExternalToolRunner is shared by every
window. It caps active operations at 50, retains 256 redacted audit records,
binds route ID to session ID, rejects completion before publication, and
reconciles cancellation on route/application shutdown. ContextManager alone
consumes the guarded executable, creates the exact platform PTY, inserts the new
route before marking the lease published, and reconciles natural completion.
The protected activation constant remains false, so this production code cannot
resolve OpenSSH or create a managed child.

As of 2026-08-22, schema 5 retains immutable schemas 1-4 and adds M5 to
the nonactivated M3/M4 source ratchet. Direct argv uses 17 fixed defensive options plus optional exact `-l`/`-p`
values and one host; config-routed argv uses the 15-option subset plus one
canonical bounded `-J` chain and one alias. Both are bound to a fresh full-review
equality check and current native executable identity. The broker
rejects option/order/count or file-identity drift. ContextManager remains the
only PTY/route owner, application child exit supplies the real result, and
route close no longer implies success. Fixed redacted notifications and a
private atomic 256-record/2-MiB receipt store cover success, failure, unavailable
status, cancel, revoke, close, and shutdown. The connection-owned private-filesystem
adapter keeps D5.2 independent from Quick Actions while enforcing no-follow opens,
stable Windows handle identity, link/reparse rejection, private Windows DACL or
Unix mode checks, bounded snapshots, atomic replacement, and directory
synchronization. Opaque reconnect identity must match current D4 source and always returns to
fresh review and approval. M4 binds complete public host-key algorithm/SHA-256
and public identity evidence; changed keys cannot bind, `known_hosts` is never
mutated, the `C` copy handoff cannot execute or add Enter, and the exact bounded
`ssh-add -l -E sha256` request/parser remains nonactivated with no secret custody.

M5 retains those M3/M4 grammars and adds a distinct reviewed typed-tunnel
grammar. Tunnel requests load no configuration (`-F none`), use exact
`-L`/`-R`/`-D` arguments, default local/dynamic binds to
`127.0.0.1`, and reject config aliases or jump routes. Remote, non-loopback, or
production forwarding requires a fresh Allow-once decision; session grants are
disabled. OpenSSH remains the only prospective listener owner. A bounded
session/generation/tunnel lifecycle rejects stale owner events and terminal
reversal and closes every nonterminal state with its route lease. Active schema
5 preserves immutable schemas 1-4 and binds these rules plus the exact redacted
native-manifest contract. Synthetic fixtures test the validator but cannot
satisfy release evidence; controlled real Windows/macOS/Linux runs remain
external.

Automexia deliberately leaves OpenSSH key-exchange defaults and weak-crypto
warnings untouched. User configuration may execute OpenSSH-owned helpers during
launch, so real descendant-tree cleanup and native resource evidence remain
activation gates. The source implementation does not satisfy package
attestation, protected approval, or native execution requirements.
The F2/D5.0 capability-free baseline is also implemented as of 2026-08-17.
`automexia-devops::connections` contains bounded public-only records, strict
validation, exhaustive state reducers, canonical approval fingerprints, and a
deterministic dry-run planner. `automexia-ui-model::connection_hub` contains
pure responsive Hub/review/planner and accessibility projections. Frozen
all-provider/all-auth fixtures, hostile/property/mutation tests, fuzz ownership,
and a 64-step benchmark are registered. Every process, network, provider,
credential, PTY, listener, renderer, and GPU authority remains absent or false.
As of 2026-08-22, M3/M4 also have a production-reachable but nonactivated
preparation route. The application maps a current D4 direct/config-jump record
or transient typed host/user/port into a stable public profile and canonical
pending F2 plan. Inventory generation and
metadata revision bind the preparation; session.launch is the only requested
capability; no current executable identity exists yet; every resolved-plan
authority remains false. The Connection Hub projects bounded public decisions into
responsive Connection/Safety/Launch groups. Its deny, allow-once, and
allow-session approval actions are pointer- and mnemonic-accessible, but
execution remains false and every attempt stops at the protected broker denial.
Exact aliases, opaque references, executable digests, and fingerprints do not
enter presentation state. Selection, route, runtime-state, generation, catalog, and
metadata changes discard or rebuild the preparation. The existing
identity-bound pure review remains the later consumer of M2-provided current
executable, identity-observation, and host-trust evidence. No process, PTY, network, credential, listener, host-trust mutation, or secret
authority is reachable while the activation gate is false.

The project owner's acceptance of this ADR records the architectural decision;
it does not enable the capability or satisfy ADR 0003's protected-path rule.
Remaining activation evidence includes two independent exact-head approvals;
green hosted S0/release/CodeQL gates; real package-loader
attestation/revocation; current executable and identity observation; execution
of the native matrix on Windows/Linux/macOS/WSL; redaction across every listed
surface; and controlled 1/10/50-session process, PTY, renderer, performance, and
leak results. Exact limits and commands are in
the [broker contract](../SESSION-LAUNCH-BROKER.md).

## Consequences

Automexia can ship production SSH in v0.5.0 without making SSH a core feature or
waiting for the public extension ecosystem. Users keep their existing OpenSSH
configuration, host-key policy, agents, certificates, and hardware. The core
gains one reusable but narrowly authorized session primitive.

The first release does not provide direct extension networking, a credential
vault, a native SSH protocol stack, embedded Termix, unrestricted subprocesses,
structured SFTP, or third-party process authority. Those exclusions reduce
attack surface and make the early release reviewable. They may be revisited only
through explicit later ADRs and the gates in the
[early DevOps and SSH delivery track](../STABILIZATION-ROADMAP.md#early-devops-and-ssh-delivery-track).

The application-owned approval and user-journey consequences of this boundary
are specified in [Connection Hub](../CONNECTION-HUB.md). Its renderer-neutral
review/state model, externally owned credential recovery warning, and native
evidence are required activation work; this document does not by itself enable managed
launch or broaden its authority.

## 2026-08-26 ownership amendment

At the time of this amendment, the active contract became
`tests/fixtures/session-launch/d0-d3-contract-v6.json`. Schema 6 ratchets the
exact schema-5 digest and changes only current source/evidence ownership paths
from the former mixed DevOps package to `automexia-connectivity`. It does not
change launch grammar, trust decisions, tunnel lifecycle, activation, authority,
or native release requirements. Schemas 1-5 remain byte-for-byte hash-checked
history. ADR 0035 owns the placement decision.

## 2026-08-27 assurance amendment

The active contract is now
`tests/fixtures/session-launch/d0-d3-contract-v7.json`. Schema 7 preserves
schemas 1-6 by exact digest and does not enable launch or widen authority. It
closes two lifecycle evidence gaps: review request identifiers permanently fail
closed before wraparound or reuse, and every reviewed tunnel produces one
generated operation-scoped opaque receipt ownership reference without carrying
the user tunnel ID, listener, target endpoint, destination, or terminal text.

The native evidence manifest is schema 2. It binds the clean source commit,
native platform/architecture, exact `automexia --version`, application binary
and package, fixed `ssh`, `sshd`, `ssh-add`, and `ssh-keygen` hashes, plus
identity-stable advisory-review and package-provenance artifacts. The security
ratchet names the official OpenSSH 10.5 baseline dated 2026-08-11 and requires
evidence for its locked-agent/session-bind and pending remote-forward cleanup
fixes in addition to post-quantum key exchange, weak-crypto warnings, and
restricted-key session binding. Version strings are not treated as proof of
patch state because supported vendors may backport fixes; exact tool and
provenance identities are required instead.

The protected manual workflow now takes evidence paths only from environment
secrets, runs the D0 checker and both mutation suites on Windows and Unix,
checks every command result, uses exact runner/concurrency/timeout/checkout
semantics, and uploads one path-free 90-day summary. Production activation
remains false. ADR 0003 exact-head approvals, package-loader
attestation/revocation, and controlled Windows/macOS/Linux OpenSSH, process,
resource, visual, and accessibility evidence remain external prerequisites.
