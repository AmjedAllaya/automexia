<p align="center">
  <img src="assets/brand/automexia-terminal-source-512.png" alt="Automexia Terminal logo" width="164">
</p>

<h1 align="center">Automexia Terminal</h1>

<p align="center">
  <strong>A flexible terminal that makes complex workflows faster, simpler, and easier to control.</strong>
</p>

<p align="center">
  <a href="docs/INSTALLATION.md"><strong>Get started</strong></a> ·
  <a href="docs/user-guide/index.md">User guide</a> ·
  <a href="docs/FEATURES.md">Feature status</a> ·
  <a href="docs/CONFIGURATION.md">Customize</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

<p align="center">
  <sub>v0.4.0 &nbsp;·&nbsp; Source build &nbsp;·&nbsp; Windows, Linux, and macOS</sub>
</p>

---

> **Fast by default. Structured when useful. Always under your control.**
>
> Automexia brings shells, files, tools, and useful context into one calm
> workspace. It keeps commands direct while removing the friction around them:
> repeated setup, scattered sessions, lost context, and hard-to-find actions.

## A better place for command-driven work

| **Organize** | **Find** | **Stay in control** |
|---|---|---|
| Keep related work together with windows, tabs, split panes, and pane-local tabs. | Search one pane or every visible pane without leaving the keyboard. | See the active shell, path, and command context. Sensitive work stays explicit and reviewable. |
| Clone a working session when you need its shell and directory, or start clean when you do not. | Reach actions quickly with the command palette, familiar shortcuts, and marked-command navigation. | Your shell remains the execution owner. Automexia does not quietly press Enter, replace credentials, or change global tool state. |

Automexia is for anyone who turns ideas into action through commands: developers,
operators, researchers, analysts, creators, and people building repeatable
workflows. Start with a simple terminal; add structure only when it helps.

## What you can do today

### Work in one focused workspace

- Open independent windows, top-level tabs, split panes, and local tabs.
- Run PowerShell, Command Prompt, WSL, Bash, Zsh, and other familiar shell
  workflows in the same application.
- Start a fresh session or clone the active session and its working directory.
- Select the pane you mean to use with the keyboard or pointer, then scroll,
  paste, resize, and navigate without losing the active context.

### Keep output and context easy to read

- See useful prompt context such as shell, location, command timing, and project
  details without changing the command line itself.
- Search the selected pane with <code>Ctrl+F</code>, or all visible panes with
  <code>Ctrl+Shift+F</code>. The same search session keeps its query and focus
  as its scope changes.
- Jump to the previous or next marked command with <code>Ctrl+Shift+Up</code>
  and <code>Ctrl+Shift+Down</code>.
- View local images and supported terminal-protocol images without uploading
  them anywhere.

### Make it feel like yours

- Adjust themes, fonts, windows, navigation, shells, and shortcuts in a
  readable configuration file.
- Keep application-owned font and appearance preferences across launches, with
  safe recovery when a new configuration is invalid.
- Start from an optional Ghostty 1.3 keyboard profile without replacing
  Automexia's defaults.

The complete current inventory, platform evidence, and known release gates are
maintained in the [feature catalog](docs/FEATURES.md).

## Start in minutes

Automexia is currently available as a source build. Official signed stable
installers are not published yet.

1. Install the prerequisites for your platform. The
   [installation guide](docs/INSTALLATION.md) lists the exact Windows, Linux,
   and macOS requirements.
2. Clone the repository and enter it:

   ~~~text
   git clone https://github.com/AmjedAllaya/automexia-terminal.git
   cd automexia-terminal
   ~~~

3. Complete the first verified launch:

   ~~~text
   cargo dev
   ~~~

4. On later launches, use the faster development command:

   ~~~text
   cargo automexia
   ~~~

<code>cargo dev</code> prepares shell support for the launched session, checks
the application, and opens Automexia only after the required validation
succeeds. For a complete setup, troubleshooting, cleanup, and removal path, use
[Install Automexia](docs/INSTALLATION.md).

### Everyday shortcuts

| Shortcut | Action |
|---|---|
| <code>Ctrl+Shift+P</code> | Open the command palette |
| <code>Ctrl+F</code> / <code>Ctrl+Shift+F</code> | Search the selected pane / all visible panes |
| <code>Ctrl+Shift+Up</code> / <code>Ctrl+Shift+Down</code> | Jump to the previous / next marked command |
| <code>Ctrl+T</code> / <code>Ctrl+Shift+T</code> | Create a window tab / local tab |
| <code>Ctrl+Shift+R</code> / <code>Ctrl+Shift+D</code> | Split right / down |
| <code>Ctrl+Shift+H</code> | Open the read-only Connection Hub |
| <code>Ctrl+Shift+O</code> | Open Quick Actions search and review |

These are the Windows, Linux, and BSD defaults. macOS uses the corresponding
Command-based shortcuts. See the [complete keyboard reference](docs/KEYBOARD.md)
for every platform, mode, and customization option.

## Connections and extensions, with clear boundaries

Automexia grows through focused components instead of putting provider,
network, credential, or automation authority on the terminal's hot path.
This keeps the base workspace predictable and lets each optional capability be
reviewed, disabled, or replaced independently.

| Area | Current status | What it means |
|---|---|---|
| Core terminal workspace | **Available now** | Panes, tabs, shells, search, configuration, keyboard navigation, and local image viewing are part of the source build. |
| Connection Hub | **Implemented locally; release-gated** | Source builds include a read-only OpenSSH inventory and review experience. It has no managed connection, login, network, or PTY-launch authority. |
| Completion and Quick Actions | **Implemented locally; release-gated** | Native shell and typed review foundations exist; a suggestion never becomes hidden execution. |
| Managed SSH and cloud providers | **Internal foundations; not activated** | The source contains reviewed boundaries for SSH, AWS, Azure, Google Cloud, Kubernetes, OpenShift, and Teleport. Live authentication, provider refresh, and managed execution are not v0.4 features. |
| Public extension marketplace | **Planned** | There is no public SDK, download, install, or enable workflow yet. |
| Automation Studio, media, and model orchestration | **Planned** | These are separately reviewed product directions, not hidden runtime features. |

Use the tools you already trust inside a normal Automexia terminal:

~~~text
ssh my-host
kubectl get pods
aws sts get-caller-identity
az account show
gcloud config list
~~~

Those commands remain owned by your shell and installed tools. Read
[Connection Hub and SSH](docs/user-guide/connection-hub-and-ssh.md),
[Extensions](docs/EXTENSIONS.md), and
[the feature catalog](docs/FEATURES.md) before relying on any gated or planned
capability.

## Built for visible control

Automexia is deliberately useful without an account, a cloud provider, an AI
model, or a paid service.

- Commands stay direct; suggestions and review surfaces do not execute them.
- Local previews stay local.
- Configuration updates are transactional: an invalid change does not replace a
  working setup.
- Credentials stay with the platform or external tool that owns them whenever
  possible.
- Optional capabilities start with the minimum authority, remain bounded and
  cancellable, and must show what they are about to do.

Read the [security policy](SECURITY.md), [architecture](docs/ARCHITECTURE.md),
and [build, wrap, or adopt decision guide](docs/BUILD-WRAP-ADOPT-ARCHITECTURE.md)
for the complete trust model.

## Explore the product

| If you want to… | Start here |
|---|---|
| Install or run Automexia | [Installation](docs/INSTALLATION.md) |
| Learn the workspace in one sitting | [Getting started](docs/GETTING-STARTED.md) and the [15-minute tour](docs/user-guide/index.md#a-15-minute-tour) |
| Organize tabs, panes, and sessions | [Workspace guide](docs/user-guide/workspace.md) |
| Use commands, shells, completion, or Quick Actions | [Commands and shell workflows](docs/user-guide/commands-and-shell.md) |
| Customize appearance and behavior | [Configuration](docs/CONFIGURATION.md) |
| Look up a command-line option or shortcut | [CLI reference](docs/CLI-REFERENCE.md) and [keyboard reference](docs/KEYBOARD.md) |
| Understand feature maturity and external release gates | [Feature catalog](docs/FEATURES.md) and [platform support](docs/PLATFORMS.md) |
| Test every implemented feature manually | [Manual testing guide](docs/MANUAL-FEATURE-TESTING.md) |
| Understand where the product is heading | [Product vision](docs/PRODUCT-VISION.md) and [roadmap](docs/ROADMAP.md) |

The [documentation home](docs/index.md) is the complete reading map.

## Contribute

Automexia welcomes focused improvements to the terminal, documentation, and
supporting tools. Before opening a pull request, read
[Contributing](CONTRIBUTING.md), follow the repository workflow in
[AGENTS.md](AGENTS.md), and run the complete non-launching gate:

~~~text
cargo ready
~~~

Please report vulnerabilities privately through the process in
[SECURITY.md](SECURITY.md). For support and project governance, see
[SUPPORT.md](SUPPORT.md) and [GOVERNANCE.md](GOVERNANCE.md).

## License and upstream history

Automexia Terminal is available under the [MIT License](LICENSE). The repository
preserves Rio's Git history and copyright notices; see
[NOTICE.md](NOTICE.md) and [UPSTREAM.md](UPSTREAM.md) for the exact upstream
relationship and port policy.
