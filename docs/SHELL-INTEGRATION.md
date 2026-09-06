# Shell integration

Automexia integrates with supported shells without replacing their editor,
history, completion, quoting, aliases, pipelines, or execution.

## Session-only integration by default

Normal application launch loads bounded integration only into the child shell.
It does not need to rewrite shell profiles, change execution policy, provision a
WSL distribution, or create persistent files.

If integration resources are missing or invalid, Automexia starts the ordinary
shell and reports a bounded redacted diagnostic.

## Supported behavior

Session-local hooks may publish:

- prompt and command boundaries;
- current directory;
- command status and duration;
- bounded pane-scoped public context; and
- object-preserving interactive listing decoration.

Metadata is untrusted, size-limited, session/generation bound, and stale results
are rejected. Automexia does not scrape commands from rendered cells or silently
persist shell history.

## Shell ownership

### Prompt ownership

### Native command completion

PowerShell/PSReadLine, CMD, Bash/Readline, Zsh/ZLE, and Fish keep native command
editing and completion. Disabling Automexia integration restores ordinary shell
behavior.

### Icon-aware listings

Enhanced listings preserve native objects and bytes in pipelines. Explicit
native commands remain available.

## Persistent maintenance

Persistent integration is optional and intended only for an explicit need such
as nested shells outside normal Automexia launch.

```text
automexia shell-integration doctor
automexia shell-integration install [--force] [--quiet]
automexia shell-integration uninstall [--quiet]
```

`doctor` is read-only. Install and uninstall affect only exact
Automexia-owned content, use preview/recovery behavior where documented, and
honor operating-system and enterprise policy. Automexia never supplies an
execution-policy bypass.

## WSL and remote shells

WSL distributions and remote hosts own their shell configuration. Session-local
integration is scoped to the launched session and must not silently edit
distribution or remote profiles. Missing integration leaves the ordinary shell
usable.

## Security

Integration resources, prompt metadata, labels, paths, and shell output are
untrusted. Bound bytes/counts, reject hostile controls in UI labels, redact
diagnostics, avoid secret environment inheritance for helpers, and clean up
temporary resources.

No integration hook presses Enter, launches an unrelated process, reads
credentials, or contacts a network merely because the user types.

## Testing

Test each supported installed shell with integration enabled, disabled, missing,
repeatedly sourced, and removed; success/failure commands; directory and Git
changes; Unicode/quoting; pipelines; narrow layouts; restart; and cleanup.
Native shell behavior is the independent oracle.

See [Shell productivity](guide/shell-productivity.md) and
[Commands and shell](user-guide/commands-and-shell.md).

## CP3.1 alias integration boundary

CP3.1 generated aliases are optional native-shell artifacts, not a replacement
editor or parser. Native definitions win, activation is explicit, and removal
restores the unchanged native fallback. See
[DEVOPS-ALIASES.md](DEVOPS-ALIASES.md).
