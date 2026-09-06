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

## Verification boundaries

Palette actions and typed shortcut contracts remain separate source structures;
the existence of both does not prove complete label or binding unification.
Likewise, a renderer-neutral test is not proof of native pointer, IME, keyboard
focus, screen-reader or pixel behavior.

Current checks, independent oracles and external evidence requirements are
documented in [Testing](TESTING.md) and
[feature reinforcement](FEATURE-TEST-REINFORCEMENT.md). No new application
behavior is introduced by this status inventory.
