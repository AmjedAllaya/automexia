# Configuration reference

Automexia is usable with zero configuration. `config.toml` contains only the
values a user wants to override; omitted keys use tested defaults. This page is
the canonical v0.4 schema reference.

## File location and precedence

Automexia uses one writable product root:

| Platform | Default root |
|---|---|
| Windows | `%LOCALAPPDATA%\Automexia\Terminal` |
| macOS | `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` |
| Linux | `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia` |

The root contains `config.toml`, `themes/`, `extensions/`, and `logs/`.
`AUTOMEXIA_CONFIG_HOME` replaces the complete root. `AUTOMEXIA_LOG_LEVEL`
overrides the configured log level. For v0.4 only, `RIO_CONFIG_HOME` is a
read-only migration source and `RIO_LOG_LEVEL` is a deprecated value fallback;
both warn once per start and are removed in v0.5.

Create a starter file containing the canonical reference pointer without overwriting an existing file:

```text
automexia --write-config
automexia --write-config D:\configs\automexia.toml
```

The main file must be a regular UTF-8 file no larger than 4 MiB. Each theme
must be a regular UTF-8 file no larger than 1 MiB. Parse/read/theme failure is
reported and runtime reload keeps the last known-good configuration.

## Minimal example

```toml
line-height = 1.22
confirm-before-quit = true
copy-on-select = false
scrollback-history-limit = 10000

[shell]
program = "pwsh"
args = ["-NoLogo"]

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

## Top-level settings

| Key | Type / default | Behavior |
|---|---|---|
| `working-dir` | path / unset | Initial directory. Invalid CLI overrides warn and use the safe default. |
| `line-height` | float / `1.22` | Shared cell-line metric across every shell and OS; it does not insert blank PTY rows. |
| `theme` | string / empty | Loads `themes/<name>.toml`. |
| `adaptive-theme` | `{ dark, light }` / unset | Names two theme files selected by appearance. Both must load successfully. |
| `force-theme` | `dark` or `light` / unset | Force appearance instead of following the host. |
| `margin` | 1, 2, or 4 floats / `[2]` | CSS-like terminal content margin. Two values mean vertical/horizontal. |
| `env-vars` | string array / `[]` | Extra `NAME=VALUE` entries for child sessions. Platform entries append. Do not store secrets in committed config. |
| `option-as-alt` | string / `"none"` | macOS Option-key behavior inherited from the engine. |
| `use-fork` | bool / true except macOS | Unix process-launch compatibility control; leave default unless diagnosing platform launch behavior. |
| `ignore-selection-foreground-color` | bool / `false` | Keep cell foreground instead of the selection foreground. |
| `confirm-before-quit` | bool / `true` | Confirm destructive application quit when required; intermediate OS-window close remains window-scoped. |
| `copy-on-select` | bool / `false` | Copy selection automatically. Selection-aware `Ctrl+C` works independently. |
| `hide-mouse-cursor-when-typing` | bool / `false` | Hide pointer during keyboard input. Legacy `hide-cursor-when-typing` is accepted. |
| `draw-bold-text-with-light-colors` | bool / `false` | Map bold ANSI colors to the bright palette. |
| `enable-scroll-bar` | bool / `true` | Show the renderer scrollbar. |
| `scrollback-history-limit` | integer / `10000` | Maximum retained history rows per terminal. |

## Shell and editor

```toml
[shell]
program = "pwsh"
args = ["-NoLogo"]

[editor]
program = "code"
args = ["--wait"]
```

`shell.program` unset/empty selects the user's default shell (PowerShell on
Windows; login shell on Unix). `shell.args` is an exact argument vector—no
shell-string parsing. The editor defaults to Notepad on Windows and `vi` on
Unix and is used by the open-config action.

## Cursor, scroll, bell, and effects

| Table/key | Type / default | Behavior |
|---|---|---|
| `cursor.shape` | cursor shape / `Block` | Terminal cursor shape. |
| `cursor.blinking` | bool / `false` | Enable cursor blink. |
| `cursor.blinking-interval` | milliseconds / `800` | Blink interval. |
| `scroll.multiplier` | float / `3.0` | Wheel/trackpad scale numerator. |
| `scroll.divider` | float / `1.0` | Wheel/trackpad scale divisor. |
| `bell.audio` | bool / Windows/macOS true, Linux false | Use the native/system audio bell. |
| `effects.custom-mouse-cursor` | bool / `false` | Enable renderer custom pointer effect. |
| `effects.trail-cursor` | bool / `false` | Enable cursor trail animation. Resize/layout changes snap rather than animate from stale geometry. |

## Window

```toml
[window]
width = 1280
height = 760
columns = 120
rows = 30
mode = "Windowed"
opacity = 1.0
opacity-cells = false
blur = false
decorations = "Disabled"
colorspace = "Srgb"
quake-width-percentage = 1.0
quake-height-percentage = 0.4
background-image = { path = "D:/wallpaper.png", opacity = 0.25 }
```

| Key | Values / default | Notes |
|---|---|---|
| `width`, `height` | integer pixels / `1280`, `760` | Initial logical window size. |
| `columns`, `rows` | optional integer | Initial grid target when supplied. |
| `mode` | `Windowed`, `Maximized`, `Fullscreen` / `Windowed` | Initial mode. |
| `opacity` | float / `1.0` | Window/default-background opacity. |
| `opacity-cells` | bool / `false` | Apply opacity to explicitly colored cells too; off preserves TUI status/syntax contrast. |
| `blur` | bool or `macos-glass-regular`, `macos-glass-clear` / `false` | Unsupported glass styles degrade to system blur with a warning. |
| `background-image` | `{ path, opacity }` / unset | Local background image; opacity is clamped to 0…1. This is separate from terminal image preview. |
| `decorations` | `Enabled`, `Disabled`, `Transparent`, `Buttonless` | Default: disabled Windows, transparent macOS, enabled elsewhere. |
| `colorspace` | `Srgb`, `DisplayP3`, `Rec2020` / `Srgb` | Interpretation of configured colors; Windows fullscreen remains SDR sRGB. |
| `initial-title` | string / unset | Initial title before terminal title metadata. |
| `macos-use-unified-titlebar` | bool / `false` | macOS titlebar style. |
| `macos-use-shadow` | bool / `true` | macOS window shadow. |
| `macos-traffic-light-position-x` | float / unset | Horizontal macOS traffic-light offset. |
| `macos-traffic-light-position-y` | float / unset | Vertical macOS traffic-light offset. |
| `windows-use-undecorated-shadow` | bool / unset | Windows custom-frame shadow override. |
| `windows-use-no-redirection-bitmap` | bool / unset | Advanced Windows compositor flag; use only for measured compatibility. |
| `windows-corner-preference` | `Default`, `DoNotRound`, `Round`, `RoundSmall` / unset | Windows DWM corner hint. |
| `quake-width-percentage` | float / `1.0` | Quake window monitor-width fraction. |
| `quake-height-percentage` | float / `0.4` | Quake window monitor-height fraction. |

## Navigation and panes

```toml
[navigation]
mode = "Tab"
current-working-directory = true
use-terminal-title = false
hide-if-single = false
use-split = true
open-config-with-split = true
unfocused-split-opacity = 0.7
unfocused-split-fill = "#00111f"
max-tab-width = 200

[panel]
margin = [2]
padding = [5]
row-gap = 0
column-gap = 0
border-width = 2
border-radius = 0
```

| Key | Type / default | Notes |
|---|---|---|
| `navigation.mode` | `Plain`, `Tab`; macOS also `NativeTab` / `Tab` | `Tab` enables renderer-owned product chrome. |
| `current-working-directory` | bool / `true` | New sessions may inherit validated directory metadata. Alias: `cwd`. |
| `use-terminal-title` | bool / `false` | Prefer terminal-provided title in navigation. |
| `hide-if-single` | bool / `false` | Hide global tab chrome only when one tab exists. |
| `use-split` | bool / `true` | Enable split actions/default bindings. |
| `open-config-with-split` | bool / `true` | Open configuration using the split-aware workflow. |
| `unfocused-split-opacity` | float / `0.7` | Clamped to `0.15…1.0`; `1.0` disables dimming. |
| `unfocused-split-fill` | hex color / theme background | Optional inactive-pane tint. |
| `max-tab-width` | float / `200` | Clamped to `80…280` logical pixels. |
| `color-automation` | array / `[]` | Program/path-specific tab colors; retained for inherited compatibility. |
| `clickable` | bool / `false` | Inherited navigation click behavior; normal Automexia tabs remain interactive through native routing. |
| `panel.margin`, `panel.padding` | 1/2/4 floats / `[2]`, `[5]` | Per-pane inner spacing. |
| `panel.row-gap`, `column-gap` | float / `0` | Space between split panes. |
| `panel.border-width`, `border-radius` | float / `2`, `0` | Split border geometry. |

Pane-local tab rails and footers use responsive product invariants and have no
v0.4 user setting. See [Liquid Hacker UX](LIQUID-HACKER-UX.md).

## Fonts

```toml
[fonts]
size = 18.0
hinting = true
family = "Cascadia Code"
features = ["calt=1", "liga=1"]
use-drawable-chars = true
disable-warnings-not-found = false
additional-dirs = ["D:/fonts"]

[fonts.regular]
family = "Cascadia Code"
style = "default"
weight = 400
```

`regular`, `bold`, `italic`, and `bold-italic` each accept `family`, `style`,
and optional CSS-style `weight` (100…900). `style` is `"default"`, `false` to
reuse regular, or a named face style. `symbol-map` entries use `start`, `end`,
and `font-family` Unicode ranges. The default family is Cascadia Code and the
default size is 18 pt. Runtime `Ctrl`/`Cmd` zoom is pane-local; reset returns to
this configured size.

## Renderer and keyboard

| Key | Values / default | Notes |
|---|---|---|
| `renderer.backend` | platform backend / native default | Metal on macOS, Vulkan on Linux, WGPU umbrella elsewhere. Availability depends on compiled features. |
| `renderer.strategy` | `Events` or `Game` / `Events` | Event-driven rendering avoids continuous idle work. |
| `renderer.disable-unfocused-render` | bool / `false` | Stop repainting unfocused windows; may delay purely visual idle updates. |
| `renderer.disable-occluded-render` | bool / `false` | Stop repainting fully occluded windows. |
| `renderer.use-cpu` | bool / `false` | Experimental solid-quad/glyph fallback; image overlays, filters, advanced underlines, and rounded corners are not fully equivalent. |
| `renderer.filters` | filter array / `[]` | WGPU-only shader filters. Treat third-party shader code as untrusted review input. |
| `keyboard.disable-ctlseqs-alt` | bool / macOS true, others false | Use conventional Alt/Option word sequences instead of modified control sequences. |
| `keyboard.ime-cursor-positioning` | bool / `true` | Place the Input Method Editor popup at the live cursor. |
| `keyboard.forward-to-ime-modifier-mask` | modifier array / all four | Modifier set eligible for macOS IME forwarding. |

## Titles and developer logging

| Key | Type / default | Notes |
|---|---|---|
| `title.placeholder` | string/none / triangle marker | Initial/fallback tab title marker. |
| `title.content` | template string / platform-specific | Inherited title template using title/path/program data. |
| `developer.log-level` | string / `OFF` | Overridden by `AUTOMEXIA_LOG_LEVEL`. |
| `developer.enable-log-file` | bool / `false` | Persist logs under the product root. Logs can contain paths; redact before sharing. |
| `developer.enable-fps-counter` | bool / `false` | Diagnostic renderer counter. |

## Themes and colors

A named theme lives at `themes/<name>.toml`:

```toml
[colors]
background = "#00111f"
foreground = "#d9e7f2"
cursor = "#2df0a0"
selection-background = "#164b70"
selection-foreground = "#ffffff"
```

Supported `[colors]` keys are:

```text
background, foreground, black, red, green, yellow, blue, magenta, cyan, white,
light-black, light-red, light-green, light-yellow, light-blue, light-magenta,
light-cyan, light-white, dim-black, dim-red, dim-green, dim-yellow, dim-blue,
dim-magenta, dim-cyan, dim-white, dim-foreground, light-foreground, cursor,
vi-cursor, tabs, tabs-active, selection-background, selection-foreground,
split, split-active, search-match-background, search-match-foreground,
search-focused-match-background, search-focused-match-foreground,
hint-foreground, hint-background
```

Colors use `#RRGGBB`. Semantic DevOps identities retain their distinct anchor
hues and apply contrast correction against the context background; user theme
colors still own terminal ANSI output, search, and selection precedence.

## Hints

Hints discover URLs/paths without executing terminal text as a shell command.
The default rule recognizes hyperlinks and local-looking paths and opens them
through a platform handler with `Ctrl+Alt+O`.

```toml
[hints]
alphabet = "jfkdls;ahgurieowpq"

[[hints.rules]]
regex = "https://[^ ]+"
hyperlinks = true
post-processing = true
persist = false
action = "Copy"
mouse = { enabled = true, mods = ["Alt"] }
binding = { key = "O", mods = ["Control", "Alt"], mode = [] }
```

Built-in actions are `Open`, `Copy`, `Paste`, `Select`, and
`MoveViModeCursor`. A command can be a string or `{ program, args }`, but matched
terminal content is hostile input: prefer `Open`, which uses exact platform
handler arguments without shell interpretation.

## Custom bindings

`[bindings].keys` accepts `key`, `with`, `action`, `esc`, and `mode`. Explicit
valid bindings replace matching defaults; invalid/unknown actions are reported
and the default survives. The complete syntax, action names, modes, and active
platform shortcuts are in [Keyboard and input reference](KEYBOARD.md).

## Platform-specific overrides

Base settings load first; the active platform table selectively overrides
`shell`, `theme`, supported `window`, `navigation`, and `renderer` fields and
appends `env-vars`:

```toml
[platform.windows.shell]
program = "pwsh"
args = ["-NoLogo"]

[platform.linux.window]
decorations = "Enabled"

[platform.macos.renderer]
backend = "Metal"
```

Only fields represented by the platform override schema are accepted there.
Top-level cursor, fonts, bindings, hints, panel, and developer settings remain
shared. Navigation safety clamps run after the platform merge.

## Runtime reload

Bind the stable `ReloadConfig` action if you want an explicit reload shortcut:

```toml
[bindings]
keys = [
  { key = "F5", with = "control | shift", action = "ReloadConfig" },
]
```

Reload parses and prepares a complete replacement before swapping it into the
application. A malformed, unreadable, oversized, or invalid-theme candidate is
reported while every window keeps the last known-good configuration. Some
launch-only settings affect newly created sessions/windows rather than a live
PTY.

## Migration and coexistence

If no Automexia config exists on v0.4 startup, bounded migration can import
Rio's `config.toml`, custom themes, and known extension activation markers.
Automexia never modifies Rio data, copies logs/caches/executable content, or
installs a `rio` alias. Existing Automexia files win. See
[Rio configuration migration](MIGRATION.md).

## Related references

- [Keyboard and input](KEYBOARD.md)
- [CLI and automation](CLI-REFERENCE.md)
- [Shell integration](SHELL-INTEGRATION.md)
- [Liquid Hacker UX](LIQUID-HACKER-UX.md)
- [Architecture](ARCHITECTURE.md#runtime-configuration-transaction)