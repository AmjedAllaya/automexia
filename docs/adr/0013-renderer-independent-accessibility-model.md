# ADR 0013: Renderer-independent accessibility model

- Status: Accepted boundary; implementation partial; native evidence external
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

This is partial implementation, not a complete native accessibility tree.
The workspace does not currently integrate AccessKit. Native screen-reader
coverage and complete platform adapter evidence remain unverified; this
document does not announce an implementation date or a future design.

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
semantic implementations in each renderer. v0.4 remains truthful: it ships the
keyboard/focus/contrast/scaling baseline and documents limitations without
claiming a complete accessibility tree.
