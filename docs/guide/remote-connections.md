# Remote connections

Automexia separates **normal shell connections**, the **read-only D5.1 Connection Hub plus source-complete, nonactivated F5.1 direct lifecycle for v0.5 source builds**, and the still-gated managed SSH/provider authority. The approval surface deliberately has no reachable process or network authority.

## Current product boundary

**Available now:** users can run `ssh`, `mosh`, cloud CLIs, `kubectl`, `oc`, and other tools normally from their shell. The PTY treats remote output as untrusted terminal bytes exactly like local process output.

**Implemented locally / release-gated:** the read-only product Hub composes the
bounded static OpenSSH inventory after exact file review, exposes public
favorites/tags and the private library snapshot, and turns one selected or
literal target into a canonical plan and actionable Connection Review. Allow
once, allow for session, and deny exercise the fail-closed policy boundary. The
activation gate is false and the linked package is unverified, so no managed
process, PTY, network connection, or provider authority is reachable.

**Implemented but activation-gated:** exact direct/config-jump/tunnel argv,
typed host/user/port/endpoints, loopback defaults, strong tunnel confirmation,
full host-key/public-identity review, safe copy, full-review/executable binding,
guarded lifecycle, child outcomes, receipts, reconnect, and compact tunnel
state. **Planned/protected:** actual status/SSH execution, controlled real
F5.4 cleanup/resource/accessibility proof, provider authentication and
multi-cloud adapters, remote-file/session-memory/collaboration, and later
ecosystem/AI features.

## Internal Connection Library boundary

The source tree contains an internal schema-2 Connection Library store for
profiles, recipes, declarative workspaces, and preferences. It remains bounded,
private, compare-and-swap protected, atomically replaced, and recoverable only
through an explicit reviewed operation. It stores public metadata and opaque
references—not passwords, private keys, tokens, live PTYs/tunnels, or provider
credentials.

The application initializes this store once and exposes read-only profile,
recipe, workspace, and recovery/migration-preview state. Internal editor and
import/export APIs are review-first: a preview binds the complete document and
base revision; material recipe/profile/workspace changes advance dependent
revisions/fingerprints and clear approvals; stale or mismatched data fails
closed. Schema 1 is upgraded only in memory until a reviewed CAS writes schema
2. The established private `library.v1.json` filename is retained for atomic
recovery continuity and is not the schema authority or a user-editing contract.

Redacted export removes credential references, private state, last-used values,
workspace labels, and all connection bindings. Import validates bounds and
assigns fresh local IDs; imported topology must be rebound locally. No supported
product editor, CLI, or configuration key exposes this internal schema, and no
migration or import enables execution. System OpenSSH in the shell remains the
supported connection path.
## Product model

The managed experience is built around a small set of explicit records rather than hidden shell commands:

- A **Connection** identifies a target and public display metadata. It can refer to an OpenSSH alias or an explicitly modeled host/user/port/route.
- A **Profile** captures reusable non-secret connection policy such as transport, working directory, environment/capsule selection, tags, and safe defaults.
- A **Recipe** is an ordered set of typed stages that can prepare, connect, initialize, verify, or recover. Each action has a risk class and explicit failure policy.
- An **Environment Capsule** is immutable, session-scoped, and non-secret. It identifies the selected environment/provider context without becoming credential storage.
- A **Connection Review** is the final value-redacted view of resolved target, route, executable, argv, capabilities, host-trust state, and recipe stages before authority is granted.

Secrets remain with established authorities such as OpenSSH agents/keychains, cloud CLI credential stores, Kubernetes configuration, hardware keys, organization identity systems, or later dedicated brokers. Automexia stores references and public metadata, not a second password/token vault.

## OpenSSH inventory

D4 is the bounded OpenSSH inventory authority used by the read-only D5.1 Hub.
It remains non-executing and is not a claim that Automexia can start managed
SSH connections.

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

D5.1 calls the package only on its joined background worker after exact file
review and renders immutable public projections. D4 itself has no renderer,
session-launch, process-spawn, network, clipboard, environment, terminal-output,
PTY, or overlay capability.

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

ADR 0012 is accepted, and the application runner, executable guard, approval
surface, and PTY/route seam exist locally. Launch remains blocked by ADR 0003's
exact-head approvals/server enforcement, package attestation and live
revocation, a fresh current-executable review, native lifecycle/resource/
accessibility evidence, and the D5 release gates. D4 bypasses none of them.


## Connection Hub workflow

> **User-facing status:** v0.5 M1 source builds expose **Connection Hub
(read-only)** through the command palette. Users explicitly select and review
OpenSSH files, browse public inventory, and review favorite/tag changes. Connect,
Login, cloud refresh, recipe execution, and every process/network/PTY action are
disabled. See [Connection Hub and SSH](../user-guide/connection-hub-and-ssh.md).

The modal adapts the renderer-neutral semantic model into one topmost,
PTY-inert surface. Its implemented journey is:

1. **Open** without scanning or contacting any provider.
2. **Choose and review** exact local files through the parented native picker;
   cancellation or replacement revokes the memory-only grant.
3. **Confirm** a bounded background scan and retain last-known-good results when
   a later scan fails.
4. **Browse** with search, grouping, source/favorite/recent/tag filters,
   virtualization, and a public inspector.
5. **Prepare a supported direct alias** with Enter, or press `L` to enter one
   bounded literal host. The route-owned review groups public Connection,
   Safety, and Launch facts without exposing aliases, paths, digests, or argv.
6. **Decide explicitly:** choose Allow once (`A` or focused Enter), Allow for
   session (`S`), or Deny (`D`). Pointer targets and accessibility buttons expose
   the same three choices. In current builds, Allow reports that protected
   review is pending and starts nothing.
7. **Review and save** favorite/tag diffs using D4 revision CAS; reload on
   conflict and never write recent-use.
8. **Stop at the authority boundary:** the false activation gate and unverified
   package deny before executable resolution, so no command, PTY, login, or
   network request occurs.

D5.2 launch remains blocked until the M2 protected, attestation, fresh executable
review, native lifecycle/resource, and accessibility gates pass. An approval
attempt does not pre-authorize a later launch.
## Reviewed typed SSH tunnels (nonactivated)

M5 preparation can represent local, remote, and dynamic TCP forwarding without
reading SSH configuration. Local and dynamic binds default to
`127.0.0.1`; the review shows the exact listen and target endpoints, direction,
session lifetime, OpenSSH ownership, confirmation strength, and planned
lifecycle state. A tunnel request is configuration-free (`-F none`) and cannot
use a config alias or jump route, hidden forwarding, shell text, or arbitrary
OpenSSH option.

Remote, non-loopback, and production tunnels require a new Allow-once decision.
The Allow for session control is visibly disabled and cannot be activated with
`S`, focus, pointer, or accessibility action. Color reinforces state but the
icon and text always carry the same meaning. Planned, starting, ready, collision,
failed, cancelled, and closed are distinct owner-reported states; Automexia
never infers readiness from terminal output.

This is currently a review and test contract, not an available tunnel command.
Managed activation is false, no OpenSSH child or listener can start, and no SSH
file or service is changed. Use the system OpenSSH client manually until the
protected activation and real Windows/macOS/Linux F5.4 evidence gates pass.

## Exact launch boundary

Managed process launch is intentionally narrower than general process-spawn authority. A launch request identifies an approved first-party publisher/capability, an expected executable kind, exact argv, a bounded environment, a validated working directory, and audit/session identifiers. The broker resolves and verifies the executable according to platform policy, checks grants and current file identity, launches without shell interpretation, associates the PTY with one route/session, and records a redacted result.

No background inventory worker may cross this boundary. Static SSH parsing, Connection Hub rendering, dry-run planning, and provider discovery remain non-executing. Completion storage runs only after a terminal outcome on the bounded connection worker, retains at most 256 provider-neutral records/2 MiB, and excludes destination, terminal content, credentials, paths, environment, PID, and executable identity. Reconnect is never automatic and requires current source plus fresh review and approval.

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
