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
| Keyboard hyperlink review | Visible link focus, exact destination preview, next/previous, explicit activation, copy, and dismissal without shell input |

## Focus and input ownership

Only the focused public surface receives keyboard, pointer, or IME input.
Opening an overlay prevents its keys from reaching the PTY. Dismissal restores
the previous valid target or a documented safe fallback.

The [keyboard hyperlink review](user-guide/hyperlinks.md) uses text labels and
underlining as well as color. Output or viewport changes cancel its captured
targets. Native accessibility-tree projection and screen-reader announcements
for this review remain unverified; keyboard and controlled raster tests alone
do not certify native assistive-technology support.

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

### Current implementation and limits

Native adapters use AccessKit for Windows UI Automation, macOS accessibility and
Linux AT-SPI. Projection activates when accessibility is requested; ordinary
terminal rendering does not build the semantic tree. Only the active visible
terminal is included, with explicit successful/failed command-result labels.
Covered terminal content is omitted while a modal is open. Accessibility clients
can request focus on the current control; terminal commands and settings changes
continue through the existing keyboard handlers.

Settings and command-palette controls have individual roles, labels, values,
states and painted bounds. Connection Hub reuses its renderer-neutral models.
Some secondary dialogs still expose summaries. Header/tab-rail/footer controls,
full document selection/caret operations and complete native control activation
are not yet projected. Ordinary keyboard navigation remains authoritative. Terminal
cells remain in application order, including mixed Arabic/Hebrew text; this does
not promise paragraph bidi reordering of terminal applications.

`rio-fonts::fallback` defines the font chain: session glyphs, user symbol maps,
configured style and regular faces, installed color emoji, then the native script
cascade. Missing fonts are optional and no font is downloaded. Windows tries
Segoe UI Emoji, macOS Apple Color Emoji, and Linux Noto Color Emoji first.
Explicit user faces take precedence. Color glyphs retain their font palette;
fallback never changes the terminal's cell dimensions.

IME preedit is transient and bounded. Its caret follows grapheme boundaries and
the complete composition is painted separately from terminal history. Pane or
overlay changes cancel terminal composition so a delayed commit cannot type
into the replacement input target.

The Windows native API fixture runs through the existing isolated GUI harness:

```powershell
powershell -NoProfile -File tests/integration/resize-stress-windows.ps1 -Binary target/debug/automexia.exe -AccessibilityOnly
```

It requires a binary built with `visual-test-hooks`. It reads the owned
window's UIA tree and text ranges, checks Unicode, modal isolation, control bounds
and resize. A feature-gated preedit event checks composition projection and
cancellation; it does not originate in the OS input service. It does not record
terminal text in its success report. This fixture
does not substitute for Narrator/NVDA review, native IME-service input, VoiceOver
or Orca validation. Their certification remains pending native evidence.

The Linux native API fixture uses an isolated session bus and X11 display. On a
Linux development machine with Xvfb, xauth, xdotool, PyAT-SPI and libxkbcommon-x11:

```sh
cargo build -p automexia-terminal --bin automexia --features visual-test-hooks --locked
GSETTINGS_BACKEND=memory timeout --signal=TERM --kill-after=8s 180s dbus-run-session -- xvfb-run -a -s '-screen 0 1280x800x24 -nolisten tcp' python3 tests/integration/accessibility-linux.py --binary target/debug/automexia
```

The fixture checks native text queries, concealed text, injected preedit,
individual modal controls, disabled states, bounds and owned-process teardown.
It rejects native client warnings. It does not modify the desktop's persistent
accessibility setting. Ubuntu 24.04 X11 API coverage does not establish Orca,
Wayland, other distribution or physical display coverage.

The manually dispatched **Native accessibility API assurance** GitHub workflow
runs the macOS adapter's tests and AppKit example on the existing Intel and
Apple Silicon macOS runner types. It requires no signing credentials and does
not publish a release. The example checks real AX text/value/geometry queries,
malformed ranges, read-only and oversized editing requests, and detached-view
lifetime handling. Its result must be checked on the exact requested commit;
a cross-compile or a workflow definition is not native runtime evidence.

All three local adapter adaptations have checked source inventories and retain
upstream licenses. Dependency audits remain separate from that inventory.
Adapter regression tests cover malformed native requests and bounded cleanup;
Linux additionally checks D-Bus cache signatures and invalid registry replies.

### Evidence requirements

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
