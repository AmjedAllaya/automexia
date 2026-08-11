# Liquid Hacker UX

Automexia's native desktop UI follows the supplied liquid-hacker mockup while
remaining a real terminal: applications still own their PTY bytes and ANSI
styles, and renderer decoration never changes command output.

## Layout contract

The first 148 logical pixels are persistent application chrome and are never
part of the terminal grid:

- a 66 px profile/tab row with an application mark, draggable tabs, new-tab
  button, command/profile menu, and native-looking window controls on Windows;
- a 47 px operational context surface at y=82 with local OS/WSL, Git,
  Kubernetes, cloud, Docker, Terraform, environment and production facts;
- a separate right surface for the real shell name and local clock on windows
  at least 760 px wide. Narrow windows give the full row to context instead.

The terminal grid begins below this reservation. Typing, output, scrollback and
resize/reflow therefore cannot erase the tabs or context bar. Windows uses a
6 px renderer-owned resize frame and supports all edges and corners when native
decorations are disabled.

The default window is 1280x760. Tabs remain visible with one session; users may
still explicitly set `navigation.hide-if-single = true` on platforms with
native decorations.

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
Git or infrastructure metadata beside it. Only the short lambda and command
buffer belong to Readline, ZLE, or PSReadLine; this prevents a line-editor
redisplay after resize from erasing or duplicating the stored path. Command
completion includes the actual exit code; Automexia measures between `C` and
`D` and draws a right-aligned success/failure badge with duration. Prompt
identity, context, and command results survive scrollback and column
shrink/grow reflow.

## File and folder icons

The reference mockup's file glyphs are produced by `eza`, not by rewriting
arbitrary terminal output in the renderer. When `eza` is installed, the Bash
and Zsh integrations provide these interactive shortcuts:

```text
ls    icon-aware listing; ordinary ls arguments remain valid
l     long listing
ll    long, all-files, Git-aware listing
la    long listing including hidden files
lA    long listing including hidden files except . and ..
tree  icon-aware directory tree
```

Icons use eza's Nerd Font vocabulary and render through Automexia's bundled
symbol fallback. They are enabled only for terminal output, so piping or
redirecting a listing remains machine-friendly. `command ls` bypasses the
function. Set `AUTOMEXIA_PLAIN_LS=1` before the integration is sourced to
disable all listing functions. Machines without `eza` retain their original
commands.

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
powershell -NoProfile -File tools/ci/test_shell_integration.ps1
```

Any chrome geometry, prompt lifecycle, icon vocabulary, title classification,
semantic precedence or resize behavior change must update its regression test.
