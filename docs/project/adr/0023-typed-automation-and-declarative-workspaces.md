# ADR 0023: Typed automation and declarative workspaces

- Status: Accepted for v0.5 M6/F6 on 2026-08-25
- Date: 2026-08-22
- Extends: [ADR 0012](0012-first-party-ssh-and-session-launch-boundary.md), [ADR 0018](0018-terminal-first-remote-operations.md), [ADR 0019](0019-acyclic-owned-crate-dependencies.md), and [ADR 0022](0022-read-only-connection-hub-activation.md)

## Context

M6 needs reusable connection recipes and multi-environment workspace intent
without granting a second owner process, PTY, network, credential, listener, or
renderer authority. Stored intent must remain reviewable after profiles,
recipes, layouts, or source revisions change. Remote OpenSSH commands are shell
text rather than a portable argv channel, so arbitrary command templates cannot
be treated like the typed local launch boundary.

## Decision

1. `automexia-devops::connections` owns pure typed recipe-run, lifecycle,
   remote-initialization, workspace, restore, and broadcast contracts. They are
   bounded, generation-scoped, deterministic, and always return execution
   disabled with an all-false authority ceiling.
2. The application Connection Library advances to schema 2 in the existing
   private filenames. Schema-1 data is upgraded in memory as an explicit
   migration preview and is not persisted until reviewed compare-and-swap.
3. Profile, recipe, and workspace edits are previews bound to the full document
   fingerprint. Material changes advance dependent entity revisions, refresh
   exact fingerprints, clear approvals, and fail on stale revision or dangling
   reference. Import/export is redacted, topology-only for workspaces, bounded,
   and assigns fresh local identifiers.
4. Recipes preserve the ten fixed lifecycle stages. Reviewed `--no-hooks`
   intent removes recipe-origin steps while retaining typed planner resolve and
   connect steps. Each step has a deadline, explicit failure behavior, bounded
   eligible retry with capped jitter/backoff, cancellation, and generation
   invalidation.
5. Remote initialization is limited to typed working-directory, public
   environment, `sudo`/`doas` user switch, and verification operations for an
   explicit POSIX-sh or PowerShell dialect. Privileged steps are revalidated at
   review and require confirmation every connection. No arbitrary code,
   template, prompt scraping, hidden key injection, or implicit Enter exists.
6. Workspaces persist only windows, pane split intent, and immutable connection
   bindings. Restore is a reviewed nonexecuting plan; it never restores a PTY,
   credential, live tunnel, automatic reconnect, or interrupted action.
7. Broadcast keeps the exact command transient and redacts debug/audit output.
   It requires an exact target preview, explicit time-bounded arming, separate
   production confirmation, no implicit Enter, per-target terminal results,
   failure isolation, cancellation, and generation checks.
8. CP3.3 trusted local workspace tasks remain insert-only and separate. M6 does
   not give them remote, provider, SSH, or broadcast authority.
9. Acceptance authorizes the bounded review/editor product surface and public
   preview-first CLI. Native execution and managed SSH adapters remain disabled
   until the existing ADR 0012/D3/M5 protected and native-evidence gates pass.

No dependency is added. The build/wrap/adopt decision is to build these small
policy/state contracts on the existing immutable models, reducers, hashing,
private filesystem adapter, and compare-and-swap store; system OpenSSH remains
the adopted protocol implementation. Official OpenSSH documentation confirms
that `RemoteCommand` is executed by the destination shell and warns against
casual agent forwarding ([`ssh_config(5)`](https://man.openbsd.org/ssh_config)).
Current upstream release notes identify OpenSSH 10.5 as the 2026-08-11 release
and document its security fixes ([release notes](https://www.openssh.org/releasenotes.html)).
Retry policy follows bounded exponential backoff with jitter rather than fixed
synchronized retries ([AWS Architecture Blog](https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/)).
The semantic UI state follows the W3C alert, modal-dialog, and switch patterns
([WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/)).

## Alternatives rejected

- Persist or replay live PTYs, tunnels, credentials, or interrupted commands.
- Compile free-form remote scripts or concatenate shell command strings.
- Infer readiness from terminal cells or inject hidden keys/Enter.
- Automatically reconnect or retry privileged, interactive, mutating, or
  cancellation-unsafe actions.
- Reuse the Quick Action store or trusted-workspace task authority as the
  Connection Library or remote automation owner.
- Add a workflow engine or embedded SSH dependency for pure bounded state.

## Verification

Local Windows x86_64 source evidence covers strict parsing, exact stage order,
privilege revalidation, deadlines/retry/cancel/generation behavior, no-hooks,
workspace cycles/rebind/clone/restore, broadcast arming/redaction/isolation,
1,000 repeated 50-target generations, schema-1 migration preview, atomic
dependent revisions, redacted topology transfer, CAS/recovery, semantic focus
restoration, mutation checks, fuzz entry points, and maximum-cardinality
Criterion cases. Exact commands and results are recorded in
[Testing](../../TESTING.md#m6-typed-automation-and-multi-environment-workspaces).

The accepted product boundary also has Windows x86_64 source evidence for the
preview-first `automexia workspaces` CLI, immutable worker-published library
snapshot, Connection Hub workspace catalog/review routes, mnemonic and pointer
navigation, bounded responsive geometry, semantic accessibility projections,
stale-library invalidation, no PTY input, and fail-closed activation blockers.

Real OpenSSH/PTY/process/socket/handle cleanup, macOS/Linux native execution,
controlled screen readers and visual inspection, and hosted release evidence
remain external. Cross-platform pure-model and renderer tests do not replace
those gates.

## Consequences

### Positive

- One immutable revision chain binds recipe, profile, workspace, and review.
- Multi-window intent is recoverable without persisting runtime authority.
- Remote initialization and broadcast have narrow auditable contracts and safe
  disabled behavior before activation.
- Core terminal input, PTY, renderer, resize, and startup hot paths gain no I/O.

### Trade-offs

- Users can manage and review workspaces from the CLI and Connection Hub, but cannot execute them until protected native activation passes.
- Schema-1 libraries require an explicit reviewed CAS before schema-2 storage.
- Workspace transfer intentionally drops connection bindings and private labels;
  users must rebind imported topology locally.
- Arbitrary scripts and automatic destructive recovery remain unavailable.

## 2026-08-26 placement amendment

Canonical [ADR 0035](../../adr/0035-core-domain-and-optional-extension-ownership.md)
moves the capability-free `connections` implementation to
`automexia-connectivity`; this ADR's typed automation, review, authority, and
lifecycle decision is unchanged.
