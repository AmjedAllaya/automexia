# Remote connections

Automexia separates **what works in a normal terminal today** from the **managed SSH/Connection Hub product being built for v0.5**. This distinction is important: the repository contains strong data models and security foundations that deliberately have no process or network authority yet.

## Current product boundary

**Available now:** users can run `ssh`, `mosh`, cloud CLIs, `kubectl`, `oc`, and other tools normally from their shell. The PTY treats remote output as untrusted terminal bytes exactly like local process output.

**Implemented internally, not activated as managed SSH:** a bounded static
OpenSSH inventory package, exact-grant/session-launch review models, connection/profile/recipe records, authentication/result reducers, deterministic dry-run planning, renderer-independent Connection Hub/review/planner view models, bounded catalog composition, and a private transactional Connection Library.

**Planned:** production Connection Hub UI integration, reviewed system-OpenSSH launch, tunnels/routes, provider authentication and multi-cloud adapters, remote-file/session-memory/collaboration features, and later ecosystem/AI features.

## Internal Connection Library boundary

The source tree contains an internal, versioned Connection Library store for
profiles, recipes, and preferences. It is bounded, compare-and-swap protected,
atomically replaced, recoverable only through an explicit reviewed operation,
and designed to persist public metadata plus opaque credential references—not
passwords, private keys, tokens, or provider credentials.

The current application root does not initialize this store and no supported UI,
CLI, or configuration key exposes it. Its internal file name and schema are not
a user-editing contract: do not create or edit a `library.v1.json` file to try
to enable Connection Hub. Redacted export deliberately removes credential
references, private local state, and last-used timestamps; import validates all
values and assigns fresh identifiers.

This internal persistence does not change today's workflow: system OpenSSH and
its normal configuration remain the only supported connection path.

## Product model

The managed experience is built around a small set of explicit records rather than hidden shell commands:

- A **Connection** identifies a target and public display metadata. It can refer to an OpenSSH alias or an explicitly modeled host/user/port/route.
- A **Profile** captures reusable non-secret connection policy such as transport, working directory, environment/capsule selection, tags, and safe defaults.
- A **Recipe** is an ordered set of typed stages that can prepare, connect, initialize, verify, or recover. Each action has a risk class and explicit failure policy.
- An **Environment Capsule** is immutable, session-scoped, and non-secret. It identifies the selected environment/provider context without becoming credential storage.
- A **Connection Review** is the final value-redacted view of resolved target, route, executable, argv, capabilities, host-trust state, and recipe stages before authority is granted.

Secrets remain with established authorities such as OpenSSH agents/keychains, cloud CLI credential stores, Kubernetes configuration, hardware keys, organization identity systems, or later dedicated brokers. Automexia stores references and public metadata, not a second password/token vault.

## OpenSSH inventory

This page documents the nonactivated D4 OpenSSH inventory package. It is a
contributor-facing foundation for the future first-party SSH experience, not a
claim that Automexia can currently start managed SSH connections.

## Using the inventory package

The private workspace package is automexia-devops-ssh at
extensions/devops-ssh. A host supplies one or more InventoryGrant values. Each
grant has:

- a short public diagnostic label;
- one canonical root;
- one or more exact entry files under that root; and
- a user or system ownership policy.

The host calls scan_inventory with explicit InventoryLimits and a generation.
The public values may lower the project limits for a narrower deployment but
cannot raise any architecture ceiling. For asynchronous refresh, the host uses
scan_inventory_cancellable with the cancellation handle issued by
RefreshCoordinator; requesting a newer generation actively stops obsolete
parsing as well as rejecting any late result.
On success it receives concrete aliases, public host/user/port hints, an opaque
identity kind, exact observed files, bounded diagnostics, and accounting for
files and bytes. A WatchPlan can be constructed only from those opaque
scanner-observed files, optionally plus the exact existing MetadataStore file;
callers cannot create a plan from arbitrary paths.

The package is disabled by default. It is not linked to the renderer, input
path, PTY transport, or production launch broker. It has no session-launch,
process-spawn, network, clipboard, environment, terminal-output, or overlay
capability.

Automexia-owned labels, tags, favorites, and recent-use timestamps are stored
only in the primary and single recovery files:

    <Automexia config root>/extensions/devops-ssh/connections.v1.json
    <Automexia config root>/extensions/devops-ssh/connections.previous.v1.json

MetadataStore creates that exact extension root, validates schema version 1,
defaults legacy documents to revision zero, and serializes under the fixed
8 MiB ceiling before acquiring its private writer lock. Compare-and-swap writes
require the reviewed revision, advance it without overflow, retain exactly one
validated previous generation, synchronize same-directory staged files, and
atomically replace the destination with user-only permissions. A malformed
primary is never silently overwritten: reads return the validated previous
generation with explicit recovery origin, while restoration requires the exact
reviewed previous revision and advances it.

Removing owned state deletes both metadata generations. A zero-value private
coordination lock may remain and contains no connection data. The store never
edits OpenSSH configuration, known_hosts, agents, certificates, or keys.

## Security and resource contract

The default ceilings are mandatory architecture invariants:

| Resource | Limit |
|---|---:|
| One OpenSSH file | 1 MiB |
| Aggregate source bytes | 8 MiB |
| Source files | 128 |
| Include depth | 8 |
| Concrete aliases | 10,000 |
| One displayed value | 4 KiB |
| One input line | 16 KiB |
| Automexia metadata document | 8 MiB |
| Metadata tags per connection | 32 |

Every entry and include is canonicalized and must remain under its exact grant.
Symbolic links and Windows reparse points are rejected. Reads use no-follow
opens, bounded allocation, pre/open/post identity and modification checks, and
fail if a source is replaced or changes during the read. Unix sources must have
the expected user/root
owner and cannot be group- or world-writable. Automexia metadata uses mode
0700 for its directory and 0600 for its file on Unix. Windows uses a protected
DACL containing only the current user.

The parser indexes Host, HostName, User, Port, ProxyJump, IdentityFile,
CertificateFile, PKCS11Provider, and SecurityKeyProvider only as static public
hints. It never reads key bytes. Wildcard, negated, token-expanded, or
command-expanded aliases are not connectable records.

Match blocks are excluded. ProxyCommand, LocalCommand, and RemoteCommand are
reported but never evaluated. Dynamic Include tokens, shell expansion,
command substitution, ssh -G, process creation, DNS, sockets, and network
traffic are outside this package.

Errors contain only the public grant label, optional line number, error class,
and bounded explanation. They never echo the source path, source line,
identity path, secret, or malformed JSON content. A failed refresh retains the
last-known-good snapshot and publishes stale status. Adjacent changes are
debounced and coalesced; obsolete generations cannot replace newer state.
Periodic reconciliation covers missed filesystem notifications.

## Why Automexia does not resolve effective SSH configuration

OpenSSH configuration is conditional and first-value-wins. Include can expand
tokens and patterns, Match can inspect runtime facts or execute a command, and
ProxyCommand can invoke arbitrary programs. Running OpenSSH configuration
evaluation in a background indexer would quietly turn a read-only UI feature
into a code-execution and network-adjacent boundary.

Automexia therefore uses an intentionally incomplete static inventory. It is
optimized for safe discovery, not semantic equivalence. A future explicitly
authorized connection will give a concrete alias to the system OpenSSH client,
which remains the execution and precedence authority. This also preserves
existing agents, keychains, hardware tokens, certificates, host-key behavior,
and user configuration without creating an Automexia secret vault.

The future launch path remains blocked by proposed ADR 0012, package identity,
visible grants, atomic check-to-spawn, native lifecycle evidence, and the D5
release gates. D4 does not bypass any of those decisions.


## Connection Hub workflow

> **User-facing status:** Connection Hub is not currently exposed as a window, menu, shortcut, CLI command, or configuration surface. Users should use normal system OpenSSH today. See [Connection Hub and SSH](../user-guide/connection-hub-and-ssh.md) for current setup instructions.

The Connection Hub is the planned renderer-neutral product surface for discovering, reviewing, and launching managed sessions. The same semantic model must work in wide, compact, high-DPI, and keyboard/screen-reader layouts; visual pixels are not the source of truth.

A typical journey is:

1. **Discover** bounded local configuration (for example concrete OpenSSH aliases) without executing configuration directives or contacting the network.
2. **Select** a connection or create a new explicit record.
3. **Explain** where each effective public value came from and whether any source is stale, unavailable, or conflicted.
4. **Review** executable identity, exact arguments, route/tunnel intent, capabilities, host-trust state, and any recipe stages.
5. **Approve** only the capability needed for that launch. Revocation and denial are first-class outcomes.
6. **Launch** through the single application-owned broker using the canonical system OpenSSH executable; OpenSSH itself continues to own its full configuration, agent, key selection, host-key handling, and protocol implementation.
7. **Observe** immutable session-scoped context contributions. One session cannot publish provider/context state into another.

The Hub is designed to degrade gracefully. Missing tools or provider authentication should produce a truthful `unavailable`, `stale`, or `error` state rather than blocking unrelated terminal sessions.

## Exact launch boundary

Managed process launch is intentionally narrower than general process-spawn authority. A launch request identifies an approved first-party publisher/capability, an expected executable kind, exact argv, a bounded environment, a validated working directory, and audit/session identifiers. The broker resolves and verifies the executable according to platform policy, checks grants and current file identity, launches without shell interpretation, associates the PTY with one route/session, and records a redacted result.

No background inventory worker may cross this boundary. Static SSH parsing, Connection Hub rendering, dry-run planning, and provider discovery remain non-executing.

## Profiles and automation recipes

Recipes exist to make common workflows explicit rather than hiding them in `.bashrc` fragments or opaque one-liners. Useful stages include:

- local preparation or capsule selection;
- reviewed connection establishment;
- remote working-directory selection;
- explicit remote initialization commands;
- verification/health checks;
- typed tunnel or route setup;
- reconnect/retry behavior that is safe for the stage's risk class.

Risk classification matters. Read-only inspection can be retried more freely than state-changing remote operations. A failure policy must say whether to stop, retry, continue, or require review; the product must not silently replay a destructive stage after reconnect.

## Terminal-first interaction grammar

Managed remote operations are intended to be usable without a screen-heavy control center. The same typed registry can surface actions through commands, a leader-key/picker flow, the command palette, and accessible structured overlays. The terminal remains the primary workspace; the Hub is for discovery, explanation, review, and state that genuinely benefits from structure.

This approach also keeps external tools authoritative. OpenSSH remains the SSH implementation. Provider CLIs remain the first choice for provider authentication/configuration. Kubernetes/OpenShift tooling remains authoritative for cluster contexts. Mature external tools are wrapped or adopted when they already solve a protocol/security problem well; Automexia owns the product policy, UI model, session isolation, and one capability/process boundary.

## Security invariants

- Remote output, hostnames, configuration text, discovered aliases, provider metadata, and workspace state are untrusted.
- Background discovery does not evaluate `Match exec`, `ProxyCommand`, `LocalCommand`, command substitution, shell expansion, or `ssh -G`.
- Includes, files, symlinks/reparse points, depth/count/byte totals, permissions, and path canonicalization are bounded and validated.
- No credential or secret is rendered in Connection Hub state, receipts, logs, or persistent connection records.
- Host-key decisions are explicit; the product must not replace OpenSSH trust policy with an invisible "accept all" path.
- Process creation uses exact arguments and one reviewed broker, never a provider-specific shell string.
- Session/provider contributions are immutable and keyed to the owning session; stale work cannot overwrite a newer session generation.
- Reconnect, retry, and automation never convert a previously reviewed low-risk operation into an unreviewed high-risk one.

## Provider and multi-cloud direction

After production system-OpenSSH launch is proven, separately enabled provider adapters can contribute authentication/availability state, environment capsule metadata, cluster/cloud context, and bounded inventory. Provider-neutral core code must not import provider SDKs or credentials. Official CLIs and local configuration are preferred first; a later out-of-process adapter host is reserved for cases that genuinely need SDK/API access.

Provider work is sequenced independently so AWS, Azure, Google Cloud, Kubernetes, OpenShift, Teleport, OpenBao, or future adapters can fail, be disabled, or be removed without destabilizing the terminal core.

## What is not shipped v0.4 behavior

Saved managed hosts, one-click cloud authentication, password/secret custody, automated tunnels, remote file browsers, broadcast workspaces, shared live PTYs, searchable session memory, public extension downloads, and AI execution are roadmap items. Do not infer them from internal record types or planning tests. The current phase status is in [Roadmap](../project/roadmap.md).
