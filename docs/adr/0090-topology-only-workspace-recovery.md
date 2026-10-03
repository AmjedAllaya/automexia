# ADR 0090: Topology-only workspace recovery

- Status: accepted
- Date: 2026-10-03
- Owners: Automexia maintainers

## Decision

The application coordinates an explicit startup recovery choice. ContextManager
remains the only terminal launch owner; ContextGrid converts its layout to and
from a bounded topology model. Restored sessions receive independent PTYs and
new runtime identities. Initial recovery UI uses a process-free placeholder.

`automexia::session_recovery` owns a separate version-1 snapshot under
`state/session-v1`. It contains window geometry, tab/split/local-tab topology,
selection, safe profile identities, working directories and explicit tab styling.
It contains no executable arguments, environment, terminal output, scrollback,
credentials, SSH destinations or live process state. UserPreferences is unchanged.

One existing BoundedWorker handles private storage and local directory checks.
The private_fs adapter supplies bounded no-follow reads, private permissions,
locking and durable temporary-file replacement. Preference and recovery schemas
and backup policies remain separate while sharing that replacement primitive.
An exclusive lifetime lock prevents competing application instances from restoring
or overwriting the same workspace. Future schemas remain untouched. A valid
previous snapshot can recover an interrupted primary write.

## Lifecycle and trust

The event loop captures bounded in-memory topology on its existing timer and
before final-window/application teardown. Saves coalesce; no disk or discovery
operation occurs during capture. Closing a recovery prompt preserves the previous
snapshot. Start clean replaces the previous topology. Restoring launches one
terminal per scheduled step and preserves the saved checkpoint until completion.
Late results cannot launch after cancellation or timeout. Shutdown hides windows
before its bounded disk flush; the shared worker retains retirement ownership.

Configured profiles resolve against current local configuration, including its
trusted interactive shell startup. Snapshots cannot supply those arguments.
Known shell identities use normal launch adapters. Remote scope metadata cannot
become a host, command, credential or automatic connection. SSH sessions reopen
locally with a reconnect notice; OpenSSH remains responsible for any subsequent
explicit connection and host-key/authentication interaction.

The model caps bytes, windows, terminals, nodes and depth and rejects unknown
fields, cycles and invalid selection/geometry. Per-profile exclusions prune both
capture and older snapshots. Native local CWDs are checked off-thread. Guest WSL
paths are structurally checked and passed as literal launch arguments; recovery
does not probe guests or remote filesystems. Native tab groups are restored as
independent windows. Offscreen geometry is clamped to the current display.

## Alternatives and evidence

Serializing a launch descriptor would preserve secrets and command authority.
Reviving live processes or restoring terminal cells would misrepresent running
state. Storing workspace data in UserPreferences would couple unrelated schemas
and write lifetimes. A second PTY/process owner is unnecessary.

Model, worker, storage and layout tests cover bounds, private ownership, backup
recovery, safe profiles, pruning, ratios and fresh identities. The Windows native
fixture exercises actual crash/restart, no child before consent, fresh shell
processes, normal close and input isolation. Linux/macOS desktop and native
screen-reader delivery require separate evidence.
