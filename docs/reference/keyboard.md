# Keyboard and input reference

This page defines Automexia's active v0.4 defaults. Explicit entries under
`[bindings]` replace matching default triggers. The separate
[Ghostty compatibility](../GHOSTTY-KEYBOARD-COMPATIBILITY.md) documents the
explicit opt-in profiles; they never replace the active default implicitly.

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
| `L` | From Hub results or first-run setup, open the typed host/user/port editor. If Search owns focus, `l` remains search text. |
| `Tab` / `Shift+Tab` | Move through the active modal focus order; Connection Review includes its Allow once action. |
| `Enter` | Review a valid host or activate the focused editor control; in Connection Review, Allow once only when Review or Allow once owns focus. |
| `A` | Request Allow once from Connection Review. |
| `S` | Request Allow for session from Connection Review. |
| `D` | Deny the managed launch and return to results. |
| `C` | Copy the exact reviewed SSH command; no execution, newline, or implicit Enter. |
| `Escape` | Cancel and clear the transient host editor; from review, return to results. |

While the host editor owns the modal, Tab/Shift+Tab cycles Host, User, Port,
Review, and Cancel. Its visible Cancel control replaces the redundant top-level
close icon, keeping focus and pointer targets unambiguous at small scaled
viewports. Modified approval/copy letters remain available to their existing
owners.

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
normal review/insert step. A published CP4 production-context row adds a second
confirmation; broker-required or non-current provider rows cannot copy or
insert. None of these shortcuts writes to or executes in the PTY. The mnemonic
letters are **H**ub, **O**pen actions, **M**arketplace, and
**L**ist fonts.

## Ghostty compatibility profiles

`automexia` remains the implicit profile. Set `keyboard.binding-profile` to
`ghostty-1.3` for the pinned Ghostty 1.3.1 profile or to `ghostty` for the
visible moving alias, which currently resolves to `ghostty-1.3`. User entries
and `unbind` directives are compiled after the selected profile. A failed strict
compile or reload leaves the previous complete registry active.

Strict Ghostty bindings use window-level tabs and independent-PTY splits. Bare
`Ctrl+R` and `Ctrl+D` fall through to the terminal. Platform-global entries are
installed through the OS hotkey owner, while focused, all-surface, sequences,
tables, chains, `performable`, and `unconsumed` policies stay isolated per
surface. Unsupported actions remain visibly unavailable; they are never mapped
to unrelated behavior.

The generated tables are the canonical inventory:

- [Ghostty 1.3 keybindings](../generated/ghostty-1.3-keybindings.md)
- [Ghostty 1.3 actions](../generated/ghostty-1.3-actions.md)

See [Ghostty keyboard compatibility](../GHOSTTY-KEYBOARD-COMPATIBILITY.md) for
configuration, migration, provenance, deviations, and current native-evidence
limits.
