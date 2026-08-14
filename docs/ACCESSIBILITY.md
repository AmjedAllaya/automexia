# Accessibility baseline

## v0.4 scope and claim

Automexia v0.4 treats accessibility as a release contract, not as a visual
style claim. The local baseline covers keyboard reachability, visible focus,
semantic labels, contrast, extreme viewport behavior, and 200% scaling. Stable
release also requires recorded smoke evidence from supported assistive
technologies. Automexia does **not** claim a complete platform accessibility
tree in v0.4; that renderer-independent model is the v0.5 decision recorded in
[ADR 0013](adr/0013-renderer-independent-accessibility-model.md).

## Custom surface inventory

| Surface | v0.4 semantic contract | Keyboard contract | Current limitation |
|---|---|---|---|
| Tab rail | Selected tab, stable title, close/add hit regions | Existing tab creation, selection, and close bindings | Native screen-reader role exposure is not yet complete |
| Split panes | Exactly one active pane with a visible outline | Split creation and next/previous pane actions | Geometric directional focus is planned |
| Command palette | Search input, selected command, visible shortcut, category label | Open, filter, move, activate, and dismiss without a pointer | Platform role announcements require the v0.5 adapter |
| Context segments | Icon plus text label; meaning never depends only on color | Passive information; no hidden pointer-only action | Freshness/error announcements remain provider-neutral roadmap work |
| Session footer | Passive pane/tab/grid/line-ending/clock status | No action is hidden in the footer | It is intentionally omitted when a pane cannot spare terminal rows |
| Terminal grid | Shell output, selection, cursor, and input remain authoritative | Standard terminal and configured shell bindings | Full text-range exposure requires the v0.5 accessibility model |
| Image quick look | Filename, dimensions, and size remain visible as text; preview never conveys required terminal state | `Ctrl+Alt+I` (`Cmd+Alt+I` on macOS) previews a selected path; `Esc` or any input dismisses it | Native screen-reader announcement of the preview card requires the v0.5 adapter |

## Automated v0.4 contract

The normal and Phase 0 gates cover:

- shortcut collision and command-palette visibility tests;
- keyboard-only creation, selection, cloning, and isolated close paths;
- active-pane outline geometry at tiny, normal, split, HiDPI, 4K, and
  8K-equivalent layouts;
- semantic color contrast correction and redundant icon/text identity;
- Proptest viewport/DPI invariants with checked-in minimized regressions;
- reviewed structured footer geometry snapshots;
- font/glyph coverage and responsive omission of low-priority chrome;
- keyboard and modifier-hover image-preview reachability, bounded geometry,
  route isolation, and a native decoded-overlay lifecycle;
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