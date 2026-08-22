# Keyboard and input reference

This page defines Automexia's active v0.4 defaults. Explicit entries under
`[bindings]` replace matching default triggers. The separate
[Ghostty compatibility](keyboard.md) page describes a
future opt-in profile and is not the active default set.

`Cmd` means the macOS Command key. Search and Vi-mode bindings apply only while
their mode is active. A terminal application can own a key when the table says
the action is mode-sensitive.

## Windows and Linux/BSD defaults

### Windows, tabs, and panes

| Shortcut | Action |
|---|---|
| `Ctrl+Shift+N` | Create an independent OS window. |
| `Ctrl+T` | Create a window-level tab. |
| `Ctrl+Shift+T` | Create an independent tab inside the selected pane. |
| `Ctrl+Shift+W` | Close the selected local tab, split, or window tab—whichever owns focus. |
| `Ctrl+F4` | Close the current local tab, or the current window-level tab when no extra local tab exists; never remove a split. |
| `Ctrl+Shift+F4` | Close every other window-level tab. |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous window-level tab. |
| `Alt+PageDown` / `Alt+PageUp` | Next / previous local tab inside the selected pane. |
| `Ctrl+1` … `Ctrl+8`; `Ctrl+9` | Select window tab 1…8; select the last tab. Windows only. |
| `Ctrl+Shift+R` / `Ctrl+Shift+D` | Fresh default-shell split right / down. |
| `Ctrl+R` / `Ctrl+D` | Clone the active shell/profile/directory into an independent split right / down. |
| `Ctrl+Alt+R` / `Ctrl+Alt+D` | Send shell history-search / EOF control byte displaced by cloning. |
| `Alt+Arrow` | Select the nearest pane geometrically. |
| `F6` / `Shift+F6` | Cycle to next / previous pane. |
| `Alt+Shift+Arrow` | Resize the selected split on Windows. |
| `Ctrl+Alt+Shift+Arrow` | Resize the selected split on Linux/BSD. |

On Windows, `Ctrl+PageUp/PageDown` also selects the previous/next window tab
and `Ctrl+Shift+PageUp/PageDown` reorders it. Linux/BSD also supports
`Ctrl+Shift+[` / `Ctrl+Shift+]` for previous/next window tab.

### Editing, selection, clipboard, and view

| Shortcut | Action |
|---|---|
| `Ctrl+C` | Copy a non-empty terminal selection; with no selection, send the shell/application interrupt unchanged. |
| `Ctrl+Shift+C` / `Ctrl+Shift+V` | Copy / paste. |
| `Shift+Insert` | Paste the primary selection when the platform provides one. |
| `Shift+Arrow` | Start at the terminal insertion cursor, then extend/reverse selection by one cell or row. |
| `Ctrl+Shift+Left/Right` | Extend/reverse selection by a Unicode word boundary. |
| `Ctrl+Shift+A` | Select all on Windows. |
| `Ctrl+0`, `Ctrl+=` or `Ctrl++`, `Ctrl+-` | Reset, increase, or decrease pane font size. |
| `Shift+Home/End` | Scroll to history top / bottom outside the alternate screen. |
| `Shift+PageUp/PageDown` | Scroll one page up / down outside the alternate screen. |
| `Ctrl+Shift+F` / `Ctrl+Shift+B` | Search forward / backward. |
| `Ctrl+Shift+Space` | Toggle Vi mode on Windows. |
| `Alt+Shift+Space` | Toggle Vi mode on Linux/BSD and all platforms through the common binding. |
| `Ctrl+Shift+K` | Clear history on Windows. |
| `F11` or `Alt+Enter` | Toggle fullscreen on Windows. |
| `Ctrl+Alt+I` | Preview the selected or pointer-targeted local raster image. |
| `Ctrl+Shift+P` | Open the command palette. |
| `Ctrl+Shift+H` | Open the read-only Connection Hub. |
| `Ctrl+Shift+O` | Open Quick Actions search and review. |
| `Ctrl+Shift+M` | Open the Extensions marketplace. |
| `Ctrl+Shift+L` | List registered font families. |
| `Ctrl+,` (Windows) / `Ctrl+Shift+,` (Linux/BSD) | Open the configuration file in the configured editor. |
| `Ctrl+Alt+Space` | Toggle the quake window on Windows. |
| `Alt+Shift+T` | Toggle light/dark appearance on Windows and Linux/BSD. |

## macOS defaults

| Shortcut | Action |
|---|---|
| `Cmd+N` | Create an independent OS window. |
| `Cmd+T` / `Cmd+Shift+T` | Create a window tab / local tab in the selected pane. |
| `Cmd+W` | Close the selected local tab, split, or window tab. |
| `Cmd+Shift+W` | Close the current local tab, or the current window-level tab when no extra local tab exists; never remove a split. |
| `Cmd+Alt+W` | Close every other window-level tab. |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous window-level tab. |
| `Cmd+Shift+[` / `Cmd+Shift+]` | Previous / next window-level tab. |
| `Cmd+Alt+[` / `Cmd+Alt+]` | Previous / next local tab in the selected pane. |
| `Cmd+1` … `Cmd+8`; `Cmd+9` | Select window tab 1…8; select the last tab. |
| `Cmd+D` / `Cmd+Shift+D` | Fresh split right / down. |
| `Ctrl+R` / `Ctrl+D` | Clone active session right / down. |
| `Ctrl+Alt+R` / `Ctrl+Alt+D` | Send history search / EOF to the shell. |
| `Cmd+Alt+Arrow` | Select a neighboring pane. |
| `Cmd+]` / `Cmd+[` | Cycle next / previous pane. |
| `Ctrl+Cmd+Arrow` | Resize the selected split. |
| `Cmd+C` / `Cmd+V` | Copy / paste. `Ctrl+C` remains selection-aware as described above. |
| `Cmd+A` | Select all. |
| `Cmd+0`, `Cmd+=` or `Cmd++`, `Cmd+-` | Reset, increase, or decrease font size. |
| `Cmd+F` / `Cmd+B` | Search forward / backward. |
| `Cmd+K` | Clear visible screen and then history. |
| `Ctrl+Cmd+F` | Toggle fullscreen. |
| `Cmd+Alt+I` | Preview selected image. |
| `Cmd+Shift+P` | Open command palette. |
| `Cmd+Shift+H` | Open the read-only Connection Hub. |
| `Cmd+Shift+O` | Open Quick Actions search and review. |
| `Cmd+Shift+M` | Open the Extensions marketplace. |
| `Cmd+Shift+L` | List registered font families. |
| `Cmd+Alt+Shift+T` | Toggle light/dark appearance. |
| `Cmd+,` | Open the configuration file. |
| `Cmd+Q`, `Cmd+H`, `Cmd+Alt+H`, `Cmd+M` | Quit, hide, hide others, minimize. |

Keyboard selection (`Shift+Arrow`, with `Ctrl` for horizontal word motion)
remains platform-neutral. A pointer click without a drag is never reused as
the keyboard anchor. Once a selection exists, an unmodified Arrow key or any
non-empty text/paste/IME input exits selection mode before the input is
forwarded to the shell. Search and Vi mode retain their own input ownership.

## Connection Hub controls (v0.5 release-gated)

These shortcuts work only while the read-only Hub or Connection Review owns
input. They never insert a command or implicit Enter into a terminal PTY.
Current Allow actions stop at the protected-review diagnostic and start no
process.

| Shortcut | Result |
|---|---|
| `L` | From Hub results or first-run setup, open the direct-host editor. If Search owns focus, `l` remains search text. |
| `Tab` / `Shift+Tab` | Move through the active modal focus order; Connection Review includes its Allow once action. |
| `Enter` | Review a valid host or activate the focused editor control; in Connection Review, Allow once only when Review or Allow once owns focus. |
| `A` | Request Allow once from Connection Review. |
| `S` | Request Allow for session from Connection Review. |
| `D` | Deny the managed launch and return to results. |
| `Escape` | Cancel and clear the transient host editor; from review, return to results. |

While the host editor owns the modal, its visible Cancel control replaces the
redundant top-level close icon. This keeps the focus order and pointer targets
unambiguous at small scaled viewports. Modified approval letters remain
available to their existing owners.

## Search mode

| Shortcut | Result |
|---|---|
| `Enter` / `Shift+Enter` | Next / previous result; in Vi search, confirm. |
| `Esc` or `Ctrl+C` | Cancel. |
| `Ctrl+U` | Clear query. |
| `Ctrl+W` | Delete the previous query word. |
| `Ctrl+P` or Up | Previous query from search history. |
| `Ctrl+N` or Down | Next query from search history. |

## Mouse input

- Drag primary/left mouse to select. Double/triple click expands by semantic
  unit/line according to terminal selection rules.
- `Ctrl+C` copies that selection; otherwise it remains interrupt.
- Right-click copies and clears an existing selection. With no selection it
  pastes through the normal bracketed-paste and control filtering path.
- Middle-click pastes the primary selection on platforms that provide it.
- Left-click never pastes; it owns focus, selection, links, local image
  preview, and pane activation.
- Mouse-reporting applications keep mouse ownership. Hold `Shift` for the
  established host-UI override where supported.

## Custom bindings

Bindings live in `config.toml`:

```toml
[bindings]
keys = [
  { key = "F5", with = "control | shift", action = "ReloadConfig" },
  { key = "O", with = "control | alt", action = "OpenCommandPalette" },
  { key = "PageUp", with = "alt", action = "SelectPrevLocalTab" },
]
```

Keys are a printable character, named key (`home`, `end`, `pageup`,
`pagedown`, `insert`, `delete`, `backspace`, `tab`, `enter`, `escape`, arrows,
or `f1`…`f20`), or supported numpad name. Modifiers are `control`, `shift`,
`alt`/`option`, and `super`/`command`, joined with `|`. Modes are `appcursor`,
`appkeypad`, `alt`, and `vi`; prefix a mode with `~` to require its absence.
`esc` sends an explicit escape sequence and takes precedence over `action`.

Stable action names are case-insensitive:

```text
Paste, Copy, SelectAll,
ExtendSelectionLeft, ExtendSelectionRight, ExtendSelectionUp,
ExtendSelectionDown, ExtendSelectionWordLeft, ExtendSelectionWordRight,
SearchForward, SearchBackward, SearchConfirm, SearchCancel, SearchClear,
SearchFocusNext, SearchFocusPrevious, SearchDeleteWord, SearchHistoryNext,
SearchHistoryPrevious, ClearHistory, ClearScreen,
ResetFontSize, IncreaseFontSize, DecreaseFontSize,
CreateWindow, CloseWindow, ReloadConfig, ToggleQuake,
ScrollToPrevPrompt, ScrollToNextPrompt,
CreateTab, CreateLocalTab, MoveCurrentTabToPrev, MoveCurrentTabToNext,
CloseTab, CloseSplitOrTab, CloseUnfocusedTabs, OpenConfigEditor,
SelectPrevTab, SelectNextTab, SelectPrevLocalTab, SelectNextLocalTab,
SelectLastTab, ScrollPageUp, ScrollPageDown, ScrollHalfPageUp,
ScrollHalfPageDown, ScrollToTop, ScrollToBottom,
SplitRight, SplitDown, CloneSplitRight, CloneSplitDown,
SelectNextSplit, SelectPrevSplit, SelectPaneLeft, SelectPaneRight,
SelectPaneUp, SelectPaneDown, SelectNextSplitOrTab,
SelectPrevSplitOrTab, MoveDividerUp, MoveDividerDown, MoveDividerLeft,
MoveDividerRight, ToggleViMode, ToggleAppearanceTheme, ToggleFullscreen,
OpenCommandPalette, OpenConnectionHub, OpenActionCenter,
OpenExtensionMarketplace, OpenFontBrowser, PreviewSelectedImage, ReceiveChar,
None
```

Parameterized actions are `SelectTab(N)`, `Scroll(N)`, and `Run(PROGRAM
ARGS...)`. `Run` currently uses a simple whitespace split, not shell parsing;
do not use it with untrusted values or arguments that require quoting. Unknown
actions are rejected and do not silently remove the matching default.

## Ownership and precedence

Explicit user bindings replace the matching key/modifier/mode trigger. Search,
Vi mode, alternate-screen applications, pinned image browsing, terminal mouse
reporting, and the line editor each have scoped ownership. Automexia never
sends terminal-owned selection motions to the PTY. See
[Architecture](../developer/architecture.md#keyboard-compatibility-boundary) for why one
global shortcut table is not used across every mode and OS.

The four app-surface launchers are inactive while Search, Vi mode, or an
alternate-screen terminal application owns input. They only open application
UI: Connection Hub remains read-only, and Quick Actions still requires its
normal review/insert step. None of these shortcuts writes to or executes in the
PTY. The mnemonic letters are **H**ub, **O**pen actions, **M**arketplace, and
**L**ist fonts.


## Ghostty compatibility policy

Automexia does not use Ghostty shortcuts as its implicit default. The original
Automexia platform mappings were restored by ADR 0011 after user evaluation.
The active shortcuts are documented in [configuration](configuration.md) and
are enforced by binding, palette-label, override, and architecture tests.

Ghostty compatibility remains a future, explicit opt-in profile. Enabling such
a profile must never silently rewrite an existing configuration or replace the
`automexia` default. The implementation must provide a typed binding registry,
versioned upstream fixtures, atomic reload, collision diagnostics, generated
documentation, and clear handling for unsupported Ghostty actions before the
profile can be advertised.

The audited comparison baseline remains Ghostty commit
[`d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0`](https://github.com/ghostty-org/ghostty/tree/d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0),
but no binding from that baseline is automatically installed merely because it
exists in Ghostty. The complete implementation and verification sequence is in
the [Ghostty compatibility roadmap](../project/roadmap.md).

### Deliberate classic differences

| Automexia default | Action | Strict Ghostty profile behavior, if implemented |
|---|---|---|
| `Ctrl`+`T` | new window-level tab | forwarded to the shell |
| `Ctrl`+`Shift`+`T` | new pane-local independent tab | new window-level tab |
| `Ctrl`+`Shift`+`R` / `Ctrl`+`Shift`+`D` | fresh right/down split | Ghostty uses `Ctrl`+`Shift`+`O` / `Ctrl`+`Shift`+`E` |
| `Ctrl`+`Shift`+`O` | open reviewed Quick Actions | Ghostty uses it for a fresh split; an explicit future Ghostty profile must replace this trigger atomically |
| `Ctrl`+`R` / `Ctrl`+`D` | clone active session right/down | forwarded to history search / EOF |
| `Ctrl`+`Alt`+`R` / `Ctrl`+`Alt`+`D` | explicit history-search / EOF passthrough | available for profile-specific actions |
| `Alt`+Arrow | focus the nearest pane geometrically | geometric split focus uses profile-specific chords |
| `F6` / `Shift`+`F6` | cycle panes in visual order | available as configurable split actions |
| `Alt`+`PageDown` / `Alt`+`PageUp` | next/previous tab inside the selected pane | Ghostty has no Automexia pane-local-tab scope |
| `Ctrl`+`Tab` / `Ctrl`+`Shift`+`Tab` | next/previous window-level tab | next/previous tab |

macOS retains the classic `Cmd`+`T` window tab, `Cmd`+`Shift`+`T` pane-local
tab, and `Cmd`+`D` / `Cmd`+`Shift`+`D` fresh splits. Session cloning remains on
`Ctrl`+`R` / `Ctrl`+`D` on every platform. It uses `Cmd`+`Alt`+Arrow for
geometric pane focus and `Cmd`+`Alt`+`]` / `Cmd`+`Alt`+`[` for pane-local tab
navigation.

### Policy

- `automexia` remains the only implicit profile for v0.4.
- A future Ghostty profile is selected explicitly and is versioned.
- User bindings remain authoritative over any profile defaults.
- Normal startup, builds, and tests never execute Ghostty or access its files.
- Compatibility claims require generated fixtures and host-independent tests.
- Missing Ghostty actions stay unbound; they are never mapped to an unrelated
  Automexia action merely to increase shortcut counts.
