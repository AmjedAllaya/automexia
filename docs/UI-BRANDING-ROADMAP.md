# UI branding roadmap and implementation audit

This focused roadmap is the source of truth for Automexia-owned renderer
surfaces. It supplements the release sequencing in [the main roadmap](ROADMAP.md)
and the visual contract in [Liquid Hacker UX](LIQUID-HACKER-UX.md); it does not
change terminal, PTY, shell, provider, or native-dialog authority.

Audit date: 2026-08-26.

## Status vocabulary

- **Fully done**: the source owner, focused contract tests, and current
  documentation agree.
- **Partially done**: source behavior exists, but an applicable native,
  accessibility, renderer, or release evidence gate remains.
- **Not done**: no authoritative implementation and test owner exists.

## Scope and acceptance criteria

The audit covers every Automexia-owned interactive renderer surface: persistent
window and pane chrome, first-run startup, command/search surfaces, dialogs,
diagnostic overlays, the tab appearance picker, status/footer presentation, and
scrollbars.

A surface is source-complete only when it:

1. uses the blue-black liquid-hacker hierarchy and semantic cyan/blue, purple,
   lime, amber, and coral roles;
2. derives readable foreground text from the configured theme at the 4.5:1
   project contrast floor;
3. fits tiny, normal, HiDPI, and large logical viewports without covering native
   window controls or leaving an invisible input trap;
4. exposes visible text or icon meaning in addition to color;
5. gives interactive pointer controls at least a 24 by 24 logical-pixel target;
6. has one input owner, consumes input behind modal scrims, and restores normal
   terminal routing after dismissal;
7. bounds untrusted text and preserves Unicode boundaries; and
8. has deterministic geometry, hit-test, state, negative-case, and strict-lint
   evidence.

Terminal cells, application ANSI styles, shell output, custom cursor effects,
and operating-system file dialogs are deliberately outside branding ownership.
They retain their terminal/application or native-platform authority.

## Evidence ledger

| Status | Item | Source and proof | Remaining evidence |
|---|---|---|---|
| **Fully done** | U0 shared application-chrome tokens | `renderer/ui_theme.rs` owns theme-aware opaque surfaces, semantic accents, source-over composition, and 4.5:1 foreground correction; focused unit tests cover dark and light configured themes | None for source implementation |
| **Fully done** | U1 previously aligned surfaces | Command palette, scoped pane/workspace search, Connection Hub, image quick look, pane/window chrome, prompt context, and session footer retain their existing responsive source and tests | Native release evidence remains tracked by their owning feature rows |
| **Fully done** | U2 diagnostic assistant | `renderer/assistant.rs` now owns a centered severity card, explicit close and troubleshooting actions, semantic amber/coral status, cyan help action, bounded text, responsive geometry, hover, and hit testing | None for source implementation |
| **Fully done** | U3 compatibility inspector | `renderer/compatibility_inspector.rs` keeps the ADR 0027 redaction allowlist while adding a centered branded card, visible close action, redaction label, empty-state success icon, hover, and bounded layout | None for source implementation |
| **Fully done** | U4 tab appearance picker | `renderer/island.rs` owns a cyan/purple final-layer card, 24-pixel swatches, check/clear icons, explicit Enter/Escape help, a 256-byte UTF-8-safe title limit, and fail-safe cancellation when the surface cannot fit | None for source implementation |
| **Fully done** | U5 quit confirmation feedback | `renderer/confirm_quit.rs` retains exclusive quit authority and responsive destructive-action layout while adding pointer hover state and visible cancel/close feedback | None for source implementation |
| **Fully done** | U6 scrollbar visual role | `renderer/scrollbar.rs` retains the allocation-free geometry, fade, hit area, and drag lifecycle while using rounded cyan/blue idle and drag roles | None for source implementation |
| **Fully done** | U7 modal input composition | `application.rs`, `router/mod.rs`, and `screen/mod.rs` prioritize the visible modal owner before resize, chrome, pane, PTY, wheel, input-method, and file-drop paths; hidden surfaces do not intercept a higher-priority command palette | None for source implementation |
| **Fully done** | U7.1 command-palette mouse scrolling | `application.rs` routes wheel/trackpad input to the visible palette before pane focus, terminal scrollback, mouse reporting, or PTY paths; `renderer/command_palette.rs` owns bounded fractional motion, direction reset, responsive offset clamping, visible selection, and a persistent branded overflow thumb that brightens during activity; focused tests cover both directions, boundaries, short/empty/filtered lists, malformed and over-limit deltas, query/keyboard/resize transitions, and modal priority | Native physical mouse/precision-trackpad pixels and accessibility scroll-position delivery on every release platform remain U10 evidence |
| **Fully done** | U8 first-run welcome | `router/routes/welcome.rs` replaces the legacy black-and-white animation with a static responsive Automexia card, shared theme-aware accents, one clear Enter action, bounded copy, and no local configuration-path disclosure; layout/DPI/target/privacy tests own the contract | None for source implementation |
| **Fully done** | U9 branded window caption controls | `renderer/responsive.rs` and `renderer/island.rs` use a compact 42-pixel comfortable shelf and 30-pixel visible caption cards inside preserved 40-pixel right-edge hit targets; the three separated cards have no shared border or decorative underline, use distinct cyan/purple/coral glyphs, expose rest/hover/held/inactive feedback, follow native maximize/restore state, and activate only on same-control release with drag-away/focus-loss cancellation; exact geometry and paint-command regressions reject enlarged chrome, undersized targets, overlap, grouped containers, and reintroduced underline rails | None for source implementation |
| **Fully done** | U9.1 command-result boundary and surface | `rio-vt` allocates stable pane-local completion IDs, carries one content-free result boundary onto the following prompt, and preserves it through prompt repaint, row reuse, reflow, viewport overflow, and complete source-prompt scrollback eviction. The renderer deduplicates by that ID and paints one persistent tint, gutter, end rule, status treatment, and reduced-motion-aware lightening without scanning history. Real parser-to-visible-render tests cover output heights on both sides of every viewport boundary, newline-only output, silent completion, and full source eviction. | Current-commit multi-platform pixels and assistive-technology delivery remain U10 release evidence, not missing U9.1 source behavior. |
| **Fully done** | U9.2 pointer-owned pane scrolling | `application.rs`, `screen/mod.rs`, `layout/mod.rs`, and `mouse/mod.rs` activate the exact pane under the pointer before delivering the initiating wheel/trackpad event, preserve wheel-focused selection, reset cross-pane fractional accumulation, and retain passive hover; focused tests cover hit-test boundaries and focus policy | Manual cross-platform mouse/trackpad interaction remains part of U10 native evidence |
| **Fully done** | U9.3 Windows Ctrl+V paste | `bindings/mod.rs` maps both `Ctrl+V` and `Ctrl+Shift+V` to the existing safe Paste action for every Windows-hosted child session, including WSL and SSH; explicit `ReceiveChar` configuration restores application ownership, while Linux/BSD and macOS defaults remain unchanged | Manual physical clipboard validation in native WSL and terminal applications remains part of U10 native evidence |
| **Fully done** | U9.4 continuous pane/workspace search | `bindings/mod.rs` keeps local/global search shortcuts active during Search; `screen/mod.rs` owns query-preserving, pane-isolated scope transitions and bounded visible-match counts; `renderer/search.rs` owns one responsive surface with clickable/keyboard scope choices, focus state, result status, and privacy-safe announcements. Focused tests cover both directions, idempotence, pointer targets, PTY isolation, checked/focused semantics, and tiny/HiDPI/ultrawide geometry; native Windows WGPU and CPU runs validate the real split-pane card. | Native Narrator/NVDA, VoiceOver, and Orca delivery remains part of U10 and the accepted v0.5 platform adapter. |
| **Partially done** | U10 native visual and assistive-technology release evidence | Renderer-neutral layout, contrast, hit-test, hostile-input, redaction, strict-lint, exact one-pixel visual-diff policy, 1,600-case visual-suite matrices, privacy/freshness checks, and release enforcement are automated. The source-completion audit is recorded in [the U10 assurance audit](research/U10-UI-BRANDING-ASSURANCE-AUDIT.md). | Execute and independently review all 24 current-commit S1 suites: Windows WGPU and CPU/RDP; Linux X11 and Wayland; macOS Intel and Apple Silicon; named resource/elevated suites; four 1,600-capture visual matrices; Narrator, NVDA, VoiceOver, and Orca X11/Wayland. A clean controlled manifest must pass `--require-complete`. |

There are no **Not done** rows in this renderer-owned branding scope. U9.1 is
**Fully done** at its source and focused-evidence boundary: long output retains
one terminal-owned completion boundary even after the originating prompt leaves
the entire retained scrollback. U10 remains **Partially done** and must not be
presented as passing until all current-commit controlled native, visual,
resource, assistive-technology, and independent-review suites are recorded.

## Interaction and trust boundaries

The diagnostic assistant and compatibility inspector are modal in every route.
Their visible scrim makes terminal cells, custom resize borders, pane controls,
wheel scrolling, input-method commits, and dropped files inert. The command
palette remains higher in the explicit modal stack when active. Escape dismisses
diagnostic surfaces; Enter also acknowledges the assistant, and `D` opens its
fixed troubleshooting URL through the existing typed platform opener.

The compatibility snapshot still excludes environment values, clipboard data,
terminal output, command history, paths, and secret material. The picker title
accepts no control characters and stores at most 256 UTF-8 bytes. None of these
changes adds a worker, dependency, process, network call, credential, persistent
store, cache, or PTY owner.

The first-run welcome is deliberately static, has one keyboard action, and does
not reveal the user's configuration path. Enter retains the existing local
starter-settings creation authority; the renderer adds no timer, worker,
network, account, credential, or alternate configuration owner.

## Build, wrap, or adopt decision

Automexia builds these small renderer primitives with the existing Sugarloaf
immediate-mode API and wraps the existing `automexia-ui-model` contrast
resolver. A new UI dependency would add a second renderer/accessibility owner,
startup/build cost, and migration surface without improving this bounded work.
No ADR is required because the change preserves the accepted renderer, modal,
security, and compatibility authorities.

## Standards checked

The implementation follows the WAI-ARIA modal-dialog interaction pattern:
content behind a modal is inert, Escape closes, and a visible close control is
provided. It also follows WCAG 2.2 AA text contrast and the 24 by 24 CSS-pixel
minimum target guidance:

- [Windows app title bar design](https://learn.microsoft.com/en-us/windows/apps/design/basics/titlebar-design)
- [WAI-ARIA Authoring Practices: modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
- [Web Content Accessibility Guidelines 2.2](https://www.w3.org/TR/WCAG22/)
- [Understanding Success Criterion 2.5.8: Target Size (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
- [Visual Studio Code: Basic editing, Find and Search](https://code.visualstudio.com/docs/editing/codebasics)
- [WCAG: Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast)
- [WCAG: Three Flashes or Below Threshold](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold)
- [WAI-ARIA Authoring Practices: radio group pattern](https://www.w3.org/WAI/ARIA/apg/patterns/radio/)
- [WCAG: Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions)
- [WCAG: Pause, Stop, Hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html)
- [Microsoft: Animation and timing guidance](https://learn.microsoft.com/en-us/windows/win32/uxguide/vis-animations)

Continuous search keeps the familiar local Find versus broader Search shortcut
distinction while avoiding a second modal owner. Its two scope choices use the
radio-group checked-state and arrow-key model in renderer-neutral form.

The completed-output cue follows the frequent-interaction guidance by changing
opacity for one 540 millisecond cycle without moving or resizing content. It
holds for the first third and then performs one smooth fade, rather than a
repeating blink; reduced-motion mode suppresses the cycle. Its redraw timer
shuts down at the end, and a visible persistent tint, gap, icon, rule, and
whitespace remain afterward.

The current v0.4 renderer does not yet expose a complete native accessibility
tree; [the accessibility baseline](ACCESSIBILITY.md) keeps that limitation and
the required external evidence explicit.

## Verification ladder

Focused Windows x86_64 and WSL source/native evidence completed through 2026-08-26:

- first-run welcome layout, DPI, target, and path-disclosure tests: 4 passed;
- shared theme contrast/source-over tests: 2 passed;
- assistant layout, target, and hit-test tests: 4 passed;
- compatibility layout, target, hover, and redaction tests: 4 passed;
- picker geometry and hostile UTF-8 input tests: 2 passed;
- quit layout, hit-test, hover, and opacity tests: 4 passed;
- scrollbar geometry, fade, lifecycle, and brand-role tests: 13 passed;
- command-palette wheel, trackpad, overflow-indicator, responsive-clamp, and
  modal-owner contracts: 54 palette tests plus 2 application routing and HiDPI
  normalization tests passed;
- native Windows WGPU and CPU command-palette runs injected wheel-down/up,
  observed offsets `0 -> 3 -> 0` with selected index `3`, preserved the active
  route/terminal display offset/cursor/raw line, and captured inspected 1750 x
  1080 frames with the persistent right-edge indicator;
- custom caption-control layout, semantic role, hit-target, maximize/restore,
  held-state, focus-loss, release-cancellation, and independent-card snapshot tests: 54 passed;
- command-result following-prompt placement, truthful output bounds, stable
  completion identity, newline-only output, silent completion, full source-row
  eviction, row reuse, repaint, reflow, and renderer deduplication passed 506 VT
  unit tests, 3 VT conformance tests, and 24 focused renderer tests;
- the real parser-to-scrollback-to-visible-render regression passes output
  heights `rows-2`, `rows-1`, `rows`, `rows+1`, `2*rows-1`, `2*rows`, and
  `2*rows+1`; the last five require the source owner to be offscreen and exactly
  one following-prompt boundary to carry the matching completion ID;
- a 512-output-row Criterion sample measured 192.69-202.72 microseconds and
  50.384-53.005 MiB/s on the local Windows x86_64 host. This is a first sample,
  not a controlled regression comparison;
- the current Windows WGPU native run passed all eight base PowerShell cases and
  the seven dynamic viewport-boundary cases at the 0.099 resting tint, including
  glyph and blank-surface pixel thresholds. The wider desktop run was later
  contaminated by a Windows Security dialog and stopped at the unrelated image
  hover stage, so neither that composed frame nor a current CPU rerun is claimed
  as complete U10 evidence;
- native Automexia/ConPTY/WSL/Bash passed stdout, multiline pipeline, stderr
  exit `7`, and silent-success ownership in the real application;
- pointer-owned pane selection, selection preservation, and cross-pane wheel
  accumulator isolation tests: 3 passed;
- Windows `Ctrl+V`/legacy paste binding and explicit terminal-input override
  test: 1 passed;
- continuous scoped-search unit/native evidence: 19 binary search contracts plus
  filtered worker/controller boundaries passed; the 1750×1080, 125%-scale,
  four-pane WGPU and CPU runs retained the query through both scope directions,
  kept the already-active scope idempotent, left the PTY cursor unchanged,
  closed cleanly, and measured the painted search region at 8,400 samples, 67
  color buckets, and a luminance spread of 216 on both renderers;
- native Windows WGPU/CPU resize and compositing gate: 19 deterministic resize
  tests plus both real-GUI passes completed on the current source; the inspected
  1750×52 top-shelf crops were identical across all 91,000 pixels under the
  zero-tolerance visual policy and showed compact separate cards without a
  shared outline; the complete commit-bound visual/assistive-technology matrix
  remains U10;
- caption geometry and interaction contracts at 1×, 1.25×, 1.5×, and 2× preserve
  40 logical-pixel right-edge targets around 30-pixel cards, disjoint action
  routing, drag-away cancellation, maximize/restore state, and focus-loss
  cleanup; both current-source native renderer runs exited cleanly;
- `cargo clippy -p automexia-terminal --all-targets --locked -- -D warnings`:
  passed.

The final repository-wide formatting, workspace Clippy, Nextest, doctest, full
QA, contributor-ready, diff, and documentation-policy results are recorded in
the delivery handoff. Native visual and assistive-technology evidence remains
U10.
