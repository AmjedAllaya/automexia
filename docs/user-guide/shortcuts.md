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

## Customize a shortcut in the palette

In current source builds, open the command palette, find a command, and either
**double-click its key badge** or select the command and press **F2**. A single
click on the badge selects the row without running the command. Categories and
provider-generated items are not editable shortcuts; open a category first.

1. Press the desired key combination. The editor shows it for review; recording
   never runs the command. Use Ctrl, Alt or Command with a letter, number or
   navigation key, or a function key (F1–F20, with modifiers if desired).
2. Resolve any conflict shown in the editor. Existing commands, typed sequence
   prefixes, explicit unbinds and custom text bindings are protected. Choose a
   different combination instead of silently taking another command's key.
3. Press **Enter** or click **Save**. The shortcut changes in all current windows
   without restarting sessions. Wait for **Saved** before relying on it after
   restart. **Done** returns to the same palette query and selected command.
4. **Esc** or **Cancel** discards an unsubmitted recording. Once Save has been
   submitted, **Back/Esc** leaves the editor without undoing the submitted change.
   **Reset** removes this command's UI override and restores its underlying
   config/profile binding; it does not necessarily restore the factory default.

Use Tab/Shift+Tab to move through recording, Save, Reset and Cancel. Key repeat
cannot save or activate a command. Focus loss or a binding reload pauses recording;
press the combination again. IME composition, dropped files, paste and mouse-wheel
events do not become terminal input while the palette owns focus. A binding change
in another window invalidates a queued edit before it can overwrite that change.

The editor reserves Enter, Esc and Tab for its controls, leaves bare text entry
alone, and rejects Ctrl+C/D/R/Z to protect shell interrupt, EOF, history and suspend.
It edits one normal-terminal chord per catalog command, not global OS shortcuts,
multi-step sequences, action chains, modal key tables or terminal application keys.
Advanced or conditional typed bindings must be edited in `config.toml`.

Saved shortcuts live in the existing private UI preference overlay; `config.toml`
is never rewritten. UI overrides take priority in normal mode only. Search, Vi
mode and alternate-screen applications retain their existing mappings. If an
external config edit makes a stored override incompatible, live reload keeps the
last good configuration. On restart, incompatible UI bindings are temporarily
ignored with a settings warning, leaving the stored data recoverable. Resolve the
config conflict or remove the conflicting `shortcuts` entries from the UI
preferences file while Automexia is closed. Other preference fields need not change.

A failed write is reported as **active this session only**, with Enter to retry.
Check writable configuration storage and available disk space. Reset failures
retry the reset, not an earlier recorded combination. The existing writer is
bounded and atomic; separate application processes can still save last-writer-wins
snapshots, so avoid editing preferences concurrently in independent processes.

Automexia cannot guarantee a combination is unused by every OS, desktop, driver
overlay, keyboard layout or shell. Keys intercepted outside the app never reach
the recorder. Alt and Ctrl+Alt combinations show relevant warnings. Test the
selected key with your desktop and keyboard layout before relying on it.

This editor is not part of the already-published 0.4.0 package. Native desktop
pixels, input-method behavior and screen-reader delivery require the validation
described in [Testing](../TESTING.md#shortcut-editor-assurance).

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
| Fresh split right / down | `Alt+Shift+R` / `Alt+Shift+D` | `Cmd+D` / `Cmd+Shift+D` |
| Clone current launch context right / down | `Alt+R` / `Alt+D` | `Cmd+Alt+Shift+R` / `Cmd+Alt+Shift+D` |
| Geometric pane focus | `Alt+Arrow` | `Cmd+Alt+Arrow` |
| Cycle next / previous pane | `F6` / `Shift+F6` | `Cmd+]` / `Cmd+[` |

### A key difference: fresh vs clone

`Alt+R` / `Alt+D` on Windows/Linux/BSD clones an independent session with the active launch profile/directory. Add `Shift` for a **fresh default-shell split**. Remember **R = right, D = down, Shift = fresh**. The same commands live under **Panes & Sessions** in the grouped palette; typing searches globally. Back uses a left arrow in both the fixed header and the list.

These Alt-letter defaults replace common shell word-deletion/line-restoration
bindings in normal terminal mode. Search, Vi and alternate-screen applications
retain those keys. See [migration and shell-key recovery](../KEYBOARD.md#command-palette).

Current source leaves `Ctrl+R` and `Ctrl+D` to the shell for history and delete/EOF behavior. The published 0.4.0 package predates this correction. Existing custom bindings remain authoritative; remove an explicit clone binding if you want native shell input. Ctrl+Alt variants are no longer rewritten to bare controls.

On macOS, fresh splits retain `Cmd+D` / `Cmd+Shift+D`; clone with `Cmd+Alt+Shift+R/D`. Custom bindings remain authoritative. See [migration and palette controls](../KEYBOARD.md#command-palette).

### Complete shortcuts and clean labels

Shortcut chips show the keys only. When a custom binding is removed, shadowed or
absent from a compatibility profile, `Enter` means select the command in the
palette and press Enter; it does not recreate a global shortcut.

| Action | Windows / Linux / BSD | macOS |
|---|---|---|
| Quit application | `Ctrl+Shift+Q` | `Cmd+Q` |
| Find backward in pane | `Alt+Shift+B` | `Cmd+B` |
| Clear screen **and history** | `Ctrl+Alt+K` | `Cmd+Alt+K` |
| Fullscreen | `F11` | `Ctrl+Cmd+F` |

Quit and clear-screen defaults on Windows/Linux are suppressed in Search, Vi
and alternate-screen applications. Clear-screen has the same safeguards on
macOS; its existing Cmd+Q behavior is unchanged. `Esc` keeps cancel/back and
normal terminal input ownership; it never becomes an application Quit default.
Clearing screen and history removes retained output; it does not send a shell
command. Existing history-only shortcuts and explicit overrides are unchanged.
Ctrl+Alt combinations can interact with AltGr layouts; rebind the clear action
or activate it with Enter in the palette if necessary.

External overlays may intercept shortcuts before Automexia receives them. NVIDIA
uses Alt+R and Alt+Shift+R for statistics. To preserve the pane keys, change those
in NVIDIA's Alt+Z > Settings > Shortcuts; the Statistics settings also own the
external overlay position. See [NVIDIA's official instructions](https://nvidia.custhelp.com/app/answers/detail/a_id/5084).
Automexia does not change another application's shortcuts or display settings.

## Resize panes

Current source preserves ordinary text selections while retained cells reflow
to a new pane width. If an endpoint is removed or cropped, the selection clears;
rectangular selections still clear on width changes. Alternate-screen programs
can redraw their content. The published 0.4.0 package predates this correction;
native visual verification remains outstanding. See the
[selection reflow contract](../adr/0043-retained-selection-reflow.md).

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

In current source, right-click and middle-click first target the terminal pane
under the pointer. Right-click preserves that pane's selection long enough to
copy it; it does not copy another pane's selection. Pane rails/footers and an
active search or command palette do not forward these gestures to the terminal.
Paste is queued once for its captured terminal and cancelled if that destination
closed. Text over 1 MiB is rejected with a warning, not truncated. No Enter is
added, but embedded newlines can execute commands when the shell does not support
bracketed paste. The published Linux 0.4.0 prerelease predates this correction.
- Rotating the mouse wheel or starting a trackpad scroll selects the pane under
  the pointer and sends that initiating scroll to it; pointer hover alone does
  not change pane selection.
- Full-screen terminal applications that enable mouse reporting keep mouse ownership. Hold `Shift` for the established host-UI override where supported.

## Search and scrollback

| Action | Windows/Linux/BSD | macOS |
|---|---|---|
| Find in selected pane | `Ctrl+F` | `Cmd+F` |
| Find backward in selected pane | `Alt+Shift+B` | `Cmd+B` |
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
| Fullscreen | `F11`; Windows also supports `Alt+Enter` | `Ctrl+Cmd+F` |
| Open config | Windows: `Ctrl+,`; Linux/BSD: `Ctrl+Shift+,` | `Cmd+,` |
| Command palette | `Ctrl+Shift+P` | `Cmd+Shift+P` |
| Preview selected image | `Ctrl+Alt+I` | `Cmd+Alt+I` |

Pane font zoom is runtime/pane-local. Reset returns to the configured font size.
Palette labels reflect effective configuration, including platform differences
and overrides. `Enter` means select the palette action and activate it there;
it does not create a global binding. Shift+Enter selects the previous match only while
search is open, not from a normal terminal prompt.

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
