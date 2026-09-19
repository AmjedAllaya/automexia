# Native evidence matrix

Select the combinations affected by the change. Record exclusions and unexecuted combinations rather than implying universal coverage.

## Platform and renderer

| Dimension | Candidates |
|---|---|
| Operating system | Windows, current supported Linux/BSD environments, macOS |
| Terminal process adapter | ConPTY, Unix PTY, WSL boundary, remote/session adapter where applicable |
| Renderer | CPU controlled raster, WGPU/native GPU presentation, fallback or last-known-good path |
| Architecture | Every architecture claimed by the release or changed platform code |
| Theme | Dark, light, configured custom theme, high-contrast or platform accessibility theme where supported |
| Scale | 100%, 125%, 150%, 200%, 300%, 400% and relevant mixed-monitor transitions |
| Viewport | One/two-cell boundaries when meaningful, tiny, narrow, ordinary, maximized, 4K, 8K |
| Font/text | Bundled/default, configured font, reload, missing/fallback, long text, Unicode, RTL/bidi where supported |

## Interaction

Check applicable combinations of:

- keyboard-only navigation, shortcut activation, repeat, conflicts, dead keys, modifier-only input, escape, back, cancel, retry, and focus restoration;
- IME composition, candidate selection, commit, cancel, focus change, resize, modal activation, and session replacement;
- mouse and touchpad hover, click, double-click, drag, scroll, selection, context action, and hit-target boundaries;
- clipboard read/write denial, multiline paste, bracketed paste, large paste, hostile control characters, and wrong-session isolation;
- file drop, hyperlink activation, external tool handoff, and explicit confirmation where the product contract requires it;
- pane, tab, route, window, overlay, and generation changes while work or input is queued.

## Accessibility

Inspect the renderer-neutral semantic model first, then native delivery:

- role, accessible name, description, value, checked/selected/expanded/disabled/busy state;
- relationships, grouping, hierarchy, table/grid semantics, position and set size;
- logical focus order, visible focus, focus trap, restoration, and no focus loss on rerender;
- announcements for state change, completion, errors, conflicts, progress, cancellation, and recovery without storms;
- text alternatives for icons, images, previews, color-only status, diagrams, and progress;
- contrast, non-color cues, zoom/reflow, reduced motion, animation pause, and timing independence;
- Narrator and NVDA on Windows, VoiceOver on macOS, and Orca on Linux where claimed.

A native accessibility API inspection and a real assistive-technology task are separate evidence. Record product version, OS, technology version, scenario, expected result, observed result, and limitations.

## Visual and layout states

Cover applicable states:

- empty, first use, loading, partial, success, warning, error, offline, permission denied, timed out, cancelled, stale, retrying, disabled, and recovery;
- long labels, long paths shown in redacted or user-safe form, localization expansion, large values, wrapped text, truncation, and tooltips or detail views;
- selection, hover, pressed, focused, disabled, conflict, validation error, saved, unsaved, and destructive-action emphasis;
- overlays with selection, search, command palette, dialogs, menus, tooltips, notifications, and nested modal prevention;
- terminal content under overlays, at scrollback boundaries, during resize, and during high-rate output.

## Performance and resources

Measure separately:

- input-to-model latency;
- model-to-draw-data latency;
- draw-data-to-present latency;
- native input-to-frame latency;
- allocations and cache growth;
- steady-state CPU/GPU and memory where controlled;
- repeated open/close, theme/font reload, resize, route replacement, and shutdown cleanup.

A model benchmark or controlled raster does not establish compositor latency. A single frame does not establish long-session resource stability.
