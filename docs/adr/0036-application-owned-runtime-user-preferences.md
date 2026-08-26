# ADR 0036: Application-owned runtime user preferences

- Status: accepted
- Date: 2026-08-26
- Owners: Automexia maintainers

## Context

Automexia already persists declarative configuration in `config.toml`, but the
font-size shortcuts and appearance toggle previously changed only live renderer
state. A restart silently returned to the configured values. Rewriting
`config.toml` from a shortcut would preserve neither comments nor unknown future
keys, and placing generic terminal preferences in an optional extension would
give that extension authority over every window and pane.

The setting path must not perform filesystem work in keyboard, PTY, layout, or
rendering hot paths. It must also preserve the hand-edited config as the
fallback, reject linked/oversized/malformed state, cope with rapid key repeats,
and bound shutdown waiting.

## Decision

The desktop application owns one versioned runtime-preference overlay below
`<config-root>/state/user-preferences-v1.toml`.

1. `config.toml` remains the canonical declarative configuration. The overlay
   contains only runtime-editable overrides: font size and forced light/dark
   appearance.
2. Startup performs one private, no-follow, 16 KiB-bounded read before the
   first window is built. Valid preferences are applied after platform config
   overrides and before renderer/window construction.
3. Font changes are application-wide. The selected screen sends a typed
   `FontSizeRequest`; the application validates the 6–100 point range, updates
   every live route, and queues the new snapshot. No PTY bytes are produced.
   Reset removes the font override and re-applies the current `config.toml`
   value.
4. Appearance toggles update every route and persist through the same owner.
5. One lazily started writer thread owns writes. Its in-memory queue has depth
   one and replaces pending snapshots, so key-repeat storms converge on the
   latest value without unbounded work.
6. Writes use a private per-store lock, same-directory temporary file,
   file/directory sync, link rejection, and replacement. The preceding valid
   snapshot is retained as `user-preferences-v1.previous.toml`; malformed or
   partial primary state recovers from it with a visible warning.
7. Shutdown requests a flush and joins for at most two seconds. Failure is
   reported without delaying or breaking terminal input/output.
8. The file is application-owned, contains no secrets, and is safe to delete
   while Automexia is closed to clear all runtime overrides. Config, extension,
   provider, credential, session, history, tab, pane, and window state keep
   their existing owners.

## Placement and alternatives

- **Chosen: application configuration boundary.** It already composes config,
  all windows/routes, platform appearance, startup, and shutdown. Storage stays
  outside `rio-vt`, PTY, renderer, and input owners.
- **Rewrite `config.toml`: rejected.** A typed shortcut writer cannot safely
  preserve user formatting, comments, and future keys without turning a simple
  preference into a competing config editor.
- **Keep pane-local transient zoom: rejected.** It contradicts the requested
  restart behavior and makes newly opened windows/panes disagree.
- **Existing or new extension: rejected.** Font and appearance are baseline
  terminal behavior, not optional domain authority. Disable/uninstall semantics
  would make fundamental settings disappear.
- **New dependency or database: rejected.** Existing `serde`, `toml`,
  `tempfile`, standard file locking, and the shared private-filesystem adapter
  satisfy the bounded schema and durability contract.

## Consequences

- Font and appearance choices now survive ordinary restarts and are consistent
  across open windows and panes.
- A runtime font override intentionally has precedence over a later config
  reload until Reset clears it; all unrelated config changes still apply.
- Multiple instances serialize writes. A contended or failed write is rejected
  and reported instead of silently corrupting state.
- Same-directory replacement materially reduces partial-write risk, but native
  power-loss guarantees still depend on the filesystem and platform. Tests do
  not claim universal crash-proof storage.
- Session topology, working directories, command history, scroll positions,
  and remote/provider state are not user preferences and are not added to this
  file.

## Verification

- unit contracts for missing, canonical, reset, boundary, malformed, future,
  unknown, oversized, recovery, permissions, contention, coalescing, flush, and
  no-staging-artifact cases;
- desktop action tests for exact font bounds, reset, invalid/no-mutation input,
  and application layering;
- `preference_worker_submission_nonblocking` Criterion measurement;
- controlled native restart procedure in `docs/MANUAL-FEATURE-TESTING.md`;
- workspace formatting, warning-denied Clippy, nextest, doc tests, assurance
  checkers, repository QA, and `cargo ready`.
