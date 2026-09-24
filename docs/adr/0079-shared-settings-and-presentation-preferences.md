# ADR 0079: Shared Settings and presentation preferences

- Status: accepted
- Date: 2026-09-24
- Owners: Automexia maintainers
- Amends: [ADR 0036](0036-application-owned-runtime-user-preferences.md)

## Context

Automatic output presentation needs explicit user controls. An extension feature
preference must remain distinct from installation and capability authorization.
Settings must preserve hand-edited configuration, terminal input, live sessions,
source coordinates, and existing font/theme/shortcut ownership.

## Decision

Reuse `automexia-ui-model` for one bounded, renderer-independent catalogue with
stable identities, typed values, origin, applicability, effect scope, search and
revision validation. The native application sheet consumes that catalogue; it
emits intents rather than performing I/O or writing terminal input. The existing
application preference writer remains the only persistence owner. No new UI
framework, service, database or dependency is introduced.

The initial core presentation options control header-table formatting, detected
status highlighting and command timestamps. They update renderer presentation
and invalidate every pane and local tab. Disabling tables clears their visual
projection on the next coherent redraw. Timestamp visibility is applied before
label layout; completion status, duration and source metadata remain intact.

The application revalidates each edit against the current catalogue revision
and installed membership. Installed built-in adapters provide supported feature
controls using host-owned labels. Disabling a feature retains its entry;
uninstall removes it. Overrides are pruned only from a successfully loaded
inventory. Context collection has its own preference revision and cancellation;
stale work cannot restore a disabled feature. Classifier membership remains
independent of context collection. One existing bounded runtime worker loads
installation markers; renderer and catalogue reads use cached state.

Version 2 extends the strict preference overlay with optional presentation
booleans and bounded declared extension-feature overrides. It retains the
16 KiB limit, private no-follow storage, previous snapshot, lock and coalescing
writer. Version-1 primary/previous snapshots are read only when both version-2
snapshots are absent, and remain unchanged for rollback. Invalid or future
version-2 data never triggers a replacement by legacy data. An older binary
continues to read its old snapshot and does not see later version-2 edits.

The additive Settings action uses Ctrl+Shift+S, or Cmd+Shift+S on macOS, outside
Search, Vi and alternate-screen modes. ConfigEditor identities and existing
shortcuts retain their external-editor behavior. Reset clears the user override
and inherits current configuration; a failed save is visible as session-only.
Modal keyboard, pointer, paste and IME handling cannot forward search text or
activation keys to the running program.

## Limits and evidence

The native sheet currently exposes these presentation controls, not all public
configuration fields. Signed ecosystem manifest v1 is unchanged and has no
arbitrary settings descriptor support. Existing protected integrations remain
unavailable; displaying a setting never grants permissions or invokes providers.

Tests cover strict catalogue bounds, stale edits, search/focus, responsive text
and clipping, coalesced saves, preference import/recovery, installed-feature
removal, parser-owned label projection and context cancellation. CPU raster and
headless event tests do not establish native GPU, IME or assistive-technology
behavior on an unexercised platform. Native evidence remains explicitly scoped.
