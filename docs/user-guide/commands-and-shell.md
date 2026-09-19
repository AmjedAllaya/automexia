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

Opening or closing a split must leave editable text beside its prompt. On
Windows, the first input immediately after a pane/window size change may wait
up to 50 ms for the native editor to settle; normal typing is not delayed.
No refresh key or command is sent to the shell, and your shell profile is not
modified. Linux/macOS keep their native PTY resize behavior.

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

## Search from the command line

Enter `amx google rust async tutorial` to open a Google search in your default
browser. [Google search](google-search.md) explains quoting, offline preview,
session helper availability and privacy.

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

Choose a category with the arrows and Enter, or click it. The first command is
selected; the Back row is one Up away. The fixed header Back button stays
available while scrolling or searching. Alt+Left also returns to the category list.
Type any command or category name to search globally without drilling down.
Clear the query to resume browsing. Esc closes the palette. See
[palette controls](../KEYBOARD.md#command-palette) for paging and mouse scrolling.

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

## Readable output rows

Completed command highlights use small gaps between row bands instead of one
solid coloured block. This applies to Kubernetes, container and other tabular
output, as well as ordinary multi-line results. The existing success/failure
colours and completion pulse are retained; reduced motion keeps the bands static.

These are visual gaps, not extra blank lines. Column positions, wrapped text,
selection, copy, search and shell cursor movement still use the original terminal
grid. The gap scales with cell height (normally 2–3 logical pixels, capped at 4), and the
larger gap before the next prompt is unchanged. Output without supported command
boundaries and full-screen applications keep their normal terminal behavior.
For more space between the text itself, the existing `line-height` setting applies
uniformly to the terminal; see [configuration](../reference/configuration.md).

## Command boundaries and pane borders

After a marked command completes, a short inset accent separates its output
from the next prompt. The accent is at most 48 logical pixels and shrinks in
narrow panes; it is not a draggable pane border. Continuous pane dividers remain
unchanged. The timestamp, duration and status symbol stay on their existing row.
Output shading, row gaps and the single completion pulse are unchanged, including
reduced-motion behavior. No empty terminal rows are inserted and copying output
does not include this decoration. Very small or invalid geometry omits the accent.

## Operational status colours

When Automexia DevOps is enabled, supported plain-text status rows use the
active terminal palette. The original words and columns are not rewritten.

| Colour | Meaning | Examples |
|---|---|---|
| Cyan | Finished, stopped cleanly, informational or running without known health | `0/1 Completed`, `Succeeded`, `Exited (0)`, `Up 2 minutes` without a health result |
| Green | Known full readiness/health or a successful command summary | `1/1 Running`, `(healthy)`, `Ready True`, `Apply complete!` |
| Amber | Partial/unknown readiness, waiting, paused or transitional | `0/1 Running`, `Pending`, `Terminating`, `NotReady`, `(Paused)`, `(health: starting)` |
| Red | Explicit failure or adverse condition | `CrashLoopBackOff`, `Init:Error`, `OOMKilled`, `Exited (137)`, `DiskPressure True` |
| Blue | Verbose diagnostics, not a health result | Declared `DEBUG` or `TRACE` log levels |

`0/1 Completed` can be normal for a job that finished successfully; it is not a
ready running service. Read the status text as well as its colour. The command
completion backdrop still describes the command's exit code: a successful
`kubectl get` does not mean every returned resource is healthy.

The recognizer covers Kubernetes/oc pod rows (including `pod/name` kind prefixes),
common container-list statuses, condition booleans, log levels and existing
build/error summaries.
Unknown/custom or wrapped fragments may remain unclassified. Applications with
explicit colours keep them; Automexia does not override `kubecolor` or a tool's
own ANSI palette. Bare zero-error counts are not treated as log-level prefixes.
Disabling the DevOps extension leaves the original output.
See the [colour decision](../adr/0052-truthful-operational-status-colours.md).

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
