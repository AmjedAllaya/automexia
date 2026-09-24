# ADR 0070: validated startup configuration

Status: accepted for current source; native interface and platform evidence remain
separately tracked.

## Decision and ownership

The existing backend configuration owner parses all configured environment entries
as one batch. Names retain the launch adapter's whitespace trimming; values retain
empty strings, Unicode and additional equals signs. Invalid entries return a
content-free typed error before any batch is applied. Every platform override is
validated when loading a candidate, and effective startup settings are validated
before process environment mutation. Application startup retains the existing
single-thread phase, and package-owned shell integration trust is resolved after
user assignments. Reload keeps its existing last-known-good transaction.

Starter creation stages complete bytes in the destination directory, synchronizes
the file and publishes with `NamedTempFile::persist_noclobber`. An existing ordinary
file is a successful no-op; linked/reparse/directory destinations and failures are
reported. An explicit target needs an existing parent; only default-root creation
retains its prior parent-creation policy. CLI failures return nonzero status and
errors do not include private paths. Owned temporary roots replace shared test
fixtures. No real user configuration is touched by tests.

## Welcome action lifecycle

The existing Router owns one application-private configuration action. Input emits
a typed intent; a capacity-one `BoundedWorker` runs the existing backend publisher.
One pending registration and one completion slot bound admission across windows.
A checked operation number, window identity, weak Welcome-route identity and
cancellation token bind the result to its origin. No worker reads UI state.

Completion is published before the existing typed Render event wakes the loop.
The application drains completion before window lookup, including when its target
has closed. Only the matching live Welcome route consumes it. Closing that window
cancels its result; closing a sibling does not. Quit stops admission and requests
retirement. Final shutdown has a two-second acknowledgement budget. A filesystem
publication already entered may finish; cancellation prevents stale UI publication
and does not pretend to interrupt an operating-system call.

Held Enter remains owned until release, including after a fast transition to the
terminal. Pending text uses the existing Welcome surface and layout. Errors use the
existing assistant diagnostic. A typed completion guard also reports an unwinding
creation handler, with no arbitrary callback during unwind. The generic worker
retains panic-payload and native-join ownership. That failed application action
closes admission permanently; the diagnostic offers CLI or restart recovery.

On Windows the native event-loop owner registers the user wake message through a
fallible preflight before creating its event target, runner or vsync worker. This
prevents the unwind completion guard from being the first caller of a lazy native
registration that could panic. Later user wakes use the initialized scalar cache;
other internal message IDs keep their existing compatibility path. Negative tests
inject zero/error returns locally and prove the target factory is not called.
A real Windows registration/cache control is distinct from forced OS-failure
coverage; no global message quota or invalid pointer is used to provoke a failure.

## Reuse and compatibility

This is core configuration behavior. An extension or second persistence service
would duplicate the owner. Reuse the already locked workspace `tempfile` dependency
on native backend targets, as the application migration already does. No package
version, credential, configuration key or persisted schema is added. Standard
process spawning cannot replace the application's existing global startup phase;
validation is shared with child-launch callers to prevent divergent parsing.

Malformed entries previously ignored or passed toward a native panic now fail the
whole candidate. Correct `NAME=VALUE` entries retain their values and order. Fix the
reported entry before retrying; do not restore partial acceptance as rollback.
Starter publication preserves concurrent creators and never truncates the winner.

[NamedTempFile persistence](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html#method.persist_noclobber)
does not guarantee removal of the staging link after every interruption or unlink
failure. File synchronization is not a universal directory or power-loss durability
guarantee. [Rust environment mutation](https://doc.rust-lang.org/std/env/fn.set_var.html)
requires particular care on Unix; this change keeps writes at existing startup and
never mutates the process environment from parallel tests.

## Evidence and remaining gates

Regression tests reproduce whole-batch rejection before launch. Backend tests cover
valid/invalid entries, all platform overrides, complete bytes, races, preservation,
errors and owned fixture cleanup. Rebuilt application CLI checks cover successful,
existing, relative, missing-parent, directory and racing destinations. Trust-checker
mutations reject absent validation and incorrect application/trust ordering.

Linux native backend tests exercised links, private modes and racing publication.
Windows lacked privilege for its symlink branch, which remains unverified there.
Worker tests cover admission, cancellation, identity replacement, close-after-
publication, handler failure, dropped wake targets and bounded shutdown. Input
regressions cover held Enter and its release after completion. Source-policy
mutations guard intent ownership and publication-before-wake but do not prove UI
responsiveness. Native Welcome frames, focus, assistive-technology delivery and
macOS startup remain separate gates. Full contributor readiness remains mandatory.
