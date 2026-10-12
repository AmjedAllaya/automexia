# Workspaces, tabs, and panes

Use windows and tabs to separate tasks. Use split panes to keep sessions visible
together, and pane-local tabs to switch sessions within one pane.

A session runs a shell or program through its own pseudo-terminal (PTY), the
connection that carries terminal input and output.

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

### Closing a window or quitting

The close control closes only its window; Quit closes the application. Current
source dismisses confirmed closing windows before waiting for shell cleanup.
Cancellation keeps the window and its sessions open. Other windows remain
usable when only one is closed.

Shells and their owned child processes are still cleaned up after dismissal;
the process may remain briefly while that finishes. Save work before confirming
close. This correction is in current source, not the earlier 0.4.0 download.

### Use a window-level tab for a different workspace

Use a window-level tab to switch workspaces within one window.

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

- Windows/Linux/BSD: `Alt+Shift+R` for right, `Alt+Shift+D` for down.
- macOS: `Cmd+D` for right, `Cmd+Shift+D` for down.

**Clone split:** starts an independent session using the active session's launch profile and validated working directory.

- Windows/Linux/BSD: `Alt+R` for right, `Alt+D` for down.
- macOS: `Cmd+Alt+Shift+R` for right, `Cmd+Alt+Shift+D` for down.
- All platforms: **Panes & Sessions** in the palette contains both clone actions.
  Ctrl+R/Ctrl+D remain shell-owned; previously published 0.4.0 binaries do not
  include these current-source corrections. Explicit user mappings are preserved.

A clone starts a separate session with its own PTY and process tree.
It copies launch context, not live terminal state.

### Use a pane-local tab when a region needs alternatives

Use pane-local tabs to switch sessions in one pane without changing the others.

Create one with:

- Windows/Linux/BSD: `Ctrl+Shift+T`
- macOS: `Cmd+Shift+T`

Move between local tabs inside the active pane with:

- Windows/Linux/BSD: `Alt+PageUp` / `Alt+PageDown`
- macOS: `Cmd+Alt+[` / `Cmd+Alt+]`

A pane displays its local tab rail only when it owns multiple local tabs and has enough height. If the rail disappears in a very small split, the sessions are not closed; keyboard and palette actions remain available.

### Use a new OS window for desktop-level separation

Open another window for a separate monitor or desktop, or to minimize and close
a workspace independently.

- Windows/Linux/BSD: `Ctrl+Shift+N`
- macOS: `Cmd+N`

Closing one Automexia window does not close sibling windows. The explicit application quit action is different from closing an individual window.

## Fresh split or cloned split?

Fresh splits use defaults. Clones reuse the active launch context.

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

Cloning does not duplicate a running child program such as `ssh`.

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

It closes the selected local tab, split, or window-level tab, depending on focus
and layout. Quit closes the whole application.

Check the selected session before closing a running process. Use its visible
close control or the command palette if the scope is unclear.

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

Pane search replaces the selected pane's footer. In very narrow panes, it uses
the bottom position while keeping the same search scope.

All-visible-pane search sits above the footer, clear of window tabs and close
controls. It searches the active local tab in each visible split of the selected
workspace tab. Hidden local tabs and other workspace tabs are excluded.

The `PANE` / `ALL PANES` label, themed icon, hint text, and colored close button
identify the scope and controls without relying on color alone. Clicks inside
the search surface cannot activate controls behind it.

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

Search the palette for tab, pane, configuration, image-preview, and other actions.
You do not need to memorize their shortcuts.

## A useful layout pattern

For a typical multi-step task:

1. Start Automexia in the working directory with `automexia --working-dir <path>`.
2. Keep the left pane as the primary shell or control point.
3. Clone right for a second shell in the same context.
4. Create a fresh split down when you want a clean independent task runner.
5. Add a pane-local tab only when one region needs multiple alternate sessions.
6. Create another window-level tab when the entire task context changes.

**Tabs separate tasks; panes show sessions together; local tabs switch sessions
within a pane.**

For all shortcuts, see [Keyboard and input reference](../KEYBOARD.md).
