# Shell productivity guide

Automexia improves the terminal around a command without replacing the shell.
PowerShell/PSReadLine, CMD, Bash/Readline, Zsh/ZLE, and Fish remain responsible
for editing, history, quoting, completion, aliases, expansion, and execution.

## Public capabilities

| Capability | Boundary |
|---|---|
| Session-local shell integration | Normal launch does not need to rewrite shell profiles |
| Prompt and command-boundary metadata | Bounded, pane-scoped, and treated as untrusted |
| Native shell completion | The shell owns its buffer, cursor, menu, and history |
| Command navigation | Moves the viewport only; never edits or submits input |
| Search and palette | Bounded overlays with PTY input isolation and focus restoration |
| Native aliases | User-owned; any generated file is previewed, explicit, reversible, and inert |
| Object-preserving listings | Interactive decoration does not change piped objects or bytes |

## Session-local by default

Packaged Automexia loads supported integration only into the child shell it
starts. A normal application launch does not install a profile hook, change
execution policy, provision WSL, or create persistent shell files.

Missing or invalid integration resources degrade to the unmodified shell.
Repeated sourcing is guarded so prompt hooks do not stack.

Persistent installation is optional and intended only for an explicit need such
as nested shells outside normal Automexia launch. The maintenance commands
provide read-only diagnosis, preview, install/repair, and removal of only
Automexia-owned content. Operating-system and enterprise policy remain
authoritative.

## Prompt metadata

Supported shells may emit semantic command boundaries, current directory,
status, duration, and other bounded public context. The selected session and
generation own the metadata. Stale or cross-pane updates are rejected.

Automexia does not recover commands by scraping rendered cells and does not
silently persist native history files.

## Listings

Enhanced interactive listings may add icons or categories, but pipelines keep
their native objects and bytes. Explicit native commands remain available.

If formatting resources are unavailable, listings fall back to ordinary shell
behavior rather than altering global configuration.

## Completion and insertion

Native completion is the fallback and authority. Any public Automexia text
offered for insertion or copy requires an explicit user action, stays bound to
the focused route, and never adds Enter.

No ordinary completion or insertion path may read credentials, hidden history,
unrelated panes, clipboard content, or network state.

## Aliases

Existing aliases and shell profiles are never overwritten automatically. A
generated alias file, where supported, is deterministic, bounded,
collision-reviewed, explicitly activated, reversible, and removable.

The shell expands and executes an alias only after the user invokes and submits
it.

## Failure and removal

Disabling integration restores ordinary shell behavior. Removal deletes only
Automexia-owned integration blocks/files after exact review. Terminal startup
and local shell use remain available if optional integration fails.

## Verification

Test supported shells with empty, quoted, Unicode, multiline, malformed, and
large inputs; integration enabled/disabled/missing; repeated startup; profile
collision; rollback/removal; route and generation changes; overlay focus;
accessibility; resource bounds; and final cleanup.

See [Shell integration](../SHELL-INTEGRATION.md),
[Shell productivity](../COMMAND-PRODUCTIVITY.md), and
[Commands and shell workflows](../user-guide/commands-and-shell.md).

Unreleased advanced command features and commercial plans are maintained
privately and are not public commitments.
