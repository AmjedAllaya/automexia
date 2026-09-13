# Terminal interaction status

This page describes current UI ownership and known limitations. It does not
define future features, new configuration options, or a delivery schedule.
[Keyboard](KEYBOARD.md), [Configuration](CONFIGURATION.md), and the
[user guide](user-guide/index.md) are the current behavior references.

## Current ownership

| Area | Existing owner | Current status |
|---|---|---|
| Command palette | `apps/automexia-terminal/src/renderer/command_palette.rs`, application `screen/mod.rs` | Existing frontend actions are dispatched by the application; this is not a separate executor. |
| Shortcuts | `automexia-keybindings/src/`, application `bindings/registry.rs` and `bindings/mod.rs` | Typed bindings and a legacy fallback both exist. Default native-shell collisions are recorded in [maintenance status](TERMINAL-MAINTENANCE-REQUIREMENTS.md#native-shell-control-keys). |
| Runtime preferences | `apps/automexia-terminal/src/automexia/preferences.rs` | Schema 1 persists font size and appearance. This does not imply persistence for every transient UI control. |
| Context and layout | Application `renderer/`, `context/renderable.rs`, `grid_emit.rs` | Current session state feeds terminal drawing and overlay layout. Native pixel/accessibility evidence is environment-specific. |
| Shell listings | Existing shell integrations and terminal output | Displayed shell output is not a second filesystem inventory. |
| Accessibility | Renderer-neutral UI state and native adapters | Source adapters exist; release coverage depends on recorded assistive-technology evidence. |

## Local image preview

The current owners are application `image_preview.rs`, `automexia-image/`,
and Sugarloaf image consumers. Decoding uses bounded workers and validates route
generations, file authority and cache limits. Available controls, supported
formats and resource bounds are listed in [Image previews](IMAGE-PREVIEWS.md).

This inventory does not certify every placement after reflow, scroll, resize,
pane replacement, scale change or native presentation. Only the scenarios and
environments recorded by the existing tests and retained evidence are claimed.

## Command discovery correction

Current source replaces the flat initial palette with six categories, global
type-to-search, a Back row, paging and retained mouse scrolling. Keyboard and
pointer activation share one application owner; empty results do not dismiss
the query, and repeated Enter cannot activate through a category transition.
See [controls and migration](KEYBOARD.md#command-palette). This is implemented
source behavior; native pixels and screen-reader evidence remain external.

## Reported usability observations

The user reports difficulty browsing a long command-palette list, ambiguous
search hints/icons, dense colored command tables, and inconsistent visual
hierarchy between context tags, command timestamps and actual split borders.
Colors, shapes, icons and fonts are otherwise positively received. The reports
do not establish a renderer root cause or a completed correction.

Current behavior and limits relevant to those observations:

- In normal search with the query focused, Enter selects the next result and
  Shift+Enter the previous result. Vi search and focused scope controls have
  separate semantics; see [search](KEYBOARD.md#search-mode).
- Existing image quick look supports hover, explicit pinning, navigation and
  dismissal. Its presence does not certify every filename association or
  narrow-pane placement; see [local quick look](IMAGE-PREVIEWS.md#local-quick-look).
- Raw shell tables obey terminal cell/wrap semantics. A wrapped or densely
  colored `kubectl get pod`/`pods` result is not evidence of a separate
  structured-table implementation.
- A reported green `0/1 Completed` Pod row can be confused with running-ready
  status. A completed workload is not necessarily failed because readiness is
  zero; the command's own exit success is separate from the resource's state.
  See [Kubernetes Pod lifecycle](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/).
- A command timestamp/output boundary can be mistaken for a pane divider.
  This reported visual ambiguity is distinct from an actual split/layout action.
- Schema 1 runtime preferences persist font size and appearance only; source
  support for those fields is not a general-purpose runtime feature-toggle UI.

No new shortcut, image mode, table workflow or configuration option is activated
by these observations. Current source and the linked references remain the
behavior authority; native usability evidence is still environment-specific.

## Verification boundaries

Palette actions and typed shortcut contracts remain separate source structures;
the existence of both does not prove complete label or binding unification.
Likewise, a renderer-neutral test is not proof of native pointer, IME, keyboard
focus, screen-reader or pixel behavior.

Current checks, independent oracles and external evidence requirements are
documented in [Testing](TESTING.md) and
[feature reinforcement](FEATURE-TEST-REINFORCEMENT.md). No new application
behavior is introduced by this status inventory.
