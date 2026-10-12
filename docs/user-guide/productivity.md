# Productivity

Your shell controls commands. Automexia adds navigation, search, selection,
clipboard, tabs, panes, and a command palette.

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

Supported shell integration marks prompts and commands and reports directory,
status, and duration. Metadata has size limits and stays with its session;
updates from replaced sessions are ignored. Automexia does not reconstruct
commands from displayed text or silently save shell history.

## Search and navigation

Search one pane or the active sessions across visible panes. Results from an
outdated search are discarded. Closing search cancels it and restores focus;
search keystrokes never reach the shell. See
[Search controls](workspace.md#search-scrollback-without-changing-the-shell-command).

Previous/next-command navigation moves only the viewport. It does not edit or
rerun a command.

## Keyboard-first use

Use shortcuts for frequent actions and the palette to find others. Public
workflows remain keyboard-accessible, with visible focus, high contrast, reduced
motion, and predictable dismissal.

## Aliases

Your shell keeps its existing aliases. Generated aliases are previewed and
checked for naming conflicts before you explicitly activate them. You can undo
or remove them. An alias runs only when you enter and submit it in the shell.

## Safe fallback

If shell integration, metadata, search, or optional alias support fails, the
ordinary local terminal and native shell remain available. Disabling integration
restores normal shell behavior.

See [Commands and shell workflows](commands-and-shell.md),
[Shell productivity](../guide/shell-productivity.md), and
[Keyboard shortcuts](shortcuts.md).
