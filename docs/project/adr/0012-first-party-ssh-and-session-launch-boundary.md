# ADR 0012: First-party SSH and scoped session-launch boundary

- Status: Proposed; acceptance is required before process-capable code is enabled
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

As of 2026-08-17, the active local D0/D3 contract is
`tests/fixtures/session-launch/d0-d3-contract-v2.json`; schema 1 is retained
byte-for-byte and hash-checked as historical evidence. Schema 2 records the
manual-shell and missing-client baseline, trusted-loader SHA-256 source/size,
package identity/version/contract/verification, grants/audits/defaults,
all-false authority ceiling, four-platform executable policy, and nineteen
native scenarios.

It also records nine trust boundaries with accepted/returned data, limits,
cancellation, logging, and failure ownership plus a hermetic fixture protocol:
loopback-only server, isolated disposable authentication state, bounded probes
and lifecycle timeouts, DNS/connect/auth cancellation, hostile argument corpus,
platform activation semantics, cleanup invariants, evidence metadata, private
artifact policy, and nine redaction surfaces. Dedicated checks reject drift.

The test-only broker now binds the verified principal to an exact reviewed
package policy including digest, version, contract version, and repository-
reviewed or first-party-signed proof. It also separates Windows, macOS, Linux,
and disabled WSL resolution, retains native file-identity revalidation, and
keeps the exact argv, bounded environment/cwd, expiring decision, session/
capsule, replay, revocation, lifecycle, and redacted-audit controls.

The F2/D5.0 capability-free baseline is also implemented as of 2026-08-17.
`automexia-devops::connections` contains bounded public-only records, strict
validation, exhaustive state reducers, canonical approval fingerprints, and a
deterministic dry-run planner. `automexia-ui-model::connection_hub` contains
pure responsive Hub/review/planner and accessibility projections. Frozen
all-provider/all-auth fixtures, hostile/property/mutation tests, fuzz ownership,
and a 64-step benchmark are registered. Every process, network, provider,
credential, PTY, listener, renderer, and GPU authority remains absent or false.
As of 2026-08-22, M3 also has a production-reachable but nonactivated
preparation route. The application maps one current direct D4 record into a
stable public profile and canonical pending F2 plan. Inventory generation and
metadata revision bind the preparation; session.launch is the only requested
capability; no executable identity exists yet; every runtime authority remains
false. The Connection Hub projects nine public decisions into responsive
Connection/Safety/Launch groups and keeps the primary action disabled. Exact
aliases, opaque references, executable digests, and fingerprints do not enter
presentation state. Selection, route, runtime-state, generation, catalog, and
metadata changes discard or rebuild the preparation. The existing
identity-bound pure review remains the later consumer of M2-provided current
executable, identity-observation, and host-trust evidence. No process, PTY,
filesystem, network, credential, listener, host-trust mutation, or secret
authority is enabled by this evidence.
This evidence does not accept this ADR and does not enable the capability. The
remaining acceptance evidence includes protected approval; binding a real
package-loader attestation/revocation result to the frozen policy; visible
decision/grant policy; atomic native check-to-spawn; application-owned process/
PTY/route binding; execution of the native matrix on Windows/Linux/macOS/WSL;
redaction across every listed surface; and controlled 1/10/50-session process,
PTY, renderer, performance, and leak results. Exact limits and commands are in
the [broker contract](../../developer/architecture.md).

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
[early DevOps and SSH delivery track](../roadmap.md#early-devops-and-ssh-delivery-track).

The application-owned approval and user-journey consequences of this boundary
are specified in [Connection Hub](../../guide/remote-connections.md). Its renderer-neutral
review/state model, externally owned credential recovery warning, and native
evidence are required acceptance work; the document does not activate this
proposed ADR or broaden its authority.
