# Terminal experience

Automexia keeps the terminal grid and shell prompt primary while providing
keyboard-first windows, tabs, panes, search, selection, clipboard, appearance,
and bounded overlays.

## Layout

A window owns top-level tabs. A workspace contains panes, and each pane may have
local tabs. Every session has an independent PTY. Focus is visible and follows a
stable route rather than an inferred coordinate.

Small windows compact secondary chrome before terminal content. High-DPI and
ultrawide layouts preserve readable cell geometry and pointer targets.

## Tabs and panes

Use documented shortcuts or the command palette to create, select, reorder, and
close tabs; split panes; move focus; and resize dividers. Closing one route does
not alter unrelated sessions.

Fresh splits use ordinary defaults. Clones copy only documented launch context
and still create an independent PTY.

## Search and command navigation

Pane search and visible-workspace search share one bounded, cancellable surface.
Switching scope preserves the query and rejects stale results.

Previous/next-command actions move the viewport between semantic prompt
boundaries. They do not edit, submit, or rerun shell input.

## Selection and clipboard

Selection respects wrapped lines, wide characters, grapheme clusters, and
scrollback. Copy and paste are explicit and route-scoped. Paste never appends
Enter.

## Appearance

Themes, fonts, cursor, opacity, line spacing, tab appearance, active-pane
outline, scrollbars, and passive status are configurable within documented
bounds. Invalid changes keep the last-known-good configuration.

## Overlays

The command palette, search, diagnostics, compatibility inspector, appearance
controls, image preview, and quit confirmation own their input while open.
Dismissal restores previous focus and never leaks keystrokes to the PTY.

## Prompt metadata and output cues

Supported session-local integration may show bounded current path, Git, status,
and duration information. It never changes the command or pipeline output.

Completed output cues remain passive and redundant with terminal text. Reduced
motion removes nonessential transitions without removing status.

## Accessibility and privacy

Every public surface is keyboard reachable and supports visible focus, semantic
labels/states, high contrast, reduced motion, scaling, Unicode, and IME.
Diagnostics and chrome avoid exposing credentials, hidden history, private
paths, and unrelated pane content.

See [Keyboard](../KEYBOARD.md), [Configuration](../CONFIGURATION.md),
[Accessibility](../ACCESSIBILITY.md), and [Liquid Hacker UX](../LIQUID-HACKER-UX.md).
