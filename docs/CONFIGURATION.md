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
and responsive workspace-action rail. Operational context belongs to each
semantic prompt instead of being duplicated in global chrome. Terminal text
defaults to 20 points with 1.20 line spacing for readable long listings and
structured output. The relevant overrides remain ordinary inherited settings:

```toml
line-height = 1.20

[window]
width = 1280
height = 760

[navigation]
mode = "Tab"
hide-if-single = false
max-tab-width = 240

[fonts]
size = 20.0
```

`Ctrl`+`+` and `Ctrl`+`-` adjust an individual terminal panel at runtime;
`Ctrl`+`0` returns it to the configured size.

Tab shortcuts have three deliberate scopes. The non-macOS defaults are:

| Shortcut | Result |
|---|---|
| `Ctrl`+`Shift`+`T` | add a window-level tab to the current Automexia window |
| `Ctrl`+`T` | additional Automexia alias for a window-level tab |
| `Ctrl`+`Alt`+`T` | add an independent tab inside the selected split/session |
| `Ctrl`+`Shift`+`N` | create a separate OS window |

The corresponding custom-binding action names are `CreateTab`,
`CreateLocalTab`, and `CreateWindow`.

Each pane-local tab owns an independent route, PTY, terminal grid, history, and
input queue while inheriting the selected pane's launch descriptor. It does not
share live process state with its sibling. Closing a pane-local tab closes only
that tab; closing a window-level tab closes only its own grid. These are
defaults, so explicit user bindings can override either scope independently.
macOS uses `Cmd`+`N`, `Cmd`+`T`, and `Cmd`+`Alt`+`T` for the same scopes.

Split shortcuts distinguish a clean default shell from an independent clone:

| Shortcut | Result |
|---|---|
| `Ctrl`+`Shift`+`O` | open the configured default shell in a right split |
| `Ctrl`+`Shift`+`E` | open the configured default shell in a lower split |
| `Ctrl`+`Alt`+`R` | clone the active shell/profile/directory into a right split |
| `Ctrl`+`Alt`+`D` | clone the active shell/profile/directory into a lower split |
| `Ctrl`+`R` | shell-owned history search |
| `Ctrl`+`D` | shell-owned EOF/logout |

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

The complete Windows/Linux/BSD and macOS defaults, collision policy,
Automexia-only extensions, and unsupported Ghostty actions are maintained in
[Ghostty keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md).

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
