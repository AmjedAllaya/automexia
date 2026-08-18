# Configuration and customization

Automexia is designed to work with **zero configuration**. The most maintainable customization strategy is to override only what you intentionally want to change.

## Choose the right kind of customization

| Need | Best approach |
|---|---|
| Different directory for one launch | `automexia --working-dir <PATH>` |
| Different shell/program for one launch | `automexia ... -e <PROGRAM> [ARGS...]` |
| Permanent terminal preference | `config.toml` |
| Different preferences by platform | Platform-specific config override tables |
| Temporary diagnostic logging | `--enable-log-file` or log environment override |
| Frequent UI action on another key | `[bindings]` custom binding |
| Occasional UI action | Command palette instead of adding a binding |
| Different project launcher | Desktop/script launcher with CLI options rather than global config |

This separation prevents the global config from becoming a collection of one-off project assumptions.

## 1. Create a starter config

Run:

```text
automexia --write-config
```

This creates a starter at the platform configuration root without overwriting an existing file.

Default roots:

| Platform | Root |
|---|---|
| Windows | `%LOCALAPPDATA%\Automexia\Terminal` |
| macOS | `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` |
| Linux | `$XDG_CONFIG_HOME/automexia` or `~/.config/automexia` |

The root can contain `config.toml`, `themes/`, `extensions/`, and `logs/`.

## 2. Open the config from Automexia

Use:

- Windows: `Ctrl+,`
- Linux/BSD: `Ctrl+Shift+,`
- macOS: `Cmd+,`

The configured editor is used. By default the reference defines Notepad on Windows and `vi` on Unix unless you override `[editor]`.

Example editor override:

```toml
[editor]
program = "code"
args = ["--wait"]
```

## 3. Start with a small config

A practical starting point is:

```toml
line-height = 1.22
confirm-before-quit = true
scrollback-history-limit = 10000

[window]
width = 1280
height = 760
mode = "Windowed"
opacity = 1.0

[navigation]
mode = "Tab"
hide-if-single = false
max-tab-width = 200

[fonts]
size = 18.0
family = "Cascadia Code"

[cursor]
shape = "Block"
blinking = false
```

Do not copy the entire schema into your personal file. A smaller file is easier to reason about and lets new defaults/improvements apply to settings you never intentionally changed.

## 4. Choose the default shell

Permanent shell selection lives under `[shell]`:

```toml
[shell]
program = "pwsh"
args = ["-NoLogo"]
```

If `shell.program` is unset/empty, Automexia selects the user's default shell.

Use this setting for your everyday default. Use `automexia -e ...` when you need a different shell for only one launch.

The argument list is an exact vector, not a shell command string. Avoid embedding pipelines/redirection there; those belong in an actual shell session.

## 5. Configure fonts without breaking runtime zoom

Example:

```toml
[fonts]
size = 18.0
family = "Cascadia Code"
features = ["calt=1", "liga=1"]
```

Runtime zoom remains pane-local:

- Windows/Linux/BSD: `Ctrl+0`, `Ctrl+=`/`Ctrl++`, `Ctrl+-`
- macOS: `Cmd+0`, `Cmd+=`/`Cmd++`, `Cmd+-`

Reset returns the pane to the configured size.

Use runtime zoom for temporary readability changes and config for the long-term default.

## 6. Configure window appearance

A simple window section might be:

```toml
[window]
width = 1280
height = 760
mode = "Windowed"
opacity = 1.0
decorations = "Disabled"
```

Other supported controls include blur, background image, colorspace, initial title, platform-specific decoration options, and quake-window dimensions. Use [Configuration reference](../reference/configuration.md#window) before changing advanced compositor/renderer settings; some are intentionally platform-specific.

### Opacity or background image?

Use `window.opacity` to change the whole background transparency. Use `window.background-image` when you want a local image behind terminal content. These are separate from terminal **image previews**, which display paths/protocol graphics as terminal content/overlays.

## 7. Configure navigation and pane appearance

Example:

```toml
[navigation]
mode = "Tab"
current-working-directory = true
hide-if-single = false
use-split = true
unfocused-split-opacity = 0.7
max-tab-width = 200
```

Use `current-working-directory = true` when new sessions should inherit validated directory metadata where the action supports it. Remember that a **clone split** is still the explicit workflow when you want the active launch context reproduced.

Pane-local tab rails and operational footers follow responsive product rules and do not have a general “turn every visual invariant into a setting” model. This keeps the terminal grid/layout contract predictable.

## 8. Themes: fixed, adaptive, or forced

The top-level config supports:

- `theme = "name"` to load `themes/name.toml`;
- `adaptive-theme = { dark = "...", light = "..." }` to switch based on appearance;
- `force-theme = "dark"` or `"light"` to force one appearance.

Choose one approach deliberately:

- **Fixed theme** when you always want one palette.
- **Adaptive theme** when you follow OS light/dark appearance.
- **Force theme** when app appearance must be independent of the host.

Both adaptive theme files must load successfully. Theme/config failures do not replace the current runtime state with partially parsed values; Automexia keeps the last known-good configuration.

## 9. Add a custom key binding

Bindings live under `[bindings]`:

```toml
[bindings]
keys = [
  { key = "F5", with = "control | shift", action = "ReloadConfig" },
  { key = "O", with = "control | alt", action = "OpenCommandPalette" },
]
```

Use custom bindings for frequent operations with a clear personal mnemonic. Do not assign a new shortcut just because an action exists; the command palette is a lower-maintenance choice for occasional actions.

Bindings are explicit overrides. Unknown action names are rejected rather than silently removing a matching default. The full stable action-name list is in [Keyboard and input reference](../reference/keyboard.md#custom-bindings).

## 10. Reload safely

Configuration is bounded and validated. If a runtime reload encounters malformed/oversized config or a theme error, the last known-good runtime configuration remains active.

Use one of these approaches:

- restart Automexia after a configuration edit;
- invoke the registered reload action where exposed;
- bind `ReloadConfig` to a shortcut you prefer.

This is safer than partially applying whatever happened to parse before an error.

## 11. Environment overrides are for environment-specific behavior

The configuration system also recognizes environment-level overrides such as:

- `AUTOMEXIA_CONFIG_HOME` — replaces the whole writable config root;
- `AUTOMEXIA_LOG_LEVEL` — overrides configured logging level.

Use these for controlled environments, test setups, portable launch scripts, or diagnostics. Prefer ordinary `config.toml` for normal personal preferences because it is easier to discover and maintain.

## 12. A maintainable customization workflow

1. Run Automexia with defaults first.
2. Identify one concrete friction point.
3. Decide whether it is temporary (CLI/runtime shortcut) or permanent (config).
4. Add the smallest possible override.
5. Reload/restart and verify only that behavior.
6. Keep comments explaining non-obvious platform workarounds.
7. Periodically remove overrides that no longer solve a real problem.

For every available key, type, range, default, and platform override, use [Configuration reference](../reference/configuration.md).
