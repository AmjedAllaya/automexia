# ADR 0013: Renderer-independent accessibility model

- Status: Accepted boundary; implementation partial; assistive-technology evidence external
- Date: 2026-08-14
- Owners: frontend, platform, and accessibility maintainers

## Context

Automexia renders tabs, panes, the command palette, context segments, the
session footer, and terminal cells through custom GPU surfaces. Keyboard,
contrast, geometry, and scaling tests establish a useful v0.4 baseline, but
they do not create the native semantic tree expected by Narrator/NVDA,
VoiceOver, or Orca. Binding accessibility directly to WGPU draw calls would
couple platform semantics to rendering and make headless verification weak.

## Decision

The accepted boundary keeps renderer-neutral public semantics separate from
GPU drawing and privileged terminal/provider work. Current source contains
bounded UI semantics, including the nodes in
`automexia-ui-model/src/connection_hub.rs`.

This is partial implementation, not a certified native accessibility tree.
`automexia-ui-model::accessibility` projects bounded nodes into AccessKit.
The frontend wraps AccessKit's UIA, AX and AT-SPI platform adapters because
the repository's windowing fork cannot use its upstream winit adapter directly.
Callbacks request a frame or queue bounded focus requests; they never read a
PTY, execute input, or obtain provider credentials. The native adapter is owned
by the window and is dropped before that window.

The active terminal supplies its current visible snapshot. Covered and inactive
terminals are omitted. Settings and palette controls use their existing painted
rectangles and focus; Connection Hub adopts its existing semantic projections.
Some secondary surfaces still expose descriptive summaries; header/tab-rail/
footer controls and native editing/activation remain incomplete. Native UIA and
AT-SPI fixtures exercise the application; the manual macOS workflow exercises
AppKit adapter APIs. These API checks are distinct from screen-reader usability,
real OS input methods and physical display scaling, which remain unverified.

Local platform adaptations retain upstream protocol implementations and licenses.
Source digests pin the reviewed boundaries: native range/value validation, stale
window/view handling, and bounded Unix worker events and registry replies. They
are not registry-package audits. Transitive dependencies keep their normal review
requirements. Ordinary keyboard behavior retains its existing input owner.

The verification requirements below remain mandatory. Model tests and visual
appearance do not substitute for native accessibility evidence.

## Verification

Implementation is accepted only when:

1. pure model tests cover role/label/action mappings, stable identities,
   generation replacement, pane/tab isolation, Unicode ranges, and bounds;
2. Proptest covers arbitrary trees, viewports, scaling, selection, and update
   order while preserving acyclicity and focus uniqueness;
3. finite Loom models cover publish/wake/replace/teardown ownership without
   platform or GPU FFI;
4. renderer and accessibility adapters consume the same geometry snapshots;
5. native smoke automation plus recorded Narrator/NVDA, VoiceOver, and Orca
   sessions pass on the supported matrix;
6. performance evidence proves updates do not block PTY parsing or rendering
   and that text exposure remains bounded;
7. security review proves terminal output, credentials, clipboard data, and
   hidden shell state cannot leak through labels, logs, or QA bundles.

## Consequences

This adds a deliberate adapter boundary and test surface, but avoids separate
semantic implementations in each renderer. Model, Unicode parser, IME and
concurrency tests are separate from native API and assistive-technology evidence.
An available adapter or a passing compile does not certify its operating system.
