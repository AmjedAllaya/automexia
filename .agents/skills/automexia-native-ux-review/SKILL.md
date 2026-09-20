---
name: automexia-native-ux-review
description: Review and plan Automexia visible native terminal UX, including hierarchy, panes/tabs, overlays, input/focus, keyboard/IME, responsive layout, scaling, motion, rendering, and accessibility. Use when user-visible behavior changes; keep controlled fixtures distinct from native proof.
---

# Automexia Native UX Review

Protect the terminal-first experience while keeping visible behavior understandable, fast, accessible, and consistent. Review semantics and interaction before appearance; verify controlled rendering and native delivery separately.

Load `$automexia-feature-planning` only when the interface change needs non-trivial planning. Load `$automexia-terminal-assurance` only when terminal state/input/rendering/resize/concurrency/resources/performance are affected.

## Targeted authorities

Search the affected topic first; do not read every authority in full:

- `../../../docs/PRODUCT-VISION.md`
- `../../../docs/ARCHITECTURE.md`
- `../../../docs/ACCESSIBILITY.md`
- `../../../docs/KEYBOARD.md`
- `../../../docs/MANUAL-FEATURE-TESTING.md`
- `../../../docs/BRANDING.md`
- relevant UI specification/ADR/user guide/renderer tests/semantic model/native-evidence policy

Read only the matching sections plus directly needed cross-references. For large files, use heading/term/line-range reads. Do not load platform material for platforms the change does not affect or claim.

## Before implementation

Record only applicable decisions:

1. user goal, entry point, current behavior, and acceptance criteria;
2. owning screen/route/pane/tab/modal/focus surface;
3. content hierarchy and information priority;
4. affected keyboard/mouse/touchpad/clipboard/drag-drop/IME behavior;
5. semantic role/name/value/state/relationship/announcement behavior;
6. affected empty/loading/success/warning/failure/disabled/stale/offline/recovery states;
7. relevant viewport/scale/localization/text-length/theme/renderer/reduced-motion behavior;
8. performance/resource budgets only where interaction or rendering cost can change;
9. controlled/native evidence actually required for the claim.

Do not generate boilerplate for unaffected categories.

## Review order

1. **Task success and recovery** — discover, understand, complete, cancel, retry, recover.
2. **Ownership and input isolation** — one owner for input/focus/modal/selection/activation.
3. **Semantics** — full values remain independent of truncation/styling/pixels.
4. **Keyboard/IME/accessibility** — reachability, escape, focus order/restoration, announcements, zoom/contrast/motion.
5. **Responsive and visual behavior** — deliberate priority across affected sizes/scales/themes.
6. **Rendering evidence** — model/draw data/controlled pixels/native frames are kept as distinct claims.
7. **Performance/cleanup** — repeated use avoids stalls, starvation, unbounded caches, leaks, or retained stale surfaces.

## Evidence separation

Use [references/native-evidence-matrix.md](references/native-evidence-matrix.md) only for platforms/renderers/scales/inputs/accessibility technologies implicated by the task. Do not enumerate the full matrix when only one combination is relevant.

Keep these claims distinct: pure model/layout behavior; semantic accessibility projection; draw data/controlled raster; native window/compositor presentation; OS accessibility-tree/assistive-technology delivery; human usability/visual assessment.

A semantic summary does not prove Narrator/NVDA/VoiceOver/Orca delivery. Controlled pixels do not prove native compositor output.

## Tool/output discipline

Use native computer control only when the requested validation requires it and current tool rules permit it. Keep accidental PTY input isolated. Store screenshots/recordings/accessibility dumps/reports only in approved private evidence locations and redact private identifiers.

For visual comparison, freeze controllable variables, use exact comparison only under controlled conditions, inspect required expected/actual artifacts, and record only the parameters relevant to the claim.

## Report

Use [references/ux-review-report.md](references/ux-review-report.md) for an actual UX audit or when a structured evidence record is required; a small implementation task may use a concise handoff instead. Tie each finding to an observed scenario and acceptance criterion. Report unavailable native combinations explicitly rather than expanding the task to simulate them.
