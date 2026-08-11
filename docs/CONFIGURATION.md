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

Automexia's default liquid-hacker window is 1280x760 with persistent tab and
context chrome. Terminal text defaults to 20 points with 1.20 line spacing for
readable long listings and structured output. The relevant overrides remain
ordinary inherited settings:

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

On Windows, disabled native decorations are the default so Automexia can draw
coherent tabs and window controls. All edges and corners remain resizable. See
`docs/LIQUID-HACKER-UX.md` for the layout and interaction contract.
