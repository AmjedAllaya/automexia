<p align="center">
  <img src="assets/brand/automexia-terminal-source-512.png" alt="Automexia Terminal logo" width="180">
</p>

<h1 align="center">Automexia Terminal</h1>

<p align="center"><strong>A fast, focused terminal for real development work.</strong></p>

<p align="center">
  <a href="docs/user-guide/index.md">User Guide</a> ·
  <a href="docs/FEATURES.md">Features</a> ·
  <a href="docs/CONFIGURATION.md">Configuration</a> ·
  <a href="docs/KEYBOARD.md">Keyboard</a> ·
  <a href="docs/ROADMAP.md">Roadmap</a>
</p>

Automexia Terminal is a hardware-accelerated desktop terminal for Windows,
Linux, and macOS. It keeps shells at the center of the experience while making
windows, tabs, panes, search, images, and project context easier to manage.

Version 0.4.0 is a standalone product built from Rio's open-source history. It
has its own executable, application identity, configuration folder, artwork,
and release process.

## Why Automexia

Terminal work should feel direct. Automexia keeps the command line familiar and
adds useful structure around it:

- arrange work with windows, window tabs, split panes, and pane-local tabs;
- open a fresh shell or clone the current shell and working directory;
- search one pane or every visible pane without sending input to the shell;
- keep the active pane, command state, path, and shell context easy to see;
- preview local images without uploading them or fetching remote content;
- use keyboard controls for the full everyday workflow;
- change appearance and behavior through a readable TOML configuration file.

The interface uses Automexia's blue-black surfaces, cyan and blue actions, and
clear status colors. Important meaning is also shown with text or icons, so it
does not depend on color alone.

## What works today

### Workspaces that stay organized

Automexia supports independent operating-system windows, window-level tabs,
split panes, and tabs inside each pane. Pane-local tabs own separate terminal
sessions and do not rearrange neighboring panes.

Fresh splits open the normal configured shell. Clone actions open an independent
session with the active shell, profile, WSL identity, and working directory.

### Search and command access

Pane search stays inside the selected terminal. Workspace search covers all
visible panes in the active workspace without opening hidden tabs. Both search
surfaces keep keyboard input away from the PTY until they close.

The command palette gives one place to find actions when a shortcut is hard to
remember.

### Shells that feel at home

Automexia has session-scoped support for PowerShell, Command Prompt, WSL, Bash,
and Zsh workflows. It can show the current path, shell, user, command timing,
and useful development context without writing decorative text into terminal
output. Icon-aware listings remain normal shell data when piped to another tool.

Shell integration used by a normal launch belongs only to that child session.
Persistent profile installation is a separate, explicit maintenance action and
can be inspected or removed.

### Local and inline images

Applications can render Sixel, Kitty Graphics, and iTerm2 inline images.
Automexia can also preview bounded local raster files selected from terminal
output. Local previews load in the background, stay with the current terminal
view, and do not fetch anything from the internet.

### Configuration that fails safely

Automexia reads TOML configuration and supports platform overrides, themes,
fonts, window settings, navigation, shell settings, and custom bindings. Live
reload publishes a complete valid configuration or keeps the last known good
one when the new file is invalid.

An optional Ghostty 1.3 keyboard profile is available for users who want a
familiar migration path. Automexia's own bindings remain the default.

## Quick start from source

Install the Rust toolchain declared in `rust-toolchain.toml`, Python 3 with
PyYAML, and the pinned `cargo-deny` release. Then run:

```text
python -m pip install PyYAML
cargo install --locked cargo-deny --version 0.20.2
cargo dev
```

`cargo dev` runs the complete contributor gate, builds Automexia, checks the
resulting executable, prepares session-only shell support, and opens the
terminal. The first run can take several minutes and requires at least 12 GiB
of free space on the selected Cargo target filesystem.

After the repository has passed the full gate, use the faster daily command:

```text
cargo automexia
```

Before opening a pull request, run the full non-launching gate:

```text
cargo ready
```

Use `cargo storage` to inspect build storage. `cargo purge` removes all Cargo
build artifacts and should only be used after Automexia windows are closed.

Windows and WSL builds should use separate native checkouts. Keep Linux builds
inside the WSL filesystem, such as `~/src/automexia-terminal`, and keep
Windows/MSVC, ConPTY, GPU, and packaging work on NTFS.

Read the [contributor guide](CONTRIBUTING.md) and
[testing guide](docs/TESTING.md) for platform tools, focused checks, native
evidence, packaging, and release validation.

## Run Automexia

With the executable on `PATH`:

```text
automexia
automexia --working-dir <PATH>
automexia -e <PROGRAM> [ARGS...]
```

The `-e` or `--command` option must be last because everything after the program
name is passed directly to that program.

Configuration is stored in these locations by default:

| Platform | Configuration folder |
|---|---|
| Windows | `%LOCALAPPDATA%\Automexia\Terminal` |
| macOS | `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` |
| Linux/BSD | `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia` |

Set `AUTOMEXIA_CONFIG_HOME` to use a different folder.

## Keyboard-first by design

Automexia can be used without a pointer. The main controls cover command search,
terminal search, selection, windows, tabs, panes, local tabs, image previews,
configuration, and appearance.

Shortcuts follow the conventions of each platform, so macOS uses Command for
common desktop actions while Windows, Linux, and BSD use Control. See the
[practical shortcut guide](docs/user-guide/shortcuts.md) or the
[complete keyboard reference](docs/KEYBOARD.md).

## Platforms

| Platform | Current position |
|---|---|
| Windows | Supported with native ConPTY, PowerShell, Command Prompt, and WSL workflows |
| Linux | Supported with native Unix PTY and desktop integration |
| macOS | Supported with native desktop and shell integration |
| BSD | Source-compatible and best effort where the Unix paths apply |

Cross-compilation is useful build evidence, but it does not replace native
runtime, graphics, packaging, or accessibility testing. The exact platform
claims are listed in [platform support](docs/PLATFORMS.md).

## Clear security boundaries

Automexia treats terminal output, paths, imported files, provider output,
completions, and generated content as untrusted input. Structured actions use
exact executables and argument lists instead of building shell command strings.
Queues, history, image dimensions, files, retries, logs, and stored data have
explicit limits.

Credentials remain in platform or external credential stores whenever possible.
The terminal does not turn background discovery into hidden process or network
authority.

Managed SSH, live cloud-provider authentication, third-party extension
downloads, public extension execution, and AI command execution are not shipped
v0.4 features. Some later foundations exist in source with execution disabled;
the documentation labels them as internal, release-gated, or planned instead of
presenting them as available product behavior.

Read [Security](SECURITY.md), [Architecture](docs/ARCHITECTURE.md), and the
[build, wrap, or adopt boundary](docs/BUILD-WRAP-ADOPT-ARCHITECTURE.md) for the
full trust model.

## Project status

The source tree contains the current v0.4 terminal and carefully separated v0.5
foundations. The [feature catalog](docs/FEATURES.md) explains what is available,
what is implemented locally but still release-gated, what is internal and
disabled, and what remains planned.

Stable v0.4 publication still requires the final vector and monochrome logo
sources, written artwork-rights approval, Windows signing, Apple signing and
notarization, and the remaining private reporting/release prerequisites. The
current raster mark is used for development and nightly packages. See the
[brand asset workflow](docs/BRANDING.md) and
[release readiness audit](docs/READINESS-AUDIT.md).

## Documentation

- [Complete User Guide](docs/user-guide/index.md)
- [Start and launch sessions](docs/user-guide/start-and-launch.md)
- [Workspaces, tabs, and panes](docs/user-guide/workspace.md)
- [Commands and shell workflows](docs/user-guide/commands-and-shell.md)
- [Files and image previews](docs/user-guide/files-and-images.md)
- [Configuration reference](docs/CONFIGURATION.md)
- [CLI reference](docs/CLI-REFERENCE.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Testing](docs/TESTING.md)
- [Roadmap](docs/ROADMAP.md)
- [Decision index](docs/DECISIONS.md)

The [documentation home](docs/index.md) has the complete reading map.

## Contributing

Contributions should be focused, tested, documented, and signed off under the
Developer Certificate of Origin. Start with [CONTRIBUTING.md](CONTRIBUTING.md)
and the repository's [AI contributor workflow](AGENTS.md).

For help or responsible reporting, read [SUPPORT.md](SUPPORT.md),
[SECURITY.md](SECURITY.md), and [GOVERNANCE.md](GOVERNANCE.md).

## License and upstream history

Automexia Terminal is available under the MIT License. The repository preserves
Rio's Git history and copyright notices. See [NOTICE.md](NOTICE.md) and
[UPSTREAM.md](UPSTREAM.md) for the exact fork point and upstream-port policy.
