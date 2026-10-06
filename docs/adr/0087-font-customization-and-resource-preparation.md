# ADR 0087: Font customization and prepared resource publication

- Status: accepted
- Date: 2026-10-03
- Owners: Automexia maintainers

## Context

The size-only customization page cannot edit font families, styles, spacing or
terminal colors. Loading fonts synchronously from a settings action would block
input; saving before loading would also persist an unavailable family. Existing
font, renderer and private preference owners already provide these primitives.

## Decision

The application owns the Fonts page as a lazy settings catalog under the stable
`appearance.font_size` group. Typed overrides cover size, installed family,
regular/bold weights, bold/italic enablement, ligatures, hinting, drawable glyphs,
OpenType features, line height and 23 terminal palette roles. Shared settings
editors, reset confirmations and preview rendering remain the only UI owners.
Family names are bounded to 128 bytes, features to 32 tags/128 bytes, and colors
are opaque RGB. Additional directories, symbol maps and named per-face styles
remain declarative configuration. No dependency or font-installation path is added.

One application-owned `BoundedWorker` prepares a Sugarloaf font library with one
admitted request and one result. Completion is published before its render wake.
The application validates the originating settings revision, window, pane and
open editor before replaying the typed edit or reset/restore transaction. Missing
faces, worker failure and a 30-second timeout retain current preferences and fonts.
Cancellation retains worker/cleanup ownership until the load returns; repeated
requests cannot spawn unbounded discovery or cleanup work. OS font discovery is
not forcibly interruptible, so a timed-out worker remains occupied until it exits.

After successful preparation, the existing router library is replaced and every
screen reuses Sugarloaf's font invalidation boundary. Per-grid caches, atlases and
all local-tab metrics refresh together. Size, line-height, drawable-character and
color changes reuse existing font resources. Palette overrides apply to both
already-loaded adaptive palettes. Explicit command-output, Kubernetes and
application RGB colors retain their separate owners.

Version-9 private preferences add a strict optional `[fonts]` section. Version-8
readers reject the section and import only if both v9 files are absent; predecessor
files stay untouched for rollback. Corrupt/future current files cannot silently
downgrade or be replaced by migration. Existing bounded, private, atomic writes
and recovery remain the only persistence implementation.

## Alternatives and consequences

- A new font engine or discovery cache would duplicate existing ownership.
- Loading on the event thread would violate input responsiveness.
- Saving first would make failed requests survive restart.
- A separate UI font preference is deferred: current rendering shares a library;
  the page explicitly describes that scope, retaining existing settings-interface
  size limits.
- Reset/restore can require asynchronous preparation. Until it succeeds, all
  current settings remain active. Closing or changing the editor cancels the edit.

Tests cover typed catalog edits, strict migration/recovery, real process restart,
bounded worker failure/timeout/cancellation, stale targets, responsive previews
and existing renderer reload/metrics contracts. Controlled raster evidence is not
native desktop or assistive-technology evidence for every OS/font combination.

## Installed-family picker refinement

The family control now opens `settings_font_picker.rs` within SettingsView.
It shares the existing Unicode search editor and semantic/painter helpers. Font
preparation also accepts an inventory request, reusing Sugarloaf's CoreText or
font-kit system-family adapter on the same single-admission worker. Names are
validated, case-insensitively deduplicated and capped at 4,096, with bounded
bundled/current-family additions. Refresh is explicit; painting never scans.

One application-owned queued selection coalesces rapid navigation. Completion
must match the picker generation, settings revision, window, pane and selected
family. Cancelled work retains its worker admission until completion. Preview
uses Screen's resource-reload boundary without modifying the router library or
preferences; immediate UI text keeps the saved library for legible controls.
Apply reuses the prepared library and the existing typed family edit. Escape,
closing, reopening or changed settings restores the current authoritative library.
There is no new persistence version, dependency or font-installation authority.

The optional online installer is not part of this refinement: the project has
no trusted downloadable font catalog or cross-platform installation boundary.
Users can install through their OS font manager and refresh the picker. Remote
and WSL guest fonts are outside the local renderer's inventory. Configured
additional directories remain declarative; the current family is retained even
if absent from the system list.
