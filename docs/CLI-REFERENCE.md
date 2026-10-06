# CLI and contributor command reference

Automexia has two public command layers: the installed application and
repository-owned contributor commands. This page intentionally omits unreleased
advanced and commercial command surfaces.

## Application command

```text
automexia [OPTIONS] [COMMAND]
```

| Option | Meaning |
|---|---|
| `-e, --command <PROGRAM> [ARGS...]` | Launch a program instead of the configured shell. It is final because remaining values are program arguments. |
| `-w, --working-dir <PATH>` | Start in an existing directory; invalid paths fall back safely with a warning. |
| `--write-config [PATH]` | Create starter configuration without overwriting an existing file. |
| `--enable-log-file` | Enable a launch log under the Automexia configuration root; review before sharing. |
| `--title-placeholder <TEXT>` | Set the initial title placeholder. |
| `--app-id <ID>` | Override Wayland app ID or X11 class on Linux/BSD. |
| `-h, --help` | Print help. |
| `-V, --version` | Print version. |

Help is generated from the same command and option definitions used by the
parser. It wraps to the terminal width when that width is available, accepts a
bounded `COLUMNS` hint for redirected shells, and defaults to 100 columns when
the width is unknown. Redirected help is plain text without artwork or ANSI
decoration. `automexia about` prints version information with the full ASCII
artwork when an interactive terminal has room, or a compact wordmark otherwise.
`automexia logo` prints the full plain artwork when redirected and uses the
compact wordmark in a small interactive terminal. The same commands are
available through the session-local `amx` helper. On Windows, builds that
include the console `amx.exe` forward arguments and standard streams to the
application; GUI-only bundles use the synchronous PowerShell compatibility
wrapper for captured or redirected output.

Everything after the explicit program is passed as an exact argument. Do not
use this option as a replacement for shell pipelines, redirects, aliases,
functions, or expansion; enter those in the real shell.

## Open a directory

`automexia open [directory]` opens an existing directory in the desktop file
manager; the default is `.`. `--preview` resolves the destination without opening
it. Files are rejected. See [directory opening and platform limits](user-guide/open-directory.md).

## Edit a file

`automexia edit <file> [--line N] [--column N]` requests an existing file in
Visual Studio Code by default. `--preview` prints the encoded destination without
opening an editor. `--editor vscode-insiders` selects Insiders for one invocation;
strict user-root `amx.toml` preferences can select or disable editing. See
[editor behavior and limits](user-guide/edit-file.md). The same command is
available as `amx edit` in integrated shells.

## Repository navigation

`automexia repo [root|issues] [--remote NAME] [--preview]` reads a local configured
Git remote and opens its GitHub.com/GitLab.com page. Origin is the default;
`--preview` is offline and does not open a browser. See [repository navigation](user-guide/open-repository.md)
for remote syntax, limits and WSL behavior. Integrated shells use `amx repo`.

## Google search

`automexia search <source> <terms>` supports `google`, `github` (repositories),
`youtube` and `ddg`. `automexia docs <tool> <terms>` searches Google restricted
to official documentation for `kubernetes`, `docker`, `rust`, `python`, `git`
or `terraform`. Both accept `--print-url` before query terms for an offline
preview. Integrated shells expose the same commands as `amx`. Unknown names
fail without fallback. See [search examples](user-guide/google-search.md).

`automexia google <terms>` opens one encoded Google search in the default
browser. Integrated sessions expose `amx google <terms>` without installing a
global alias. `--print-url` before the query previews it offline; `--` allows
leading search operators. See [usage, limits and privacy](user-guide/google-search.md).

## Local actions, aliases, packs, and workspaces

These source-owned local command families are part of the free product. Their
read paths do not start a provider, connection, terminal session, task, or
action. Mutations are preview-first and require the exact `--apply` and revision
arguments shown by current `--help` output.

| Command family | Public behavior |
|---|---|
| `automexia actions <SUBCOMMAND> [OPTIONS]` | List/show/diagnose reviewed Quick Actions; preview or explicitly put, import, export, remove, recover, import simple native aliases, and manage explicit trusted workspace task bridges. |
| `automexia aliases <SUBCOMMAND> [OPTIONS]` | List, preview, test, explicitly publish, disable, regenerate, roll back, diagnose, reload, or remove Automexia-owned shell alias projections. |
| `automexia packs <SUBCOMMAND> [OPTIONS]` | List/show immutable built-in packs, evaluate caller-supplied tool observations, and preview or explicitly materialize one reviewed action; aliases remain disabled. |
| `automexia workspaces <SUBCOMMAND> [OPTIONS]` | List/show declarative workspace metadata and preview explicit put/remove, restore-plan, and recipe-plan operations without implicitly starting a PTY or process. |

Use `automexia <family> --help` and `automexia <family> <subcommand> --help` as
the exact option authority for the installed version. Stable `--json` output is
available where the command help advertises it. Credential values, provider
login, native configuration mutation, implicit Enter, and hidden shell
evaluation are outside these command contracts.

## Shell integration maintenance

| Command | Behavior |
|---|---|
| `automexia shell-integration doctor` | Read-only health and resource-root report. |
| `automexia shell-integration install [--force] [--quiet]` | Explicitly install or repair only Automexia-owned persistent integration. |
| `automexia shell-integration uninstall [--quiet]` | Remove only Automexia-owned persistent integration. |

Normal application launch uses session-local integration and does not require
these mutation commands. On Windows, effective PowerShell execution policy is
honored; Automexia does not bypass it.

## Keyboard compatibility inspection

Where supported by the current build, inspection exits before GUI launch and
does not start another terminal:

```text
automexia --list-actions [--aliases] [--unavailable] [--json]
automexia --list-keybinds [--profile automexia|ghostty|ghostty-1.3]
  [--platform linux-bsd|macos|windows] [--origin ORIGIN]
  [--effective] [--shadowing] [--explain TRIGGER_OR_ACTION] [--json]
```

Migration is dry-run first, bounded, explicit to apply, validates the complete
result, creates a backup, and publishes atomically:

```text
automexia migrate ghostty [--input PATH] [--dry-run] [--json]
automexia migrate ghostty [--input PATH] [--output PATH] --apply --confirm
```

See [Keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md).

## Daily contributor commands

| Command | Result |
|---|---|
| `cargo automexia [-- APP_ARGS...]` | Incremental build, smoke, and launch after the full gate has passed. |
| `cargo ready` | Complete contributor gate without launching. |
| `cargo ci` | Repository CI alias. |
| `cargo qa` | Deeper assurance profile. |
| `cargo storage` | Report repository build-storage use. |
| `cargo purge` | Remove verified workspace build artifacts after Automexia closes. |
| `cargo xtask doctor` | Report contributor prerequisites, storage, and platform placement. |
| `cargo xtask check` | Locked metadata, formatting, contracts, lint, tests, build, and smoke. |
| `cargo xtask verify architecture` | Enforce public architecture and capability boundaries. |
| `cargo xtask verify identity` | Enforce public product identity. |
| `cargo xtask verify provenance` | Verify license, notice, attribution, and publication policy. |
| `cargo xtask test conformance` | Run terminal/Unicode conformance. |
| `cargo xtask test resize-stress [--native-gui]` | Run deterministic resize/reflow stress and optional native GUI evidence. |
| `cargo xtask test image-rendering [--native-gui]` | Run image decoder/preview/rendering checks and optional native evidence. |
| `cargo xtask package --check` | Validate package metadata and prerequisites without publishing. |

### Complete source-owned xtask surface

The following commands are also source-owned. Their current `--help` output is
the exact option authority; release, packaging, installation, generation, and
cleanup commands are mutating and require deliberate review.

| Command | Purpose |
|---|---|
| `cargo xtask ready` | Run the complete contributor-ready gate without launching the application. |
| `cargo xtask ci` | Run the repository CI profile. |
| `cargo xtask run [-- APP_ARGS...]` | Build and deliberately launch the application with optional arguments. |
| `cargo xtask dev [-- APP_ARGS...]` | Run the development build/launch workflow. |
| `cargo xtask qa --full [--bundle]` | Run the full assurance profile and optionally validate the bundle. |
| `cargo xtask storage` | Report bounded repository build-storage use. |
| `cargo xtask cache status [--warn-gib N]` | Inventory generated development caches and warn at the selected distinct-usage budget. |
| `cargo xtask cache gc [--scope automatic\|tools\|worktrees\|all] [--grace-hours N] [--apply]` | Preview or apply exact, bounded cache cleanup; cleanup is a dry run without `--apply`. |
| `cargo xtask completion COMMAND [OPTIONS]` | Generate shell completion for the selected contributor command. |
| `cargo xtask verify all` | Run all repository verification owners. |
| `cargo xtask verify keybindings` | Verify the keyboard contract and generated artifacts. |
| `cargo xtask test keybindings` | Run keyboard-contract tests. |
| `cargo xtask test session-clone [--native-windows|--native-wsl]` | Exercise session cloning in the selected native environment. |
| `cargo xtask test image-decoder-fuzz [--seconds N]` | Run the bounded image-decoder fuzz campaign. |
| `cargo xtask generate keybindings <--version 1.3.1|--check>` | Generate or check the pinned keybinding data. |
| `cargo xtask package --target TARGET` | Build the explicitly selected package target. |
| `cargo xtask release --version VERSION` | Run the versioned release workflow; it does not grant publication authority by itself. |
| `cargo xtask visual-diff --expected PATH --actual PATH --config PATH --diff PATH --report PATH` | Compare controlled raster artifacts and write the requested diff/report artifacts. |
| `cargo xtask assurance check-policy` | Validate the local assurance policy. |
| `cargo xtask assurance deep-source` | Run the deep source-assurance profile. |
| `cargo xtask assurance audit-history-secrets` | Audit retained repository history through the bounded secret-scanner workflow. |
| `cargo xtask assurance initialize-vet` | Initialize the dependency-vetting workflow explicitly. |
| `cargo xtask assurance install-hook` | Install a non-blocking local pre-push placeholder; the assurance command remains commented. |
| `cargo xtask assurance install-tools` | Install declared assurance tooling explicitly. |
| `cargo xtask assurance pre-push` | Run the repository-owned local assurance profile explicitly; it is not invoked by the dormant hook. |
| `cargo xtask assurance release-local` | Run the local release-assurance profile without claiming hosted publication. |

Commands that launch, install, generate, package, release, or clean are
mutating by their nature. Review their current help before use.

## Public environment variables

| Variable | Scope |
|---|---|
| `AUTOMEXIA_CONFIG_HOME` | Override the complete writable product root. |
| `AUTOMEXIA_LOG_LEVEL` | Override configured log level. |
| `AUTOMEXIA_SHELL_INTEGRATION` | Marker used for child-shell integration. |
| `CARGO_TARGET_DIR` | Relocate Cargo artifacts; keep it native to the active OS. |
| `AUTOMEXIA_KEEP_VERIFY_TARGET` | Retain an isolated verification target for diagnosis. |
| `AUTOMEXIA_VERIFY_MIN_FREE_GIB` | Override the exhaustive-gate free-space minimum. |
| `AUTOMEXIA_BUILD_MIN_FREE_GIB` | Override the app-build free-space minimum. |
| `AUTOMEXIA_TARGET_WARN_GIB` | Override the target-size warning. |

Do not lower storage guards in routine development or CI. See
[Configuration](CONFIGURATION.md), [Testing](TESTING.md), and
[WSL development](WSL-DEVELOPMENT.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Focused Xtask Commands

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Native Alias Imports And Workspace Task Bridges Cp33

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Reviewed Devops Packs Cp32

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## CP3.2-CP3.3 lifecycle boundary

CP3.2 pack commands list, inspect, preview, verify, enable, disable, and remove
reviewed static candidates without implicit activation. CP3.3 import and
workspace commands produce bounded candidates only; read paths never start a
provider, network connection, task, terminal session, or action.

## Explicit local tools

`automexia find file <literal>` and `automexia find text <literal>` search the current
directory subtree using installed ripgrep. `automexia explain <command> [subcommand]`
displays offline tealdeer examples without executing them. Each accepts
`--preview` to show exact arguments without launching its client. Missing tools
are never installed automatically. Integrated shells expose these through `amx`.
See [limits, privacy, cancellation and WSL
requirements](user-guide/local-tools.md).

## SSH integration planning

### Interactive SSH wrapper

`automexia +ssh --shell bash -- host-alias` starts system OpenSSH with bundled
remote shell integration. On Windows, invoke `amx +ssh` through the installed
console launcher. `automexia ssh` is an alias for `automexia +ssh`.
Select the account's
actual shell with `--shell bash|zsh|fish|powershell|pwsh`; `powershell` selects
Windows PowerShell and `pwsh` selects PowerShell 7. Native SSH arguments follow
`--` unchanged. Unknown shells use native SSH; no account-shell syntax is guessed.

`--integration off` preserves native behavior. `--integration required` rejects
unsupported local invocation/configuration conditions; remote capabilities still
depend on the installed shell and profile. Redirected streams, explicit remote
commands, control operations and incompatible configuration use native SSH in
automatic mode. SSH's exit status is preserved.
If effective configuration cannot be read or classified, automatic mode stops
before connecting; choose `--integration off` for explicit native passthrough.
Classified interactive fallback isolates remote metadata without claiming an
integrated shell. Native noninteractive operations and integration off do not
inject terminal metadata.

`--force-tty` explicitly requests a remote PTY through redirected streams. It
cannot restore local terminal behavior lost by a process bridge. Use the Linux
Automexia executable inside WSL for native interruption and resize behavior.

Enhanced startup evaluates effective OpenSSH configuration only after this
explicit command. Authentication, host-key checks, agents and proxies remain
OpenSSH-owned. Session files are temporary: no remote profile, credential store
or service is installed or changed. Remote context is display-only. Cloning an
active remote pane requires opening a terminal and connecting explicitly.

`--helper-upload PATH` adds automatic remote Git, Kubernetes, Docker, Terraform
and cloud context discovery using a trusted helper built for the remote OS and
architecture. It requires a known shell, an enhanced interactive session, and
integration `auto` or `required`; incompatible conditions are errors, not silent
native fallback. Supported upload targets are POSIX Bash/Zsh and Windows
PowerShell 5/7. Fish and PowerShell on Unix can use ordinary integration without
upload; the helper requires a verified nonblocking shell request channel.
POSIX upload staging also requires Bash 3.2 or newer, including Zsh sessions.
The helper is copied to a private temporary directory for the session, without
remote installation or profile edits. Upload requires two SSH invocations and
may authenticate twice; staging has a 120-second deadline and a 64 MiB limit.
Interrupted setup or forced termination can leave temporary files. See
[helper behavior and cleanup](SSH-INTEGRATION-LIBRARY.md#temporary-discovery-helper).

### Read-only planning

`automexia ssh-integration status` reports the existing managed-SSH gate, the
non-executing planning boundary and the separate explicit wrapper's availability.
It performs no SSH or config probe.
`automexia ssh-integration inspect -- host-alias` classifies a proposed invocation
and prints a redacted JSON preview, not an executable command or authorization.

Inspect options are `--mode auto|off|required`, `--shell unknown|bash|zsh|fish|powershell`,
`--startup login|interactive`, `--tty`, `--posix-account-shell`,
`--permit-session-files`, and `--assume-compatible-config`. These describe preview
assumptions only; they do not observe a remote server, grant capabilities, or
activate execution. Native SSH arguments follow the `--` separator unchanged.

The read-only planner exposes only its non-login Bash candidate. Unknown
shells/configuration, login startup, remote commands, tunnels, control operations,
non-TTY input/output, and unsupported options retain non-enhanced classifications.
There is no `--execute`, `--apply`, or security-gate override on this command.

See [SSH integration library](SSH-INTEGRATION-LIBRARY.md) for scope, contracts,
current limitations and the distinction between candidate and live features.

`cargo xtask test ssh-integration` runs the Unix actual-generator shell scope.
It is a development test command, not a connection or activation command.
