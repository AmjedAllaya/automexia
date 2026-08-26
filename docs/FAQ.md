# Frequently asked questions

This page answers the questions a new user is most likely to ask before and
after trying Automexia. Exact commands, settings, shortcuts, and feature status
remain in the linked reference pages.

## What is Automexia?

Automexia is a flexible terminal workspace that makes complex command-driven
work faster, simpler, and easier to control. It keeps shells, tools, files,
sessions, and useful context together without replacing the commands people
already know.

Read the [product vision](PRODUCT-VISION.md) for the wider purpose and the
[feature catalog](FEATURES.md) for what exists today.

## Is Automexia only for developers?

No. Developers are one audience, but the project is also intended for system
administrators, DevOps and site reliability engineering (SRE) teams,
researchers, analysts, automation users, creators, and anyone whose work uses
commands or repeatable tools.

The core terminal is general. Focused extensions are intended to adapt it to a
field without forcing that field's tools on every user.

## Does Automexia replace my shell or command-line tools?

No. PowerShell, Command Prompt, WSL, Bash, Zsh, SSH, Git, Kubernetes tools,
cloud command-line interfaces (CLIs), Python, FFmpeg, text user interfaces, and
other programs keep their normal behavior. Automexia organizes the workspace
around them.

Your shell owns normal command editing and execution. Selecting a reviewed
suggestion should never silently press Enter.

## What can I use today?

The current v0.4 user-facing product focuses on the terminal workspace:

- windows, tabs, split panes, and pane-local tabs;
- PowerShell, Command Prompt, WSL, Bash, and Zsh workflows;
- terminal and visible-workspace search;
- keyboard-first navigation, selection, and command discovery;
- visible shell, path, command, and project context;
- terminal-protocol images and bounded local image previews;
- configurable appearance, behavior, and shortcuts.

The repository also contains later-release foundations. Always check the status
label in the [feature catalog](FEATURES.md) before assuming source code is a
released user feature.

## Are stable installers available?

Not yet. Automexia is currently available as a source build. Repository-owned
Windows, macOS, and Linux package definitions exist, but stable publication is
blocked until the remaining brand, signing, native, accessibility, and release
checks are complete.

Follow [Install Automexia](INSTALLATION.md) for the supported source path. Do
not treat an unverified third-party download as an official Automexia package.

## How do I try Automexia from source?

After installing the platform prerequisites, Rust, Python, PyYAML, and the
pinned `cargo-deny` version, run from the repository root:

```text
cargo xtask doctor
cargo dev
```

The first command checks the computer. The second performs a verified build and
opens Automexia. Use `cargo automexia` for later incremental launches. The
complete steps and expected results are in [Install Automexia](INSTALLATION.md).

## Why does the first build take several minutes and use a lot of space?

Automexia is a native Rust desktop application with a terminal engine,
renderer, font system, window integration, and platform adapters. The first
verified build compiles these dependencies and runs broad checks in an isolated
directory.

Keep at least 12 GiB free for the first complete gate. Run `cargo storage` to
see project build usage and, after closing Automexia, `cargo purge` to remove
only the repository-owned build artifacts. See
[Reclaim build space](INSTALLATION.md#reclaim-build-space).

## Which operating systems are supported?

Automexia has release and continuous-integration ownership for Windows, Linux,
and macOS. BSD uses shared Unix paths on a best-effort basis and is not part of
the current certified artifact set.

Windows supports PowerShell, Command Prompt, and WSL workflows. Linux declares
X11 and Wayland builds. macOS uses its native application and graphics path.
Read [Platform support](PLATFORMS.md) for exact evidence and limitations.

## Can Automexia run on a Raspberry Pi, smartphone, Nintendo Switch, or Arduino?

The planned package matrix includes Linux ARM64, so a suitable 64-bit ARM Linux
desktop computer may become a practical target. That does not currently prove
native support for a Raspberry Pi model, graphics driver, distribution, or
small-board setup; those combinations still need real builds and runtime tests.

Smartphones and Nintendo Switch are not supported platforms today. They would
need dedicated window, input, packaging, sandbox, pseudo-terminal (PTY), and
graphics work rather than a simple cross-compile.

Arduino-class microcontrollers cannot run the current application. Automexia
expects a desktop operating system, virtual memory, filesystem, PTY, windowing
environment, and graphics stack. A lightweight remote companion could be a
future separate project, but it is not part of the current terminal.

## Can I use Automexia without creating a configuration file?

Yes. Tested defaults are used when no personal configuration exists. Create a
small starter file only when you want to change something:

```text
automexia --write-config
```

The command refuses to overwrite an existing file. Invalid or oversized
configuration is rejected while the last working runtime configuration remains
active. See [Configuration and customization](user-guide/customization.md).

## Where is the configuration stored?

The default folders are:

| Platform | Folder |
|---|---|
| Windows | `%LOCALAPPDATA%\Automexia\Terminal` |
| macOS | `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` |
| Linux/BSD | `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia` |

Set `AUTOMEXIA_CONFIG_HOME` before launch to isolate a test profile. Exact
configuration behavior is documented in [Configuration reference](CONFIGURATION.md).

## What is the difference between a window tab, split, and pane-local tab?

- A **window-level tab** changes the whole workspace shown in the window.
- A **split** keeps two or more sessions visible side by side.
- A **pane-local tab** changes only the session inside one pane while its
  neighboring panes stay visible.
- A separate **operating-system window** is independent from the others.

The diagrams and decision guide are in
[Workspaces, tabs, and panes](user-guide/workspace.md).

## How do I discover shortcuts without memorizing them?

Open the command palette with `Ctrl+Shift+P` on Windows/Linux/BSD or
`Cmd+Shift+P` on macOS. Search for the action you need. The
[practical shortcut guide](user-guide/shortcuts.md) explains everyday controls,
and the [keyboard reference](KEYBOARD.md) lists every supported action and
platform default.

## Does Automexia support image output?

Yes. It supports terminal graphics protocols and bounded previews of explicitly
selected local raster images. Local quick look does not fetch a remote URL and
does not change the command's text output.

SVG and PDF quick look are not current v0.4 claims. See
[Files, output, and images](user-guide/files-and-images.md).

## Can I use SSH, Kubernetes, and cloud CLIs today?

Yes—use their normal commands in your shell, just as you would in another
terminal. For example, system `ssh`, `kubectl`, `aws`, `az`, and `gcloud` remain
owned by those tools.

Automexia's managed Connection Hub, provider authentication, and managed
execution have stricter status boundaries. Some read-only and review models are
implemented locally, while live provider work remains gated or unavailable.
Read [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md) before
testing a source-only provider surface.

## Does Automexia already know which Kubernetes deployment I should restart?

No. Situation-aware completion, workload prioritization, production passports,
incident evidence, and managed operations are planned work. The design includes
the `kubectl rollout` example, but v0.4 does not claim a live cluster watcher,
ranker, or automatic restart path.

The planned behavior will show evidence and uncertainty, offer native commands,
and leave the final decision with the user. See the
[Production Operations specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md).

## Can I install extensions now?

No public extension installation or marketplace workflow is released in v0.4.
The source has first-party extension foundations and a disabled verified-bundle
store, but activation and public downloads remain blocked.

Read [Extensions](EXTENSIONS.md) for the current first-party inventory, future
installation experience, and exact safety boundary.

## Will there be extensions for work outside DevOps?

That is the direction. Automexia is intended to support focused extensions for
automation, data, research, media, video, and other command-driven fields. Each
extension should be optional, separately reviewed, and removable without
weakening the core terminal.

These are future domains, not current product claims. See the
[product vision](PRODUCT-VISION.md#long-term-direction).

## Does Automexia include an editor or IDE?

Not today. Automation Studio is a proposal for an optional editor and integrated
development environment (IDE) inside the Automexia window. It is not required
by the terminal or the separately planned DevOps/SRE extension.

No editor dependency, document service, language server, Studio user interface,
or Studio script-execution path is shipped. See the
[Automation Studio architecture](AUTOMATION-STUDIO-ARCHITECTURE.md).

## Is video editing available?

No. A dedicated video-editing extension is part of the longer-term product
direction. It is deliberately sequenced after the first stable terminal and a
useful minimal Automation Studio slice. Video work will remain a separate domain
extension and will not depend on a large language model.

## Is Automexia an AI terminal?

Automexia is AI-capable, not AI-dependent. The core terminal and domain
extensions must remain useful without a model, paid application programming
interface (API), provider account, or network connection.

A future large language model (LLM) Orchestration extension may help a user
compose a reviewable workflow. It will be separately installed and will not be
built into core, the DevOps/SRE extension, Automation Studio, or video editing.
Models will propose bounded plans; they will not receive direct credentials,
shell access, provider tools,
or hidden execution authority. See the
[LLM Orchestration proposal](LLM-ORCHESTRATION-EXTENSION.md).

## Does Automexia upload my terminal output or local images?

Local image quick look stays local and remote image fetching is not a v0.4
feature. The disabled extension and model foundations do not receive ambient
terminal history, clipboard, files, credentials, provider caches, or telemetry
data.

Commands that you explicitly run may use the network according to the behavior
of that command—for example `ssh`, a cloud CLI, or a package manager. Automexia
does not make an external tool local merely because it runs inside the terminal.
Review [Security](../SECURITY.md) for the exact trust boundaries.

## Where are passwords, tokens, and cloud credentials stored?

Automexia's design keeps credentials with platform stores or the external tools
that already own them whenever possible. Provider-facing models use public
context and opaque references rather than persisting secret values.

The current provider foundations do not authorize live credential-cache access
or login. Never place a token, password, private key, or secret directly in a
Quick Action, screenshot, support log, or configuration example.

## What happens if a configuration change is invalid?

Automexia rejects malformed, oversized, or invalid configuration and keeps the
last known-good runtime configuration. Correct the file and reload or restart.
The application should not replace a working setup with a partially parsed one.

Use [Troubleshooting](TROUBLESHOOTING.md#configuration-reload-fails) for the
recovery steps.

## How do I update or remove a source build?

For a clean checkout, inspect `git status --short`, update with
`git pull --ff-only`, and launch again. Do not discard local changes to force an
update.

To remove the source build, close Automexia, uninstall persistent shell
integration if you explicitly enabled it, preserve any configuration you want,
and remove the checkout. The separate configuration folder is not intentionally
removed with the checkout. See
[source updates](INSTALLATION.md#update-a-source-checkout) and
[source removal](INSTALLATION.md#remove-a-source-installation).

## Where should I report a problem or ask for help?

Start with [Troubleshooting](TROUBLESHOOTING.md). If the problem remains, follow
[Support](../SUPPORT.md) and include the Automexia version, operating system,
shell, exact steps, expected result, actual result, and the first useful error.

Remove credentials, private hostnames, personal paths, terminal history, and
other sensitive content before sharing logs or screenshots. Report security
issues through the private process in [Security](../SECURITY.md), not a public
issue.

## Where should a new user go next?

Use this order:

1. [Install Automexia](INSTALLATION.md).
2. [Getting started](GETTING-STARTED.md).
3. [Automexia User Guide](user-guide/index.md).
4. [Workflow recipes](user-guide/recipes.md).
5. [Troubleshooting](TROUBLESHOOTING.md) when something does not work.

For exact behavior, use [CLI reference](CLI-REFERENCE.md),
[Keyboard reference](KEYBOARD.md), and
[Configuration reference](CONFIGURATION.md).
