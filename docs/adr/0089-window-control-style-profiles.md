# ADR 0089: Window control style profiles

- Status: accepted
- Date: 2026-10-03
- Owners: Automexia maintainers

## Decision

The existing application caption painter owns Soft, Glass, Outline and Circles.
Settings previews adapt their canvas to that painter; samples do not register
window-action targets. The existing island layout and screen press/release owner
retain native minimize, maximize/restore, close confirmation and drag-away behavior.
Native system decorations remain owned by the platform.

Backend `Presentation` owns a bounded, copyable `WindowControlsAppearance` with
four explicit profile slots. Each profile admits only enumerated size, spacing,
icon-size and weight choices, integer percentages, and validated RGB/RGBA colors.
Unknown keys and invalid values fail deserialization. Styles preserve independent
edits; colors follow the theme until overridden. Symbols retain contrast after
composition, including inactive windows and user-selected backgrounds.

The application uses its existing preference overlay, settings catalogue, numeric
and color editors, asynchronous private writer, Reset preview and Restore saved
transactions. Version 11 imports version-10 snapshots only when the current pair
is absent, preserving predecessor bytes for rollback. Earlier schemas reject new
window-control fields, including empty tables. Older binaries continue reading
their older snapshots; they do not interpret or overwrite version-11 choices.

## Boundaries and alternatives

No dependencies, workers, I/O, caches or OS action adapters are added. Appearance
choices vary only visible geometry within the existing caption hit slots. Paint
uses finite bounded logical-pixel geometry and existing CPU/WGPU primitives;
only Glass uses the existing static reflection layers. There is no idle animation.

A second preview painter would risk visual drift. Unbounded named profiles would
add unnecessary persistence and management policy. Platform-drawn controls cannot
provide the requested styles, and remain the appropriate owner when native
decorations are selected.

## Verification

Tests cover configuration admission and invalid values, independent overlays,
strict migration/recovery, all catalogue editors and resets, retained RGB during
opacity changes, stale revisions, nonexecuting previews, keyboard navigation,
small geometry, state feedback, contrast and maximize/restore symbols. Native
Windows CPU/WGPU scenarios must verify the real settings and caption surfaces;
other native desktops and assistive-technology delivery remain separate evidence.
