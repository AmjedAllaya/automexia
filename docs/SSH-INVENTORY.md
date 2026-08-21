# OpenSSH inventory and Automexia metadata

This page documents the D4 OpenSSH inventory package. D5.1 activates it only
through an explicit read-only Connection Hub grant/review flow; it remains
non-executing and does not claim that Automexia can start managed SSH sessions.

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

The package remains incapable of rendering or launching. The D5.1 application
service calls it only on a joined background worker after exact file review;
the Screen and renderer consume immutable public projections. D4 has no session-
launch, process-spawn, network, clipboard, environment, terminal-output, PTY,
or overlay capability.

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

## Testing and performance

Run the focused gate:

    cargo test -p automexia-devops-ssh --all-targets --locked
    cargo clippy -p automexia-devops-ssh --all-targets -- -D warnings
    cargo bench -p automexia-devops-ssh --bench openssh_inventory -- --noplot

Named controlled hardware runs the same benchmark through
AUTOMEXIA_QA_BENCHMARKS=1 cargo xtask qa --full. Nightly compilation and the
controlled QA contract both fail if this benchmark becomes orphaned.

The unit/property suite covers immutable maximum ceilings, bounded entry
grants, race-resistant no-follow reads, active obsolete-generation
cancellation, scanner-derived watcher provenance, bounded serialization,
concrete versus wildcard aliases, first-value behavior, lexical includes,
cycles, out-of-grant and dynamic includes, oversized and malformed input,
redacted diagnostics, Unix permissions, Windows DACL round trips, atomic
replacement, interrupted staging, revision-zero migration, CAS stale-writer
rejection, writer contention, single-generation rotation, truthful fallback,
explicit recovery, exact removal, event filtering, refresh coalescing, stale
inventory recovery, and arbitrary byte input. Nightly runs the
openssh_inventory libFuzzer target with explicit time and RSS limits. The
benchmark parses the maximum 10,000 concrete aliases.

Native Windows, Linux, and macOS workspace jobs compile and execute the same
portable package. Unix permission behavior is executed on Linux and macOS;
the Windows job executes current-user DACL persistence. Controlled watcher
behavior on unusual network filesystems remains advisory because native event
delivery can be incomplete; periodic reconciliation is the correctness path.

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
