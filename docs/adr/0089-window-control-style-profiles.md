# ADR 0089: Window control style profiles

- Status: accepted
- Date: 2026-10-03
- Owners: Automexia maintainers

## Decision

The existing application caption painter owns Soft, Glass, Outline and Circles.
Settings previews adapt their canvas to that painter; samples do not register
window-action targets. The existing island layout and screen press/release owner
retain native minimize, maximize/restore, close confirmation and drag-away behavior.
Native system decorations remain owned by the platform. The default uses shared application
controls on Windows/Linux (`Disabled`) and macOS (`Buttonless`). The latter keeps
AppKit's native titled, resizable frame and hides native traffic lights. Explicit
`Enabled`/`Transparent` choices keep native controls. One `Decorations` policy
selects caption ownership; the created window and renderer retain it across live
configuration reloads. Appearance remains runtime-overridable.

Tab layout reserves caption slots only when those controls are drawn. The macOS
traffic-light inset exists only with native controls. Painting and hit-testing
consume the same layout, including at fractional scales. Invalid pointer
coordinates cannot activate a caption action. macOS retains AppKit edge resizing;
Windows/Linux use the existing manual resize adapter and consume the press only
when that adapter accepts it.

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
the Unix native scenarios additionally assert shared caption geometry and exercise
real session/menu/pane/resize workflows on X11, isolated Wayland, and
AppKit. Driver tests reject foreign or ambiguous windows, lost focus, unsupported
input and failed compositor startup. Physical-display edge dragging, input methods
and assistive-technology delivery remain separate evidence.
