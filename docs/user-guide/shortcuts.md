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

## Workspace shortcuts

| Action | Windows / Linux / BSD | macOS |
|---|---|---|
| New OS window | `Ctrl+Shift+N` | `Cmd+N` |
| New window-level tab | `Ctrl+T` | `Cmd+T` |
| New local tab in selected pane | `Ctrl+Shift+T` | `Cmd+Shift+T` |
| Close selected local tab/split/window tab | `Ctrl+Shift+W` | `Cmd+W` |
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

`Ctrl+Shift+C` and `Ctrl+Shift+V` are the explicit copy/paste bindings on Windows/Linux/BSD.

On macOS, use `Cmd+C` / `Cmd+V` for normal copy/paste. `Ctrl+C` remains available for shell/application semantics when no terminal-owned behavior takes precedence.

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
- Full-screen terminal applications that enable mouse reporting keep mouse ownership. Hold `Shift` for the established host-UI override where supported.

## Search and scrollback

| Action | Windows/Linux/BSD | macOS |
|---|---|---|
| Search forward | `Ctrl+Shift+F` | `Cmd+F` |
| Search backward | `Ctrl+Shift+B` | `Cmd+B` |
| Scroll to history top / bottom | `Shift+Home` / `Shift+End` | Use registered action/palette if no preferred custom chord |
| Scroll a page | `Shift+PageUp` / `Shift+PageDown` | Use registered action/palette if no preferred custom chord |

While search mode is open:

| Key | Result |
|---|---|
| `Enter` / `Shift+Enter` | Next / previous match |
| `Esc` or `Ctrl+C` | Cancel |
| `Ctrl+U` | Clear the query |
| `Ctrl+W` | Delete the previous query word |
| `Ctrl+P` or Up | Previous search query |
| `Ctrl+N` or Down | Next search query |

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
- `Alt+Shift+T` to toggle light/dark appearance.

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
