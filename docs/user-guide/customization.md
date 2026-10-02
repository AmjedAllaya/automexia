# Configuration and customization

Automexia is designed to work with **zero configuration**. The most maintainable customization strategy is to override only what you intentionally want to change.

## Choose the right kind of customization

| Need | Best approach |
|---|---|
| Different directory for one launch | `automexia --working-dir <PATH>` |
| Different shell/program for one launch | `automexia ... -e <PROGRAM> [ARGS...]` |
| Permanent declarative terminal preference | `config.toml` |
| Font size, appearance, shortcuts or supported customization choices changed in the running UI | Saved automatically for the next launch |
| Turn table formatting, output highlighting or timestamps on or off | Open the feature page under **Customizations → Open Customizations** |
| Turn an installed extension feature on or off | Open its feature page under **Customizations → Open Customizations** |
| Show, hide or style information tags | Open **Information tags** under **Customizations → Open Customizations** |
| Change command backgrounds or log colors | Open **Terminal output colors** under **Customizations → Open Customizations** |
| Change Kubernetes status colors | Open **Kubernetes status colors** under **Customizations → Open Customizations** |
| Different preferences by platform | Platform-specific config override tables |
| Temporary diagnostic logging | `--enable-log-file` or log environment override |
| Frequent UI action on another key | Double-click its palette badge or select it and press F2; use `[bindings]` for advanced mappings |
| Occasional UI action | Command palette instead of adding a binding |
| Different project launcher | Desktop/script launcher with CLI options rather than global config |

This separation prevents the global config from becoming a collection of one-off project assumptions.

See [Customize a shortcut](shortcuts.md#customize-a-shortcut-in-the-palette) for
recording, conflict checks, Save/Reset, persistence and recovery. UI shortcuts use
the same private preference store as font size and appearance, without rewriting
your declarative configuration. Advanced bindings remain configuration-owned.

## Understand what controls the appearance

Fonts and line height control text density; themes control colors; window and
navigation settings control the surrounding terminal chrome. Cursor settings
control the insertion indicator. Start with a readable font and a solid
background, then adjust one of these groups at a time. Check both a narrow split
and a full-size pane before keeping a change.

The [visual language](../LIQUID-HACKER-UX.md) explains the existing hierarchy:
terminal content first, a clear active pane, restrained accents and compact
supporting controls. Appearance controls do not rewrite retained output or grant
execution authority. Platform-specific effects and unsupported settings must not be
assumed available merely because a theme looks similar on another system.

Application dialogs retain Automexia's blue-black surfaces and brighter focus
cues. The palette, Connection Hub and quit confirmation use quieter borders and
clearer label hierarchy; palette shortcut badges are visually shortened when
space is limited. This does not change a binding or your terminal font, ANSI
colours, tab colour, line height or saved settings. No reset or migration is needed.

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

The root can contain `config.toml`, `themes/`, `extensions/`, `logs/`, and the
application-owned `state/` directory.

## 2. Open the config from Automexia

Use:

- Windows: `Ctrl+,`
- Linux/BSD: `Ctrl+Shift+,`
- macOS: `Cmd+,`

These shortcuts retain the external configuration editor. The palette calls it
**Edit Configuration File**. By default the reference defines Notepad on Windows
and `vi` on Unix unless you override `[editor]`.

To change supported feature switches inside Automexia, open **Customizations
→ Open Customizations**, or press **Ctrl+Shift+S** (**Cmd+Shift+S** on macOS). The
shortcut opens the same feature list. It shows separate pages for Information tags,
Terminal output colors, Kubernetes status colors, Inline tables, Command timestamps, Theme, Font size,
and features of installed extensions. The **Information tags** page keeps shared
controls such as visibility, format, shape and spacing. Click a tag in the
sample or its button in the list below. Its enabled state, text,
icon, position and appearance controls open in the left half of the same sheet.
The tag's own color and its inherited role color are separate controls. Use
the tag list to select disabled tags too. **On** and **Off** describe each tag's
visibility setting. Every enabled tag has sample data, even when live DevOps
detection is off or its extension is absent. Real terminal tags still require
detected context. **E** or the preview's **Edit** button enables stable selection through
the same list, including **Add custom tag**. **Terminal output colors** controls
ordinary command backgrounds and the separate detected-log highlighting.
**Kubernetes status colors** controls Kubernetes readiness and status colors
independently. Recognized tables include pods, deployments, StatefulSets,
ReplicaSets, DaemonSets, jobs, CronJobs, nodes, volumes, namespaces, services,
events, autoscalers, certificates and API services. Colors follow status fields
through wrapping; resource names do not decide health. Unknown states stay
neutral, and resources without health evidence are informational. Custom output
needs recognizable headers, resource prefixes or condition fields; arbitrary
JSON/YAML and custom-resource semantics are not inferred.
Each page keeps shared switches and styles in the list; its
interactive preview opens the selected sample's colors. Disabling either feature
keeps its saved colors. Command backgrounds preserve ANSI text, explicit source
backgrounds and selections, and tint ordinary table data without changing headers
or borders. Kubernetes rows always keep their separate color ownership; recognized
logs use their own colors while log highlighting is on. Blank rows are not tinted.
The footer shows **E: edit preview**, including after saving and inside an
element's controls. Cyan keys distinguish shortcuts from actions; the compact
preview header turns green while editing. Press **E** outside a text
field or click **Edit** to select preview elements. **Tab / Shift+Tab**
cycles only those elements and wraps at either end; arrows, Home, End, PageUp
and PageDown also move selection. **Enter / Space** opens the selected element's
controls and leaves preview selection mode. From those controls, **E** returns
to preview selection with the same element selected; **Esc** returns to the
shared feature controls. **Done**, **Esc** or **Alt+Left** in preview selection leaves
that mode and focuses Edit. Search and value editors keep typed letters,
including E; Escape first cancels a draft or IME composition. Clicking another
control transfers focus out of preview selection. The mouse wheel scrolls the
sample in a short pane. Escape or Alt+Left returns from shared feature controls
to the feature list; Escape from the list closes the sheet. These routes use the
same saved preferences. Each feature page shows a
live sample beside its
controls on a wide window, or beneath them in a narrow one. The sample updates
when a choice is applied without closing the sheet; its fictional text never
runs an extension or queries a provider. In a feature page,
outside preview selection, Tab moves between search, the list, Edit,
Reset default, Restore saved when available, and Close; up/down selects a control,
left/right adjusts a number or choice, and Space changes a boolean. For a number,
press Enter or click its center value to type or paste the value directly. Enter
applies a valid value, Tab applies it and moves focus, and Escape cancels the
draft. The minus and plus buttons remain available. Values outside the control's
range or step cannot be applied. Select a color row in an item editor to open
its Current/Draft preview. Shift+Arrow/Home/End selects field text; Ctrl+C/X/V
(Cmd+C/X/V on macOS) copies, cuts, or pastes it. A clipboard failure keeps the
draft and selection available to retry. **Reset all** on the feature list previews
defaults across Customizations, enabling the core feature switches; installed
extension features use their declared defaults. **Reset default** on a feature
page previews that feature's defaults, and **Reset tag** affects only the
selected tag.
Appearance resets restore your configured colors and styles, or built-in defaults
where none are configured. Core feature switches are enabled in the affected scope.
These previews do not change saved preference files. Further changes made
while a reset preview is active stay in memory. **Restore saved** returns to
the choices from before the first reset, even after closing and reopening
the sheet; restarting also reloads the saved choices. A Reset inside an
individual value or color editor still clears that value's override.

Buttons show their shortcuts: **R** resets, **S** restores saved customizations,
**C** closes the sheet, and **Esc** goes back. Reset and Restore open a confirmation
dialog naming the affected scope. **Cancel** is selected first; **Esc** or **N**
cancels, **Y** confirms, and **Tab** then **Enter** selects and activates the other
button. Cancel keeps the current draft and choices. Restore is disabled when no
temporary defaults are active. Letters stay text in search and value fields;
outside the color field, **A** applies and **R** requests a reset. If the window
is too small to show the confirmation, enlarge it or press **Esc** to cancel.

Turning Information tags off hides their prompt badges while retaining
context detection and the saved tag colors. The Information-bar format
control offers twelve layouts; changing the format updates open terminal
panes and, outside a temporary reset preview, is saved for the next launch.
The Information tags page also controls the overall shape, arrangement, and
**Space between tags**
from 0% to 300% of the selected format's normal gap (100% is unchanged).
The shape choices include **Soft chevrons**, **Hexagonal chips**, **Puzzle joins**,
**Slanted tags**, **Pills and arrows**, **Folded ribbon**, **Cut corners**,
**Alternating triangles**, **Top notch**, and **Separator wedges**. Capsule,
Flat, Chevron, Card, and Underline remain available. Connected shapes have
matching joints at 0% spacing; increase **Space between tags** to separate them.
Each wrapped row or split group gets its own end caps. Click the colored tag
itself to edit it; empty corners and cutouts do not select the neighboring tag.
Plain tags keep a subtle outline of the selected shape without a background
tint; underline tags keep their colored bottom rule. The preview uses the same
shape and wrapping rules as the terminal, with fictional sample context.
Each **Tag slot:** page reached through the preview has controls for visibility,
text source, literal text,
icon source, icon context or fixed icon, side, order, color, prefix, and suffix.
The thirteen standard context slots can be hidden and reordered. **Add custom
tag** creates up to three additional display-only slots, each with its own
page and Remove action, within the 16-slot recipe limit. Imported recipe slots
with other IDs remain saved when these controls are changed, although they do
not receive a dedicated page.
Choosing a slot source or changing its appearance creates a custom copy of the
selected format; switching back to a preset keeps that copy available under
**Custom layout**. Text and icons can come from different context sources: for
example, choose **User** as the Windows slot's text source and **Windows** as
its fixed icon. Switching source modes retains the previous literal, context,
and fixed-icon choices so they can be selected again. Custom text is display-only
and never runs a command or provider.
Select a text row to open its bounded editor; it accepts keyboard, mouse,
clipboard, and IME input, with Apply, Cancel, and Reset controls.
Each recognized output status has its own text and translucent background color.
Background-only style tints all five status types, while Text and background
keeps the familiar default unless you choose more backgrounds. Type an RGB hex
color, or RGBA for an output background; compare current and valid draft
graphics before applying. Tab and Shift+Tab cycle the input, Apply, Cancel and Reset;
Enter activates the selected control. Mouse clicks work on the same controls.
Escape cancels the editor; in Customizations, subsequent Escape presses return
through the current page hierarchy. An invalid hex value cannot be
applied. Reset inherits current configuration or the active theme palette
instead of copying a default into the override file. Saving status appears in
the sheet.

Each command or status preview opens a **Background opacity (%)** control. Click
the number and type a value from **0** (transparent) to **100** (solid), or use
the step buttons. The preview and retained output update after applying the value;
zero opacity stays transparent during a completion pulse. Opacity uses the existing
background color setting. Resetting opacity restores the configured alpha while
keeping the chosen color; resetting the color restores both. Feature switches and
Text-only style keep those choices for later without displaying a background.

Installed built-in extensions add their supported feature pages to this list. Turning
off a feature keeps its control available to turn back on. Removing its
extension removes its entries. An unavailable integration explains why its
control cannot be changed.

Packages accepted by the installer that declare `settings.v1.json` add a package
category automatically. Open it to see a separate page for each declared
feature, then open a feature to save its enabled preference and supported
boolean, choice, or integer options. These are saved choices only; package
component execution is unavailable. After a confirmed uninstall and successful
inventory refresh, the package pages and saved choices disappear. If package
inventory cannot be read, Customizations
keeps its built-in controls available and does not treat the read failure as an
uninstall. The sheet reports the failure; reopen Customizations to retry.
An uninitialized package store also preserves saved choices. A confirmed uninstall
during a temporary reset removes that package's choices from the restore snapshot
without writing your files. Reinstalling during that preview uses its declared
defaults; restarting still loads saved files.
Editing a new option at the saved-choice limit frees retired options from that
package only. Feature and package resets also clear their retired choices.

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
max-tab-width = 184

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

## 5. Configure fonts and saved runtime zoom

Example:

```toml
[fonts]
size = 18.0
family = "Cascadia Code"
features = ["calt=1", "liga=1"]
```

Runtime zoom applies to all open panes and windows:

- Windows/Linux/BSD: `Ctrl+0`, `Ctrl+=`/`Ctrl++`, `Ctrl+-`
- macOS: `Cmd+0`, `Cmd+=`/`Cmd++`, `Cmd+-`

The chosen size is saved automatically and restored before the first window is
created on the next launch. Reset clears that saved override and returns every
pane to the current configured size.

**Settings → Font size** uses the same saved value. Its controls change one point
within 6–100 points while retaining fractional sizes; Reset inherits configuration.
All local tabs update, including inactive ones. Font family and features remain
configuration-owned.

Use runtime zoom for a convenient remembered preference and config for the
declarative default you want Reset to recover. The UI does not rewrite or
reformat `config.toml`.

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
max-tab-width = 184
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

**Settings → Appearance** offers **Use configuration**, **Light** and **Dark**
when both adaptive palettes are loaded. Use configuration or Reset removes the
override and follows the configured force-theme, or the host if none is configured.
A fixed palette explains why this control is unavailable and retains the saved
choice. Settings does not discover or load named themes.

The appearance shortcut also saves the selected light/dark choice. To try defaults
without changing saved files, use **Reset all** in Customizations; **Restore saved**
returns to your previous choices. Resetting an individual value instead clears
that value's saved override when no temporary preview is active.

For a deliberate persistent reset, first close every Automexia instance. Back up
and move only these snapshots out of the configuration root's `state/` directory:

- `user-preferences-v6.toml` and `user-preferences-v6.previous.toml`;
- any retained `user-preferences-v5.toml`, `user-preferences-v4.toml`, `user-preferences-v3.toml`, `user-preferences-v2.toml`,
  `user-preferences-v1.toml`, and their matching `.previous.toml` files.

Moving both current and older snapshots prevents recovery or migration from
bringing those choices back. To also reset installed-package choices, back up
and move `package-preferences-v1.toml` and `package-preferences-v1.previous.toml`.
Keep `config.toml` and the rest of `state/` untouched. The next launch inherits
your configuration and declared feature defaults; appearance follows its
configured theme policy. Keep the backups if you want to restore your choices
while Automexia is closed.

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

## Current implementation limits

The [terminal interaction status](../TERMINAL-INTERACTION-REQUIREMENTS.md)
identifies current UI and persistence owners. This guide describes only
settings supported by current source, not additional configuration options.

The Customizations sheet edits font size, supported adaptive appearance,
output-presentation switches and declared built-in extension presentation controls.
It groups these controls by feature and does not expose every `config.toml` option.
It does not yet edit every setting in this guide. Existing font, appearance and
shortcut controls share the same application preference writer. Other
declarative preferences remain in `config.toml`. See
[runtime preference ownership](../adr/0036-application-owned-runtime-user-preferences.md)
for precedence, reset, recovery and storage limits. Existing configuration
support does not establish release or native evidence for every combination.
