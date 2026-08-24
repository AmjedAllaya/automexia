# Accessibility baseline

## v0.4 scope and claim

Automexia v0.4 treats accessibility as a release contract, not as a visual
style claim. The local baseline covers keyboard reachability, visible focus,
semantic labels, contrast, extreme viewport behavior, and 200% scaling. Stable
release also requires recorded smoke evidence from supported assistive
technologies. Automexia does **not** claim a complete platform accessibility
tree in v0.4; that renderer-independent model is the v0.5 decision recorded in
[ADR 0013](adr/0013-renderer-independent-accessibility-model.md).

The v0.5 platform adapter will adopt AccessKit over that model, as sequenced in
the [build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md).
Automexia continues to own stable semantic IDs, roles, names, states, actions,
focus, privacy, update cadence, terminal text/document projection, and
renderer-neutral tests. AccessKit owns translation to Windows UI Automation,
macOS accessibility, and Unix AT-SPI; it does not become a second renderer or
make pixels the semantic source of truth. Initial delivery covers application
chrome and structured overlays. Complete terminal-grid range semantics remain
a separate Automexia text/document milestone.

## Custom surface inventory

| Surface | v0.4 semantic contract | Keyboard contract | Current limitation |
|---|---|---|---|
| Tab rail | Selected tab, stable title, close/add hit regions | Existing tab creation, selection, and close bindings | Native screen-reader role exposure is not yet complete |
| Window caption controls | Distinct minimize, maximize/restore, and close shapes inside one outlined capsule; cyan/purple/coral accents are redundant | 40–46 pixel pointer targets latch on press, activate on release over the same control, and cancel on drag-away or focus loss; existing window shortcuts remain available | Native caption-button names and state announcements require the v0.5 platform adapter |
| Split panes | Exactly one active pane with a visible outline | `Alt`+Arrow (`Cmd`+`Alt`+Arrow on macOS) geometric focus plus next/previous cycling | Native screen-reader focus announcements require the v0.5 adapter |
| Command palette | Search input, selected command, visible shortcut, category label | Open, filter, move, activate, and dismiss without a pointer | Platform role announcements require the v0.5 adapter |
| Pane and workspace search | Scope icon and label, editable query, result position, previous/next, and visible close | Ctrl/Cmd+F searches one pane; Ctrl/Cmd+Shift+F searches visible panes; arrows/Enter/Escape operate without a pointer | Platform role and live-result announcements require the v0.5 adapter |
| First-run welcome | Automexia title, concise time/effort/flexibility value statement, and one visible Enter instruction; no local path is rendered | Enter creates starter settings and continues; no pointer-only action or background animation | Platform heading/action announcements require the v0.5 adapter |
| Diagnostic assistant | Error/warning text, severity label, visible close, and troubleshooting action | Escape or Enter dismisses; D opens the fixed guide; all terminal input behind the scrim is inert | Platform dialog/action announcements require the v0.5 adapter |
| Compatibility inspector | Explicit REDACTED label, public snapshot fields, diagnostic empty state, and visible close | Escape dismisses; all terminal input behind the scrim is inert | Platform dialog/list announcements require the v0.5 adapter |
| Quit confirmation | Destructive consequence, explicit Cancel/Close labels, key hints, and pointer hover | Escape/N cancels; Y confirms; both buttons have large hit targets | Platform alert-dialog announcements require the v0.5 adapter |
| Tab appearance picker | Labeled title field, 24-pixel swatches, selected check, clear icon, and apply/cancel help | Type/backspace edits; Enter applies; Escape cancels; non-fitting layouts cancel safely | Platform field/radio-group announcements require the v0.5 adapter |
| Connection Hub | Search/filter/group controls, labeled actions, status and empty/error states | Complete keyboard navigation and visible close; terminal input behind the scrim is inert | Native picker and screen-reader evidence remains release-gated |
| Scrollbar | Rounded cyan/blue thumb plus a wider invisible grab area; drag state differs in opacity and color | Wheel and terminal navigation remain primary; pointer dragging is optional | Native screen readers use terminal/document scrolling rather than this visual thumb |
| Context segments | Icon plus text label; meaning never depends only on color | Passive information; no hidden pointer-only action | Freshness/error announcements remain provider-neutral roadmap work |
| Session footer | Passive pane/tab/grid/line-ending/clock status | No action is hidden in the footer | It is intentionally omitted when a pane cannot spare terminal rows |
| Terminal grid | Shell output, selection, cursor, and input remain authoritative | Standard terminal and configured shell bindings | Full text-range exposure requires the v0.5 accessibility model |
| Image quick look | Filename, dimensions, and size remain visible as text; preview never conveys required terminal state | Hover previews; click pins; arrows browse visible image paths; `Ctrl+Alt+I`/`Cmd+Alt+I` previews selection; `Esc` dismisses | Native screen-reader announcement of the preview card requires the v0.5 adapter |

## Automated v0.4 contract

The normal and Phase 0 gates cover:

- shortcut collision and command-palette visibility tests;
- keyboard-only creation, selection, cloning, and isolated close paths;
- active-pane outline geometry at tiny, normal, split, HiDPI, 4K, and
  8K-equivalent layouts;
- semantic color contrast correction and redundant icon/text identity;
- first-run startup layout/DPI/target invariants and a regression guard against
  rendering local configuration paths;
- diagnostic/inspector/quit layout, 24-pixel pointer-target, hover, hit-test,
  redaction, modal-input, and opaque-surface tests;
- tab-appearance fit, 24-pixel swatch, redundant selected/clear meaning, and
  256-byte control-free UTF-8 title tests;
- Proptest viewport/DPI invariants with checked-in minimized regressions;
- reviewed structured footer geometry snapshots;
- font/glyph coverage and responsive omission of low-priority chrome;
- keyboard, plain-hover, click-to-pin, arrow-browse, and Escape image-preview
  reachability, bounded geometry, route isolation, complete mouse-pair
  ownership, and a native decoded-overlay lifecycle;
- Clippy, Nextest/JUnit, Cargo doctests, deterministic resize storms, and the
  native Windows GUI gate.

Run the locally applicable evidence with:

```text
cargo qa
cargo xtask qa --full --bundle
```

The bundle never captures terminal contents, clipboard data, inherited
environment values, or an environment dump. Private ETL traces are excluded.

## Required manual smoke matrix

A stable release record must identify date, commit, OS/build, display scale,
GPU/driver, assistive technology/version, operator, and result for:

| Platform | Required tools | Required tasks |
|---|---|---|
| Windows | Narrator and NVDA | Launch, identify tabs and active pane, open/filter/activate/dismiss palette, split/select/close, 200% scale |
| macOS | VoiceOver | Same tasks, including native window/tab behavior and full keyboard access |
| Linux X11 and Wayland | Orca with the supported desktop stack | Same tasks, including focus restoration after split close |

Record failures with renderer-neutral state, a screenshot when safe, and exact
reproduction steps. Never include shell output, paths containing private user
data, clipboard content, credentials, or tokens in a public artifact.

## Release limitations

Until the v0.5 model and platform adapters land, release notes must state that
custom chrome has a tested keyboard/visual baseline but incomplete native
screen-reader semantics. Manual smoke results cannot be generalized to an
untested desktop, screen reader, locale, or renderer backend.
