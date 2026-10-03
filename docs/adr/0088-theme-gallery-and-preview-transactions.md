# ADR 0088: Theme gallery and preview transactions

- Status: accepted
- Date: 2026-10-03
- Owners: Automexia maintainers

## Context

The adaptive-appearance selector cannot browse fixed palettes, preview without
saving, or create local variations. Theme colors already belong to the backend
configuration parser; Settings, the private preference writer, native file
picker and bounded worker runtime already supply the required boundaries.

## Decision

Reuse `Theme` and `Colors` for both configured and gallery palettes. Canonical
serialization adds no competing color model. Gallery validation rejects unknown
keys and missing colors while the legacy configuration parser retains partial
palette/default compatibility. Five built-in TOML assets embed into the app;
`ThemeDescriptor` indexes identity, name, source, light/dark classification,
swatches, validation and the resolved palette. No new dependency is introduced.

The existing Settings surface owns keyboard, pointer, IME and editor focus.
Arrow selection publishes a temporary palette only to the originating window;
Enter/Apply updates the application preference owner and all open windows.
Escape/close restores current applied configuration. Session generation, route
and relevant settings revisions reject stale completions. Customization edits a
copy through the existing color/text editor; built-ins are immutable.

One lazy worker scans/imports/exports/copies. Admission and completion capacity
are one; files are limited to 64 KiB, local inventory to 128, inspected entries to
512 and scan bytes to 4 MiB. Completion publishes before waking rendering.
Cancellation is checked between files and before atomic publication; a 20-second
UI timeout retains worker cleanup ownership. Native dialogs select paths;
validated file operations use existing private-filesystem primitives and
collision-safe canonical TOML writes. An optional bounded comment preserves the
user-visible name without changing the theme schema. No shell evaluation,
network, fonts, credentials or extension work occurs on a preview path.

Version-10 preferences persist the selected name and resolved palette snapshot.
Strict version-9 migration runs only when both current snapshots are absent;
old files remain for rollback and future/corrupt files never silently downgrade.
Use configuration clears theme/legacy appearance overrides. Font palette choices
remain explicit higher-priority overrides; semantic output, Kubernetes, table,
tag and application-supplied RGB ownership is unchanged. App chrome adapts its
surfaces and semantic accents through the existing immutable `UiTheme` projection.
Headers, pane tab rails, footers, all command categories, settings, search,
connection views and dialogs consume this same effective palette. Tab colors are
foregrounds; contrast is checked against each actual tab fill, including older
saved palettes and custom fills. Decorative borders remain distinct from focus
indicators. Dialogs remain opaque and explicit semantic color overrides retain
their own ownership. No palette scanning occurs during painting.

## Consequences and evidence

Removing or editing a local source cannot silently change the applied snapshot;
Refresh and Apply adopt updates. Cancellation cannot undo an already completed
export/copy, but never applies a stale theme. A blocked filesystem operation can
occupy the single worker until the OS call returns. No theme execution is allowed.

Parser, strict migration, canonical I/O, cancellation, copy isolation, keyboard,
layout and chrome-contrast tests exercise the owning paths. The Windows native
ThemeGalleryOnly fixture checks five palettes, cancel, Apply, copy and configuration
on CPU/WGPU. Applied dark/light checkpoints check header, footer, title and every
command-category surface against independent palette values; retained captures
cover Customizations and close dialogs. Quantized contrast tests include imported
midtone palettes, inactive tabs and custom tab fills. Linux/macOS compositor and
assistive-technology evidence remain external; unit tests are not substitutes for those environments.
