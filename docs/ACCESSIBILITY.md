# Accessibility

Accessibility is a release contract for Automexia's public free terminal, not a
visual-style claim.

## Public baseline

The terminal must support:

- complete keyboard access to public controls;
- visible and predictable focus;
- role, name, value, state, and relationship semantics;
- focus order and restoration;
- error and live-status announcements without flooding;
- high contrast and meaning that does not rely on color;
- reduced motion;
- long and localized text;
- Unicode, combining characters, bidirectional text, and IME;
- 200% and 400% text/scale where applicable; and
- layouts from small windows through high-resolution displays.

## Surface contracts

| Surface | Keyboard and semantic requirement |
|---|---|
| Tab rail | Selected state, stable title, add/close controls, and documented navigation |
| Split panes | One visible active pane, geometric/cyclic focus, and independent session identity |
| Command palette | Labeled search, selected result, shortcut/category text, scroll state, activate, and dismiss |
| Search | Labeled query, pane/workspace scope, result status, previous/next, and focus restoration |
| First run | Clear heading, one primary action, no pointer-only requirement |
| Diagnostics | Severity, actionable text, close/recovery actions, and PTY input isolation |
| Compatibility inspector | Redacted public fields, empty state, and dismiss |
| Quit confirmation | Explicit consequence, cancel/confirm labels, keyboard and pointer access |
| Appearance controls | Labeled values, selected states, apply/cancel, and safe narrow-layout behavior |
| Scrollbar | Optional pointer target with terminal/document scrolling still available by keyboard |
| Terminal grid | Text, cursor, selection, input, scroll, and screen-reader document semantics |
| Local image preview | Filename/dimensions/size text, keyboard dismissal, and no image-only required meaning |

## Focus and input ownership

Only the focused public surface receives keyboard, pointer, or IME input.
Opening an overlay prevents its keys from reaching the PTY. Dismissal restores
the previous valid target or a documented safe fallback.

Focus never moves merely because background output, search, metadata, or an
optional worker updates.

## Motion, contrast, and status

Persistent structure, text, shape, and icons carry meaning; color and animation
are redundant. Reduced-motion mode suppresses nonessential transitions without
removing state.

Transient completion or status cues do not blink repeatedly, resize content, or
become required to understand terminal text.

## Terminal text and privacy

Accessibility projection follows current terminal state and route ownership.
Announcements and diagnostics do not expose hidden history, credentials,
environment values, unrelated panes, private paths, or raw control characters.

Large output is bounded and coalesced. Stale generations cannot publish events
after route replacement or closure.

## Testing

Visible changes require:

1. renderer-neutral geometry, hierarchy, focus, semantic, clipping, contrast,
   and reduced-motion assertions;
2. exact deterministic raster comparisons in a controlled environment; and
3. native frame and accessibility-tree/event evidence on each claimed
   operating system.

Automated checks do not replace manual Narrator/NVDA, VoiceOver, and Orca review
for a release claiming those environments. Record the exact commit, package,
OS, display, assistive-technology version, scenarios, failures, and cleanup.

## Reporting problems

Accessibility reports should include the public workflow, expected result,
actual result, operating system, input method or assistive technology, display
scale, and a redacted reproduction. Never attach credentials, real host data,
terminal history, local paths, or private planning documents.

Unreleased product surfaces are not part of this public accessibility
specification.
