# UI branding roadmap and implementation audit

This focused roadmap is the source of truth for Automexia-owned renderer
surfaces. It supplements the release sequencing in [the main roadmap](ROADMAP.md)
and the visual contract in [Liquid Hacker UX](LIQUID-HACKER-UX.md); it does not
change terminal, PTY, shell, provider, or native-dialog authority.

Audit date: 2026-08-24.

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
| **Fully done** | U8 first-run welcome | `router/routes/welcome.rs` replaces the legacy black-and-white animation with a static responsive Automexia card, shared theme-aware accents, one clear Enter action, bounded copy, and no local configuration-path disclosure; layout/DPI/target/privacy tests own the contract | None for source implementation |
| **Partially done** | U9 native visual and assistive-technology release evidence | Renderer-neutral layout, contrast, hit-test, hostile-input, redaction, and strict-lint evidence is automated | Record safe screenshots on supported WGPU/CPU backends at tiny, normal, split, 100–300% scale, light/dark custom themes, and complete Narrator/NVDA, VoiceOver, and Orca smoke evidence on the supported native operating systems |

There are no remaining **Not done** source items in this renderer-owned branding
scope. U9 is an external release-assurance gate and must not be presented as
passing until those native runs are recorded.

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

- [WAI-ARIA Authoring Practices: modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
- [Web Content Accessibility Guidelines 2.2](https://www.w3.org/TR/WCAG22/)
- [Understanding Success Criterion 2.5.8: Target Size (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)

The current v0.4 renderer does not yet expose a complete native accessibility
tree; [the accessibility baseline](ACCESSIBILITY.md) keeps that limitation and
the required external evidence explicit.

## Verification ladder

Focused Windows x86_64 source evidence completed on 2026-08-24:

- first-run welcome layout, DPI, target, and path-disclosure tests: 4 passed;
- shared theme contrast/source-over tests: 2 passed;
- assistant layout, target, and hit-test tests: 4 passed;
- compatibility layout, target, hover, and redaction tests: 4 passed;
- picker geometry and hostile UTF-8 input tests: 2 passed;
- quit layout, hit-test, hover, and opacity tests: 4 passed;
- scrollbar geometry, fade, lifecycle, and brand-role tests: 13 passed;
- `cargo clippy -p automexia-terminal --all-targets --locked -- -D warnings`:
  passed.

The final repository-wide formatting, workspace Clippy, Nextest, doctest, full
QA, contributor-ready, diff, and documentation-policy results are recorded in
the delivery handoff. Native visual and assistive-technology evidence remains
U9.
