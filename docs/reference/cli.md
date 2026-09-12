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

Everything after the explicit program is passed as an exact argument. Do not
use this option as a replacement for shell pipelines, redirects, aliases,
functions, or expansion; enter those in the real shell.

## Open a directory

`automexia open [directory]` opens an existing directory in the desktop file
manager; the default is `.`. `--preview` resolves the destination without opening
it. Files are rejected. See [directory opening and platform limits](../user-guide/open-directory.md).

## Edit a file

`automexia edit <file> [--line N] [--column N]` requests an existing file in
Visual Studio Code by default. `--preview` prints the encoded destination without
opening an editor. `--editor vscode-insiders` selects Insiders for one invocation;
strict user-root `amx.toml` preferences can select or disable editing. See
[editor behavior and limits](../user-guide/edit-file.md). The same command is
available as `amx edit` in integrated shells.

## Google search

`automexia search <source> <terms>` supports `google`, `github` (repositories),
`youtube` and `ddg`. `automexia docs <tool> <terms>` searches Google restricted
to official documentation for `kubernetes`, `docker`, `rust`, `python`, `git`
or `terraform`. Both accept `--print-url` before query terms for an offline
preview. Integrated shells expose the same commands as `amx`. Unknown names
fail without fallback. See [search examples](../user-guide/google-search.md).

`automexia google <terms>` opens one encoded Google search in the default
browser. Integrated sessions expose `amx google <terms>` without installing a
global alias. `--print-url` before the query previews it offline; `--` allows
leading search operators. See [usage, limits and privacy](../user-guide/google-search.md).

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

## Ghostty compatibility inspection and migration

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

See [Keyboard compatibility](../GHOSTTY-KEYBOARD-COMPATIBILITY.md).

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
[Configuration](../CONFIGURATION.md), [Testing](../TESTING.md), and
[WSL development](../WSL-DEVELOPMENT.md).

## Local tools

`automexia find file <literal>`, `automexia find text <literal>` and
`automexia explain <command> [subcommand]` are also available through `amx` in
integrated shells. `--preview` performs no client execution. See [local tools](../user-guide/local-tools.md)
for required clients, bounded search semantics, offline cache preparation and
guest cancellation. These commands do not enable generic Quick Action execution.
