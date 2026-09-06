# Automexia Terminal documentation

This public documentation covers current open-source terminal behavior and
ordinary free terminal capabilities. It intentionally excludes advanced future
features and commercial strategy.

## Start here

| Goal | Page |
|---|---|
| Install and launch | [Installation](INSTALLATION.md) |
| Learn the basics | [Getting started](GETTING-STARTED.md) |
| Follow the complete user guide | [User guide](user-guide/index.md) |
| Check exact feature status | [Features](FEATURES.md) |
| Find a common answer | [FAQ](FAQ.md) |
| Fix a problem | [Troubleshooting](TROUBLESHOOTING.md) |

## Use the terminal

| Goal | Page |
|---|---|
| Windows, tabs, panes, and sessions | [Workspace](user-guide/workspace.md) |
| Commands, shells, completion, and aliases | [Commands and shell](user-guide/commands-and-shell.md) |
| Search and productivity | [Productivity](user-guide/productivity.md) |
| Files and images | [Files and images](user-guide/files-and-images.md) |
| Remote tools and WSL | [Remote and WSL](user-guide/remote-and-wsl.md) |
| System OpenSSH and explicit inventory | [OpenSSH and remote shells](user-guide/connection-hub-and-ssh.md) |
| Appearance and behavior | [Customization](user-guide/customization.md) |
| Shortcuts | [Shortcuts](user-guide/shortcuts.md) |

## Reference

| Subject | Page |
|---|---|
| Configuration | [Configuration](CONFIGURATION.md), [compact reference](reference/configuration.md) |
| Keyboard | [Keyboard](KEYBOARD.md), [compact reference](reference/keyboard.md) |
| Command line | [CLI](CLI-REFERENCE.md), [compact reference](reference/cli.md) |
| Shell integration | [Shell integration](SHELL-INTEGRATION.md) |
| Image behavior | [Image previews](IMAGE-PREVIEWS.md) |
| Platforms | [Platform support](PLATFORMS.md) |
| Accessibility | [Accessibility](ACCESSIBILITY.md) |
| OpenSSH inventory | [SSH inventory](SSH-INVENTORY.md) |
| Shell productivity and aliases | [Shell productivity](COMMAND-PRODUCTIVITY.md), [aliases](DEVOPS-ALIASES.md) |

## Current status and assurance

| Subject | Page |
|---|---|
| Human-readable feature inventory | [Features](FEATURES.md) |
| Public implementation summary | [Phase audit](PHASE-IMPLEMENTATION-AUDIT.md) |
| Release readiness | [Readiness audit](READINESS-AUDIT.md) |
| Testing | [Testing](TESTING.md) |
| Manual workflows | [Manual feature testing](MANUAL-FEATURE-TESTING.md) |
| Release trust | [Release trust](RELEASE-TRUST.md) |
| Linux distribution | [Public release distribution](PUBLIC-RELEASE-DISTRIBUTION.md) |

## Contributor documentation

| Subject | Page |
|---|---|
| Current architecture | [Architecture](ARCHITECTURE.md), [developer summary](developer/architecture.md) |
| Required terminal fixes and their evidence | [Terminal maintenance requirements](TERMINAL-MAINTENANCE-REQUIREMENTS.md) |
| Keyboard-first usability and customization corrections | [Terminal interaction requirements](TERMINAL-INTERACTION-REQUIREMENTS.md) |
| Dependency placement | [Build, wrap, and adopt](BUILD-WRAP-ADOPT-ARCHITECTURE.md) |
| Feature ownership | [Feature ownership audit](FEATURE-OWNERSHIP-AUDIT.md) |
| Architecture decisions | [Decision index](DECISIONS.md) |
| Documentation rules | [Documentation guide](DOCUMENTATION.md) |
| Testing and release | [Developer testing](developer/testing-release.md) |
| Build storage and cache cleanup | [Development cache](DEVELOPMENT-CACHE.md) |
| Public/private boundary | [Private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md) |

## Public feature-status boundary

The source contains some disabled or release-gated foundations. Their public
pages document only current code, safety boundaries, and remaining evidence.
They are not promises of a future product.

No public page should infer an unreleased advanced feature, commercial product,
price, hosted service, specialist extension, or delivery plan from the absence
or presence of an internal source module.

## Documentation maintenance

When public behavior changes, update its guide, reference, architecture,
feature status, platform evidence, tests, and release notes together. When a
draft includes advanced future or commercial material, preserve it only in the
ignored private documentation workspace and remove public links to it.

## Command-productivity evidence status

Accepted source work partial overall: command-productivity foundations have
source and local assurance owners, while native and release evidence remains
explicitly gated. See [CP5.0 native autocomplete research](research/CP5-AUTOCOMPLETE-RESEARCH.md)
and the CP3.3 boundary in [DevOps aliases](DEVOPS-ALIASES.md).
