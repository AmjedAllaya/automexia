# Terminal experience

Automexia is designed to keep demanding command-driven work clear and
manageable. The terminal should make it easy to understand the current task,
arrange related work, find information, and stay in control without getting
between the user and the tools they chose.

## Experience goals

- Keep the current task, shell, path, and scope easy to understand.
- Keep related work visible across windows, tabs, and panes.
- Make common actions reachable by keyboard and pointer.
- Add useful context without changing application output.
- Stay responsive during heavy output, resizing, and long sessions.
- Keep sensitive or mutating actions explicit and reviewable.

These goals follow the [Product vision](../PRODUCT-VISION.md). This page records
the detailed experience contract: layout, tabs and panes, prompt context,
selection, footer state, semantic output, and image handling. Detailed test
commands live in [Testing and release](../developer/testing-release.md); exact
shortcuts live in [Keyboard and input](../reference/keyboard.md).

Automexia's native desktop UI follows the supplied liquid-hacker mockup while
remaining a real terminal: applications still own their PTY bytes and ANSI
styles, and renderer decoration never changes command output.

## Layout contract

At comfortable sizes, the persistent application chrome is never part of the
terminal grid:

- a 48 px profile/tab row with an application mark, draggable tabs, new-tab
  button, command/profile menu, and native-looking window controls on Windows;
- a tab rail inside every pane that owns multiple local tabs. The rail begins
  at that pane's top edge and never floats above or changes the height of a
  sibling pane. A single-tab pane has no empty shelf or workspace-action buttons;
  search, split, and focus commands remain available through keyboard bindings
  and the command palette.

Chrome has one shared responsive contract for drawing, hit-testing and terminal
grid reservation:

| Density | Trigger (logical viewport) | Window-header reservation |
|---|---|---:|
| comfortable | at least 840 px wide and 480 px high | 56 px |
| compact | below either comfortable threshold | 50 px |
| minimal | below 480 px wide or 280 px high | 44 px |

Each pane with multiple local tabs independently reserves a DPI-stable 36
logical pixels inside that pane. Below 96 logical pixels of pane height, its
rail folds away without closing or merging sessions and returns automatically
when the pane grows. Tab commands remain available through shortcuts and the
command palette, while prompt-level context remains available in the grid.
Keyboard focus mirrors visual ownership: directional pane navigation uses
`Alt`+Arrow on Windows/Linux/BSD and `Cmd`+`Alt`+Arrow on macOS, with the active
outline moving to the chosen geometric neighbour. Pane-local tab navigation
uses `Alt`+`PageUp`/`PageDown` or macOS `Cmd`+`Alt`+`[`/`]`; it wraps only
inside the active pane. Window-level `Ctrl`+`Tab` navigation remains a separate
scope. All navigation actions are also available in the command palette.
As width contracts, controls fold in priority order: the product mark,
command-center control, and then new-tab button hide before the active tab can
collide with the always-reachable minimize, maximize and close controls. Narrow
multi-tab strips use icon-only tabs when a readable title no longer fits.

The terminal grid begins below the live reservation, which is recomputed on
every viewport and DPI change. Typing, output, scrollback and resize/reflow
therefore cannot erase the tabs, nor can a pane's tab rail consume rows from a
single-tab sibling. Windows uses a 6 px renderer-owned resize frame and
supports all edges and corners when native decorations are disabled.

The per-command prompt uses a strict ownership boundary. Automexia writes the
context spacer and the complete, unabridged path once; the shell line editor
redraws only the lambda and editable input row. Resizing reflows the semantic
block by its stable `aid`, retains the entire logical path at physically tiny
sizes, and reveals it again immediately when enough columns return. A user
viewing scrollback is never forced back to the active cursor by resize.
Resize alone never invokes prompt reconstruction. Prompt repair is armed only
when the shell editor explicitly erases terminal-owned cells, and replacement
compacts the archived prompt block without clearing completed command output or
leaving a blank band before the insertion cursor. Optional cursor animation
snaps across window and pane geometry changes instead of travelling from stale
pixel coordinates.

The default window is 1280x760. Tabs remain visible with one session; users may
still explicitly set `navigation.hide-if-single = true` on platforms with
native decorations.

A new top-level tab displays its actual launch profile immediately: PowerShell,
Command Prompt, a WSL distribution, or the configured shell. Semantic shell
metadata may refine that identity after integration starts, but arbitrary OSC
titles emitted while a profile loads never make the tab flicker through setup
commands, paths, or a generic product persona. The close mark shared by window,
top-level-tab, and pane-local-tab chrome is shaped through the text rasterizer
for consistent antialiasing at every DPI and uses an always-available glyph.
The renderer-owned window close control has window scope, not application
scope. Closing a window created with `Ctrl+Shift+N` leaves every sibling OS
window and its independent PTYs running. The native close button, custom
button, and configured `WindowClose` action share that behavior; only the
explicit `Quit` action exits all windows. Last-window confirmation remains
available without interrupting intermediate window closes.


Command palette, search, diagnostic and quit overlays fit to the logical
viewport. The command palette reduces its visible result count with height,
long labels are ellipsized on Unicode boundaries, and editable input keeps its
tail visible. Split containers clamp negative available space and preserve the
combined adjacent-panel size when a divider reaches a compact limit. These
rules apply equally at 1× and HiDPI scale factors and do not upscale UI on very
large displays; the terminal grid simply gains rows and columns.

The tab-row command control is a DPI-independent three-line vector mark rather
than a font-dependent chevron or tile grid. Its quiet blue-black well gains a
cyan focus outline on hover. The palette is intentionally headerless: the
search field is the first visual anchor, followed directly by the results.
Every action uses a purpose-drawn 22 px outline icon with one stroke weight and
optical grid, so no user font can replace it with a fallback glyph. Restrained
semantic accents, outlined key badges, a slim active indicator, and generous
spacing provide hierarchy without colored icon blocks or redundant category
labels. Command behavior and keyboard navigation remain unchanged.

## Keyboard selection

Keyboard selection uses the same renderer-owned highlight and footer state as
mouse selection. `Shift`+Arrow begins at the live terminal insertion cursor,
never at an empty pointer-click anchor, or extends the active end of a real
selection by one cell/row; `Ctrl`+`Shift`+Left/Right moves that end by a
Unicode-aware word boundary. Reversing direction shrinks the same range instead
of creating a second highlight. An Arrow without `Shift`, printable text,
clipboard paste, or IME commit clears the terminal selection before that
non-empty input is forwarded, matching the shell editor's normal ownership.
Motion is scoped to the selected pane, follows scrollback, skips wide-glyph
continuation cells, allocates no per-key text buffer, and does not alter shell
editor state or PTY history. Search, Vi mode, pinned image preview navigation,
and explicit user overrides keep priority.

## Per-pane session footer
Every usable pane ends with a 32 logical-pixel operational footer. It is a
renderer-owned surface with a real grid reservation, so PTY output, the cursor,
images, prompt rows, selections, and the scrollbar stop above it rather than
being covered by it. The rail reaches the exact left, right, and bottom pane
edges instead of floating inside terminal padding. A single pane therefore reads
as one continuous window footer. In a split, adjacent footer segments meet at
the exact divider with no gap or overlap; the selected segment carries a cyan
top and side accent while inactive segments retain a quiet slate keyline,
matching the pane-focus contract used around the terminal surface.

The normal footer is a minimal, right-aligned status line: `UTF-8`, the
session-appropriate `LF` or `CRLF` convention, effective terminal columns and
rows, and the local 24-hour clock. Hairline separators provide hierarchy
without making any value look like a button. Pane and local-tab position appear
on the left only when there is more than one, while selection and an exact
scrollback offset appear only when relevant. It is deliberately read-only:
there are no `LIVE`, `FIND`, icon-only, or invisible action targets. Clicking
anywhere in it focuses the pane without creating a terminal selection or
writing input. Search and scroll-to-bottom remain available through normal
bindings, the scrollbar, and the command palette.

Footer drawing and passive pane routing use the same edge-connected geometry.
The first and last pane absorb horizontal terminal padding so the rail reaches
the viewport sides, but every segment retains the top-chrome origin of Taffy's
pane root; this keeps the footer attached to the real pane bottom instead of
painting it partway through the terminal. Optional status labels progressively
disappear as a pane narrows, with slightly smaller type and padding below 300 logical pixels.
Below 112 logical
pixels of pane height the footer yields all 32 pixels back to the terminal, so
an extreme split always retains a usable PTY row. The footer returns
automatically when the pane grows and requires no user-facing configuration.
The clock reuses the focused-window maintenance tick and therefore advances
while the terminal is idle without sending bytes to the PTY or adding another
polling worker.

## Per-pane operational context

Every semantic prompt owns a session-scoped live snapshot, not a static list
of logos. Automexia refreshes completed snapshots every three seconds and
repaints a worker result within 100 ms. Route-keyed scheduling is de-duplicated,
so typing or resizing cannot create duplicate refresh loops and one pane cannot
publish another pane's context.

In a split layout, every visible pane renders its own prompt-level operational
snapshot, including inactive panes. This keeps each pane's OS, Git branch,
cluster, cloud, Docker, Terraform, environment, and user identity readable at
a glance in multi-cloud workspaces. A four-sided `split_active` accent outline
marks the selected pane without consuming terminal cells or changing PTY
dimensions. Global chrome never substitutes one pane's facts for another.

Each identity is presented as a compact, passive semantic tag. Tags use type
derived from the current prompt-row height and capped below the default 20 pt
path/command text, so metadata remains visibly secondary at normal zoom and
scales down safely in narrow splits. A low-opacity role tint groups each icon
with its label without resembling an action button; tags have no click target.
Whole tags disappear by priority when horizontal space is exhausted and return
automatically when the pane grows. The renderer never truncates a tag into an
orphaned icon or label.

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
palette slots. The same resolver colors live and frozen prompt-history
snapshots, and uses the color for both icon and label:

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
contrast against the effective tinted tag surface. Icons and labels remain
present together, so color is never the only identifier.

## Prompt and command lifecycle

The bundled PowerShell, Bash, and Zsh integrations reserve three logical rows
for every prompt. The first is a renderer-owned context snapshot, the second
stores the complete path, and the third is the short editable command row:

```text
[OS] [git-branch] [Kubernetes] [cloud] [Docker] [environment] [user]
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
repaint therefore cannot erase the path or duplicate renderer metadata.
Command completion includes the actual exit code; Automexia measures between
`C` and `D`. When semantic prompt ownership proves the output limits and next
prompt, the renderer applies the same quiet result surface to listing and
non-listing commands, success and error exits, single- and multiline output,
and managed input wrapped beyond eight rows. Its tinted band, slim left accent,
four- to eight-pixel breathing gutter, end rule, and right-aligned
success/failure badge with duration keep the boundary redundant. The newest
live result lightens once for 540 milliseconds, with no repeated blink or
movement. These renderer-only cues do not insert rows or bytes, so selection,
copy, search, history, prompt identity, context, and command results survive
scrollback and column shrink/grow reflow unchanged.

CMD reserves the same visible three-row structure and publishes OSC 7 plus
OSC 133 `A/B`, but stock `cmd.exe` exposes no pre/post-command hook from which
to generate a monotonic identity, true exit status, or completion timestamp.
Automexia therefore keeps CMD path/context reflow resilient without fabricating
success badges or durations. The richer `A/B/C/D` lifecycle above remains
available whenever the active shell is PowerShell, Bash, or Zsh.

The complete path uses a restrained hierarchy shared by PowerShell, Bash, and
Zsh: separators are muted slate, the root or first component is light blue,
intermediate components cycle through cyan, violet, and Automexia blue, and the
active directory is lime. This produces visible segment boundaries even in a
long WSL mount path without assigning a random color to each name. ANSI style
changes never alter the copied path, OSC 7 directory, Unicode components, or
reflow text. The PowerShell formatter caches an unchanged path, and the POSIX
formatters use shell builtins only, so styling adds no process to prompt input
or history navigation. CMD keeps the complete path in the same blue family as
one dynamic `$P` token because its prompt language cannot style individual path
components without changing the literal directory.

Use `Ctrl`+`R` or `Ctrl`+`D` to create an independent clone of the active
session to the right or below. The clone preserves the current
PowerShell/pwsh, Command Prompt, Bash, Zsh, or WSL launch identity and directory while keeping
its process, input, scrollback, and DevOps discovery state isolated.
Classic `Ctrl`+`Shift`+`R`/`D` shortcuts open a configured default shell in a
right/lower split. `Ctrl`+`Alt`+`R` and `Ctrl`+`Alt`+`D` explicitly send history
search and EOF/logout to the shell. Both clone actions are also
discoverable in the command palette with distinct duplicated-pane icons.

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

Automexia supports both current eza releases and the eza 0.18.x packages still
shipped by common Ubuntu/WSL installations. Because eza before 0.19.2 cannot
override a directory icon by basename, the integration applies a small
TTY-only compatibility filter to eza's own generic folder token. It swaps that
one cell for the corresponding composite folder badge and category color. The
filter never changes a filename, never runs on a pipe or redirect, and never
examines unrelated command output, so scripts and machine-readable listings
remain native eza output. The wrapper forwards the live viewport width while
eza writes through the filter, preserving responsive grid and table layouts.

Folder and file names also receive a name-based visual category. Icons remain
the primary signal and colors are secondary, so the categories remain readable
with color-vision differences and customized themes:

| Category | Common names and files | Default identity |
|---|---|---|
| Sensitive | `secret`, `private`, `credentials`, `vault`, `.env*`, keys and certificates | folder + lock, coral red |
| Configuration | `config`, `settings`, `profiles`, JSON/TOML/YAML/config files | folder + cog, amber |
| Logs and traces | `log`, `logs`, telemetry, `.log`, `.trace` | folder + text, gold |
| Source and engines | `apps`, `src`, `lib*`, `rio-*`, `corcovado`, `sugarloaf` | folder + source file, cyan |
| Documentation | `docs`, `documentation`, `guides`, Markdown and text | folder + information, green |
| Tests and quality | `test*`, `specs`, `fixtures`, `fuzz`, benchmarks | folder + check, violet |
| Build output | `target`, `build`, `dist`, `out`, `coverage`, `changes` | folder + sync, coral |
| Media and assets | `assets`, `public`, `static`, images, icons and fonts | folder + image, pink |
| Packages | `.cargo`, `node_modules`, `vendor`, `packages`, dependencies | multiple folders, purple |
| Repository | `.git`, `.github`, `.gitlab`, Git control files | folder + Git branch, violet |
| Tools | `scripts`, `tools`, `shell-integration`, `ci` | folder + wrench, mint |
| Data | `data`, `db`, `database`, migrations, SQL/SQLite | folder + table, indigo |
| Ephemeral | `cache`, `tmp`, `temp`, sessions and backups | folder + clock, slate |
| Infrastructure | `infra`, Terraform, Kubernetes, Helm, Docker and cloud | folder + network, provider blue |
| Packaging | `packaging`, installers and archives | folder + archive, amber |

Classification uses only the displayed basename and extension; it never opens
or scans file contents and is not a security verdict. A lock icon means “this
name commonly contains sensitive material,” not that the item is encrypted or
permission-protected.

Native Windows PowerShell does not require `eza`. Automexia installs a native
PowerShell format view for `DirectoryInfo` and `FileInfo`, so the existing
`ls` alias and `Get-ChildItem` display folder and file-type icons automatically.
Recognized directories use the same single-cell folder-with-badge vocabulary as
Bash and Zsh; a category symbol never replaces the folder silhouette.
The native metadata columns are `Mode`, `Last Modified`, `Size`, and `Name`;
each glyph is kept together with its filename in the final `Name` column, and
directories keep their trailing `\`. Narrow windows shorten only the displayed
name while preserving the leading type glyph. There is no separate icon column.
The command still returns the original filesystem objects: `Where-Object`,
`Sort-Object`, property access, pipelines, scripts, and redirection keep normal
PowerShell behavior. `AUTOMEXIA_PLAIN_LS=1` disables this presentation layer
before the integration is loaded on every supported shell.

PowerShell 7 applies full RGB category color to the icon and basename because
its formatter understands ANSI display width. Windows PowerShell 5 uses a
short 8-bit category palette in a direct VT window at 96 columns or wider. Its
legacy formatter counts invisible ANSI bytes as table cells, so narrow windows
and redirected output automatically retain the same composite badges without
injected color. This adaptive rule adds color at normal working sizes while
preserving complete names in the unsafe cases.

Command Prompt has a dedicated integration rather than inheriting PowerShell
state. Typing `cmd` or `cmd.exe` with no arguments from an integrated
PowerShell session invokes `%ComSpec%` directly in the same ConPTY; it never
starts a detached console or another terminal application. `exit` returns to
the original PowerShell prompt. Explicit invocations such as `cmd /c build.cmd`
are forwarded unchanged, and `AUTOMEXIA_PLAIN_CMD=1` disables only the
interactive wrapper.

The CMD prompt publishes its real shell/user/executable identity, clears stale
WSL identity, updates OSC 7 and its title from dynamic `$P` after every `cd`,
and renders the same terminal-owned context spacer, complete path, and editable
lambda rows. Identity is embedded into every prompt repaint so a nested shell
cannot leave stale metadata; the PowerShell parent likewise republishes itself
on the first prompt after `exit`. The installed CMD batch is BOM-free ASCII and
receives its Unicode lambda through the launch command, avoiding active-code-page
parser corruption. The installed `ls` and `ll` DOSKEY macros explicitly emit
UTF-8, use the shared Automexia filesystem taxonomy, and keep icons beside names.
Built-in `dir` is deliberately
not replaced, so batch files, redirection, native switches, and existing CMD
automation retain Microsoft semantics. CMD itself has no supported pre/post
command hook equivalent to PSReadLine, Readline, or ZLE; therefore the visual
prompt/context/listing integration is complete, while per-command exit-status
timing remains a PowerShell/Bash/Zsh capability rather than displaying an
incorrect synthetic result.

Interactive long listings (`ls -l`, `l`, `ll`, `la`, and `lA`) are tables with
bold column headers, owner and group columns, ISO timestamps, and stable color
roles: read permissions are cyan, write permissions gold, execute permissions
green, owners violet, groups blue, sizes orange, and dates muted teal. File and
directory names retain their type colors and icons. DrvFs executable filenames
remain neutral so a Windows mount does not turn every filename green. A
user-defined `EZA_COLORS` value is never replaced; the TTY-only folder badge
layer owns only recognized directory icons and their matching basename color.

`cargo dev` and `cargo automexia` expose the repository integration only to
the child shell they launch; signed releases use the package-adjacent resource
tree. Normal launch never writes a profile or starts WSL for provisioning.
Persistent PowerShell/CMD/WSL or Unix profile support for shells opened outside
Automexia requires the explicit `automexia shell-integration install` command.
The Windows installer generates CMD clone metadata for the current account
without storing plaintext credentials, and uninstall removes only marked
Automexia blocks.

## Semantic output

Automexia recognizes common Kubernetes, Docker, Terraform, build, test and
structured-log states. Error and warning rows may receive a restrained
translucent background and semantic foreground. Explicit application ANSI
colors win; search and selection remain higher priority. This is presentation
only and never rewrites terminal cells or copied text.

## Interaction map

- Click a top-row tab to select or drag a window-level workspace.
- Click the top-row `+`, or press `Ctrl`+`T`, to add a window-level tab.
- Press `Ctrl`+`Shift`+`T` to create an independent tab inside the selected
  split/session. That pane then exposes an internal top rail with direct
  selection, an exact per-tab close target, and a local `+` button. Other panes
  remain unchanged; every pane with multiple tabs can expose its own rail.
- Press `Ctrl`+`Shift`+`N` to create a separate OS window.
- On macOS, use `Cmd`+`N`, `Cmd`+`T`, and `Cmd`+`Shift`+`T` for the equivalent
  new-window, window-tab, and selected-session-tab scopes.
- Click the three-line command button to open the searchable command palette.
- Right-click a tab to rename it or choose its accent.
- Use the custom minimize, maximize/restore and close controls on Windows.
- Drag empty space in the first row to move the window; drag any edge/corner to
  resize it.
- In normal terminal input, `Ctrl`+`C` copies a non-empty selection to the
  system clipboard. With no selection it remains the shell/application
  interrupt key; search, Vi mode, and explicit user bindings keep precedence.
- Right-click, including an OS-mapped two-finger touchpad click, copies the
  current selection and clears it when one exists. With no selection it pastes the system
  clipboard through the normal bracketed-paste and control filtering path.
- Middle-click retains primary-selection paste on platforms that provide it.
- Primary/left click remains reserved for focus, selection, links, image
  previews, and pane activation. It never pastes, preventing an ordinary focus
  click from unexpectedly submitting clipboard text.

## Images

### Inline terminal graphics

A program running in the PTY can explicitly send image escape sequences. The
inherited Rio engine parses the sequence, stores bounded image state, and the
Automexia renderer places the resulting texture inside the owning pane. Images
scroll, clip, clear, and switch screen buffers with terminal content.

Supported protocol families are:

- Kitty Graphics, including direct placements and Unicode placeholders;
- iTerm2 OSC 1337 inline images, including pixel/cell/percentage sizing and the
  `doNotMoveCursor=1` extension;
- Sixel.

This is the normal path for image-aware programs. Kitty's official protocol
uses `kitten icat image.png` as its basic example; iTerm2 uses `imgcat`; WezTerm
provides `wezterm imgcat image.png`; and file managers such as Yazi render the
currently selected file through an appropriate terminal graphics protocol.
Automexia preserves its own `TERM_PROGRAM=Automexia` identity and does not
pretend to be Kitty, Rio, iTerm2, or WezTerm. A client can query the advertised
`sixel`, `iterm2`, or `kitty` terminal capabilities or explicitly emit one of
the supported protocols.

Protocol support does not mean that every third-party application recognizes a
new terminal brand automatically. If an application's detection table has not
yet learned Automexia, use its explicit Kitty/iTerm2/Sixel adapter setting
rather than changing `TERM_PROGRAM` or `TERM` to impersonate another terminal.
Multiplexers also need to pass the selected graphics protocol through.

### Local quick look

Automexia also previews a local raster path already visible in terminal output.
This is an emulator-owned overlay; it does not write escape sequences into the
PTY or alter scrollback.

- Hover a supported image filename or path. A 100 ms stability delay avoids
  decoding every cell crossed during fast pointer movement.
- Click the filename/path to pin its preview.
- While pinned, `Down`/`Right` selects the next visible image path and
  `Up`/`Left` selects the previous one. Navigation wraps at both ends.
- Press `Esc` to close a pinned preview. Typing another key or clicking outside
  it also returns input ownership to the shell.
- Keyboard: select a path and press `Ctrl`+`Alt`+`I` on Windows/Linux/BSD or
  `Cmd`+`Alt`+`I` on macOS.
- Command palette: run **Preview Selected Image**. With no selection, the
  command uses the supported path under the pointer.

Bare filenames from ordinary `ls`, quoted names containing spaces, rooted
paths, explicit relative paths, Unicode names, and Automexia's file-listing
glyph prefixes are supported. Relative paths resolve
against validated OSC 7 current-directory metadata, then the immutable launch
directory while the first prompt is still starting. On Windows, a known WSL
session may translate `/mnt/<drive>/...` to the local drive or another absolute
Linux path through `\\wsl.localhost\\<validated-distro>\\...`.

When a full-screen terminal application enables mouse reporting, Automexia
does not steal its mouse events. Hold `Shift` while hovering or clicking to use
quick look through the terminal's standard host-UI override. Hover previews do
not capture arrow keys; only an explicit click or keyboard/palette action pins
the card and enables image navigation.

The card keeps the source aspect ratio, never enlarges a small image, limits a
large GPU upload to 1280x960, flips at pane edges, and disappears instead of
overlapping the terminal when the pane is physically unusable. An unpinned
hover card disappears when the pointer leaves the candidate or window. A
pinned card survives pointer movement and is dismissed by `Esc`, ordinary
typing, an outside click, scrolling, selection, route replacement, or explicit
dismissal. Resizing recomputes its position from the current pane rectangle.

Quick look supports BMP, GIF, ICO, JPEG, PNG/PNM, TIFF, and WebP raster files.
It intentionally does not open SVG, PDF, URLs, remote hosts, directories,
symlinks, named pipes, or device files. Use an explicit trusted application for
those formats. Animated formats receive a static quick-look frame; protocol
clients own richer playback behavior.

### Security and performance contract

Terminal text is attacker-controlled, even when it comes from a local command.
Consequently quick look:

- performs candidate discovery and hit testing without filesystem access;
- waits for a stable 100 ms hover target or explicit click before submitting
  work, and never reads or decodes on the UI, renderer, or PTY threads;
- rejects URL schemes, arbitrary Windows UNC paths, control characters,
  symlinks, and non-regular files;
- limits the source file to 20 MiB and accepts only supported raster magic;
- parses dimensions before full decode, then enforces 4096x4096, 16,777,216
  decoded pixels, and a 96 MiB decoder-allocation ceiling;
- verifies the opened file's length and modification version before and after
  reading so a changing file is not cached as a stable thumbnail;
- decodes and downsizes on one worker with a 16-owner latest-request queue;
  a newer request replaces queued work from the same window;
- gives every window one bounded completion mailbox and a generation token, so
  replacement, dismissal, route changes, and window isolation reject obsolete
  completion without a shared result queue or cross-window eviction;
- retains at most 16 decoded thumbnails and 32 MiB in an access-ordered cache;
  cache hits share the exact pixel allocation with Sugarloaf and use a stable
  texture key/time rather than copying, hashing, decoding, or re-uploading;
- uses the same positive-z paint contract on WGPU, Metal, Vulkan, and CPU:
  card/UI geometry first, image pixels second, then dedicated UI labels;
- measures metadata before fitting the preview title, eliding long filenames
  into the remaining width so the title and dimensions never overlap;
- removes the active overlay, route pixels, and matching GPU texture immediately
  on dismissal; only the separately bounded CPU thumbnail LRU retains reusable
  decoded pixels, and cache replacement uses exact entry/byte accounting.

The iTerm2 protocol decoder separately caps decoded input at 64 MiB, validates
an optional declared `size`, and applies the same 4096x4096 and 96 MiB decoder
limits before constructing terminal graphics. Kitty/Sixel retain their
existing protocol-specific payload, dimension, and shared graphics-quota
checks.
