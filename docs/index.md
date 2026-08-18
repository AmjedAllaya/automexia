# Automexia Terminal documentation

This documentation is organized around **reader intent**, not implementation phases. New and everyday users should begin with the [Automexia User Guide](user-guide/index.md); use reference pages for exact syntax, developer pages for architecture and assurance, and project pages only for future work and decision history.

> **Status rule:** current behavior says **Available now**; repository features that exist but still need release evidence say **Implemented locally / release-gated**; non-authoritative foundations say **Implemented internally, not activated**; future work says **Planned**. Roadmap status never overrides user-facing instructions.

## Start here

| I want to… | Read |
|---|---|
| **Learn how to use Automexia** | **[Complete User Guide](user-guide/index.md)** |
| Launch a session, directory, or shell | [Start and launch sessions](user-guide/start-and-launch.md) |
| Decide between windows, tabs, local tabs, fresh splits, and cloned splits | [Workspaces, tabs, and panes](user-guide/workspace.md) |
| Learn command lines and which command surface to use | [Commands and shell workflows](user-guide/commands-and-shell.md) |
| Learn the practical keyboard/mouse controls | [Shortcuts and input](user-guide/shortcuts.md) |
| Use completion, Quick Actions, aliases, and workspace tasks | [Command productivity](user-guide/productivity.md) |
| Customize shell, font, window, theme, navigation, and bindings | [Configuration and customization](user-guide/customization.md) |
| Configure SSH or find Connection Hub | **[Connection Hub and SSH](user-guide/connection-hub-and-ssh.md)** |
| Work with images, listings, semantic output, or WSL | [Files and images](user-guide/files-and-images.md), [Remote and WSL](user-guide/remote-and-wsl.md) |
| Follow practical development/operations examples | [Workflow recipes](user-guide/recipes.md) |
| Build Automexia from source | [Contributor getting started](guide/getting-started.md) |
| Diagnose a problem | [Troubleshooting](guide/troubleshooting.md) |
| Look up exact CLI syntax | [CLI reference](reference/cli.md) |
| Look up every shortcut/action | [Keyboard reference](reference/keyboard.md) |
| Look up every config key/default | [Configuration reference](reference/configuration.md) |
| Understand the technical design | [Architecture](developer/architecture.md) |
| Run verification or understand release gates | [Testing and release](developer/testing-release.md) |
| See future work / decision history | [Roadmap](project/roadmap.md), [Decision index](project/decisions.md) |

## What is available today

Automexia v0.4 is a standalone hardware-accelerated terminal for Windows, Linux, and macOS. Its shipped product surface includes the VT/PTY terminal core, tabs and split panes, renderer-owned operational chrome, shell/context integration, icon-aware listings, local and protocol image rendering, TOML configuration with last-known-good reload, and non-destructive Rio migration.

The repository also contains substantial v0.5 foundations. Native shell completion and the CP2/CP3 Quick Action/alias pipeline are implemented locally, but some stable-release claims still depend on hosted native, accessibility, and performance evidence. OpenSSH inventory and Connection Hub planning models exist with process/network authority deliberately disabled. Managed SSH, multi-cloud provider authentication, public extensions, and AI execution are **not** shipped v0.4 behavior.

| Area | Product status | Where to read |
|---|---|---|
| Terminal core, panes, tabs, selection, prompt context | **Available now** | [Terminal experience](guide/terminal-experience.md) |
| Configuration, themes, key bindings | **Available now** | [Configuration](reference/configuration.md), [Keyboard](reference/keyboard.md) |
| Session-only shell integration | **Available now** | [Shell and command productivity](guide/shell-productivity.md) |
| Native shell completion | **Implemented locally; release evidence still gated** | [Shell and command productivity](guide/shell-productivity.md) |
| Typed Quick Actions and opt-in aliases | **Implemented locally; release evidence still gated** | [Shell and command productivity](guide/shell-productivity.md) |
| Static OpenSSH inventory | **Implemented internally, not activated as managed SSH** | [Remote connections](guide/remote-connections.md) |
| Connection Hub records/review/dry-run models | **Implemented internally, authority disabled** | [Remote connections](guide/remote-connections.md) |
| Managed OpenSSH sessions | **Planned** | [Remote connections](guide/remote-connections.md), [Roadmap](project/roadmap.md) |
| Multi-cloud/provider adapters | **Planned** | [Roadmap](project/roadmap.md) |
| Public extension SDK / sandbox / AI execution | **Deferred** | [Roadmap](project/roadmap.md) |

## Documentation model

The set intentionally separates five kinds of information:

- **User Guide** pages teach practical use, choices, commands, shortcuts, and end-to-end workflows.
- **Guide** pages explain deeper product behavior and specialized workflows without becoming exact schema tables.
- **Reference** pages contain exact settings, bindings, commands, defaults, and limits.
- **Developer** pages explain architecture, security boundaries, testing, and release trust.
- **Project** pages contain the roadmap and Architecture Decision Records (ADRs). They are not instructions for current product behavior.

The original documentation mixed these roles heavily. The [source consolidation map](project/source-map.md) shows where every previous page was merged and which page is now canonical.
