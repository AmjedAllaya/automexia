# Keyboard and input reference

## Keyboard links

The classic default `Ctrl+Alt+O` starts link review. Tab/Shift+Tab or Up/Down
navigate; labels select; Enter activates; Ctrl+Shift+C copies the destination;
Left/Right inspect long targets; Escape returns to the shell. See
[hyperlinks](user-guide/hyperlinks.md) for configuration and safety limits.

This page defines Automexia's active v0.4 defaults. Explicit entries under
`[bindings]` replace matching default triggers. The separate
[Ghostty compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md) documents the
explicit opt-in profiles; they never replace the active default implicitly.

`Cmd` means the macOS Command key. Search and Vi-mode bindings apply only while
their mode is active. A terminal application can own a key when the table says
the action is mode-sensitive.

## Windows and Linux/BSD defaults

**Ctrl+Shift+S** opens the native Settings sheet on Windows/Linux/BSD;
**Cmd+Shift+S** does so on macOS. The action is `OpenSettings`, applies outside
Search, Vi and alternate-screen modes, and can be customized in the palette.
The existing configuration-editor shortcut remains unchanged.

**Ctrl+Shift+F7** opens the [focused table view](user-guide/table-output.md)
with the Automexia profile on all platforms, including macOS. The configurable
action is `ViewTableOutput`; Escape returns to the terminal. The command is
also available under the command palette's Tools category.

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
| `Alt+Shift+R` / `Alt+Shift+D` | Fresh default-shell split right / down. Add Shift to start fresh. |
| `Ctrl+R` / `Ctrl+D` | Native shell controls: history search / delete-or-EOF, depending on the shell and editing mode. |
| `Alt+R` / `Alt+D` | Clone the active shell/profile/directory into an independent session right / down. |
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
| `Ctrl+Shift+C` | Copy on Windows/Linux/BSD. |
| `Ctrl+V` / `Ctrl+Shift+V` | Paste with the Automexia Windows profile in local, WSL, SSH, and other sessions. |
| `Ctrl+Shift+V` | Paste on Linux/BSD. |
| `Shift+Insert` | Paste the primary selection when the platform provides one. |
| `Shift+Arrow` | Start at the terminal insertion cursor, then extend/reverse selection by one cell or row. |
| `Ctrl+Shift+Left/Right` | Extend/reverse selection by a Unicode word boundary. |
| `Ctrl+Shift+A` | Select all on Windows. |
| `Ctrl+0`, `Ctrl+=` or `Ctrl++`, `Ctrl+-` | Clear the saved override, increase, or decrease the application-wide font size; the result is restored next launch. |
| `Shift+Home/End` | Scroll to history top / bottom outside the alternate screen. |
| `Shift+PageUp/PageDown` | Scroll one page up / down outside the alternate screen. |
| `Ctrl+Shift+Up/Down` | Jump to the previous / next shell-integrated command in the selected pane. |
| `Ctrl+F` | Select or refocus the current-pane scope in the active search session. |
| `Ctrl+Shift+F` / `Ctrl+Shift+B` | Select or refocus the all-visible-panes scope in the active search session. |
| `Ctrl+Shift+Space` | Toggle Vi mode on Windows. |
| `Alt+Shift+Space` | Toggle Vi mode on Linux/BSD and all platforms through the common binding. |
| `Ctrl+Shift+K` | Clear history on Windows. |
| `F11` or `Alt+Enter` | Toggle fullscreen on Windows. |
| `Ctrl+Alt+I` | Preview the selected or pointer-targeted local raster image. |
| `Ctrl+Shift+P` | Open the command palette. |
| `Ctrl+Shift+L` | List registered font families. |
| `Ctrl+,` (Windows) / `Ctrl+Shift+,` (Linux/BSD) | Open the configuration file in the configured editor. |
| `Ctrl+Alt+Space` | Toggle the quake window on Windows. |
| `Alt+Shift+T` | Toggle light/dark appearance on Windows and Linux/BSD. |

Automexia's Windows `Ctrl+V` is a host shortcut, so the clipboard reaches the selected
pane through the same bracketed-paste and control-filtering path for every child
session without synthesizing an extra Enter. To let a terminal application receive the
original control character instead, add:

```toml
[bindings]
keys = [{ key = "V", with = "control", action = "ReceiveChar" }]
```

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
| `Ctrl+R` / `Ctrl+D` | Native shell controls: history search / delete-or-EOF, depending on the shell and editing mode. |
| `Cmd+Alt+Shift+R` / `Cmd+Alt+Shift+D` | Clone active session right / down with an independent PTY. |
| `Cmd+Alt+Arrow` | Select a neighboring pane. |
| `Cmd+]` / `Cmd+[` | Cycle next / previous pane. |
| `Ctrl+Cmd+Arrow` | Resize the selected split. |
| `Cmd+C` / `Cmd+V` | Copy / paste. `Ctrl+C` remains selection-aware as described above. |
| `Cmd+A` | Select all. |
| `Cmd+0`, `Cmd+=` or `Cmd++`, `Cmd+-` | Clear the saved override, increase, or decrease the application-wide font size; the result is restored next launch. |
| `Cmd+F` / `Cmd+B` | Select or refocus the current-pane scope in the active search session. |
| `Cmd+Shift+F` / `Cmd+Shift+B` | Select or refocus the all-visible-panes scope in the active search session. |
| `Cmd+Shift+Up` / `Cmd+Shift+Down` | Jump to the previous / next marked command in the selected pane's scrollback. |
| `Cmd+K` | Clear visible screen and then history. |
| `Ctrl+Cmd+F` | Toggle fullscreen. |
| `Cmd+Alt+I` | Preview selected image. |
| `Cmd+Shift+P` | Open command palette. |
| `Cmd+Shift+L` | List registered font families. |
| `Cmd+Alt+Shift+T` | Toggle light/dark appearance. |
| `Cmd+,` | Open the configuration file. |
| `Cmd+Q`, `Cmd+H`, `Cmd+Alt+H`, `Cmd+M` | Quit, hide, hide others, minimize. |

Keyboard selection (`Shift+Arrow`, with `Ctrl` for horizontal word motion)
remains platform-neutral. A pointer click without a drag is never reused as
the keyboard anchor. Once a selection exists, an unmodified Arrow key or any
non-empty text/paste/IME input exits selection mode before the input is
forwarded to the shell. Search and Vi mode retain their own input ownership.

## Command palette

`Ctrl+Shift+P` (`Cmd+Shift+P` on macOS) opens six categories: Tabs & Windows,
Panes & Sessions, Search & History, Clipboard & Input, Appearance, and Tools.
Type to search all commands, even inside a category; category names also match.
Clear the query to return to that category. No match leaves the palette open.

| Control | Result |
|---|---|
| Up / Down, Tab / Shift+Tab | Select the previous / next row without executing. |
| Enter or click | Open the selected category or explicitly activate the selected command. |
| Right | Open a selected category only. |
| Fixed Back button, Back row, Alt+Left, or Backspace with an empty query | Return to categories and restore the category selection. |
| Home / End; PageUp / PageDown | First / last row; previous / next page. |
| Mouse wheel / trackpad | Scroll the palette, keeping selection visible. |
| Esc | Close without executing; reviewed Quick Action detail retains its existing Back behavior. |

The palette fits its contents and available height. Overflow retains a vertical
indicator. Query input is limited to 4 KiB, stays in the palette and never reaches
the PTY. Holding Enter across a category transition does not run its first action.
The header Back button stays visible when the list scrolls; narrow panes use
its arrow-only variant. Font and extension browsing support Back or Alt+Left to
return to their parent category with the originating command selected.

The pane mnemonics are **R = right**, **D = down**, **Shift = fresh** on
Windows/Linux/BSD. These replace Alt+Shift+Plus/Minus and the previous
Alt+Shift+R/D clone defaults in current source only. Ctrl+R/D still belong to the
shell. Alt+R/D now belong to pane creation in normal terminal mode; Search, Vi
and alternate-screen applications keep those keys. macOS Command defaults and
opt-in Ghostty profiles are unchanged. Explicit custom mappings remain authoritative.

All palette commands now use effective bindings after configuration loading.
Shortcut chips show only the actual keys, without Legacy, Profile or User source
badges. Typed profile/user mappings retain precedence. Removed, unavailable or
conditional direct keys show `Enter`: select that palette action and press Enter.
This is palette-local activation, not a new global binding. A shortcut that is
conditional or shadowed is not presented as
an unconditional launch key. Search-only Shift+Enter is not a global shortcut,
and clearing history is not the same action as clearing screen and history.
Every catalog action has a direct classic default when navigation and splits are
enabled. Explicit removals and strict compatibility profiles remain authoritative.
To restore the earlier
Ctrl+Shift fresh-split preference explicitly:

```toml
[bindings]
keys = [
  { key = "R", with = "control | shift", action = "SplitRight" },
  { key = "D", with = "control | shift", action = "SplitDown" },
]
```

Alt+D and Alt+R normally perform word deletion and line restoration in common
shell editors. This is an intentional, user-approved tradeoff for simpler pane
shortcuts. To restore shell ownership, add these entries to your existing keys list:

```toml
{ key = "R", with = "alt", action = "ReceiveChar" },
{ key = "D", with = "alt", action = "ReceiveChar" },
```

No settings file is rewritten. See [shortcut audit and decision](adr/0051-mnemonic-pane-shortcuts-and-honest-discovery.md)
for platform rationale, retained controls and native verification limits.

## Search mode

Pane and workspace search are two scopes of one continuous session. `Ctrl+F` /
`Cmd+F` immediately selects the current pane; `Ctrl+Shift+F` / `Cmd+Shift+F`
immediately expands to the active local tab in every visible split. Switching
keeps the query and query focus, recompiles the match set, and moves the same
surface between the pane footer and the safe bottom-centered workspace position.
Repeating the shortcut for the active scope only refocuses the query. Hidden
pane-local tabs and other window-level tabs remain deliberately excluded.

The `PANE` and `ALL PANES` controls are separate, mutually exclusive choices.
They are clickable, and `Tab` / `Shift+Tab` moves between the query and scope
group. With the scope group focused, any arrow key selects the other scope;
`Space` or `Enter` keeps the selected scope. A pointer selection returns focus
to the query. `Esc` still closes the session. Moving focus to another pane
closes a pane-local search instead of silently changing its pane owner.

The result badge reports matches in the visible viewport of the routes in
scope, capped at `999+`; previous/next navigation still uses the terminal
search engine beyond the viewport. Invalid patterns receive an explicit status.
The complete painted surface consumes pointer input, and query/control input is
limited to 4 KiB of UTF-8 and never reaches the PTY.

| Shortcut | Result |
|---|---|
| `Enter` / `Shift+Enter` | Next / previous result when the query is focused; in Vi search, confirm. |
| `Tab` / `Shift+Tab` | Move focus between the query and scope choices. |
| Arrow key with scope focused | Switch between `PANE` and `ALL PANES`. |
| `Space` / `Enter` with scope focused | Keep the selected scope. |
| `Esc` or `Ctrl+C` | Cancel. |
| `Ctrl+U` | Clear query. |
| `Ctrl+W` | Delete the previous query word. |
| `Ctrl+P` or Up with query focused | Previous query from search history. |
| `Ctrl+N` or Down with query focused | Next query from search history. |

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
SearchForward, SearchBackward, SearchGlobalForward, SearchGlobalBackward, SearchConfirm, SearchCancel, SearchClear,
SearchFocusNext, SearchFocusPrevious, SearchDeleteWord, SearchHistoryNext,
SearchHistoryPrevious, ClearHistory, ClearScreen,
ResetFontSize, IncreaseFontSize, DecreaseFontSize,
CreateWindow, CloseWindow, ReloadConfig, ToggleQuake,
ScrollToPrevPrompt, ScrollToNextPrompt,
CreateTab, CreateLocalTab, MoveCurrentTabToPrev, MoveCurrentTabToNext,
CloseTab, CloseSplitOrTab, CloseUnfocusedTabs, OpenConfigEditor, OpenSettings,
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

Current source defaults leave Ctrl+R/Ctrl+D to the shell. The published 0.4.0
package predates this correction. Clone actions remain in the command palette
and can be assigned explicitly using `CloneSplitRight` / `CloneSplitDown`.
Existing user mappings are preserved; remove a custom mapping to regain shell
ownership. Ctrl+Alt+R/Ctrl+Alt+D are no longer rewritten as bare control bytes.
See [ADR 0041](adr/0041-shell-owned-history-and-eof-shortcuts.md).

Explicit user bindings replace the matching key/modifier/mode trigger. Search,
Vi mode, alternate-screen applications, pinned image browsing, terminal mouse
reporting, and the line editor each have scoped ownership. Automexia never
sends terminal-owned selection motions to the PTY. See
[Architecture](ARCHITECTURE.md#keyboard-compatibility-boundary) for why one
global shortcut table is not used across every mode and OS.

Application-surface shortcuts are inactive while Search, Vi mode, or an
alternate-screen terminal application owns input. They open application UI
only and never write to or execute in the PTY.
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

- [Ghostty 1.3 keybindings](generated/ghostty-1.3-keybindings.md)
- [Ghostty 1.3 actions](generated/ghostty-1.3-actions.md)

See [Ghostty keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md) for
configuration, migration, provenance, deviations, and current native-evidence
limits.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Connection Hub Controls V05 Release Gated

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.
