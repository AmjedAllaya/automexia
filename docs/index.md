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
| Review the exact Quick Action and alias architecture | [DevOps alias specification](DEVOPS-ALIASES.md) |
| Customize shell, font, window, theme, navigation, and bindings | [Configuration and customization](user-guide/customization.md) |
| Configure SSH or find Connection Hub | **[Connection Hub and SSH](user-guide/connection-hub-and-ssh.md)** |
| Work with images, listings, semantic output, or WSL | [Files and images](user-guide/files-and-images.md), [Remote and WSL](user-guide/remote-and-wsl.md) |
| Follow practical development/operations examples | [Workflow recipes](user-guide/recipes.md) |
| Build Automexia from source | [Contributor getting started](guide/getting-started.md) |
| Diagnose a problem | [Troubleshooting](guide/troubleshooting.md) |
| Look up exact CLI syntax | [CLI reference](reference/cli.md) |
| Look up every shortcut/action | [Keyboard reference](reference/keyboard.md) |
| Configure or inspect Ghostty-compatible bindings | [Ghostty compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md), [generated bindings](generated/ghostty-1.3-keybindings.md), [generated actions](generated/ghostty-1.3-actions.md) |
| Look up every config key/default | [Configuration reference](reference/configuration.md) |
| Understand the technical design | [Architecture](developer/architecture.md) |
| Run verification or understand release gates | [Testing and release](developer/testing-release.md) |
| Review stabilization/S2 status and external evidence | [Stabilization roadmap](STABILIZATION-ROADMAP.md), [S1/S2 implementation audit](research/S1-S2-IMPLEMENTATION-AUDIT.md) |
| See future work / decision history | [Roadmap](project/roadmap.md), [Decision index](project/decisions.md) |
| Follow the detailed SSH and multi-cloud implementation phases | [SSH, connectivity, multi-environment, and multi-cloud plan](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md), [provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |

## What is available today

Automexia v0.4 is a standalone hardware-accelerated terminal for Windows, Linux, and macOS. Its shipped product surface includes the VT/PTY terminal core, tabs and split panes, renderer-owned operational chrome, shell/context integration, icon-aware listings, local and protocol image rendering, TOML configuration with last-known-good reload, and non-destructive Rio migration.

The repository also contains substantial v0.5 foundations. Native shell completion and the CP2/CP3 Quick Action/alias pipeline are implemented locally, while S1 visual/model tooling and the S2 release ratchet are source-complete at their boundaries. Stable release still depends on hosted native/accessibility evidence, approved visual matrices, and 30 consecutive controlled performance days. The v0.5 read-only Connection Hub is implemented locally with process/network authority deliberately disabled; native macOS/Linux and controlled accessibility evidence remain release-gated. Managed SSH, multi-cloud provider authentication, public extensions, and AI execution are **not** shipped v0.4 behavior.

| Area | Product status | Where to read |
|---|---|---|
| Terminal core, panes, tabs, selection, prompt context | **Available now** | [Terminal experience](guide/terminal-experience.md) |
| Configuration, themes, key bindings | **Available now** | [Configuration](reference/configuration.md), [Keyboard](reference/keyboard.md) |
| Session-only shell integration | **Available now** | [Shell and command productivity](guide/shell-productivity.md) |
| Native shell completion | **Implemented locally; release evidence still gated** | [Shell and command productivity](guide/shell-productivity.md) |
| Typed Quick Actions and opt-in aliases | **Implemented locally; release evidence still gated** | [Shell and command productivity](guide/shell-productivity.md) |
| Provider-aware Quick Actions (CP4) | **Source-complete internally; provider publication/execution not activated** | [Commands and shell](user-guide/commands-and-shell.md#provider-aware-quick-actions), [CP4 audit](PROVIDER-AWARE-QUICK-ACTIONS-IMPLEMENTATION-AUDIT.md), [testing](TESTING.md#m13-provider-aware-quick-actions) |
| Optional Automexia autocomplete surface | **Proposal and threat contract complete; runtime not authorized** | [CP5 implementation audit](research/CP51-CP56-IMPLEMENTATION-AUDIT.md), [proposed ADR 0025](adr/0025-authenticated-native-editor-suggestion-bridge.md), [CP5.0 research](research/CP5-AUTOCOMPLETE-RESEARCH.md) |
| Static OpenSSH inventory, read-only Hub, and disabled direct-review preparation | **Implemented locally; release evidence gated; no launch authority** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md) |
| Connection Hub records, pending selected-host review, and dry-run models | **Implemented locally; authority disabled** | [Remote connections](guide/remote-connections.md) |
| Recipes and multi-environment workspace review (M6) | **Implemented internally; product activation/execution disabled** | [Connection automation](SSH-CONNECTION-AUTOMATION.md#m6-review-only-implementation), [M6 roadmap](project/roadmap.md) |
| Managed OpenSSH sessions | **Planned** | [Remote connections](guide/remote-connections.md), [Roadmap](project/roadmap.md) |
| AWS provider source contracts | **Implemented internally, not activated** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md), [M8 testing](TESTING.md#m8-aws-adapter-source-contracts) |
| Azure provider source contracts | **Implemented internally, not activated** | [Connection Hub and SSH](user-guide/connection-hub-and-ssh.md), [M9 testing](TESTING.md#m9-azure-adapter-source-contracts) |
| Google Cloud provider source contracts | **Implemented internally, not activated** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m10-google-cloud-adapter-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Kubernetes/OpenShift source contracts | **Implemented internally, not activated** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m11-kubernetes-and-openshift-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Teleport adapter / OpenBao adapter | **Teleport implemented internally, not activated / OpenBao blocked on ADR 0024** | [Provider testing](MULTI-CLOUD-PROVIDERS-TESTING.md#m12-teleport-source-contracts), [Provider audit](MULTI-CLOUD-PROVIDERS-IMPLEMENTATION-AUDIT.md) |
| Public extension SDK / sandbox / AI execution | **Deferred** | [Roadmap](project/roadmap.md) |

## Documentation model

The set intentionally separates five kinds of information:

- **User Guide** pages teach practical use, choices, commands, shortcuts, and end-to-end workflows.
- **Guide** pages explain deeper product behavior and specialized workflows without becoming exact schema tables.
- **Reference** pages contain exact settings, bindings, commands, defaults, and limits.
- **Developer** pages explain architecture, security boundaries, testing, and release trust.
- **Project** pages contain the roadmap and Architecture Decision Records (ADRs). They are not instructions for current product behavior.

The original documentation mixed these roles heavily. The [source consolidation map](project/source-map.md) shows where every previous page was merged and which page is now canonical.

## CP3.3 evidence

CP3.3 native imports and trusted workspace task bridges are part of the reviewed command-productivity architecture contract.
