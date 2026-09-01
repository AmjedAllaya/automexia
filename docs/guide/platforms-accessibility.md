# Platforms and accessibility

Automexia treats platform support and accessibility as evidence-backed product contracts. This page states the supported hosts, what native evidence means, and the accessibility baseline without duplicating release-audit ledgers.

Automexia v0.4 has release and CI ownership for Windows, Linux, and macOS.
Portable Rust tests are necessary but not sufficient: an OS-specific adapter is
accepted on the native host that owns its window server, PTY, shell, graphics,
packaging, and accessibility APIs.

## Support matrix

| Surface | Windows | Linux | macOS |
|---|---|---|---|
| Native architecture | x86_64; ARM64 compile/package checks | x86_64 and ARM64 artifacts | Universal x86_64 + ARM64 app |
| Shells | PowerShell 5/7, CMD, WSL Bash/Zsh | Bash, Zsh, other shells without enhanced integration | Zsh, Bash; other shells without enhanced integration |
| PTY | ConPTY | Unix PTY | Unix PTY |
| Display/render | WGPU/DX path plus experimental CPU fallback | X11-only, Wayland-only, and combined builds | Metal/WGPU path plus experimental CPU fallback |
| Package | Signed MSI and ZIP | DEB, RPM, tar.gz | Signed/notarized universal app in DMG |
| Native deep evidence | ConPTY/resize/clone/image/AppVerifier/WPR controlled gates | controlled X11/Wayland GPU/PTY and package containers | controlled Metal/GPU/PTY, VoiceOver, Gatekeeper/notarization |

Ordinary pull requests run the locked all-feature Rust quality gate on the
GitHub-Free Ubuntu runner. Windows and macOS native, package, graphics, shell,
and accessibility evidence is collected by controlled release and assurance
jobs; it is never inferred from the Linux result. Workflow coverage is itself
mutation-tested, so removing a required runner, read-only permission, quality
command, package, or release job fails the policy gate.

## Windows and WSL

Windows owns PowerShell formatting, CMD integration, ConPTY, custom chrome,
window ownership, WSL distribution/user/directory cloning, Authenticode, and
MSI lifecycle. WSL is a Windows-launched Linux session, not a substitute for a
native Windows build. Keep Linux Cargo work under the WSL filesystem and MSVC
work on NTFS; see [Windows and WSL development](troubleshooting.md).

## Linux

The declared Linux gate uses Ubuntu runners, separate X11/Wayland feature
builds, and DEB/RPM clean-container validation. Packages declare their runtime
libraries and desktop/AppStream/URL/terminfo metadata. This proves the supported
interfaces and package families; it does not claim manual certification of
every distribution, compositor, GPU, or driver combination.

## macOS

macOS owns native window behavior, Metal/WGPU integration, universal binary
assembly, Developer ID signing, notarization, Gatekeeper, and VoiceOver checks.
Cross-compiling an Apple target from another host is not accepted as native
evidence because the SDK, window server, GPU stack, and signing policy are
host-provided.

## BSD and other Unix systems

Unix PTY, shell, and X11 code paths may compile on BSD and documentation uses
Linux/BSD where the behavior is shared. BSD is currently best-effort: it has no
required CI runner or v0.4 artifact and is not included in the machine-declared
supported platform set. A future support claim requires a native job, package
ownership, shell tests, and release policy rather than an inferred Linux pass.

## Evidence levels

- **PR:** deterministic GitHub-Free Ubuntu checks.
- **Nightly:** longer fuzz, sanitizer, benchmark, and unsigned package work.
- **Controlled:** real display/hardware, elevated instrumentation, credentials,
  or manual assistive-technology evidence.
- **External:** explicitly required evidence that the current environment did
  not observe. External never means passed.

The exact jobs and outstanding controlled evidence are maintained in
[Testing and verification](../developer/testing-release.md#native-platform-ownership) and the
repository `tests/assurance/feature-matrix.json` feature assurance ledger.

## Accessibility scope and claim

Automexia v0.4 treats accessibility as a release contract, not as a visual
style claim. The local baseline covers keyboard reachability, visible focus,
semantic labels, contrast, extreme viewport behavior, and 200% scaling. Stable
release also requires recorded smoke evidence from supported assistive
technologies. Automexia does **not** claim a complete platform accessibility
tree in v0.4; that renderer-independent model is the v0.5 decision recorded in
[ADR 0013](../project/adr/0013-renderer-independent-accessibility-model.md).

The v0.5 platform adapter will adopt AccessKit over that model, as sequenced in
the [build, wrap, and adopt architecture](../developer/architecture.md).
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
| Split panes | Exactly one active pane with a visible outline | `Alt`+Arrow (`Cmd`+`Alt`+Arrow on macOS) geometric focus plus next/previous cycling | Native screen-reader focus announcements require the v0.5 adapter |
| Command palette | Search input, selected command, visible shortcut, category label | Open, filter, move, activate, and dismiss without a pointer | Platform role announcements require the v0.5 adapter |
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
