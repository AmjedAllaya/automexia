# ADR 0055: Session-local prompt discovery

Status: Accepted

Kubernetes prompt context belongs to the existing optional DevOps extension.
Optional shell home and configuration search paths are untrusted location
hints accepted by the bounded snapshot adapter. Bundled Bash, Zsh, Fish and
PowerShell hooks publish HOME and exported KUBECONFIG with a 4096-byte per-value
ceiling, explicit clearing, and a begin/commit marker around each pair. Encoded
frames are cached until values change and replayed after nested shells return.
These hints are not
credentials, authentication state, or permission to execute a command. The
application transports generic environment facts; only the extension interprets
the allowlisted names. No Kubernetes dependency enters the VT or PTY owner.

Generic session serialization excludes location hints, and session Debug output
redacts values. Only the application-owned helper transport opts in to the two
names. No persistence or upload is introduced. Native Windows discovery rejects
UNC and device paths so terminal metadata cannot request implicit network
authentication. `AUTOMEXIA_CONTEXT_PATH_HINTS=0` clears the pair. CMD uses the
application-inherited environment; its PROMPT mechanism cannot safely publish
dynamic encoded variables without a new per-prompt process or command hook.
The receiver discards a previous guest's hints once CMD restores its identity.

The former Windows-host fallback cannot represent a WSL session truthfully.
Guest-backed filesystem reads run in a short-lived copy of the application,
before logging, configuration migration, GUI or provider initialization. The
application owns the executable identity, request, output limit, deadline,
cancellation and child reaping. The helper reads bounded configuration documents
and returns only context and namespace. It never launches WSL, kubectl, a shell,
credential plugins or a network client. Failure cannot select the host cluster.
This is isolation of the host's existing filesystem-read implementation, not a
general process-launch capability granted to the extension or terminal output.

The already pinned serde-saphyr reader replaces order-sensitive text scanning.
Its explicit byte, node, depth, alias and document limits apply before projection.
Native first-source-wins semantics are intentionally distinct from the managed
import adapter's stricter review/collision policy. Only context/namespace fields
are retained; parser diagnostics and credential-bearing documents are not logged.

Refresh still uses one bounded worker, session revisions and stale-result checks.
Discovery identity excludes mutable titles, which are not provider inputs, while
retaining route, process, integration, directory, identity and path-hint isolation.
The capability-free UI model projects a validated shell username immediately.
Initial WSL discovery publishes available local fields through the same snapshot
owner before the guest read; an intermediate snapshot remains refreshing and
does not release the pending-operation latch. Existing snapshots are not replaced
by partial progress. The pending timer consumes progress and the one-shot route
wake is retained for completion. No new worker or persistent cache is needed.
The input, resize and rendering paths neither read configuration nor wait for the
helper. Disabling DevOps cancels outstanding work. No persistent cache, service,
new external executable, network credential or packaging sidecar is introduced.
Rollback is a normal revert; shell metadata is optional and safely ignored by an
older host. Native helper, shell mutation, parser, isolation, timeout, cancellation,
resource and benchmark evidence are separate from native pixels and screen readers.

Native shell adapters deliberately remain separate: export attributes, Unicode
length, prompt callbacks and encoding APIs differ. Existing GNU/BSD base64 is
reused only on changed values for Unix shells; PowerShell uses in-process UTF-8
encoding. Bash 3.2 needs a builtin subshell to inspect a nonempty override's export
attribute; modern Bash and Zsh inspect it without that fallback. This does not
introduce a new dependency, shared installer asset or shell-evaluation authority.
Zsh's `commands` array caused full PATH enumeration in cold encoding subshells.
Targeted `builtin whence -p` checks preserve executable lookup and user hash
options without preloading unrelated commands. A native sentinel test covers
the real encoder; startup and replay measurements remain separate.

References: [Kubernetes configuration merging](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/),
[serde-saphyr](https://docs.rs/serde-saphyr/1.2.0/serde_saphyr/), and
[Windows non-consuming pipe inspection](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-peeknamedpipe).

Microsoft documents cross-filesystem overhead in
[WSL filesystem guidance](https://learn.microsoft.com/en-us/windows/wsl/filesystems).
This informs isolation and staged publication; it is not a measurement of an
individual machine, and the implementation does not move the user's files.
See also the official [Zsh command table](https://zsh.sourceforge.io/Doc/Release/Zsh-Modules.html#The-zsh_002fparameter-Module)
and [command lookup](https://zsh.sourceforge.io/Doc/Release/Shell-Builtin-Commands.html#index-whence).
