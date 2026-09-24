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

The root contains `config.toml`, `themes/`, `extensions/`, `logs/`, and
application-owned `state/`. Runtime font, appearance, shortcut and supported
Settings choices use the private, versioned `state/user-preferences-v2.toml`
overlay. It contains explicit UI overrides, not a second general configuration.
Version-1 preferences import only when both version-2 snapshots are absent;
the old files remain unchanged for rollback.
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

## Explicit command preferences

Optional `amx.toml` beside `config.toml` configures only the explicit `amx edit`
desktop handoff. It is not loaded at terminal startup and does not replace the
terminal settings editor. Use `version = 1` and `editor = "vscode"`,
`"vscode-insiders"` or `"disabled"`. The strict regular-file limit is 16 KiB;
unknown/invalid fields fail closed. Automexia never writes this file. See
[editor configuration, overrides and rollback](user-guide/edit-file.md).

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
max-tab-width = 184

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
max-tab-width = 184

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
| `max-tab-width` | float / `184` | Clamped to `80…280` logical pixels. |
| `color-automation` | array / `[]` | Program/path-specific tab colors; retained for inherited compatibility. |
| `clickable` | bool / `false` | Inherited navigation click behavior; normal Automexia tabs remain interactive through native routing. |
| `panel.margin`, `panel.padding` | 1/2/4 floats / `[2]`, `[5]` | Per-pane inner spacing. |
| `panel.row-gap`, `column-gap` | float / `0` | Space between split panes. |
| `panel.border-width`, `border-radius` | float / `2`, `0` | Split border geometry. |

Pane-local tab rails and footers use responsive product invariants and have no
v0.4 user setting. See [Liquid Hacker UX](LIQUID-HACKER-UX.md).

Command information wraps automatically when the pane is too narrow for its
context badges and completion timestamp. No setting is required. Use scrolling
or keyboard paging to reach additional information lines in a short window;
typing returns to the live command. Widening restores a single row when it fits.
These display rows do not change the shell's dimensions or copied output.

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
default size is 18 pt. Runtime `Ctrl`/`Cmd` zoom applies to every open pane and
window and is restored on the next launch. Reset clears the saved font override
and returns to this configured size. The saved override has precedence during a
config reload until Reset is used.

## Saved runtime preferences

Automexia automatically saves the settings that can be changed directly from
the running UI:

- font size changed with `Ctrl`/`Cmd` plus `+`, `-`, or `0`;
- the forced light/dark appearance selected by the appearance shortcut;
- shortcut overrides saved by the existing palette shortcut editor;
- table formatting, status highlighting and command timestamp display;
- declared presentation features of installed built-in extensions, currently
  DevOps prompt context.

Writes are coalesced off the input/rendering path, bounded to 16 KiB, restricted
to the current user, and retain one last-known-good snapshot. A malformed,
oversized, linked, permission-denied, or contended file never replaces live
configuration; Automexia reports a warning and uses the recovered snapshot or
`config.toml` values. Invalid or newer version-2 data is never replaced by an
automatic legacy import. To clear individual choices, use Reset. To clear all
runtime overrides, close Automexia and remove the version-2 primary and
previous snapshots and any retained version-1 snapshots; leaving version-1
files would import their choices again. This file never stores credentials, terminal contents,
history, paths, tabs, panes, sessions, or provider state.

## Output presentation

The command palette's **Settings** opens the native application settings sheet.
Changes apply across windows, panes and tabs; Reset clears the UI override and
inherits the current configuration. **Edit Configuration File** and its existing
shortcut still open the external editor.

```toml
[presentation]
inline-tables = true
output-highlighting = true
command-timestamps = true
```

| Key | Default | Effect |
|---|---|---|
| `presentation.inline-tables` | `true` | Add borders and per-cell wrapping to recognized header tables. Disabled output uses the original terminal grid. |
| `presentation.output-highlighting` | `true` | Color recognized statuses while the classifier extension is installed. Producer ANSI colors retain precedence. |
| `presentation.command-timestamps` | `true` | Show completion timestamps. Disabling this keeps exit status, duration and terminal-owned command metadata. |

The installed-extension list is loaded asynchronously. Supported extension
controls appear from that inventory; disabling a feature retains its control,
while removing its extension removes the entry and its saved feature override.
An unavailable inventory does not imply uninstall. Settings never grants
provider permissions or activates protected integrations.

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

Colors accept six ASCII hexadecimal digits (`RRGGBB`) or eight (`RRGGBBAA`),
with an optional leading `#` and either letter case. Alpha defaults to opaque.
Whitespace, extra markers and Unicode lookalike digits are rejected. Conversion
inspects at most nine input bytes before rejecting oversized values; errors never
echo the input. Failed theme reload retains the last working configuration.
Semantic DevOps identities retain their distinct anchor
hues and apply contrast correction against the context background; user theme
colors still own terminal ANSI output, search, and selection precedence.

## Hints

Hints discover URLs/paths without executing terminal text as a shell command.
The default rule recognizes hyperlinks and local-looking paths and opens them
through a platform handler with `Ctrl+Alt+O`.
Keyboard labels now select for review; Enter performs the configured action.
Tab/Shift+Tab navigate, Ctrl+Shift+C copies, Left/Right inspect long destinations,
and Escape returns without shell input. See [keyboard hyperlinks](user-guide/hyperlinks.md)
for default-opener safety, stale-capture behavior and resource limits.

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

The classic `[bindings].keys` table remains supported. Typed compatibility
configuration selects a profile independently and layers exact entries after it:

```toml
[keyboard]
binding-profile = "ghostty-1.3"
binding-strict = true

[bindings]
keybinds = [
  "ctrl+shift+t=new_tab:inherit",
  "ctrl+x>ctrl+s=write_screen_file:copy,plain",
  "ctrl+u=unbind",
]
```

Profiles are `automexia` (the implicit default), `ghostty-1.3` (pinned), and
`ghostty` (a visible moving alias). Typed lines support logical, physical, and
named keys; sequences; tables; chains; scopes; performability; consumption; and
explicit unbinds. Compilation is bounded and atomic. Strict errors reject the
candidate registry, so reload keeps the complete last-known-good profile,
shortcuts, palette hints, and OS global hotkeys.

Legacy `[bindings].keys` entries still accept `key`, `with`, `action`, `esc`,
and `mode`. A valid legacy user entry replaces its exact classic trigger;
invalid or unknown actions are reported and do not erase a default. Full syntax,
limits, migration, rollback, and generated inventories are in
[Ghostty keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md) and
[Keyboard and input reference](KEYBOARD.md).
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
- [Architecture](ARCHITECTURE.md#configuration-transaction)
