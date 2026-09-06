# Workspaces, tabs, and panes

Automexia lets you arrange command-driven work around the way the task
actually unfolds. Windows, tabs, panes, and sessions have different scopes;
choosing the right one keeps related work visible and separates contexts that
should not interfere with each other.

## The workspace hierarchy

```text
OS window
└── window-level tab
    └── split layout
        ├── pane
        │   ├── local tab
        │   └── local tab
        └── pane
            └── local tab
```

| Object | What it changes | Best use |
|---|---|---|
| **OS window** | Completely separate desktop window and PTY collection | Separate monitor, desktop, or independent project context |
| **Window-level tab** | Switches the whole workspace shown in that window | Large context changes: project A vs project B, local vs operations |
| **Split pane** | Shows another live session beside the current one | Work that must remain visible simultaneously |
| **Pane-local tab** | Replaces only one pane's active session | Alternate shells/tasks inside one region while sibling panes stay fixed |
| **Session** | The actual shell/program attached to a PTY | Where commands and terminal applications run |

## Which approach should I use?

### Use a window-level tab for a different workspace

A window-level tab is the simplest way to separate substantial contexts without opening another desktop window.

Examples:

- application work vs infrastructure work;
- a data or media task vs its logs and generated output;
- project A vs project B;
- one long-running TUI workspace vs another.

Create one with:

- Windows/Linux/BSD: `Ctrl+T`
- macOS: `Cmd+T`

Switch with `Ctrl+Tab` / `Ctrl+Shift+Tab`. On macOS, `Cmd+Shift+[` / `Cmd+Shift+]` also changes the window-level tab.

### Use a split when two sessions must stay visible

When a closed workspace is retained for undo, a session that exits does not
close its healthy sibling panes or pane-local tabs. Undo restores the remaining
sessions using the current window size and display scale. This history is
temporary: it is not a saved session or a way to restart an exited shell.

Splits are for simultaneous visibility: processing command + logs, source
files + generated results, server + monitor, shell + database console, or local
+ remote.

Automexia offers two different split behaviors.

**Fresh split:** starts the default shell/session definition.

- Windows/Linux/BSD: `Ctrl+Shift+R` for right, `Ctrl+Shift+D` for down.
- macOS: `Cmd+D` for right, `Cmd+Shift+D` for down.

**Clone split:** starts an independent session using the active session's launch profile and validated working directory.

- All platforms: choose **Clone Active Session Right / Down** in the command
  palette. Current source leaves cloning unbound and Ctrl+R/Ctrl+D shell-owned;
  the published 0.4.0 package predates this correction. Explicit user mappings
  remain available and are not rewritten.

A clone is not shared terminal state. The new pane owns its own PTY and process tree. Think “start another session from the same launch context,” not “mirror this terminal.”

### Use a pane-local tab when a region needs alternatives

A pane-local tab is useful when you want one area of the layout to switch between several independent sessions while other panes remain unchanged.

Create one with:

- Windows/Linux/BSD: `Ctrl+Shift+T`
- macOS: `Cmd+Shift+T`

Move between local tabs inside the active pane with:

- Windows/Linux/BSD: `Alt+PageUp` / `Alt+PageDown`
- macOS: `Cmd+Alt+[` / `Cmd+Alt+]`

A pane displays its local tab rail only when it owns multiple local tabs and has enough height. If the rail disappears in a very small split, the sessions are not closed; keyboard and palette actions remain available.

### Use a new OS window for desktop-level separation

Use another OS window when you need true desktop independence: another monitor, separate virtual desktop, or a workspace you want to minimize/close without affecting the first window.

- Windows/Linux/BSD: `Ctrl+Shift+N`
- macOS: `Cmd+N`

Closing one Automexia window does not close sibling windows. The explicit application quit action is different from closing an individual window.

## Fresh split or cloned split?

This is one of the most important Automexia choices.

| Need | Fresh split | Clone split |
|---|:---:|:---:|
| Default configured shell | Yes | Maybe—the active launch profile is reused |
| Same validated working directory as active session | Not necessarily | Yes |
| Same launch profile/session type | No | Yes |
| Same live process state | No | No |
| Independent PTY/process tree | Yes | Yes |
| Best for “another standard shell” | **Yes** | Sometimes |
| Best for “another shell beside this project context” | Sometimes | **Yes** |

Example: you are in `D:\work\api` using PowerShell and want another shell in the same project. Clone the pane. If instead you want a clean default shell that is unrelated to the current pane's launch profile, create a fresh split.

If the active shell is currently running an interactive child program such as `ssh`, cloning reproduces the Automexia session launch context; it should not be treated as duplicating the exact live child-process state.

## Moving between panes

Use geometric navigation when the layout is spatial:

- Windows/Linux/BSD: `Alt+Arrow`
- macOS: `Cmd+Alt+Arrow`

Use cycling when you just want the next/previous pane:

- Windows/Linux/BSD: `F6` / `Shift+F6`
- macOS: `Cmd+]` / `Cmd+[`

The selected pane owns keyboard input, its local-tab scope, search state, selection, and footer emphasis.

## Resize a split

Resize the selected split with:

- Windows: `Alt+Shift+Arrow`
- Linux/BSD: `Ctrl+Alt+Shift+Arrow`
- macOS: `Ctrl+Cmd+Arrow`

Automexia clamps extremely small layouts so terminal rows remain usable. Pane-local tab rails and footers can temporarily fold away at tiny sizes and return when the pane grows.

## Close the thing that currently owns focus

The standard close-session action is scope-aware:

- Windows/Linux/BSD: `Ctrl+Shift+W`
- macOS: `Cmd+W`

Depending on focus/layout it closes the selected local tab, split, or window-level tab. This is intentionally different from “quit every Automexia window.”

Use extra care when the selected session has a running process. If you are unsure what scope is selected, use the visible tab/pane close target or the command palette so the intended action is explicit.

## Selection is pane-local

Terminal selection belongs to the selected pane.

- Drag with the primary/left button for normal mouse selection.
- `Shift+Arrow` starts keyboard selection at the terminal insertion cursor and extends it.
- `Ctrl+Shift+Left/Right` extends by Unicode-aware word boundaries.
- `Ctrl+C` copies when a non-empty terminal selection exists; otherwise it remains the normal shell/application interrupt.
- Typing, pasting, or pressing an Arrow without `Shift` exits terminal selection before that input is sent to the shell.

This behavior prevents Automexia's selection controls from secretly changing shell-editor history or line-editor state.

When you inspect retained scrollback, resizing keeps the row containing the
previously first-visible content at the top where history permits. Wider rows
may also show preceding text from the same wrapped line. A taller pane can
absorb that content into the live screen; history that was actually evicted
cannot be restored. Resizing does not send a command to the shell.

## Search scrollback without changing the shell command

Search is a terminal view operation, not a shell command:

- Pane search: Windows/Linux/BSD `Ctrl+F`; macOS `Cmd+F` forward or `Cmd+B` backward.
- All-visible-pane search: Windows/Linux/BSD `Ctrl+Shift+F` forward or `Ctrl+Shift+B` backward; macOS `Cmd+Shift+F` forward or `Cmd+Shift+B` backward.

Pane search replaces the selected pane's footer; an exceptionally narrow pane falls back to the same safe bottom position without changing its pane scope. All-visible-pane search is bottom-centered above the footer and cannot cover top-level tab or window-close controls. Its deterministic scope is the active local tab in every visible split of the selected workspace tab; hidden local tabs and other window tabs are not activated. The `PANE` / `ALL PANES` chip, themed icon, placeholder, and colored close control keep scope and actions recognizable without relying on color alone. The full surface captures pointer input so a click cannot activate UI behind it.

Inside search mode:

| Key | Result |
|---|---|
| `Enter` / `Shift+Enter` | Next / previous result |
| `Esc` or `Ctrl+C` | Cancel search |
| `Ctrl+U` | Clear query |
| `Ctrl+W` | Delete previous query word |
| `Ctrl+P` or Up | Older search-history query |
| `Ctrl+N` or Down | Newer search-history query |

## Use the command palette when you forget the hierarchy

Open the command palette with:

- Windows/Linux/BSD: `Ctrl+Shift+P`
- macOS: `Cmd+Shift+P`

The palette is the discoverable path for tab, split, pane, config, image-preview, and other registered actions. It is often the better approach for occasional operations because you can search by intent rather than memorize every key chord.

## A useful layout pattern

For a typical multi-step task:

1. Start Automexia in the working directory with `automexia --working-dir <path>`.
2. Keep the left pane as the primary shell or control point.
3. Clone right for a second shell in the same context.
4. Create a fresh split down when you want a clean independent task runner.
5. Add a pane-local tab only when one region needs multiple alternate sessions.
6. Create another window-level tab when the entire task context changes.

This keeps the visual hierarchy meaningful: **tabs separate contexts; panes preserve simultaneous visibility; local tabs provide alternatives inside one region.**

For the complete binding table, see [Keyboard and input reference](../reference/keyboard.md).
