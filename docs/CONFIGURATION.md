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
Settings choices use the private, versioned `state/user-preferences-v9.toml`
overlay. It contains explicit UI overrides, not a second general configuration.
Version-8 preferences import only when both version-9 snapshots are absent;
older versions import only when every newer snapshot pair is absent.
Version 9 adds font and terminal palette overrides. Version 8 added command
timestamp appearance; version 7 added inline table appearance.
Version 6 added connected tag shapes. Older files remain unchanged
for rollback; corrupt or future current snapshots do not fall back to older files.
`AUTOMEXIA_CONFIG_HOME` replaces the complete root. `AUTOMEXIA_LOG_LEVEL`
overrides the configured log level. For v0.4 only, `RIO_CONFIG_HOME` is a
read-only migration source and `RIO_LOG_LEVEL` is a deprecated value fallback;
both warn once per start and are removed in v0.5.

Create a starter file containing the canonical reference pointer without overwriting an existing file:

```text
automexia --write-config
automexia --write-config D:\configs\automexia.toml
```

Creation stages and synchronizes the complete starter file in the destination
directory, then publishes without replacing an existing ordinary file. Links,
reparse points, directory destinations and I/O failures are reported; CLI failure
returns a nonzero status. Explicit paths require their parent directory to exist.
File synchronization does not promise recovery from every filesystem or power-loss
failure; see [ADR 0070](adr/0070-validated-startup-configuration.md).

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
| `env-vars` | string array / `[]` | Extra `NAME=VALUE` entries for child sessions. Names are trimmed; values may be empty or contain additional equals signs. Platform entries append. Missing separators, empty names or NUL reject the whole candidate before application. Do not store secrets in committed config. |
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
- font family, weights, styles, spacing, features and terminal colors from Fonts;
- the forced light/dark appearance selected by the appearance shortcut;
- shortcut overrides saved by the existing palette shortcut editor;
- table formatting, status highlighting and command timestamp display;
- information-bar format, tag recipes, spacing and appearance;
- declared presentation features of installed built-in extensions, currently
  DevOps prompt context.

Writes are coalesced off the input/rendering path, bounded to 16 KiB, restricted
to the current user, and retain one last-known-good snapshot. A malformed,
oversized, linked, permission-denied, or contended file never replaces live
configuration; Automexia reports a warning and uses the recovered snapshot or
`config.toml` values. Invalid or newer current-version data is never replaced by an
automatic predecessor import. To clear individual choices, use Reset. To clear all
runtime overrides, close Automexia and move the version-9 primary and
previous snapshots and all retained version-1 through version-8 snapshots to a backup; leaving
older files would import their choices again. This file never stores credentials, terminal contents,
history, paths, tabs, panes, sessions, or provider state.

## Runtime font and appearance

Customizations includes **Fonts**, with these controls:

- terminal size (6–100 points), installed family, regular/bold weights (100–900);
- bold and italic faces, line spacing (0.8–3 times), ligatures, hinting and box glyphs;
- OpenType features (up to 32 comma-separated four-character tags, optionally `=0`
  or `=1`, within 128 bytes); the Ligatures choice overrides `liga` and `calt`;
- default, bright and dim text, terminal background, cursor, selection colors,
  and the 16 normal/bright ANSI palette colors.

Family names are limited to 128 bytes and cannot contain path separators or
control characters. Font family and faces are shared by terminal and interface
text. Settings text follows the terminal size within its existing 10–32 point
limits. Actual weights,
ligatures and hinting depend on the installed font and platform renderer.
Explicit application RGB and custom output/Kubernetes colors retain their owners.
Window transparency remains an appearance setting.

The font-size control uses the existing runtime range.
The arrow controls change one point while retaining fractional sizes; the center
field also accepts a typed value within the allowed range. Reset
clears the saved override and inherits the current configured size. An unsupported
configured size has an explanatory unavailable control; Customizations does not rewrite
or clamp the configuration file.

**Appearance** offers **Use configuration**, **Light** and **Dark** when both
adaptive palettes are loaded. Use configuration and Reset remove the saved
appearance override: a configured force-theme takes precedence, otherwise the
host appearance applies. A fixed palette has an unavailable control with an
explanation; its saved choice is retained. Customizations does not discover or load
theme files.

Font and appearance edits update open windows, panes and inactive local tabs
through the existing preference owner. Resource changes prepare one font library
on a bounded background worker before publishing. Missing fonts, loading failures
or a 30-second timeout leave the current choices active and show an explanation.
Closing the editor, switching its pane or changing settings cancels stale results.
Size, spacing and palette edits reuse loaded fonts; adaptive palette overrides apply
to both loaded appearances. These changes never reload background images or rewrite
configuration. The existing asynchronous writer persists the overrides.

Reset default previews the configured Fonts values after confirmation; Restore saved
returns to the earlier choices. Individual value resets clear only that override.
Per-face named styles, symbol maps and additional font directories remain in
`config.toml`. No fonts are installed or downloaded by this page.

## Output presentation

The command palette's **Customizations → Open Customizations** opens a feature
list; each feature has a separate page for its supported switch and appearance
controls. The existing Settings shortcut opens this same list for compatibility.
Changes apply across windows, panes and tabs. **Reset all** or a feature's
**Reset default** previews defaults for the current session without changing
saved preference files; **Restore saved** returns to the choices from before
the first reset. Appearance resets restore configured colors and styles, or
built-in defaults where none are configured, and enable affected core switches.
Reset inside an individual value editor still clears that
value's UI override and inherits the current configuration.
**Edit Configuration File** and its existing
shortcut still open the external editor.

```toml
[presentation]
inline-tables = true
output-highlighting = true
command-output-highlighting = true
kubernetes-highlighting = true
command-timestamps = true
```

| Key | Default | Effect |
|---|---|---|
| `presentation.inline-tables` | `true` | Add borders and per-cell wrapping to recognized header tables. Disabled output uses the original terminal grid. |
| `presentation.command-output-highlighting` | `true` | Tint ordinary completed-command output by exit status in Customizations → Terminal output colors. |
| `presentation.output-highlighting` | `true` | Color detected logs and general statuses, independently of command backgrounds and Kubernetes. |
| `presentation.kubernetes-highlighting` | `true` | Color recognized Kubernetes readiness and status rows in Customizations → Kubernetes status colors. |
| `presentation.command-timestamps` | `true` | Show completion timestamps. Disabling this keeps exit status, duration and terminal-owned command metadata. |

**Command timestamps** customizes the date, time and result independently. Each
can appear before/after the tags or on a row above/below them, aligned left or
right. Components in the same position follow the selected order and separators.
Verified command-information rows wrap without changing terminal text or copied
output. Legacy anchors without a verified blank row keep a safe right-hand lane;
if the clock cannot fit there, only the enabled result details are shown.

Optional overrides under `[presentation.timestamps]`:

| Key | Default | Values / effect |
|---|---|---|
| `date-format` | `"year-month-day"` | `"year-month-day"`, `"day-month-year"`, `"month-day-year"`, `"day-month-name"`, `"month-name-day"`, `"hidden"`. Named months use English. |
| `date-separator` | `"dash"` | `"dash"`, `"slash"`, `"dot"`, `"space"`; numeric dates only. |
| `time-format` | `"24-hour"` | `"24-hour"`, `"12-hour"` with AM/PM, or `"hidden"`. |
| `precision` | `"seconds"` | `"minutes"`, `"seconds"`, `"milliseconds"`. |
| `timezone` | `"recorded"` | Local calendar values captured at completion, or `"utc"` derived from that completion's epoch. No live timezone lookup. |
| `weekday` | `false` | Prefix the date with the English weekday. |
| `zone-label` | `false` | Append UTC or the captured local offset; follows the date if time is hidden. |
| `date-position`, `time-position`, `result-position` | `"right"` | Independently choose `"left"`, `"right"`, `"above-left"`, `"above-right"`, `"below-left"`, `"below-right"`. |
| `order` | `"result-date-time"` | Any of the six permutations of `result`, `date`, `time`, joined with hyphens. |
| `separator` | `"dot"` | `"dot"`, `"space"`, `"pipe"`, `"dash"` between result and clock components sharing a position. |
| `date-time-separator` | `"space"` | The same choices, between adjacent date and time. |
| `show-status`, `show-duration` | `true` | Independent visibility; unknown status uses a neutral symbol and missing duration reads `done`. |
| `show-exit-code` | `false` | Show the shell's reported numeric exit code; `?` means unavailable. |
| `duration-format` | `"auto"` | `"auto"`, `"milliseconds"`, `"seconds"`, `"clock"` (hours:minutes:seconds.milliseconds). |
| `size` | `"normal"` | `"small"`, `"normal"`, `"large"`, relative to information-tag text. |
| `bold` | `false` | Emphasize completion text. |
| `status-colors` | `true` | Follow the theme's success/failure/unknown accents; false follows normal text. |
| `date-color`, `time-color`, `result-color` | inherited | Independent `#RRGGBB` overrides. Reset removes the UI override. |
| `background` | transparent | Label-only `#RRGGBBAA`; the UI also exposes opacity from 0% to 100%. |

Date/time visibility does not erase captured command metadata or appearance
choices. Result controls remain independent of the date/time switch. Label colors
and backgrounds do not change command-output, log, table or Kubernetes colors.
The live sample uses the terminal's formatter, layout and text styling.

Open **Inline tables** to choose solid, dashed, dotted, double or no borders,
adjust their weight, and toggle the outer frame, row separators, column separators
and header separator independently. Choose **Alternating backgrounds** for zebra
rows, alternating columns or a checkerboard. Wrapped lines retain their logical
row color. Header, ordinary and alternate cells have separate text/background
colors; border and background colors also have opacity controls. The sample
uses ordinary file data so status highlighting does not mask your choices.
Explicit terminal colors, selection and enabled semantic status colors keep
priority in real output. Disabling inline tables retains these appearance choices.

Configuration equivalents are optional overrides under `[presentation.tables]`:

| Key | Default | Values / effect |
|---|---|---|
| `border-style` | `"solid"` | `"none"`, `"solid"`, `"dashed"`, `"dotted"`, `"double"`. |
| `border-weight` | `"thin"` | `"thin"`, `"medium"`, `"thick"`. |
| `outer-border` | `true` | Draw the table frame. |
| `row-lines` | `true` | Draw separators between data rows. |
| `column-lines` | `true` | Draw separators between columns. |
| `header-separator` | `true` | Draw the line beneath the header. |
| `header-bold` | `false` | Use bold header text. |
| `banding` | `"none"` | `"none"`, `"rows"`, `"columns"`, `"checkerboard"`. |
| `border-color` | Theme outline | `#RRGGBBAA`; alpha controls border opacity. |
| `header-foreground` | Terminal foreground | `#RRGGBB`. |
| `header-background` | Theme raised surface | `#RRGGBBAA`. |
| `body-foreground` | Terminal foreground | `#RRGGBB`. |
| `body-background` | Terminal background | `#RRGGBBAA`. |
| `alternate-foreground` | Body foreground | `#RRGGBB`. |
| `alternate-background` | Theme raised surface | `#RRGGBBAA`. |

For a striped table without internal rules:

```toml
[presentation.tables]
banding = "rows"
row-lines = false
column-lines = false
border-style = "dotted"
border-color = "#60708080"
header-background = "#243848FF"
alternate-background = "#20304080"
```

Open **Information tags** in Customizations to show or hide the prompt tags,
choose one of twelve **Information-bar format** presets, adjust **Space between
tags** from 0% to 300% of the format's normal gap, and use **Context tag
style** and **Context tag background opacity** for shared appearance. Use the
preview's Edit button to select a tag or default role color. Disabling tags
leaves context detection and command metadata intact. Tinted tags use an integer opacity from
0 to 100 percent; Plain removes the tint while retaining readable text. User
colors take precedence over configured colors and built-in anchors. The sheet
checks text contrast against the rendered tag surface.

Open **Terminal output colors** in Customizations to enable command backgrounds
and the optional completion pulse. Select a successful, failed, or unknown-exit
sample to edit its RGBA background, including opacity. The separate detected-log
switch and style control log text and backgrounds. Completion labels and
timestamps remain available when backgrounds are off.

Open **Kubernetes status colors** for an independent switch, style and palette.
Select a sample status to edit its text or background. An incomplete `0/1 Running`
row remains a warning; `1/1 Running` is successful. Unknown recognized statuses
stay neutral instead of borrowing a command's successful exit color.

The detected-log and Kubernetes styles select Text, Background, or Text and
background independently.
Recognized error, warning, success, info and debug text and RGBA background
colors can each be changed. Background style tints all five by default; Text
and background retains the original error/warning-only default backgrounds,
and applies any background colors you explicitly choose for the other three.
Highlighting
uses the bounded core classifier; original ANSI colors, inverse video,
selection and hover keep their normal precedence. Command backgrounds never
overlay recognized status rows or inline tables, which own their cell colors.
Complete softwrapped rows share one classification; incomplete or oversized
rows remain neutral. Full-screen and mouse-reporting applications keep their
own colors. Turning a feature off retains its saved appearance choices.

For older configurations, omitted Kubernetes controls inherit the old
`output-highlighting` and `highlight` choices when loaded. Set the new Kubernetes
fields explicitly to make them independent. Private preference schema v5 imports
older choices once and leaves predecessor files unchanged for rollback.

```toml
[presentation.tags]
enabled = true
style = "tinted"
opacity = 12
[presentation.tags.colors]
kubernetes = "#48c7ef"

[presentation.highlight]
style = "both"
error-background = "#690c1956"
success-background = "#09572e4e"
[presentation.highlight.colors]
error = "#ff1261"
[presentation.command-output]
success = "#00d08019"
failure = "#f0406019"
neutral = "#5080d019"
pulse = true
[presentation.kubernetes]
style = "both"
[presentation.kubernetes.colors]
warning = "#ffd166"
success = "#06d6a0"
```

In Information tags or either output color page, choose the preview's Edit button,
then select a rendered tag or status directly in the sample. A selected tag's
own color and inherited role-default color have separate rows. Use **Show
hidden/other tags** in the preview when a tag is absent or disabled.
Enter an exact RGB hex
value (or RGBA for a background), then choose Apply, Cancel or Reset by mouse,
Tab/Shift+Tab and Enter; Escape cancels. Apply is unavailable for malformed
input. Reset inherits the current configuration or active palette. The color
editor shows current and draft graphics before saving. Escape returns one level
at a time through the element controls, shared feature controls and feature list.

The installed-extension list is loaded asynchronously. Supported built-in DevOps
controls appear from that inventory in Customizations; disabling a feature retains its
control, while removing the built-in extension removes the entry and its saved
feature override. Packages verified at install time with valid `settings.v1.json`
metadata add one package page, then a separate page for each declared feature.
The feature page saves its enabled preference and declared boolean, choice, and
integer options. Those choices are stored separately in the private, versioned
`state/package-preferences-v1.toml` snapshot. After a confirmed uninstall and
successful inventory refresh, its pages and saved choices disappear. An
unavailable inventory does not imply
uninstall. Package controls save preferences only: they do not execute package
components, grant provider permissions, or activate protected integrations.
On refresh, extracted content is checked against its stored receipt and content
digest; this does not renew publisher signature trust after local profile tampering.

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
