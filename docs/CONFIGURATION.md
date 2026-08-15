# Configuration

Automexia loads `config.toml` and `themes/` from one platform root:

- Windows: `%LOCALAPPDATA%\Automexia\Terminal`
- macOS: `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal`
- Linux/BSD: `$XDG_CONFIG_HOME/automexia`, falling back to
  `~/.config/automexia`

The root contains `config.toml`, `themes/`, `extensions/`, and `logs/`.
`AUTOMEXIA_CONFIG_HOME` has highest priority. `AUTOMEXIA_LOG_LEVEL` overrides
the configured log level. Shells launched by Automexia receive
`TERM_PROGRAM=Automexia` and `AUTOMEXIA_SHELL_INTEGRATION=1`.

For v0.4.x only, `RIO_CONFIG_HOME` is accepted as a read-only locator for the
Rio migration source; it is never Automexia's writable destination.
`RIO_LOG_LEVEL` remains a deprecated value fallback. Each use emits a
once-per-start warning. These aliases are removed in v0.5.0.

The installed terminfo names are `automexia` and `xterm-automexia`; Automexia
falls back to `xterm-256color` if neither is installed.

The inherited configuration schema remains compatible unless a release note
explicitly documents a change. Generate a default file with
`automexia --write-config`.

Automexia's default liquid-hacker window is 1280x760 with a persistent tab row
and responsive pane-local tab rail. Operational context belongs to each
semantic prompt instead of being duplicated in global chrome. Terminal text
defaults to 18 points with 1.22 line spacing: command blocks and dense output
remain easy to scan while split panes retain useful working space. The shared
renderer metric applies identically to PowerShell, CMD, WSL, Unix shells, and
full-screen applications on every supported OS without inserting characters
into terminal history. When semantic context exists, its compact tags use the
pane's actual row height to preserve a minimum 1.22-row rhythm after preceding
command output; absent context adds no layout. The result is clamped within the
already reserved semantic row, so paths, cursors, copied text, PTY history, and
full-screen applications are unaffected. Context tags and chrome use a smaller
secondary type scale, while click targets retain their accessible dimensions.
The relevant overrides remain ordinary inherited settings:

```toml
line-height = 1.22

[window]
width = 1280
height = 760

[navigation]
mode = "Tab"
hide-if-single = false
max-tab-width = 200

[fonts]
size = 18.0
```

`Ctrl`+`+` and `Ctrl`+`-` adjust an individual terminal panel at runtime;
`Ctrl`+`0` returns it to the configured size.

Keyboard selection is terminal-owned and common to Windows, Linux/BSD, macOS,
PowerShell, CMD, WSL, Bash, and Zsh:

| Shortcut | Result |
|---|---|
| `Shift`+Left/Right | extend or reverse the selection by one visible cell |
| `Shift`+Up/Down | extend or reverse the selection by one row |
| `Ctrl`+`Shift`+Left/Right | extend or reverse by a Unicode-aware word boundary |

The first motion anchors at the live terminal cursor; following motions move
the active end of the existing selection. Horizontal motion crosses wrapped
rows and scrollback, vertical motion preserves the visual column, and neither
stops inside a wide-character spacer. These actions never send bytes to the
PTY. Search, Vi mode, and pinned image-preview browsing retain arrow-key
ownership. User bindings may override the defaults with the stable action
names `ExtendSelectionLeft`, `ExtendSelectionRight`, `ExtendSelectionUp`,
`ExtendSelectionDown`, `ExtendSelectionWordLeft`, and
`ExtendSelectionWordRight`.

Tab shortcuts have three deliberate scopes. The non-macOS defaults are:

| Shortcut | Result |
|---|---|
| `Ctrl`+`T` | add a window-level tab to the current Automexia window |
| `Ctrl`+`Shift`+`T` | add an independent tab inside the selected split/session |
| `Ctrl`+`Shift`+`N` | create a separate OS window |

The corresponding custom-binding action names are `CreateTab`,
`CreateLocalTab`, and `CreateWindow`.

Each pane-local tab owns an independent route, PTY, terminal grid, history, and
input queue while inheriting the selected pane's launch descriptor. It does not
share live process state with its sibling. Closing a pane-local tab closes only
that tab; closing a window-level tab closes only its own grid. These are
defaults, so explicit user bindings can override either scope independently.
macOS uses `Cmd`+`N`, `Cmd`+`T`, and `Cmd`+`Shift`+`T` for the same scopes.

When a pane contains multiple local tabs, Automexia renders their rail inside
that pane. The rail uses 36 logical pixels, scales with DPI, does not affect
sibling panes, and temporarily hides when that pane is below 96 logical pixels
high. No configuration is required and hidden tabs remain active and reachable
through their keyboard actions.

Pane and tab navigation have separate scopes:

| Scope | Windows/Linux/BSD | macOS | Behavior |
|---|---|---|---|
| nearest pane | `Alt`+Arrow | `Cmd`+`Alt`+Arrow | focus the nearest pane in that direction; stop at the outer edge |
| next/previous pane | `F6` / `Shift`+`F6` | `Cmd`+`]` / `Cmd`+`[` | cycle panes in visual order |
| next/previous tab inside selected pane | `Alt`+`PageDown` / `Alt`+`PageUp` | `Cmd`+`Alt`+`]` / `Cmd`+`Alt`+`[` | wrap inside that pane without entering another pane or window tab |
| next/previous window-level tab | `Ctrl`+`Tab` / `Ctrl`+`Shift`+`Tab` | `Ctrl`+`Tab` / `Ctrl`+`Shift`+`Tab` | switch the complete window workspace |

The configurable action names are `SelectPaneLeft`, `SelectPaneRight`,
`SelectPaneUp`, `SelectPaneDown`, `SelectNextLocalTab`, and
`SelectPrevLocalTab`. User bindings retain precedence. Directional focus uses
the current rendered pane rectangles, prefers candidates that overlap the
active pane on the perpendicular axis, and never wraps across an outer edge.

Split shortcuts distinguish a clean default shell from an independent clone:

| Shortcut | Result |
|---|---|
| `Ctrl`+`Shift`+`R` | open the configured default shell in a right split |
| `Ctrl`+`Shift`+`D` | open the configured default shell in a lower split |
| `Ctrl`+`R` | clone the active shell/profile/directory into a right split |
| `Ctrl`+`D` | clone the active shell/profile/directory into a lower split |
| `Ctrl`+`Alt`+`R` | send history search (`Ctrl+R`) to the shell |
| `Ctrl`+`Alt`+`D` | send EOF/logout (`Ctrl+D`) to the shell |

Clones are independent sessions: they receive a new PTY, process, route,
scrollback, input queue, and extension state. PowerShell/pwsh, CMD, Bash, Zsh, and
WSL retain their active executable/profile/current directory; WSL also retains
its distro, user, and shell. Jobs, process memory, command history position,
partially typed input, and scrollback are never copied. The clone actions can
be rebound as `clonesplitright` and `clonesplitdown`.
Only absolute, control-free OSC 7 directories can replace the stored launch
directory. Missing or invalid metadata keeps the safe profile fallback. If the
profile, WSL distribution, or PTY cannot be recreated, Automexia leaves the
layout unchanged and displays the concrete failure; it never opens PowerShell
as a silent substitute for a failed WSL clone.

Image preview has one stable binding action, `PreviewSelectedImage`. Select a
local raster path and press `Ctrl`+`Alt`+`I` on Windows/Linux/BSD or
`Cmd`+`Alt`+`I` on macOS. The same action appears as **Preview Selected Image**
in the command palette. Plain hover uses a 100 ms stability delay; clicking
pins the path, unmodified arrows browse other visible image paths, and `Esc`
closes it. Mouse-reporting terminal applications retain ownership unless
`Shift` is held. This release adds no image-preview configuration; normal user
binding overrides still apply. See
[image previews](IMAGE-PREVIEWS.md) for protocols, formats, WSL behavior,
limits, and testing.

These are Automexia's classic defaults. The separate
[Ghostty keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md) document is
a future opt-in compatibility contract, not the active default table.

Automexia v0.4 does not expose `keyboard.profile`, versioned Ghostty profiles,
multi-key tables, action chains, or explicit unbind directives. The existing
`[bindings].keys` list remains the supported override mechanism: a valid user
entry replaces a matching default trigger/mode scope. Compatibility profiles
will be opt-in and migration will require an explicit apply step if the
[full compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) is implemented;
startup and reload must never silently rewrite configuration.

On Windows, disabled native decorations are the default so Automexia can draw
coherent tabs and window controls. All edges and corners remain resizable. See
`docs/LIQUID-HACKER-UX.md` for the layout and interaction contract.
