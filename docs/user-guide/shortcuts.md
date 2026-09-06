# Shortcuts and input

This page is the practical shortcut guide: the keys users reach for every day, how they differ by platform, and when the command palette or mouse is a better approach. The exhaustive binding/action list remains in [Keyboard and input reference](../reference/keyboard.md).

## Choose an input approach

| Approach | Best for |
|---|---|
| **Keyboard shortcuts** | Frequent actions where speed and muscle memory matter |
| **Command palette** | Discovering an action or using something you rarely invoke |
| **Mouse** | Direct tab/pane selection, text selection, resizing/window movement, and image hover/click |
| **Custom binding** | A frequent action whose default chord conflicts with your workflow |

Open the command palette with `Ctrl+Shift+P` on Windows/Linux/BSD or `Cmd+Shift+P` on macOS. If you forget a key chord, use the palette rather than guessing.

## Feature launchers

| Feature | Windows / Linux / BSD | macOS |
|---|---|---|
| **L**ist fonts | `Ctrl+Shift+L` | `Cmd+Shift+L` |
| Toggle appearance **T**heme | `Alt+Shift+T` | `Cmd+Alt+Shift+T` |

Application UI shortcuts do not take over Search, Vi mode, or alternate-screen
terminal applications and never write to or execute in the PTY.

## Workspace shortcuts

| Action | Windows / Linux / BSD | macOS |
|---|---|---|
| New OS window | `Ctrl+Shift+N` | `Cmd+N` |
| New window-level tab | `Ctrl+T` | `Cmd+T` |
| New local tab in selected pane | `Ctrl+Shift+T` | `Cmd+Shift+T` |
| Close selected local tab/split/window tab | `Ctrl+Shift+W` | `Cmd+W` |
| Close current tab without removing a split | `Ctrl+F4` | `Cmd+Shift+W` |
| Close other window-level tabs | `Ctrl+Shift+F4` | `Cmd+Alt+W` |
| Next / previous window tab | `Ctrl+Tab` / `Ctrl+Shift+Tab` | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| Next / previous local tab | `Alt+PageDown` / `Alt+PageUp` | `Cmd+Alt+]` / `Cmd+Alt+[` |
| Fresh split right / down | `Ctrl+Shift+R` / `Ctrl+Shift+D` | `Cmd+D` / `Cmd+Shift+D` |
| Clone current launch context right / down | `Ctrl+R` / `Ctrl+D` | `Ctrl+R` / `Ctrl+D` |
| Geometric pane focus | `Alt+Arrow` | `Cmd+Alt+Arrow` |
| Cycle next / previous pane | `F6` / `Shift+F6` | `Cmd+]` / `Cmd+[` |

### A key difference: fresh vs clone

`Ctrl+Shift+R` / `Ctrl+Shift+D` on Windows/Linux/BSD creates a **fresh default-shell split**. `Ctrl+R` / `Ctrl+D` creates an independent split that **clones the active launch profile/directory**.

Because `Ctrl+R` and `Ctrl+D` are traditionally shell control keys, their original shell bytes remain available as `Ctrl+Alt+R` and `Ctrl+Alt+D` when you need history-search/EOF passthrough.

On macOS, fresh splits use native-style `Cmd+D` / `Cmd+Shift+D`; cloning still uses `Ctrl+R` / `Ctrl+D`.

## Resize panes

| Platform | Resize selected split |
|---|---|
| Windows | `Alt+Shift+Arrow` |
| Linux/BSD | `Ctrl+Alt+Shift+Arrow` |
| macOS | `Ctrl+Cmd+Arrow` |

Use the mouse/divider when you are making a large one-time resize. Use the keyboard when making small controlled adjustments while staying in the terminal flow.

## Clipboard and selection

### The selection-aware `Ctrl+C`

On Windows/Linux/BSD, `Ctrl+C` has a deliberate dual behavior:

- if Automexia has a non-empty terminal selection, it copies that selection;
- if there is no selection, it sends the normal interrupt to the shell/application.

`Ctrl+Shift+C` is the explicit copy binding on Windows/Linux/BSD. With the
Automexia Windows profile, both `Ctrl+V` and `Ctrl+Shift+V` paste into the selected pane, whether it
hosts PowerShell, Command Prompt, WSL, SSH, or another terminal session.
Linux/BSD keeps `Ctrl+Shift+V` as its paste binding.

On macOS, use `Cmd+C` / `Cmd+V` for normal copy/paste. `Ctrl+C` remains available for shell/application semantics when no terminal-owned behavior takes precedence.

Paste always uses Automexia's existing clipboard filtering and bracketed-paste
path and does not synthesize an extra Enter. If a Windows terminal application such as Vim
needs to receive `Ctrl+V` itself, restore terminal ownership explicitly:

```toml
[bindings]
keys = [
  { key = "V", with = "control", action = "ReceiveChar" },
]
```

### Keyboard selection

Use:

- `Shift+Arrow` to start/extend/reverse selection by cell or row.
- `Ctrl+Shift+Left/Right` to move the active end by a Unicode-aware word boundary.

Once a selection exists, an Arrow without `Shift`, printable input, paste, or IME commit clears terminal selection before the input goes to the shell. Search and Vi mode keep their own input ownership.

### Mouse behavior

- Primary/left drag selects text.
- Double/triple click expands selection according to terminal semantic-unit/line rules.
- Right-click copies and clears an existing selection; with no selection it pastes through normal paste filtering.
- Middle-click pastes the primary selection on platforms that provide one.
- Left-click never pastes; it is reserved for focus, selection, links, image previews, and pane activation.
- Rotating the mouse wheel or starting a trackpad scroll selects the pane under
  the pointer and sends that initiating scroll to it; pointer hover alone does
  not change pane selection.
- Full-screen terminal applications that enable mouse reporting keep mouse ownership. Hold `Shift` for the established host-UI override where supported.

## Search and scrollback

| Action | Windows/Linux/BSD | macOS |
|---|---|---|
| Find in selected pane | `Ctrl+F` | `Cmd+F` |
| Find backward in selected pane | Use `Shift+Enter` while search is open | `Cmd+B` |
| Search all visible panes | `Ctrl+Shift+F` | `Cmd+Shift+F` |
| Search all visible panes backward | `Ctrl+Shift+B` | `Cmd+Shift+B` |
| Scroll to history top / bottom | `Shift+Home` / `Shift+End` | Use registered action/palette if no preferred custom chord |
| Scroll a page | `Shift+PageUp` / `Shift+PageDown` | Use registered action/palette if no preferred custom chord |
| Jump to previous / next command | `Ctrl+Shift+Up` / `Ctrl+Shift+Down` | `Cmd+Shift+Up` / `Cmd+Shift+Down` |

Command jumping moves only the selected pane's viewport between trusted OSC
133 prompt marks. It does not recall, edit, rerun, or send any bytes to a
command. At the first or last retained command it stays put. Search, Vi mode,
and alternate-screen applications retain their normal key ownership. If a
custom shell does not publish semantic prompt marks, use ordinary scrollback or
enable the supported session-only shell integration; Automexia does not guess
prompt boundaries from terminal text.

`Ctrl+F` / `Cmd+F` and `Ctrl+Shift+F` / `Cmd+Shift+F` switch one
continuous search session between the current pane and all visible panes. The
query and query focus stay in place, matches are recalculated, and the same
surface moves between the selected pane's footer and the safe bottom-centered
workspace position. Repeating the active scope shortcut only refocuses the
query. An exceptionally narrow pane uses the safe position without changing
its pane ownership. All-pane search covers only the active local tab in every
visible split; hidden tabs are not activated.

`PANE` and `ALL PANES` are clickable, mutually exclusive choices. Pointer
selection returns focus to the query. The result badge counts visible matches
across the current scope and caps display work at `999+`; next/previous
navigation can continue through scrollback. The complete surface captures
pointer and keyboard input, so no search interaction falls through to window
controls or sends bytes or an implicit Enter to the shell.

While search mode is open:

| Key | Result |
|---|---|
| `Enter` / `Shift+Enter` with query focused | Next / previous match |
| `Tab` / `Shift+Tab` | Move between query and scope focus |
| Any arrow with scope focused | Switch `PANE` / `ALL PANES` |
| `Space` / `Enter` with scope focused | Keep the selected scope |
| `Esc` or `Ctrl+C` | Cancel |
| `Ctrl+U` | Clear the query |
| `Ctrl+W` | Delete the previous query word |
| `Ctrl+P` or Up with query focused | Previous search query |
| `Ctrl+N` or Down with query focused | Next search query |

## Font and view

| Action | Windows/Linux/BSD | macOS |
|---|---|---|
| Reset font size | `Ctrl+0` | `Cmd+0` |
| Increase font size | `Ctrl+=` / `Ctrl++` | `Cmd+=` / `Cmd++` |
| Decrease font size | `Ctrl+-` | `Cmd+-` |
| Fullscreen | Windows: `F11` or `Alt+Enter` | `Ctrl+Cmd+F` |
| Open config | Windows: `Ctrl+,`; Linux/BSD: `Ctrl+Shift+,` | `Cmd+,` |
| Command palette | `Ctrl+Shift+P` | `Cmd+Shift+P` |
| Preview selected image | `Ctrl+Alt+I` | `Cmd+Alt+I` |

Pane font zoom is runtime/pane-local. Reset returns to the configured font size.

## Windows-only convenience defaults

The current Windows defaults also include:

- `Ctrl+1` … `Ctrl+8` for window tabs 1…8 and `Ctrl+9` for the last tab;
- `Ctrl+PageUp/PageDown` for previous/next window tab;
- `Ctrl+Shift+PageUp/PageDown` to reorder a window tab;
- `Ctrl+Shift+A` to select all;
- `Ctrl+Shift+Space` for Vi mode;
- `Ctrl+Shift+K` to clear history;
- `Ctrl+Alt+Space` to toggle the quake window;
- `Alt+Shift+T` to toggle light/dark appearance (also available on Linux/BSD).

Linux/BSD additionally supports `Ctrl+Shift+[` / `Ctrl+Shift+]` for previous/next window tab and uses `Alt+Shift+Space` for the common Vi-mode toggle.

## macOS application shortcuts

The native defaults include:

- `Cmd+Q` quit;
- `Cmd+H` hide;
- `Cmd+Alt+H` hide others;
- `Cmd+M` minimize;
- `Cmd+A` select all;
- `Cmd+K` clear visible screen then history;
- `Cmd+Shift+[` / `Cmd+Shift+]` previous/next window tab;
- `Cmd+1` … `Cmd+8`, `Cmd+9` direct window-tab selection.

## Image-preview navigation

To preview a local image path:

1. Hover a supported filename/path for a temporary card, or click it to pin.
2. When pinned, use `Down`/`Right` for the next visible image path and `Up`/`Left` for the previous one.
3. Press `Esc` to close the pinned preview.
4. Alternatively, select a path and press `Ctrl+Alt+I` / `Cmd+Alt+I`.

See [Files, output, and images](files-and-images.md) for the three different image approaches.

## Custom shortcuts

Add explicit bindings under `[bindings]` in `config.toml`:

```toml
[bindings]
keys = [
  { key = "F5", with = "control | shift", action = "ReloadConfig" },
  { key = "O", with = "control | alt", action = "OpenCommandPalette" },
  { key = "PageUp", with = "alt", action = "SelectPrevLocalTab" },
]
```

Explicit user bindings replace matching default triggers. Unknown actions are rejected rather than silently disabling a default.

Use a custom binding when the action is frequent enough to justify muscle memory. For occasional actions, the command palette is easier to maintain and avoids unnecessary shortcut collisions.

The complete action-name list, mode syntax, key names, and platform defaults are in [Keyboard and input reference](../reference/keyboard.md).
