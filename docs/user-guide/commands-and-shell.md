# Commands and shell workflows

Normal commands belong to the real shell. Automexia commands control the
terminal around it.

## Command ownership

| Layer | Owner | Use it for |
|---|---|---|
| Shell command line | PowerShell, CMD, Bash, Zsh, Fish, or another shell | Programs, scripts, pipes, redirects, aliases, environment expansion |
| Automexia CLI | Automexia | Launch directory, shell choice, logging, and shell-integration maintenance |
| Command palette | Automexia | Discoverable terminal, window, tab, pane, search, and appearance actions |
| Repository commands | Cargo and repository tools | Building, testing, packaging, and contributing to Automexia |

Automexia does not reconstruct shell commands from rendered cells. The shell
owns quoting, history, cursor movement, expansion, pipelines, completion, and
execution.

## Run ordinary commands

Inside a pane, use tools exactly as in that shell:

```text
git status
cargo test
ssh example-host
```

The same rule applies to Python, FFmpeg, database clients, build tools, and
project scripts. If an expression uses pipes, redirects, globs, command
substitution, functions, or environment expansion, enter it in the shell.

## Launch-time CLI

The installed syntax is:

```text
automexia [OPTIONS] [COMMAND]
```

Use documented options to choose a working directory, shell, configuration, or
logging behavior. Arguments after an explicit executable belong to that
executable, not to Automexia. Prefer typed program plus argument arrays in
shortcuts and automation.

## Command palette

Open the palette with the documented platform shortcut. Use it when an action is
infrequent, its shortcut is unknown, or compact layout hides a control.

The palette and shortcuts invoke the same application action. While the palette
is open, its keystrokes do not reach the PTY. Closing it restores the prior
focus target.

## Session-only shell integration

Automexia can provide bounded prompt lifecycle metadata, path/status
presentation, command-boundary navigation, and object-preserving enhanced
listings while hosting the real shell.

Normal launch uses session-only integration. Persistent profile changes are not
required for ordinary Automexia sessions.

Use the documented maintenance commands to inspect, preview, install, repair, or
remove only Automexia-owned persistent integration. Automexia does not bypass
PowerShell execution policy or other operating-system policy.

Choose persistent installation only for an explicit need such as nested shells
outside normal Automexia launch. Session-only behavior is the preferred default.

## Jump between completed commands

Use the documented previous/next-command shortcuts or palette actions to move
the selected pane between retained semantic command boundaries.

This is viewport navigation, not shell history. It does not alter the editable
line, submit input, rerun a command, or move another pane. A shell that emits no
supported prompt markers remains unchanged rather than being parsed
heuristically.

## Search, selection, and clipboard

Search operates on the selected pane or documented visible-workspace scope.
Selection and copy are explicit. Paste never adds Enter. Clipboard operations
and overlays remain bound to the focused route.

See [Keyboard](../KEYBOARD.md) and
[Productivity](productivity.md).

## Aliases

Use native shell aliases for short, stable commands. Existing user aliases and
profiles remain user-owned.

If Automexia generates a supported alias file, preview the exact content and
destination first. Activation, collision decisions, rollback, disable, and
removal are explicit. The generated file remains inert until the user invokes
an alias in the native shell.

See [Shell aliases](../DEVOPS-ALIASES.md).

## Contributor commands

Repository commands are for developing Automexia, not for ordinary terminal
work:

| Command | Purpose |
|---|---|
| `cargo ready` | Run the contributor gate without launching |
| `cargo ci` | Run the repository CI alias |
| `cargo qa` | Run deeper assurance checks |
| `cargo storage` | Inspect repository build storage |
| `cargo purge` | Remove verified repository build artifacts |
| `cargo xtask doctor` | Inspect contributor prerequisites |

Use the current [Contributor CLI](../CLI-REFERENCE.md) as the authority for exact
commands and platform limitations.

## Decision guide

1. For a normal tool or shell command, type it in the shell.
2. For a window, tab, pane, search, or appearance action, use a shortcut or the
   command palette.
3. For launch-time behavior, invoke the Automexia CLI from the calling shell or
   shortcut.
4. For a frequent stable shell command, use a native alias.
5. For Automexia development, use the repository contributor commands.

Advanced unreleased command products, integrations, and commercial workflows
are private and are not described by this guide.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Provider Aware Quick Actions

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.
