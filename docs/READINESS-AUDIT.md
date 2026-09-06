# Public readiness audit

This audit covers the free terminal documented in
[Features](FEATURES.md). It intentionally excludes unreleased advanced and
commercial plans.

## Current readiness areas

| Area | Public contract |
|---|---|
| Identity and configuration | Automexia identity is consistent; starter configuration is non-overwriting; reload is transactional; migration is explicit and reversible |
| Terminal core | Supported control sequences, Unicode, scrollback, reflow, search, selection, cursor, and alternate-screen behavior have deterministic coverage |
| PTY lifecycle | Launch, ordered input, resize, output pressure, exit, close, descendant cleanup, and shutdown have platform-specific owners |
| Windows, tabs, and panes | Routes, focus, splits, pane-local tabs, selection, search, clipboard, and overlays stay isolated |
| Rendering | Snapshot generation, geometry, clipping, cursor, selection, themes, scale, and image lifecycle are bounded |
| Shell integration | Supported shells remain authoritative; integration is session-local by default and fails back to the ordinary shell |
| Images | Protocol images and local preview enforce input, dimension, pixel, cache, link, identity, and cleanup limits |
| OpenSSH | Manual system OpenSSH behaves as an ordinary terminal client; explicit local inventory has no credential or passive network authority |
| Accessibility | Keyboard, focus, semantics, contrast, reduced motion, scale, and native assistive-technology evidence are tracked |
| Packaging | Identity, licenses, checksums, provenance, signatures, install, upgrade, rollback, uninstall, and cleanup are verified per claimed platform |

## Evidence rule

Local source checks do not prove native platform behavior. Native evidence names
the exact operating system, architecture, package, shell, renderer, display,
assistive technology, hardware where relevant, and commit.

A failure remains recorded until understood. Missing signing, notarization,
accounts, native displays, assistive technology, or long-duration campaigns is
external, not passing.

## Release blockers

The public baseline is not ready for a claimed platform when any applicable
correctness, security, cleanup, visual, accessibility, performance, packaging,
or provenance gate lacks current exact-artifact evidence.

## Documentation readiness

Every public Markdown file must load, link correctly, contain no confidential or
machine-local values, and avoid advanced/commercial plan details. The complete
pre-privatization source remains in the ignored private archive for recovery.

See [Testing](TESTING.md), [Manual testing](MANUAL-FEATURE-TESTING.md), and
[Release trust](RELEASE-TRUST.md).

## D0 readiness rule

D0 and ADR 0012 require exact native evidence for session ownership, argument
boundaries, cancellation, no implicit Enter, descendant cleanup, and unchanged
manual system-SSH fallback before any broader availability claim.
