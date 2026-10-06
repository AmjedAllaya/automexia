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

Font size and adaptive appearance reuse the existing preference fields. Font
controls accept finite 6–100 point values with one-point increments that retain
fractions; explicit continuous-number metadata leaves discrete-number validation
unchanged. Appearance selects already-loaded Light/Dark palettes or removes the
override to inherit configuration. Fixed palettes explain unavailability and
retain stored choices. Runtime publication reuses loaded resources and updates
inactive local tabs; image/font discovery remains configuration-owned.

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

## 2026-09-29 appearance amendment

A **Customizations → Open Customizations** palette action opens a feature list
in the existing Settings owner. Each feature page projects the relevant controls
from the same validated catalog, including enable switches owned by other
Settings sections. The root actions are navigation only and never become edits.
Search and focus are retained when returning from a page; removing an installed
extension also removes its page. **Settings → Settings**
opens the full sheet, including the independent Terminal & Output switch for
status coloring. Both routes edit the same saved preferences. The
application-owned catalogue adds a context-tag visibility switch and fixed core
controls for 13 context tag colors, tag tint/plain style and opacity, five
semantic output text colors and five
background colors, and output text/background style. The background-only style
tints every recognized severity by default; the combined style preserves the
previous error/warning-only default until the user sets the other backgrounds.
Color editing uses one bounded RGB/RGBA modal in the existing Settings view. Stale revisions,
removal and unavailable settings cancel pending edits; pointer and keyboard
input remain modal. User-selected colors override configured visual colors;
reset inherits the current configuration or resolved active palette. Terminal
ANSI colors, inverse video, selection and source bytes retain precedence.

Schema version 3 adds fixed typed appearance overrides to the existing private
preference writer. Version-2 files are read only when both version-3 snapshots
are absent; version 1 remains the final read-only predecessor. The 16 KiB
limit, no-follow storage, atomic replacement, private lock and previous snapshot
remain. Invalid or future-current data never downgrades to an older snapshot.
The fixed controls do not grant an extension capability or turn the signed
extension manifest into an arbitrary Settings schema. Installed built-in
extension controls still follow inventory and removal.

The additive Settings action uses Ctrl+Shift+S, or Cmd+Shift+S on macOS, outside
Search, Vi and alternate-screen modes. ConfigEditor identities and existing
shortcuts retain their external-editor behavior. Reset clears the user override
and inherits current configuration; a failed save is visible as session-only.
Modal keyboard, pointer, paste and IME handling cannot forward search text or
activation keys to the running program.

## Limits and evidence

The native sheet currently exposes these font, appearance and presentation controls, not all public
configuration fields. Signed ecosystem manifest v1 is unchanged and has no
arbitrary settings descriptor support. Existing protected integrations remain
unavailable; displaying a setting never grants permissions or invokes providers.

Tests cover strict catalogue bounds, stale edits, search/focus, responsive text
and clipping, coalesced saves, preference import/recovery, installed-feature
removal, parser-owned label projection and context cancellation. CPU raster and
headless event tests do not establish native GPU, IME or assistive-technology
behavior on an unexercised platform. Native evidence remains explicitly scoped.

## 2026-09-30 editor consolidation amendment

The feature-organized Customizations route is now the only visible preference
editor entry in the palette. The existing `OpenSettings` action and shortcut
remain accepted and open that same feature list; `ConfigEditor` continues to
open the external configuration editor under Tools. The full-catalog Settings
sheet remains an internal presentation path, not a second visible preference
owner. This supersedes the two visible palette routes described in the
2026-09-29 amendment without changing the catalogue, saved values, reset
semantics or shortcut identity.

## 2026-09-30 direct editing and preview amendment

Numeric controls retain their increment and decrement buttons and accept a
typed or pasted value in the center field. The existing settings input owner
handles focus, clipboard, IME, cancellation and application; the catalogue
validates finite values, bounds and discrete steps before an edit is queued.
An invalid draft stays in the editor and never reaches the PTY or saved state.

Customization previews use fictional, local sample content. Information-tag
previews and terminal painting share pure shape geometry and layout hints from
`automexia-ui-model`; both consume the selected recipe and colors without
running providers. The preview remains a visual aid rather than a replacement
for native terminal evidence or an extension capability.

The main feature list retains shared controls. A bounded preview edit mode
selects individual tag slots, role defaults, and output severities by stable
setting ID, including hidden or currently absent roles. Contextual pages reuse
the admitted catalogue descriptors and the same `apply_edit` owner. Mouse and
keyboard input remain modal to Settings; the preview has its own clipped hit
targets and scroll range. Draft color graphics update locally before Apply,
without writing the preference overlay or terminal input.

## 2026-10-01 independent output colors amendment

The shared presentation owner separates completed-command backgrounds, general
detected log/status colors and Kubernetes readiness/status colors. Terminal
output colors groups command backgrounds and the independent log controls;
Kubernetes status colors has its own category and preview. Existing typed
RGB/RGBA editors, reset scopes and publication paths remain authoritative.
The completion-label and timestamp controls remain independent of backgrounds.

Configuration preserves legacy semantic choices when Kubernetes fields are
omitted. Private preferences version 5 imports predecessor choices once,
without rewriting version-4 or earlier files. Future or corrupt current
snapshots never fall back to older schema versions; the existing size, no-follow, lock,
atomic-write and previous-snapshot boundaries remain in force.

Output classification retains its domain even when coloring is disabled or the
status is unknown. It classifies bounded complete visible logical rows using
terminal wrap provenance. Incomplete spans degrade to neutral. Command bands
consume the same snapshot and projection and exclude source/style/selection
regions and inline tables. They cannot tint a Kubernetes row merely because
the command that printed it exited successfully. No provider, shell discovery,
filesystem work, or additional history scan occurs in painting.

## 2026-10-02 connected tag shapes amendment

Information tags add soft chevrons, puzzle joins, slanted tags, pills with arrow
joints, folded ribbons, cut corners, alternating triangles, top notches and
separator wedges alongside the existing shapes. The shared UI model owns their
bounded contours and horizontal-strip triangulation; preview and terminal
consume the same geometry. The existing fragment packer supplies row/group
position and connector overlap, preserving adjustable spacing, wrap boundaries,
split lanes and completion labels. Preview hit testing uses those contours.
No new renderer, dependency, provider or shell behavior is introduced.
Native comparison also exposed the CPU compositor interpreting consecutive
polygon triangles as bounding rectangles and dropping an odd final triangle.
Its existing base/modal primitive owner now scan-converts solid triangles with
half-open pixel-center spans and the existing SIMD alpha blender, without a
per-triangle allocation. Atlas glyph handling and instanced quads retain their
existing paths. Literal pixel masks cover tips, gaps, winding and shared edges.
Polygon calls also carry their surface's painter order, so GPU preview tags
follow the opaque panel instead of disappearing underneath it. Display-list
ordering and native top-center fill assertions protect that contract.

Private preference version 6 admits the new shape identities. Version 5 and
earlier import read-only when every newer snapshot pair is absent, retaining
the existing lock, size, no-follow, atomic-write and fail-closed recovery rules.
Legacy shape identities remain unchanged, and older files and `config.toml`
remain untouched for rollback. Temporary reset/restore keeps its existing owner.


## 2026-10-06 terminal appearance organization

Customizations now separates Workflow & Output from Terminal Appearance. Theme,
Fonts and Window controls have a single home in the latter, beside Header & Tabs,
Footer, Panes & Borders and Background & Spacing. The existing action identity
continues to open Workflow & Output; a separate bindable action opens Terminal
Appearance. Both reuse navigation, dependency visibility, color editing and
preference publication. Detail catalogs are projected on demand so new controls
do not consume the installed-extension capacity of the root catalog.

Private preference version 13 adds optional typed interface overrides. Version 12
imports only when both current snapshots are absent and remains unchanged for
rollback. Corrupt current data cannot silently downgrade. Existing panel, margin,
window and navigation configuration remain singular owners; new header/footer
paint options belong to Presentation. Individual reset clears an override;
section reset is temporary and cannot reset the other section. Native compositor
effects remain platform-dependent and require native evidence beyond model tests.
