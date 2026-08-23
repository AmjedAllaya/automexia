# ADR 0013: Renderer-independent accessibility model

- Status: Accepted for v0.5 implementation
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

v0.5 will introduce a bounded renderer-independent accessibility model in the
Automexia-owned UI layer. Platform adapters will translate that model through
AccessKit (or the platform API behind its adapter) while the renderer consumes
the same stable node identities and geometry.

The model will contain only public UI semantics:

- stable window, tab, pane, command, status, text-range, selection, and cursor
  node identities;
- role, label, selected/active/disabled state, parent/child order, bounds, and
  supported actions;
- generation identifiers so stale renderer or worker updates cannot mutate a
  newer tree;
- bounded terminal text ranges scoped to the visible/explicitly requested
  region, with password/secret modes excluded;
- provider-neutral freshness and error labels without credentials or raw
  environment values.

PTY parsing, shell integration, provider discovery, GPU painting, and extension
workers do not call platform accessibility APIs. They publish bounded model
updates through existing route/session ownership. Each OS adapter owns native
thread affinity, event coalescing, focus handoff, and teardown.

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
