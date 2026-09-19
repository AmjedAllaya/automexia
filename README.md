<p align="center">
  <img src="assets/brand/automexia-terminal-source-512.png" alt="Automexia Terminal logo" width="176">
</p>

<h1 align="center">Automexia Terminal</h1>

<p align="center">
  <strong>A flexible terminal for focused command-line work.</strong>
</p>

<p align="center">
  Keyboard-first · Direct shells · Structured workspaces · Local control
</p>

<p align="center">
  <a href="docs/INSTALLATION.md"><strong>Install</strong></a> ·
  <a href="docs/GETTING-STARTED.md">Get started</a> ·
  <a href="docs/user-guide/index.md">User guide</a> ·
  <a href="docs/FEATURES.md">Features</a> ·
  <a href="docs/CONFIGURATION.md">Customize</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

<p align="center">
  <strong>Linux v0.4.0 Early Access</strong> &nbsp;·&nbsp; x64 + Arm64 &nbsp;·&nbsp; DEB, RPM and portable archives &nbsp;·&nbsp; MIT
</p>

---

Automexia is a keyboard-first terminal that keeps the shell and the tools you
already use in control while giving command-driven work a more organized place
to live. It combines native shell sessions, windows, tabs, split panes,
pane-local tabs, search, command navigation, images, configuration, keyboard
discovery, and explicit local tooling in one focused application.

The released Linux terminal is deliberately useful without an account, hosted service,
provider login, paid dependency, or model. Automexia does not replace your
shell, SSH client, credential store, cloud CLI, editor, or specialist tools; it
provides the terminal workspace around them.

> **Direct commands. Visible state. Optional structure. Local control.**
>
> Automexia favors explicit actions, bounded behavior, safe recovery, and honest
> release status over hidden automation.

## Why Automexia

| Organize | Find | Customize | Stay in control |
|---|---|---|---|
| Keep related work together with windows, global tabs, split panes, and pane-local tabs. | Search the selected pane or visible workspace, navigate marked commands, and discover actions from the keyboard. | Configure themes, fonts, cursor behavior, shells, navigation, shortcuts, and platform overrides in readable TOML. | The active shell owns command editing and execution; Automexia never silently adds Enter or turns displayed text into a command. |
| Start clean sessions or clone the documented launch context into an independent PTY. | Selection, scrollback, hyperlinks, command boundaries, and scoped search remain tied to the exact active route. | Invalid configuration keeps the last-known-good state instead of replacing a working setup. | External tools keep ownership of credentials, networking, authentication, and provider state. |

## Current release status

Automexia v0.4.0 is publicly available as **Linux Early Access** for x64 and
Arm64. The official binary archive contains DEB, RPM, and portable packages.
Verify the signature and package checksum before installation.

**Official release:**
[Automexia v0.4.0 Linux Early Access](https://github.com/AmjedAllaya/automexia-releases/releases/tag/v0.4.0)

Windows and macOS have source, native-platform, packaging, and release ownership
in the project, but **stable Windows and macOS installers are not currently
published**. BSD and other Unix systems remain best-effort and have no required
v0.4 release artifact.

| Platform | Terminal/session path | Public release status |
|---|---|---|
| Linux | Unix PTY; X11 and Wayland build paths | **v0.4.0 Early Access published** for x64 and Arm64 |
| Windows | ConPTY; PowerShell, CMD, and WSL integration | Stable installer not published |
| macOS | Unix PTY; native macOS/Metal/WGPU release path | Stable installer not published |
| BSD / other Unix | Shared Unix PTY and selected X11-compatible paths | Best-effort; no v0.4 artifact |

See [Installation](docs/INSTALLATION.md), [Platform support](docs/PLATFORMS.md),
and [Release trust](docs/RELEASE-TRUST.md) for the exact evidence and package
boundaries.

## What Automexia provides

### Workspaces, windows, tabs, and panes

Automexia has explicit ownership for independent terminal sessions and the UI
routes that display them.

- open independent application windows;
- use top-level tabs within a window;
- split work into panes;
- keep pane-local tabs inside a pane;
- move focus and resize dividers without changing session ownership;
- start a fresh shell when you want a clean session;
- clone the documented launch context into a new, independent session;
- close a view without silently reassigning another view to its PTY.

The terminal/session model is built around stable route identities rather than
inferred screen coordinates. Search, selection, input, resize, exit reporting,
and cleanup operate on the intended route and session.

### Real shells and native PTYs

Automexia keeps the native shell as the command-line authority.

Supported public workflows include PowerShell, Command Prompt, WSL, Bash, Zsh,
Fish, and other shells that can run through the platform terminal path.
Enhanced integration is shell-specific; ordinary shells still work without
claiming identical integration behavior.

On Unix, sessions use Unix PTYs. On Windows, sessions use ConPTY. Each session
owns its process lifecycle, ordered input, resize state, terminal state, exit
outcome, and descendant cleanup.

Automexia does **not** reconstruct shell commands from terminal cells. It does
not replace shell history, quoting, aliases, functions, completion, pipelines,
redirection, or expansion.

Run normal tools normally:

```text
git status
cargo test
ssh alice@example.invalid
kubectl get pods
python script.py
```

### Terminal behavior

The public terminal engine includes:

- VT/CSI/OSC/DCS parsing;
- Unicode text and grapheme-aware behavior;
- scrollback and resize reflow;
- keyboard and pointer selection;
- copy and route-scoped paste;
- mouse reporting;
- hyperlinks;
- cursor and terminal-title behavior;
- command-boundary navigation;
- bounded terminal control-string handling; and
- explicit exit and cleanup reporting.

Terminal output is treated as untrusted input. Displayed text, search results,
links, previews, control sequences, and optional-component output cannot become
PTY input without an explicit user action.

### Search and command discovery

Automexia keeps navigation close to the keyboard:

- search within the selected pane;
- search across visible panes where supported by the current build;
- keep search scope tied to the active route;
- navigate marked command boundaries;
- use the command palette to discover application actions;
- inspect keybindings and compatibility profiles from the CLI; and
- keep modal input out of the underlying PTY while a palette, search surface,
  or confirmation is active.

[Keyboard](docs/KEYBOARD.md) is the exact source-level shortcut authority.
Published package shortcuts can differ from newer source corrections, so use the
documentation and `--list-keybinds` support that matches the version you run.

### Images and rendering

Automexia supports terminal and local-image workflows including:

- Sixel images;
- Kitty image protocol content;
- iTerm2 image protocol content; and
- bounded local raster preview.

The renderer preserves terminal-cell geometry, clipping, cursor position,
selection, scroll offsets, modal composition, and route identity while using
immutable generation-labelled snapshots. Rendering components are built on the
project's Sugarloaf/Rio graphics stack with native platform and WGPU-backed paths
where supported.

See [Image previews](docs/IMAGE-PREVIEWS.md) and
[Architecture](docs/ARCHITECTURE.md).

### Appearance, configuration, and migration

Automexia uses TOML configuration with transactional reload and
last-known-good behavior.

Public configuration covers terminal and application behavior such as:

- themes and terminal colors;
- fonts and font sizing;
- cursor choices;
- opacity and line spacing;
- shell selection;
- windows and navigation behavior;
- keyboard bindings and compatibility profiles; and
- platform-specific overrides.

Starter configuration generation refuses to overwrite an existing file.
Migration paths are explicit, bounded, validate the complete result, and keep a
backup before applying changes.

See [Configuration](docs/CONFIGURATION.md), [Migration](docs/MIGRATION.md), and
[Ghostty keyboard compatibility](docs/GHOSTTY-KEYBOARD-COMPATIBILITY.md).

### Accessibility

Automexia owns keyboard-only operation, focus behavior, high-contrast
support, reduced-motion behavior, and renderer-neutral accessibility semantics.
Native assistive-technology evidence is treated separately from portable source
tests.

See [Accessibility](docs/ACCESSIBILITY.md).

### OpenSSH interoperability

Use the operating system's normal `ssh`, `scp`, and `sftp` clients inside an
Automexia pane.

OpenSSH and the operating system keep ownership of configuration, credentials,
agents, host keys, authentication, proxy behavior, and networking. Automexia
owns the local terminal session around those tools.

Automexia also contains explicit, bounded local OpenSSH inventory
behavior. Inventory reads selected local metadata only; it does not silently log
in, scan a network, or take ownership of SSH credentials.

See [Remote connections](docs/guide/remote-connections.md) and
[SSH inventory](docs/SSH-INVENTORY.md).

### Local command productivity

Current source includes local command-productivity surfaces. Their read
paths do not start a provider, network connection, terminal session, task, or
action, and mutating operations are preview-first and explicit.

| Command family | Purpose |
|---|---|
| `automexia actions ...` | Inspect and manage reviewed local actions, imports, exports, trusted native alias imports, and explicit workspace task bridges. |
| `automexia aliases ...` | Inspect, preview, publish, disable, regenerate, diagnose, or remove Automexia-owned shell alias projections. |
| `automexia packs ...` | Inspect reviewed built-in packs and explicitly materialize selected local action data; aliases remain disabled unless deliberately managed. |
| `automexia workspaces ...` | Inspect declarative workspace metadata and preview explicit changes without implicitly launching a process or PTY. |

The exact `--help` output for the installed version is authoritative. Source
modules, fixtures, tests, or internal package names do not by themselves prove
that an unlisted capability is publicly released.

See [CLI reference](docs/CLI-REFERENCE.md),
[Command productivity](docs/COMMAND-PRODUCTIVITY.md), and
[Features](docs/FEATURES.md).

## Install Automexia

### Linux Early Access

For a published Linux build, download the exact versioned package for your
architecture from the official v0.4.0 release together with its checksum and
signature files. Follow the verification steps in
[Installation](docs/INSTALLATION.md) before installing.

Do not treat GitHub-generated source archives or an unverified third-party
package as an Automexia application release.

### Build from source for authorized contributors

If you have authorized access to the private source repository, use an approved
checkout. Install the platform prerequisites described in
[Installation](docs/INSTALLATION.md), then install the repository tools and run
the project doctor before the first expensive build.

A contributor-oriented first launch is:

```text
cargo dev
```

After the full gate has passed, the faster incremental application workflow is:

```text
cargo automexia
```

The project currently requires the Rust toolchain selected by
`rust-toolchain.toml`; the workspace minimum Rust version is declared separately
in `Cargo.toml`.

## Application command line

The installed application entry point is:

```text
automexia [OPTIONS] [COMMAND]
```

Important public options include:

| Option | Behavior |
|---|---|
| `-e, --command <PROGRAM> [ARGS...]` | Launch an explicit program. This option is final because all remaining values are exact program arguments. |
| `-w, --working-dir <PATH>` | Start in an existing directory; invalid paths fall back safely with a warning. |
| `--write-config [PATH]` | Create starter configuration without overwriting an existing file. |
| `--enable-log-file` | Enable a launch log under the Automexia configuration root. Review it before sharing. |
| `--title-placeholder <TEXT>` | Set the initial title placeholder. |
| `--app-id <ID>` | Override the Wayland app ID or X11 class on Linux/BSD. |
| `-h, --help` | Print help. |
| `-V, --version` | Print version. |

Use the real shell for pipelines, redirects, aliases, functions, and expansion.
For example, run the shell and enter `git log | less` there rather than trying
to make `--command` perform shell evaluation.

Automexia also exposes explicit shell-integration maintenance, keybinding
inspection, compatibility migration, local action, alias, pack, and workspace
command families where supported by the current build. See the complete
[CLI reference](docs/CLI-REFERENCE.md).

## Architecture

Automexia's architecture keeps authority flowing in one direction:

```text
application
  -> terminal/session coordination
  -> renderer-neutral snapshots and input routing
  -> VT state and PTY/process adapters
  -> platform adapters

optional installed component
  -> versioned capability contract
  -> application-owned broker
```

The VT parser, PTY/session owner, renderer, configuration store, application,
and optional capability broker each have explicit responsibilities. Optional
code cannot become a second owner for terminal cells, process lifetime, focus,
routes, or persisted settings.

A normal output path is:

```text
PTY bytes
  -> bounded decoder and VT parser
  -> terminal-state mutation
  -> immutable renderer snapshot
  -> publish snapshot
  -> request a frame
```

Input travels back toward the exact session owner:

```text
keyboard / pointer / IME
  -> focused application route
  -> modal and binding policy
  -> exact session input queue
  -> PTY / ConPTY
```

This separation is central to Automexia's reliability and security model.

### Workspace structure

The Rust workspace is split into the application, Automexia-owned product
contracts, terminal/rendering engine crates, optional capability foundations,
and repository tooling.

| Area | Main workspace members | Responsibility |
|---|---|---|
| Desktop application | `apps/automexia-terminal` | Product entry point, windows/routes, orchestration, UI, CLI, platform integration |
| Input and UX contracts | `automexia-keybindings`, `automexia-command-productivity`, `automexia-ui-model`, `automexia-image`, `automexia-connectivity` | Keybindings, local productivity models, renderer-neutral UI data, image handling, connection metadata boundaries |
| Optional capability contracts | `automexia-extension-api`, `automexia-extension-runtime`, `automexia-ecosystem`, `automexia-ecosystem-runtime`, `automexia-devops` | Versioned capability, lifecycle, ecosystem, and bounded optional-data contracts |
| Reviewed source adapters | `extensions/devops-aws`, `extensions/devops-azure`, `extensions/devops-gcp`, `extensions/devops-kubernetes`, `extensions/devops-openshift`, `extensions/devops-teleport`, `extensions/devops-ssh` | Source-level adapters behind the project's public capability and publication boundaries; package names are not release announcements |
| Terminal and PTY engine | `teletypewriter`, `rio-vt`, `rio-backend`, `rio-window`, `rio-notifier`, `rio-grapheme-width` | PTY/process ownership, VT state, platform event/window integration, terminal behavior |
| Rendering and graphics | `sugarloaf`, `rio-graphics`, `rio-fonts`, `corcovado` | Text shaping, graphics, fonts, rendering primitives, event-loop support |
| Embedding / libraries | `librio`, `librio-wasm` | Native and Wasm-facing library surfaces retained by the workspace |
| Tooling | `tools/xtask`, `tools/renderer-benchmarks` | Contributor workflows, validation, packaging/release orchestration, controlled renderer benchmarks |

The current workspace version is **0.4.0**, uses Rust 2021, and is licensed
under MIT. `Cargo.toml` is the exact package and dependency authority.

## Security and trust model

Automexia treats terminal output, control sequences, shell metadata,
configuration, paths, files, clipboard data, imported records, optional
component data, and generated text as untrusted input.

Key public guarantees include:

- one authoritative owner for each PTY, process tree, terminal state, route,
  snapshot, persisted record, and UI surface;
- typed program launches with exact argument arrays rather than hidden shell
  concatenation;
- no implicit Enter for paste, insertion, suggestions, actions, or imported
  text;
- bounded parsing for terminal control strings and image protocols;
- route-scoped paste and modal input ownership;
- transactional configuration and last-known-good recovery;
- deny-by-default optional capability boundaries;
- explicit cancellation, cleanup, and worker joining;
- credentials remaining with the platform or external tool that owns them
  whenever possible; and
- release artifacts tied to exact source, lockfiles, checksums, provenance,
  signatures, and platform evidence.

Never open a public issue containing credentials, real private infrastructure,
terminal history, exploit details, or identifying provider output. Follow
[Security policy](SECURITY.md) for private reporting.

## Extension and capability boundary

The codebase contains versioned infrastructure for optional components,
but Automexia v0.4 does **not** claim a public third-party extension marketplace,
download service, or unrestricted component-execution model.

Optional code is deny-by-default. It receives only explicit, reviewed,
application-owned capabilities and cannot silently gain ambient terminal
history, clipboard, filesystem, network, credential, process, or PTY authority.
Disabling or removing optional behavior must leave the base terminal usable.

This README documents only implemented behavior and current release status.
Unreleased and private work remains outside this documentation boundary.

## Development and contribution

Automexia uses repository-owned commands so contributors do not have to
reconstruct the normal validation sequence manually.

| Command | Purpose |
|---|---|
| `cargo dev` | Complete the required first development workflow, then launch Automexia. |
| `cargo automexia` | Fast incremental build, smoke, and launch after the full gate has passed. |
| `cargo ready` | Complete contributor gate without launching. |
| `cargo ci` | Repository CI profile. |
| `cargo qa` | Deeper assurance profile. |
| `cargo storage` | Report build-storage use. |
| `cargo purge` | Remove verified project build artifacts after Automexia closes. |
| `cargo xtask doctor` | Check contributor prerequisites, storage, and platform placement. |
| `cargo xtask verify architecture` | Enforce architecture and capability boundaries. |
| `cargo xtask verify identity` | Enforce Automexia product identity. |
| `cargo xtask verify provenance` | Verify licensing, notices, attribution, and publication policy. |
| `cargo xtask test conformance` | Run terminal and Unicode conformance coverage. |

The full contributor gate includes structured-file validation, project
verification, package metadata, rustfmt, locked all-feature checks,
warning-denied Clippy, all-feature workspace tests, dependency policy, a debug
build, and executable identity smoke. Platform-specific changes require native
host evidence in addition to portable source checks.

Contributions require DCO 1.1 sign-off (`git commit -s`). Read
[Contributing](CONTRIBUTING.md), [Testing](docs/TESTING.md), and the repository
[AI contributor workflow](AGENTS.md) before making substantial changes.

## Testing and assurance

The repository separates source correctness from native-platform and release
evidence. A passing portable test is never treated as proof of a native display,
PTY, package, signing, accessibility, or hardware path it did not exercise.

Assurance includes, where applicable:

- unit and integration tests;
- property tests;
- deterministic concurrency checks;
- parser and image fuzzing;
- terminal/Unicode conformance;
- resize/reflow stress;
- renderer goldens and controlled visual evidence;
- dependency, license, advisory, and provenance policy;
- secret scanning;
- workflow-policy and mutation checks;
- package validation; and
- controlled native Windows, Linux, and macOS evidence.

See [Testing](docs/TESTING.md), [Readiness audit](docs/READINESS-AUDIT.md), and
[Feature test reinforcement](docs/FEATURE-TEST-REINFORCEMENT.md).

## Documentation map

The documentation is intentionally organized by task and authority.

| Goal | Start here |
|---|---|
| Install or verify a package | [Installation](docs/INSTALLATION.md) |
| Learn the terminal | [Getting started](docs/GETTING-STARTED.md) and [User guide](docs/user-guide/index.md) |
| Check what is actually public/current | [Feature catalog](docs/FEATURES.md) |
| Use windows, tabs, panes, and sessions | [Workspace guide](docs/user-guide/workspace.md) |
| Understand commands and shells | [Commands and shell](docs/user-guide/commands-and-shell.md) |
| Search and navigate | [Productivity](docs/user-guide/productivity.md) |
| Work with files and images | [Files and images](docs/user-guide/files-and-images.md) |
| Use system OpenSSH / remote shells | [Remote connections](docs/guide/remote-connections.md) |
| Customize Automexia | [Configuration](docs/CONFIGURATION.md) |
| Look up shortcuts | [Keyboard](docs/KEYBOARD.md) |
| Look up CLI commands | [CLI reference](docs/CLI-REFERENCE.md) |
| Understand shell integration | [Shell integration](docs/SHELL-INTEGRATION.md) |
| Check platform status | [Platforms](docs/PLATFORMS.md) |
| Understand accessibility | [Accessibility](docs/ACCESSIBILITY.md) |
| Understand architecture | [Architecture](docs/ARCHITECTURE.md) and [ADRs](docs/DECISIONS.md) |
| Understand release evidence | [Release trust](docs/RELEASE-TRUST.md) |
| Contribute | [Contributing](CONTRIBUTING.md) |
| Report security issues | [Security](SECURITY.md) |
| Browse everything | [Documentation home](docs/index.md) |

## Documentation and release boundary

This repository documents the **current implemented terminal and released free
terminal capabilities**.

Some source behavior can be source-complete, disabled, release-gated, or still
waiting for native evidence. Those states are intentionally different from
"available in the published package."

The presence of an internal package name, fixture, test, disabled source path,
or capability contract is not a product announcement. Public availability is
owned by the feature catalog and release documentation.

Unreleased and private work is intentionally maintained outside the
documentation boundary.

See [Features](docs/FEATURES.md), [Product vision](docs/PRODUCT-VISION.md),
[Brand guide](docs/BRANDING.md), and
[Private documentation policy](docs/PRIVATE-DOCUMENTATION-POLICY.md).

## Brand and visual identity

The README uses Automexia's canonical supplied raster application mark:

`assets/brand/automexia-terminal-source-512.png`

The repository derives platform PNG, ICO, and ICNS assets from that source for
development and non-stable packaging. Stable brand publication remains a
separate reviewed gate: editable source assets, platform variants,
redistribution-rights evidence, and approval status are tracked in
`assets/brand/ASSET-MANIFEST.toml`.

Public copy follows the project's brand principles: explain real user value
first, use precise terminal language, distinguish implementation from release
evidence, and avoid claims that Automexia replaces shells, provider tools,
credential stores, or specialist software.

## Upstream heritage

Automexia preserves Rio's Git history and inherited copyright notices. The
project's audited Rio base is recorded in [UPSTREAM.md](UPSTREAM.md), and later
upstream changes are selectively reviewed and adapted rather than blindly
merging a moving upstream branch.

This keeps terminal-engine provenance visible while allowing Automexia to own
its separate application identity, workspace model, session behavior,
capability boundaries, tests, packaging, documentation, and release process.

See [NOTICE.md](NOTICE.md), [UPSTREAM.md](UPSTREAM.md), and
[Third-party notices](THIRD_PARTY_NOTICES.md).

## License

Automexia Terminal is available under the [MIT License](LICENSE).

---

<p align="center">
  <strong>Automexia Terminal</strong><br>
  A flexible terminal for focused command-line work.<br><br>
  <a href="docs/INSTALLATION.md">Install</a> ·
  <a href="docs/index.md">Documentation</a> ·
  <a href="docs/FEATURES.md">Feature status</a> ·
  <a href="SECURITY.md">Security</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>
