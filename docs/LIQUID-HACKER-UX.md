# Liquid Hacker UX

Automexia's native desktop UI follows the supplied liquid-hacker mockup while
remaining a real terminal: applications still own their PTY bytes and ANSI
styles, and renderer decoration never changes command output.

## Layout contract

At comfortable sizes, the first 148 logical pixels are persistent application
chrome and are never part of the terminal grid:

- a 66 px profile/tab row with an application mark, draggable tabs, new-tab
  button, command/profile menu, and native-looking window controls on Windows;
- a 47 px operational context surface at y=82 with local OS/WSL, Git,
  Kubernetes, cloud, Docker, Terraform, environment and production facts;
- a separate right surface for the real shell name and local clock on
  comfortable windows. Two inset glass chips establish clear hierarchy with
  terminal/clock glyph wells, restrained uppercase labels, emphasized values,
  shell-aware accents, and theme-corrected contrast. Narrow windows give the
  full row to context instead.

Chrome has one shared responsive contract for drawing, hit-testing and terminal
grid reservation:

| Density | Trigger (logical viewport) | Header | Context behavior |
|---|---|---:|---|
| comfortable | at least 840 px wide and 480 px high | 66 px | 47 px row; separate shell/clock surface |
| compact | below either comfortable threshold | 54 px | 41 px single context surface |
| minimal | below 480 px wide or 280 px high | 46 px | 38 px row when height permits |

Below 260 logical pixels of height, the context surface folds away and the
minimal header reserves only 54 px, leaving 146 px for terminal content at the
supported 300×200 minimum. The prompt-level context row remains available in
the grid. As width contracts, controls fold in priority order: the product mark,
palette chevron, and then new-tab button hide before the active tab can collide
with the always-reachable minimize, maximize and close controls. Narrow
multi-tab strips use icon-only tabs when a readable title no longer fits.

The terminal grid begins below the live reservation, which is recomputed on
every viewport and DPI change. Typing, output, scrollback and resize/reflow
therefore cannot erase the tabs or context bar, nor can a stale 148 px margin
consume a compact window. Windows uses a 6 px renderer-owned resize frame and
supports all edges and corners when native decorations are disabled.

The per-command prompt uses a strict ownership boundary. Automexia writes the
context spacer and the complete, unabridged path once; the shell line editor
redraws only the lambda and editable input row. Resizing reflows the semantic
block by its stable `aid`, retains the entire logical path at physically tiny
sizes, and reveals it again immediately when enough columns return. A user
viewing scrollback is never forced back to the active cursor by resize.

The default window is 1280x760. Tabs remain visible with one session; users may
still explicitly set `navigation.hide-if-single = true` on platforms with
native decorations.

Command palette, search, diagnostic and quit overlays fit to the logical
viewport. The command palette reduces its visible result count with height,
long labels are ellipsized on Unicode boundaries, and editable input keeps its
tail visible. Split containers clamp negative available space and preserve the
combined adjacent-panel size when a divider reaches a compact limit. These
rules apply equally at 1× and HiDPI scale factors and do not upscale UI on very
large displays; the terminal grid simply gains rows and columns.

## Live operational context

The second row is a session-scoped live snapshot, not a static list of logos.
Automexia refreshes completed snapshots every three seconds and repaints a
worker result within 100 ms. Route-keyed scheduling is de-duplicated, so typing
or resizing cannot create duplicate refresh loops and one pane cannot publish
another pane's context.

| Segment | Appears when |
|---|---|
| OS/WSL | The active shell publishes a real distro, or the native host is known |
| Git | The current directory belongs to a branch |
| Kubernetes | `kubectl` or kubeconfig has a non-empty current context |
| Cloud | A local AWS, Azure, or GCP profile/account is selected |
| Docker | Docker is installed or has a selected local context |
| Terraform | The current project has an active workspace |
| Environment | Project or inherited Automexia environment metadata is set |
| User | The active shell publishes a user identity |

For a Windows-hosted WSL session, host environment variables do not describe
the Linux child. Automexia therefore runs one fixed, bounded metadata probe in
the active distro and current directory. It reads only local CLI/configuration
state (`docker context show`, `kubectl config`, Git, Terraform and cached cloud
configuration); it does not contact the Docker daemon, a Kubernetes cluster,
or a cloud API. Individual optional commands are limited to one second and the
complete probe has a 4.5-second hard deadline. Missing or unconfigured tools
remain hidden instead of displaying misleading badges.

## Visual language

The default canvas is blue-black (`#020B16`) with cyan/blue information,
purple Git state, green success, amber warning and red failure roles. The
application mark and window controls are vector primitives so DPI scaling does
not blur them. Product/context icons use the bundled Cascadia Code Nerd Font
and Symbols Nerd Font assets, never platform emoji, so the same codepoint has a
stable shape on every machine.

Operational identities use semantic brand anchors rather than generic terminal
palette slots. The same resolver colors both the persistent header and frozen
prompt-history snapshot, and uses the color for both icon and label:

| Identity | Anchor |
|---|---|
| Production | `#FF5C7A` |
| Ubuntu/WSL | `#FF6A00` |
| Windows | `#62B0FF` |
| Git | `#DC78FF` |
| Kubernetes | `#50D5FF` |
| Docker | `#2496ED` |
| Azure | `#147DDB` |
| AWS | `#FFB020` |
| GCP | `#F46F61` |
| Unknown cloud | `#FFD166` |
| Terraform | `#A78BFA` |
| Environment | `#2DD4BF` |
| User | `#B8F36B` |

Custom themes retain each anchor's hue and saturation. Automexia changes only
HSL lightness when required to keep the rendered 8-bit color at or above 4.5:1
contrast against the configured context background. Icons and labels remain
present together, so color is never the only identifier.

## Prompt and command lifecycle

The bundled PowerShell, Bash and Zsh integrations reserve three logical rows
for every prompt. The first is a renderer-owned context snapshot, the second
stores the complete path, and the third is the short editable command row:

```text
OS | git-branch | Kubernetes | cloud | Docker | environment | user
complete/current/path
lambda command
```

They publish OSC 7 current-directory data, explicit shell identity, and the OSC
133 `A/B/C/D` lifecycle. All three rows share a monotonic `aid`.
Automexia snapshots every available fact on the context row, updates the active
row in real time, and freezes it when a command starts. The path row always
uses the shell's complete path and never abbreviates it to `.../` or duplicates
Git or infrastructure metadata beside it. Automexia owns the context spacer
and complete path as durable terminal rows; Readline, ZLE, or PSReadLine owns
only the lambda, editable command, and cursor row. A delayed SIGWINCH editor
repaint therefore cannot erase the path or duplicate renderer metadata. Command
completion includes the actual exit code; Automexia measures between `C` and
`D` and draws a right-aligned success/failure badge with duration. Prompt
identity, context, and command results survive scrollback and column
shrink/grow reflow.

Use `Ctrl`+`Alt`+`R` or `Ctrl`+`Alt`+`D` to create an independent clone of the
active session to the right or below. The clone preserves the current
PowerShell/pwsh, Bash, Zsh, or WSL launch identity and directory while keeping
its process, input, scrollback, and DevOps discovery state isolated. Existing
`Ctrl`+`Shift`+`R`/`D` shortcuts continue to open the configured default shell.

## File and folder icons

The reference mockup's file glyphs are produced by the shell integration, not
by rewriting arbitrary terminal output in the renderer. Bash and Zsh use
`eza` when it is installed and provide these interactive shortcuts:

```text
ls    icon-aware listing; ordinary ls arguments remain valid
l     long listing
ll    long, all-files, Git-aware listing
la    long listing including hidden files
lA    long listing including hidden files except . and ..
tree  icon-aware directory tree
```

POSIX icons use eza's Nerd Font vocabulary and render through Automexia's
bundled symbol fallback. They are enabled only for terminal output, so piping
or redirecting a listing remains machine-friendly. `command ls` bypasses the
POSIX function. Set `AUTOMEXIA_PLAIN_LS=1` before the integration is sourced to
disable the icon presentation. Machines without `eza` retain their original
POSIX commands.

Native Windows PowerShell does not require `eza`. Automexia installs a native
PowerShell format view for `DirectoryInfo` and `FileInfo`, so the existing
`ls` alias and `Get-ChildItem` display folder and file-type icons automatically.
The native metadata columns are `Mode`, `Last Modified`, `Size`, and `Name`;
each glyph is kept together with its filename in the final `Name` column, and
directories keep their trailing `\`. Narrow windows shorten only the displayed
name while preserving the leading type glyph. There is no separate icon column.
The command still returns the original filesystem objects: `Where-Object`,
`Sort-Object`, property access, pipelines, scripts, and redirection keep normal
PowerShell behavior. `AUTOMEXIA_PLAIN_LS=1` disables this presentation layer
before the integration is loaded on every supported shell.

Interactive long listings (`ls -l`, `l`, `ll`, `la`, and `lA`) are tables with
bold column headers, owner and group columns, ISO timestamps, and stable color
roles: read permissions are cyan, write permissions gold, execute permissions
green, owners violet, groups blue, sizes orange, and dates muted teal. File and
directory names retain their type colors and icons. DrvFs executable filenames
remain neutral so a Windows mount does not turn every filename green. A
user-defined `EZA_COLORS` value is never replaced.

Install or refresh the integrations on Windows (including WSL) with:

```powershell
.\shell-integration\install-windows.ps1
```

Restart Automexia after installation. The installer is idempotent and the
matching uninstall script removes only marked Automexia blocks.

## Semantic output

Automexia recognizes common Kubernetes, Docker, Terraform, build, test and
structured-log states. Error and warning rows may receive a restrained
translucent background and semantic foreground. Explicit application ANSI
colors win; search and selection remain higher priority. This is presentation
only and never rewrites terminal cells or copied text.

## Interaction map

- Click a tab to select it; drag to reorder it.
- Click `+` to open another terminal with the configured shell.
- Click the chevron to open the searchable command palette.
- Right-click a tab to rename it or choose its accent.
- Use the custom minimize, maximize/restore and close controls on Windows.
- Drag empty space in the first row to move the window; drag any edge/corner to
  resize it.

## Verification

`cargo ready` is the complete contributor gate. Focused UX checks are:

```text
cargo test -p rio-vt semantic
cargo test -p automexia-terminal renderer::island::tests
cargo test -p automexia-terminal renderer::devops_status::tests
cargo test -p automexia-terminal renderer::responsive::tests
powershell -NoProfile -File tools/ci/test_shell_integration.ps1
```

Any chrome geometry, prompt lifecycle, icon vocabulary, title classification,
semantic precedence or resize behavior change must update its regression test.
