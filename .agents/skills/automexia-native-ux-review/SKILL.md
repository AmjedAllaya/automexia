---
name: automexia-native-ux-review
description: Review and plan Automexia's visible native terminal UX, including hierarchy, panes/tabs, overlays, input/focus, keyboard, IME, responsive layout, scaling, motion, rendering, and accessibility. Use before user-visible implementation and for native UX evidence; keep controlled fixtures distinct from native proof.
---

# Automexia Native UX Review

Protect the terminal-first experience while making visible behavior understandable, fast, accessible, and consistent across supported platforms. Review semantic behavior before appearance, then verify controlled rendering and native delivery separately.

Load `$automexia-feature-planning` before implementing a new or materially changed interface. Load `$automexia-terminal-assurance` when the interface touches terminal state, input routing, rendering, resize, concurrency, resources, or performance.

## Authorities

Read the affected parts of:

- `../../../docs/PRODUCT-VISION.md`
- `../../../docs/ARCHITECTURE.md`
- `../../../docs/ACCESSIBILITY.md`
- `../../../docs/KEYBOARD.md`
- `../../../docs/MANUAL-FEATURE-TESTING.md`
- `../../../docs/BRANDING.md`
- the relevant UI specification, ADR, user guide, renderer tests, semantic model, and native evidence policy

Keep public product behavior aligned with the product vision. Do not expose implementation detail in user flows unless it helps a user decide or recover.

## Before implementation

Record:

1. user goal and entry point;
2. current behavior and evidence;
3. screen, route, pane, tab, modal, and focus owners;
4. content hierarchy and information priority;
5. keyboard, mouse, touchpad, clipboard, drag/drop, and IME behavior;
6. semantic roles, names, values, descriptions, relationships, and announcements;
7. empty, loading, success, warning, failure, disabled, stale, offline, and recovery states;
8. viewport, scale, localization, text-length, theme, renderer, and reduced-motion behavior;
9. performance and resource budgets for input-to-state, state-to-draw, input-to-frame, cache, allocation, and cleanup;
10. controlled and native evidence needed for acceptance.

Do not begin production editing until the interaction and focus model is explicit enough to test.

## Review order

1. **Task success:** Can a user discover, understand, complete, cancel, retry, and recover from the flow?
2. **Ownership:** Does one surface own input, focus, modal state, selection, and activation at a time?
3. **Semantics:** Are full values preserved independently of truncation, styling, animation, or pixels?
4. **Keyboard and IME:** Can every action be reached and escaped without leaking input to the PTY or another surface?
5. **Responsive behavior:** Does priority degrade deliberately from tiny windows through 8K rather than overlap, disappear, or force unsafe activation?
6. **Accessibility:** Are role, name, value, state, relationships, focus order, announcements, contrast, motion, and zoom behavior correct?
7. **Visual system:** Are spacing, typography, color, hierarchy, density, alignment, hit targets, and states consistent with shared tokens and components?
8. **Rendering evidence:** Do renderer-neutral state, controlled pixels, and native frames agree within the limits of each oracle?
9. **Performance and cleanup:** Does repeated use avoid input stalls, frame starvation, unbounded caches, leaked handles, or retained stale surfaces?

## Evidence separation

Use [references/native-evidence-matrix.md](references/native-evidence-matrix.md) to select platforms, renderers, scales, viewports, inputs, and accessibility technologies.

Keep these claims distinct:

- pure model and layout behavior;
- semantic accessibility projection;
- draw data and controlled raster output;
- native window and compositor presentation;
- OS accessibility-tree and assistive-technology delivery;
- human usability and visual assessment.

A semantic summary does not prove Narrator, NVDA, VoiceOver, or Orca delivery. Controlled CPU pixels do not prove WGPU/native compositor output. OCR does not replace exact renderer-owned state when that state exists.

## Tool use

Use native computer control when the requested work or validation requires a local native scenario and current tool rules permit it. Keep a sibling input sentinel where accidental PTY input would be harmful.

Store screenshots, recordings, accessibility dumps, and reports only in approved private evidence locations. Use fictional stable data. Redact usernames, paths, hosts, commands, history, provider output, credentials, account identifiers, and private environment details before sharing or committing anything.

## Visual comparison

- Freeze clocks, animation, accounts, and public fixture data when test hooks support it.
- Use exact comparison only where fonts, geometry, renderer, scale, and fixture are controlled.
- Inspect both expected and actual images; a numeric pass alone is insufficient for a required visual artifact.
- Record tolerance, crop, color space, renderer, scale, font, theme, viewport, and revision.
- Preserve independent geometry and color oracles; do not derive expected pixels from the same production path.
- Treat anti-aliasing, font rasterization, compositor color management, HDR, and GPU differences as native variables, not excuses to widen every tolerance.

## Report

Use [references/ux-review-report.md](references/ux-review-report.md) for audits and handoff evidence. Rank findings by effect on task completion, safety, accessibility, data loss, input isolation, and frequency. Tie each finding to an observed scenario and an acceptance criterion.

Do not mark a UX change complete until required keyboard, focus, responsive, accessibility, visual, error, and recovery behaviors have evidence. Report each unavailable native combination explicitly.
