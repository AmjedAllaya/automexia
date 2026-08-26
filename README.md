<p align="center">
  <img src="assets/brand/automexia-terminal-source-512.png" alt="Automexia Terminal logo" width="180">
</p>

<h1 align="center">Automexia Terminal</h1>

<p align="center"><strong>A flexible terminal that makes complex workflows faster, simpler, and easier to control.</strong></p>

<p align="center">
  <a href="docs/INSTALLATION.md">Install</a> ·
  <a href="docs/GETTING-STARTED.md">Get Started</a> ·
  <a href="docs/user-guide/index.md">User Guide</a> ·
  <a href="docs/FEATURES.md">Features</a> ·
  <a href="docs/EXTENSIONS.md">Extensions</a> ·
  <a href="docs/CONFIGURATION.md">Configuration</a> ·
  <a href="docs/FAQ.md">FAQ</a>
</p>

Automexia Terminal is for anyone who turns ideas into action through commands.
It brings shells, tools, files, and project context into one organized workspace
so you can move between tasks with less friction and less repetitive setup.

Flexibility is central to Automexia. It is designed to grow through focused
extensions, so people and teams can add the tools, context, and repeatable
workflows their field needs without making the core terminal crowded for
everyone else.

Today, Automexia provides a strong terminal experience for software work,
system operations, automation, and everyday command-line tasks. Its wider
direction goes beyond development: the same flexible workspace can grow to
support creative and specialized workflows, including media processing and
video editing, without hiding what runs or taking control away from you.

## Why Automexia

### Move faster with less friction

Keep related tools together, open or copy a working setup in seconds, and find
commands or results without breaking your flow. Automexia reduces the small,
repeated steps that make complex work feel slower than it should.

### Keep complex work understandable

Separate tasks into windows, tabs, and panes while keeping the active shell,
path, and project context visible. You can focus on one task without losing
sight of the wider workflow.

### Shape the workspace around you

Choose how work is arranged, how shortcuts behave, and how the terminal looks.
Automexia is meant to adapt to different people, platforms, tools, and fields
instead of forcing every workflow into the same layout. Its extension model
keeps that flexibility open: each domain can add a focused experience while
the terminal remains useful on its own.

### Stay in control

Automexia keeps important actions visible and deliberate. Local previews stay
local, invalid settings do not replace a working configuration, and planned
integrations do not quietly gain permission to run commands or use credentials.

## What works today

### Organize work without losing context

Use separate windows, top-level tabs, split panes, and tabs inside each pane.
Open a fresh shell when you want a clean start, or copy the active shell and
working directory when you want to continue the same task elsewhere.

### Find what you need quickly

Search the selected terminal or every visible pane in the workspace. Use the
command palette when you know what you want to do but do not remember the
shortcut.

### Use familiar shells

PowerShell, Command Prompt, WSL, Bash, and Zsh fit into the same workspace.
Automexia can keep useful details such as the current path, shell, and command
timing visible without adding noise to command output.

### See more than text

View inline terminal images and preview local image files directly from command
output. Local previews stay on your computer and do not require an upload.

### Make it yours

Adjust themes, fonts, windows, navigation, shells, and shortcuts through a
readable configuration file. Changes apply safely, and the last working setup
remains active when a new setting is invalid. An optional Ghostty keyboard
profile offers a familiar starting point without replacing Automexia's defaults.

## Extensions that fit the work

Automexia is designed to be more than a fixed collection of terminal features.
A focused extension can adapt the workspace to a field, toolset, or team without
forcing the same setup on every user. People can keep the terminal simple and
add only the capabilities that make their own work faster and easier to manage.

Different domains can have their own extensions. An infrastructure extension
can organize cloud accounts, clusters, connections, and environments. An
automation extension can turn repeated work into clear, reusable recipes. Data,
research, media, and video extensions can bring their own tools and focused
controls while continuing to use the terminal as the common workspace.

### First-party extensions already in the source

The repository already contains separate first-party extension foundations for:

- OpenSSH and Teleport connections;
- Amazon Web Services, Microsoft Azure, and Google Cloud;
- Kubernetes and OpenShift;
- multi-environment workspaces and reviewable automation recipes;
- provider-aware actions that retain the selected account, project, cluster,
  environment, and production-risk context.

These foundations are built to make multi-cloud and production work easier to
understand and safer to repeat. They keep environments separate, show the exact
target before a sensitive action, and avoid turning hidden global state into the
source of truth.

The code and tested contracts for these first-party extensions exist today, but
they remain behind release gates while product activation, native provider
testing, and the remaining safety checks are completed. They are not public
downloads in Automexia v0.4.

### Production guidance that stays under your control

A planned DevOps/SRE feature will help people see the exact production account,
region, cluster, namespace, identity, and incident before choosing an action. It
will use fresh, bounded evidence to place the most relevant native commands
first, explain why they are useful, show uncertainty and impact, and keep
diagnosis, approval, monitoring, and recovery visible.

For example, after `kubectl rollout`, Automexia may suggest checking status,
reviewing history, undoing a bad rollout, restarting a specific Deployment, or
continuing diagnosis. It will not assume that every unhealthy Pod needs a
restart, and it will never run the selected command automatically.

This is planned work, not a current v0.4 feature. It will remain optional,
perform no provider work on every keystroke, keep credentials with their
existing owners, and use deterministic safety rules without requiring an LLM.
See the [Production Operations proposal](docs/SITUATION-AWARE-PRODUCTION-OPERATIONS.md)
and its [evidence plan](docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md).

### A future workspace for scripts and automation

The proposed Automation Studio will make scripts and configuration easier to
write, compare, understand, and review beside the terminal. It is planned as an
optional part of the Automexia window, while the DevOps/SRE extension remains a
separate package that still works through terminal-native actions when the
editor is not installed.

This keeps the experience flexible: install one DevOps/SRE Pack for a complete
starting point, choose only the language and tool support you need, or keep a
light terminal-only setup. The Studio is an architecture proposal today, not a
shipped editor. See the [proposed architecture](docs/AUTOMATION-STUDIO-ARCHITECTURE.md)
and its [evidence plan](docs/AUTOMATION-STUDIO-TESTING.md).

Product delivery is deliberately ordered: finish the first stable terminal,
prove the shared extension and DevOps foundations, then deliver the separately
gated production context, evidence, situation-aware completion, and preflight
slices. A reusable workflow contract and a useful minimal Studio follow, and
only then does the dedicated video-editing extension release. Studio,
orchestration, and video research can continue
earlier, but none should delay the first stable terminal. Video will reuse
general workspace and task services, not depend on Studio's editor internals.

### Optional orchestration, without an AI-dependent terminal

Automexia is designed to stay useful without an AI model, an account, or a paid
API. A future, separately installed LLM Orchestration extension may help people
turn a goal into a reviewable workflow across extensions. It will not be built
into the terminal core, the DevOps/SRE extension, Automation Studio, or the
video-editing extension.

The model will only propose a bounded, typed plan. Automexia will validate that
plan, show the important steps and risks, ask for the required approval, and
execute through the same controlled actions available without AI. Local or
self-hosted models will be the default path; optional remote providers will
remain adapters inside the extension. See the
[LLM Orchestration proposal](docs/LLM-ORCHESTRATION-EXTENSION.md) for the exact
boundary and current status.

### A simple extension experience is the goal

The planned public extension experience will let people create, review, install,
update, disable, and remove extensions for the work they do. An installed
extension should add useful capabilities without silently gaining access to
commands, credentials, files, networks, or production systems.

This is how Automexia can grow from a flexible terminal into a workspace for
many fields without losing its speed, clarity, or user control. See the
[feature catalog](docs/FEATURES.md) for the current implementation status, the
[ecosystem boundary](docs/ECOSYSTEM-PLATFORM.md) for the planned installation
model, and the [product vision](docs/PRODUCT-VISION.md) for the wider direction.

## Quick start from source

Automexia is currently built from source; official signed stable installers are
not published yet. The complete [installation guide](docs/INSTALLATION.md)
explains the Windows, Linux, and macOS requirements, expected first-run result,
updates, build-space cleanup, and removal.

After installing the Rust toolchain declared in `rust-toolchain.toml`, Python 3
with PyYAML, and the pinned `cargo-deny` release, run:

```text
python -m pip install PyYAML
cargo install --locked cargo-deny --version 0.20.2
cargo dev
```

`cargo dev` checks the project, builds Automexia, prepares its shell support,
and opens the terminal. The first run can take several minutes and requires at
least 12 GiB of free build space.

After the repository has passed the full gate, use the faster daily command:

```text
cargo automexia
```

Before opening a pull request, run the complete non-launching gate:

```text
cargo ready
```

Read the [contributor guide](CONTRIBUTING.md),
[testing guide](docs/TESTING.md), and
[Windows/WSL guide](docs/WSL-DEVELOPMENT.md) for the complete development and
release workflow.

Once the window opens, follow the [getting-started tutorial](docs/GETTING-STARTED.md)
or take the [15-minute User Guide tour](docs/user-guide/index.md#a-15-minute-tour).

## Run Automexia

With the executable on `PATH`:

```text
automexia
automexia --working-dir <PATH>
automexia -e <PROGRAM> [ARGS...]
```

The `-e` or `--command` option must be last because the remaining arguments
belong to the program being opened.

Configuration is stored in these locations by default:

| Platform | Configuration folder |
|---|---|
| Windows | `%LOCALAPPDATA%\Automexia\Terminal` |
| macOS | `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` |
| Linux/BSD | `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia` |

Set `AUTOMEXIA_CONFIG_HOME` to use a different folder.

## Stay in the flow with the keyboard

Everyday work can be completed without reaching for a pointer. Keyboard
controls cover command search, terminal search, selection, windows, tabs,
panes, image previews, configuration, and appearance.

Shortcuts follow the conventions of each platform, so macOS uses Command for
common desktop actions while Windows, Linux, and BSD use Control. See the
[practical shortcut guide](docs/user-guide/shortcuts.md) or the
[complete keyboard reference](docs/KEYBOARD.md).

## Platforms

| Platform | Current position |
|---|---|
| Windows | Supported with PowerShell, Command Prompt, and WSL workflows |
| Linux | Supported with native desktop and shell integration |
| macOS | Supported with native desktop and shell integration |
| BSD | Source-compatible and best effort where the Unix paths apply |

See [platform support](docs/PLATFORMS.md) for current limitations and validation
details.

## Move quickly without giving up control

Automexia is designed to help you work faster without making hidden decisions
for you:

- local image previews stay on your computer;
- actions that can change something remain visible and deliberate;
- credentials stay in platform or external credential stores when possible;
- invalid configuration changes leave the last working setup in place;
- new providers and extensions begin without command, network, or credential
  access until their boundaries are reviewed.

Managed SSH, live cloud-provider access, public extensions, and automated
command execution are not shipped v0.4 features. The project labels unfinished
or disabled work clearly instead of presenting it as ready.

Read [Security](SECURITY.md), [Architecture](docs/ARCHITECTURE.md), and the
[build, wrap, or adopt boundary](docs/BUILD-WRAP-ADOPT-ARCHITECTURE.md) for the
full trust model.

## Project status

Automexia is under active development. The current source tree contains the
v0.4 terminal and separately gated work for later releases. The
[feature catalog](docs/FEATURES.md) distinguishes what people can use today
from what is internal, awaiting release evidence, or still planned.

Official stable packages still depend on the remaining brand, signing, and
release prerequisites. See the [brand asset workflow](docs/BRANDING.md) and
[release readiness audit](docs/READINESS-AUDIT.md) for the details.

## Documentation

- [Product vision](docs/PRODUCT-VISION.md)
- [Install Automexia](docs/INSTALLATION.md)
- [Getting started](docs/GETTING-STARTED.md)
- [Complete User Guide](docs/user-guide/index.md)
- [Start and launch sessions](docs/user-guide/start-and-launch.md)
- [Workspaces, tabs, and panes](docs/user-guide/workspace.md)
- [Commands and shell workflows](docs/user-guide/commands-and-shell.md)
- [Files and image previews](docs/user-guide/files-and-images.md)
- [Extensions](docs/EXTENSIONS.md)
- [Frequently asked questions](docs/FAQ.md)
- [Configuration reference](docs/CONFIGURATION.md)
- [CLI reference](docs/CLI-REFERENCE.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Situation-aware production operations proposal](docs/SITUATION-AWARE-PRODUCTION-OPERATIONS.md)
- [Production operations UX and implementation blueprint](docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md)
- [Optional LLM orchestration strategy](docs/LLM-ORCHESTRATION-EXTENSION.md)
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
