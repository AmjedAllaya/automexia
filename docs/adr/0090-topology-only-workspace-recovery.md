# ADR 0090: Bounded workspace and display recovery

- Status: accepted; extended 2026-10-04
- Date: 2026-10-03
- Owners: Automexia maintainers

## Decision

The application coordinates startup and manual recovery. ContextManager remains
the launch owner; ContextGrid converts layout descriptors. Every restored terminal
gets an independent PTY and new runtime identities. Startup consent uses a
process-free placeholder. Manual recovery opens additional windows.

`automexia::session_recovery` owns version-2 encrypted checkpoints beneath the
existing `state/session-v1` directory. Version-1 topology records migrate without
adding authority. Preferences remain separate. Descriptors retain window geometry,
tab/split/local-tab topology, selection, validated profile identities, working
folders and explicit tab styling. They never contain executable arguments,
environment capsules, credentials, remote destinations or process handles.

At the user's request, the terminal grid owner also exports an inert display
archive of up to 10,000 physical lines per terminal. Unicode, cell styles, wraps
and display-only command metadata survive; OSC commands, hyperlinks, images and
alternate-screen application state do not. Restore installs cells directly before
the new PTY reader starts, without replaying terminal escape sequences or stdin.
Recovered history stays above the new live viewport. This is historical output,
not resumed jobs or a saved running editor.

## Ownership, protection and bounds

One BoundedWorker owns capture, private storage, protection and local directory
checks. The event loop captures only descriptors and runtime-only terminal
references. The worker copies bounded grid data under a short lock, then releases
the lock before conversion, compression, encryption and serialization. Saves
coalesce at ten-second intervals with a final close checkpoint. Unchanged encoded
content avoids another write. The existing private_fs adapter supplies no-follow
reads, private permissions, exclusive locking and durable atomic replacement.

Windows uses current-user DPAPI with UI disabled. Unix uses authenticated
XChaCha20-Poly1305 with a random nonce and a per-store key in Secret Service or
macOS Keychain. Missing or locked protection fails closed; decrypt never invents
a replacement key. Platform adapters stay on the worker. Tests on Unix substitute
only key storage, retaining the real authenticated encryption implementation.
Unsupported platforms preserve existing files and report unavailable protection.

The macOS adapter uses narrowly scoped Security framework bindings and retained
Core Foundation values. It disables native credential dialogs once per process,
checks that policy, and never unlocks, updates or deletes existing keys.
Only a missing entry permits creation; ACL, locked-store and type errors fail
closed. Native macOS credential-store behavior remains unverified.

The RustCrypto InOut adapter uses a reviewed local source correction. Its exact
files are pinned by the recovery architecture checker and its regression tests
run with the workspace. This modified source is distinguished from registry
certification; transitive registry dependencies retain their normal audit gate.
The correction preserves pointer provenance without adding a cryptographic
implementation. See [source provenance](../../third-party/inout/UPSTREAM.md).

The Secret Service adapter searches unlocked items and checks the default
collection before creating a key; it never calls unlock. The adopted provider's
create operation may still request authorization, including if the collection
locks after that check. Its interaction stays on the single recovery worker;
bounded shutdown retains worker ownership rather than blocking the event loop.
No claim of race-proof prompt suppression is made for that provider.

Bounds cover encoded/decoded bytes, windows, terminals, tree depth, dimensions,
styles and history cells. A workspace cell budget can reduce the retained line
count for very large terminals. Deserialization rejects invalid data and future
schemas; decompression is bounded. Raw command activity counters remain live;
only the significance and incomplete-recovery decisions are persisted.

## Lifecycle and retention

Smart prompting uses multiple terminals or admitted command/SSH activity. Idle
age alone is insufficient. The menu retains manual recovery after quiet startup;
configuration can request always/never prompting and exclude individual profiles.
A short trivial visit cannot replace useful previous work.

Successful recovery saves the new workspace before tombstoning and removing the
consumed candidate and obsolete backup. Capture failure preserves the candidate
and retries with the newest coalesced capture. Failed partial restoration is
marked so restarting cannot replace the complete retry candidate. Cancellation
never consumes it. Closing a meaningful new workspace, or one used for at least
30 minutes, retires stale previous data. Current, transactional backup and manual
candidate have separate bounded roles; no historical journal grows without limit.
Orphaned private staging files are removed only under the exclusive store lock.

Restoration is paced, late results cannot launch after cancellation, and shutdown
retains worker ownership through a bounded flush. SSH stays disconnected; the
normal connection flow still owns authentication and host-key checks. Guest CWDs
are structurally validated without network probes. Missing local folders fall
back to current configuration; unavailable profiles fail independently. Geometry
is clamped to the current monitor; native macOS tab groups reopen as windows.

## Alternatives and evidence

Persisting process descriptors or feeding saved commands back into a shell would
restore execution authority. Raw VT replay would revive control-sequence effects.
Unbounded transcripts would grow indefinitely. Those approaches are rejected.
The existing grid, PTY, worker and private-file owners are reused instead.

Replacing a grid with recovered history invalidates live command-action handles.
Historical input retains display metadata without a completed-input capability;
last-command actions become available only for commands in the new session.

Model, parser, archive, worker and layout tests cover limits, Unicode/reflow,
protection, corruption, migration, fresh identities, coalescing and consumption.
The Windows native fixture exercises crash/restart, no child before consent,
history sentinels without rerun, fresh shells, quiet startup, manual recovery and
old-copy cleanup on both renderers. Native Unix credential stores, desktops and
screen-reader delivery require separate evidence; cross-compilation is not proof.
