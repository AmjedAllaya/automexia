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

On Windows and Linux, tab shortcuts have two deliberate scopes:

| Shortcut | Result |
|---|---|
| `Ctrl`+`T` | create a separate Automexia window containing its initial tab |
| `Ctrl`+`Shift`+`T` | add a global tab to the current window's tab strip |
| `Ctrl`+`Shift`+`N` | create a separate window (compatibility alias) |

The new window owns an independent route, terminal session, and tab strip.
These are defaults, so an explicit user binding can override `Ctrl`+`T` without
changing `Ctrl`+`Shift`+`T`. macOS retains the platform conventions `Cmd`+`N`
for a new window and `Cmd`+`T` for a tab in the current window.

Split shortcuts distinguish a clean default shell from an independent clone:

| Shortcut | Result |
|---|---|
| `Ctrl`+`Shift`+`R` | open the configured default shell in a right split |
| `Ctrl`+`Shift`+`D` | open the configured default shell in a lower split |
| `Ctrl`+`Alt`+`R` | clone the active shell/profile/directory into a right split |
| `Ctrl`+`Alt`+`D` | clone the active shell/profile/directory into a lower split |

Clones are independent sessions: they receive a new PTY, process, route,
scrollback, input queue, and extension state. PowerShell/pwsh, CMD, Bash, Zsh, and
WSL retain their active executable/profile/current directory; WSL also retains
its distro, user, and shell. Jobs, process memory, command history position,
partially typed input, and scrollback are never copied. Bare `Ctrl`+`R` and
`Ctrl`+`D` remain available to the shell for history search and EOF behavior.
The actions can be rebound as `clonesplitright` and `clonesplitdown`.
Only absolute, control-free OSC 7 directories can replace the stored launch
directory. Missing or invalid metadata keeps the safe profile fallback. If the
profile, WSL distribution, or PTY cannot be recreated, Automexia leaves the
layout unchanged and displays the concrete failure; it never opens PowerShell
as a silent substitute for a failed WSL clone.

On Windows, disabled native decorations are the default so Automexia can draw
coherent tabs and window controls. All edges and corners remain resizable. See
`docs/LIQUID-HACKER-UX.md` for the layout and interaction contract.
