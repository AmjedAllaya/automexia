# Terminal recipes

These recipes use ordinary command-line tools and public terminal behavior.
Replace every fictional value with an authorized target appropriate to your own
environment.

## Open a project

1. Launch Automexia in the project directory or change directory in the shell.
2. Open a second pane for tests or logs.
3. Use pane-scoped search and command navigation to review output.
4. Keep long-running and interactive processes in separate sessions.

## Compare two commands

1. Split the workspace.
2. Run one command in each pane.
3. Use independent search and selection.
4. Close the temporary pane when finished.

Each pane owns an independent PTY; input and resize remain route-scoped.

## Use a native alias

1. Define the alias in the native shell using that shell's documented syntax.
2. Restart or reload the shell as required.
3. Invoke the alias without automatic submission by Automexia.
4. Remove it from the same user-owned configuration when no longer needed.

If using a supported generated alias file, preview, collision review,
activation, rollback, and removal are explicit.

## Connect with OpenSSH

```text
ssh alice@example.invalid
```

OpenSSH owns host keys, credentials, agents, configuration, authentication, and
networking. Automexia behaves as an ordinary terminal and never adds Enter to
paste.

Use a disposable loopback fixture for tests and never publish real connection
details.

## Process media with command-line tools

Run tools such as FFmpeg using their normal shell syntax. Automexia does not
replace the tool, parse its command language, or imply a separate editing
product.

Keep source media backed up, preview commands before running destructive
operations, and write outputs to a new file first.

## Inspect structured data

Use a native CLI, REPL, pager, or text-processing pipeline. Keep secrets out of
terminal recordings and screenshots. Use separate panes for an interactive
client and logs when useful.

## Recover from a failed command

1. Preserve the tool's original diagnostic.
2. Confirm the current pane, directory, shell, and command.
3. Search retained output for the first failure.
4. Correct the command in the shell.
5. Rerun only after reviewing the exact text.
6. Close temporary panes and clean up tool-owned processes or files.

These examples intentionally avoid unreleased Automexia workflows, services,
and commercial features.
