# Automexia User Guide

This category is the practical manual for **using Automexia as a terminal**. It is organized around tasks and choices: how to launch a session, when to use a tab versus a split, which command surface to use, how shortcuts behave, how to customize the terminal, and how to work with local or remote tools.

The User Guide intentionally does not replace the exact reference pages. A guide explains **what to do and why**; a reference page remains the authority for every supported option, key binding, setting, default, and limit.

## Feature status legend

Automexia contains shipped features and work that exists only in local/development builds. User instructions in this category use these labels consistently:

- **Available now** — part of the current v0.4 user-facing terminal behavior.
- **Implemented locally / release-gated** — code exists in the repository, but it should not be assumed to be present in every stable package until the remaining release evidence is complete.
- **Implemented internally, not activated** — supporting models exist, but there is intentionally no user authority to perform the final operation yet.
- **Planned** — roadmap work; do not treat it as a current command or workflow.

## Start with the task you have

| I want to… | Read |
|---|---|
| Launch Automexia, choose a directory or shell, and understand the first window | [Start and launch sessions](start-and-launch.md) |
| Organize work using windows, window tabs, panes, and pane-local tabs | [Workspaces, tabs, and panes](workspace.md) |
| Understand which command layer to use | [Commands and shell workflows](commands-and-shell.md) |
| Learn the everyday keyboard and mouse controls | [Shortcuts and input](shortcuts.md) |
| Use completion, Quick Actions, aliases, or workspace tasks | [Command productivity](productivity.md) |
| Change the shell, font, window, theme, navigation, or bindings | [Configuration and customization](customization.md) |
| Work with listings, semantic output, selection, and image previews | [Files, output, and images](files-and-images.md) |
| Review SSH inventory in the read-only Hub or connect with system OpenSSH | **[Connection Hub and SSH](connection-hub-and-ssh.md)** |
| Use WSL or combine local and remote sessions | [Remote sessions and WSL](remote-and-wsl.md) |
| Copy a practical setup for common development/operations workflows | [Workflow recipes](recipes.md) |
| Fix a problem | [Troubleshooting](../guide/troubleshooting.md) |

## A 15-minute tour

If you are new to Automexia, this sequence covers the core mental model without requiring you to read the whole documentation set.

1. **Open one session.** Start `automexia`, or use `cargo dev` from a source checkout. Confirm that the tab title identifies the real shell and that the prompt context/path appears before input.
2. **Create a window-level tab.** Use `Ctrl+T` on Windows/Linux/BSD or `Cmd+T` on macOS. Use this for another top-level workspace.
3. **Create a fresh split.** Use `Ctrl+Shift+R` / `Ctrl+Shift+D` on Windows/Linux/BSD or `Cmd+D` / `Cmd+Shift+D` on macOS. A fresh split starts the default shell.
4. **Clone the current launch context.** Use `Ctrl+R` / `Ctrl+D`. This is useful when the second pane should begin with the same shell/profile/directory as the active pane.
5. **Move between panes.** Use `Alt+Arrow` on Windows/Linux/BSD or `Cmd+Alt+Arrow` on macOS.
6. **Open the command palette.** Use `Ctrl+Shift+P` or `Cmd+Shift+P`. It is the discoverable alternative when you do not remember a shortcut.
7. **Try terminal selection.** Hold `Shift` and press an Arrow key. `Ctrl+C` copies a non-empty selection; with no selection it still sends the shell interrupt.
8. **Search scrollback.** Use `Ctrl+Shift+F` on Windows/Linux/BSD or `Cmd+F` on macOS.
9. **Create a starter configuration.** Run `automexia --write-config`, then change only the settings you actually need.
10. **Use your normal tools normally.** `git`, `cargo`, `kubectl`, `ssh`, editors, TUIs, REPLs, and shell scripts still run inside the real shell. Automexia does not replace their command syntax.

## The five command surfaces

Users often confuse terminal commands with Automexia commands. The distinction is important:

| Surface | Best for | Example |
|---|---|---|
| **Your shell** | Normal work and third-party tools | `git status`, `cargo test`, `ssh host` |
| **Automexia application CLI** | Launch-time choices and Automexia-owned maintenance | `automexia --working-dir D:\\work`, `automexia --write-config` |
| **Command palette** | Discovering UI actions without memorizing bindings | Open palette, search for split/config/image actions |
| **Quick Actions / aliases** | Reviewed repeatable command templates *(release-gated)* | Search → review → insert/copy; optionally publish an alias |
| **Repository Cargo/xtask commands** | Building, testing, and running Automexia from source | `cargo dev`, `cargo automexia`, `cargo ready` |

See [Commands and shell workflows](commands-and-shell.md) for the decision guide.

## The workspace model in one picture

Automexia has more than one level of organization:

```text
OS window
└── window-level tab
    └── split layout
        ├── pane A
        │   ├── local tab A1
        │   └── local tab A2
        └── pane B
            └── local tab B1
```

A **window-level tab** changes the whole workspace. A **split** shows sessions side-by-side. A **pane-local tab** swaps only the session inside one pane without changing its siblings. A separate **OS window** is independent from the others.

Read [Workspaces, tabs, and panes](workspace.md) before creating complex layouts; it explains which level is appropriate for each kind of work.

## Where exact information lives

Use the User Guide while learning and working. Jump to these canonical references when you need exact syntax:

- [CLI and automation reference](../reference/cli.md) — every application and repository command.
- [Keyboard and input reference](../reference/keyboard.md) — complete platform defaults and custom action names.
- [Configuration reference](../reference/configuration.md) — every supported setting and default.
- [Terminal experience](../guide/terminal-experience.md) — detailed behavior of the renderer, prompt context, footer, images, and semantic presentation.
- [Shell integration and command productivity](../guide/shell-productivity.md) — implementation/status boundaries for completion, Quick Actions, aliases, and trusted workspace tasks.
- [Remote connections](../guide/remote-connections.md) — current-vs-planned SSH/Connection Hub boundary.

## Recommended reading paths

**Everyday terminal user:** Start and launch → Workspace → Shortcuts → Customization.

**Developer using Automexia as a daily terminal:** Start and launch → Workspace → Commands → Productivity → Recipes.

**DevOps / remote user:** Commands → Workspace → Connection Hub and SSH → Remote sessions and WSL → Productivity → Recipes.

**Contributor to Automexia itself:** User Guide first, then [Getting started for source contributors](../guide/getting-started.md), [Architecture](../developer/architecture.md), and [Testing and release](../developer/testing-release.md).
