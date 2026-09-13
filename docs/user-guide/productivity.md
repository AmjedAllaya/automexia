# Productivity

Automexia's public productivity model is deliberately simple: the shell owns
commands, while the terminal provides fast navigation, search, selection,
clipboard, tabs, panes, a command palette, and optional session-local metadata.

## Work quickly without changing command ownership

- Use the real shell for programs, scripts, pipelines, redirects, aliases, and
  completion.
- Use tabs and panes for independent sessions.
- Use scoped search to find visible or retained terminal text.
- Use command-boundary navigation to move through retained output.
- Use selection, copy, and paste explicitly; paste never adds Enter.
- Use the command palette to discover terminal actions.
- Use native aliases for short, stable commands.

## Prompt and command metadata

Supported session-local shell integration can mark prompt and command
boundaries, directory, status, and duration. Metadata is bounded, pane-scoped,
and generation-aware. Automexia does not scrape commands from terminal cells or
silently persist shell history.

## Search and navigation

Choose the documented pane or visible-workspace scope. Search updates are
cancellable and stale results are discarded. Closing search restores prior
focus and never sends its keystrokes to the PTY.

Previous/next-command navigation moves only the viewport. It does not edit or
rerun a command.

## Keyboard-first use

Use documented shortcuts for frequent actions and the command palette for
discoverability. All public workflows remain reachable without a pointer and
retain visible focus, high contrast, reduced motion, and predictable dismissal.

## Aliases

User shell aliases remain authoritative. Any Automexia-generated alias file is
previewed, collision-reviewed, explicit to activate, reversible, and removable.
The shell executes it only after the user invokes and submits the alias.

## Safe fallback

If shell integration, metadata, search, or optional alias support fails, the
ordinary local terminal and native shell remain available. Disabling integration
restores normal shell behavior.

See [Commands and shell workflows](commands-and-shell.md),
[Shell productivity](../guide/shell-productivity.md), and
[Keyboard shortcuts](shortcuts.md).

Unreleased advanced productivity features and commercial plans are private.
