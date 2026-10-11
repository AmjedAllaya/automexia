# ADR 0096: Shared and individual information-tag styling

- Status: accepted
- Date: 2026-10-11
- Owners: Automexia maintainers

## Context

Information tags expose semantic accents and layout but cannot independently
choose typography, background, icon or border styling. A second font system or
preference writer would duplicate ownership and risk blocking terminal input.

## Decision

The existing Information tags page lazily projects shared style descriptors;
existing selected-slot pages project the same descriptors with stable slot IDs.
One typed edit owner applies optional overrides. Per-field Reset means inherit;
explicit false can override a shared true. Inherit shared styling removes only
appearance overrides, retaining content, placement and visibility. Layout preset
changes retain styles. Custom-slot deletion removes its style. Existing extension
availability still controls both preview and terminal projection.

Version 15 adds `visual.information-bar.styles` to UserPreferences. Shared fields
and up to 16 recognized standard/custom slot maps are bounded and validated.
Shared opacity retains its existing ContextTags owner; only individual styles
may carry an opacity override. Strict v14 migration preserves predecessors; all
writes use existing private atomic recovery rules. No credentials or terminal
text is persisted by this feature. BarRecipe carries a derived serde-skipped
projection, avoiding a second serialized authority.

The renderer-neutral TagTypography model owns inheritance, shape strokes,
contrast correction, decoration geometry and row-bounded metrics. Both painters
measure and draw with the same prepared font options. Mixed-height connectors do
not overlap; interior spacing preserves connector clearance. Text and icons
remain opaque and contrast-corrected against their composited background.

Font family selection reuses the installed-font picker and bounded font worker.
Only the initiating window receives temporary previews; successful typed edits
publish normally to all windows. Worker completion validates setting, picker
session, revision, pane and family. Cancellation restores both styles and fonts.
Resource identity includes a sorted deduplicated inventory of up to 17 auxiliary
families with four style faces each. Missing variants reuse their regular face;
missing families keep the prior choice. Auxiliary faces are excluded from the
ordinary terminal fallback walk. Preferred UI faces retain existing fallback for
uncovered Unicode graphemes. At most 68 ASCII coverage results are cached; font
replacement clears this cache alongside shaped-label resources. Painting never
scans the filesystem or invokes font discovery.

## Consequences and validation

No new dependency, thread, capability or persistence writer is introduced.
Preparing many distinct families costs more memory than one shared family, but
the inventory and caches are bounded. Terminal grid geometry and PTY contents
remain unchanged; typography fits the existing prompt row rather than moving
shell cursor coordinates.

Regression tests cover shared/individual inheritance, every typed field, stale
edits, migration/recovery, reset/layout/custom-slot behavior, extension absence,
font fallback and reload, keyboard picker cancellation, all shape bounds, color
contrast, preview geometry, and timestamp measurement ownership. The isolated
Unix native scenario edits shared and individual sizing, restores inheritance,
checks saved preferences and leaves terminal source text unchanged. Platform
execution evidence is reported separately; pure tests do not certify a native
Windows or macOS run.
