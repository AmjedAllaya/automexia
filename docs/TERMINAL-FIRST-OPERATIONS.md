# Terminal-first operations

Automexia's public terminal-first model is:

1. use the real shell for commands, scripts, pipelines, aliases, and tools;
2. use tabs and panes for independent sessions;
3. use search, selection, clipboard, and command navigation to inspect output;
4. use the command palette for terminal UI actions; and
5. use system OpenSSH directly for authorized remote shells.

The shell and invoked tools retain command, credential, configuration,
authentication, and network ownership. Automexia owns terminal input/output,
routes, PTYs, rendering, and cleanup.

No public operation reconstructs commands from rendered cells, presses Enter
automatically, or performs background login/network work because the user types
or searches.

Advanced operational products and commercial workflows are private.
