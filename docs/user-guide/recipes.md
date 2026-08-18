# Workflow recipes

These recipes combine the User Guide concepts into practical setups. They are examples, not a requirement to use Automexia in one fixed way.

## Recipe 1: Open a project and create a useful development layout

Start in the project directory:

```text
automexia --working-dir D:\work\my-project
```

or:

```text
automexia --working-dir ~/work/my-project
```

Then:

1. Keep the first pane for normal commands/editing launchers.
2. Clone right with `Ctrl+R` so the second pane begins from the same launch profile/directory.
3. Use the right pane for a server, watcher, REPL, or logs.
4. Add a fresh lower split with `Ctrl+Shift+D` on Windows/Linux/BSD or `Cmd+Shift+D` on macOS when you want an independent default-shell task runner.
5. Navigate panes with `Alt+Arrow` or `Cmd+Alt+Arrow`.

Why this works: cloning is ideal for “same project, another shell”; a fresh split is ideal for “another standard session.”

## Recipe 2: Separate projects without creating desktop clutter

Use window-level tabs:

1. Open project A.
2. Create another window-level tab with `Ctrl+T` / `Cmd+T`.
3. Change to project B in that tab.
4. Switch whole contexts with `Ctrl+Tab` / `Ctrl+Shift+Tab`.

Use separate OS windows instead when the projects belong on different monitors/desktops or you want independent minimize/close behavior.

## Recipe 3: Keep one pane stable while rotating utility sessions

Suppose the left pane is your main development shell and the right pane is your utility area.

1. Split right.
2. Focus the right pane.
3. Create pane-local tabs with `Ctrl+Shift+T` / `Cmd+Shift+T` for a database console, log view, and secondary shell.
4. Switch only those right-pane sessions using `Alt+PageUp/PageDown` or `Cmd+Alt+[` / `Cmd+Alt+]`.

The left pane remains visible and unchanged. This is exactly what pane-local tabs are for; using window tabs would swap too much of the workspace.

## Recipe 4: Search old output without changing the command line

Use terminal search rather than shell history:

- Windows/Linux/BSD: `Ctrl+Shift+F`
- macOS: `Cmd+F`

Type the text, press `Enter` for the next match and `Shift+Enter` for the previous. `Esc` cancels.

Use shell history (`Up`, `Ctrl+R`, etc.) when you want to retrieve **commands you typed**. Use Automexia search when you want to find **text that appeared in terminal scrollback**.

## Recipe 5: Copy terminal text without losing normal `Ctrl+C`

On Windows/Linux/BSD:

1. Select text with the mouse or `Shift+Arrow`.
2. Press `Ctrl+C` to copy it.
3. Clear/exit selection by typing or using an unmodified Arrow.
4. With no selection, `Ctrl+C` once again sends the normal interrupt to the shell/application.

If you prefer a dedicated copy chord, use `Ctrl+Shift+C`.

This design avoids forcing you to choose between terminal copy and shell interrupt globally.

## Recipe 6: Preview an image filename from output

If a command prints a local PNG/JPEG/WebP/etc. path:

**Fast mouse approach:** hover the filename, then click to pin.

**Keyboard approach:** select the path and press `Ctrl+Alt+I` or `Cmd+Alt+I`.

When pinned, use Arrow keys to move through other visible image paths and `Esc` to dismiss.

Use inline terminal graphics instead when an application itself is image-aware. Use an external app for PDF/SVG/editing/unsupported or remote resources.

## Recipe 7: Launch a dedicated REPL/tool directly

For a one-purpose terminal window, use `-e`:

```text
automexia --working-dir ~/work/project -e python
```

This is appropriate for a REPL or interactive program that is itself the session. If the command depends on shell syntax such as `|`, `>`, aliases, or command substitution, open the normal shell and type it there instead.

## Recipe 8: Make a temporary shell choice without changing config

Need PowerShell just for this session?

```text
automexia -e pwsh -NoLogo
```

Need Zsh for one launch?

```text
automexia -e zsh -l
```

If you keep doing this every day, move the choice into `[shell]` in `config.toml`. Temporary launch options should remain temporary; permanent preferences belong in config.

## Recipe 9: Diagnose shell integration before modifying profiles

If prompt context or integrated listings look wrong:

```text
automexia shell-integration doctor
```

Then check:

- whether the session is really running inside Automexia;
- whether `TERM_PROGRAM=Automexia` / the integration marker is present where expected;
- whether the packaged/repository integration resources are available;
- the relevant item in [Troubleshooting](../guide/troubleshooting.md).

Only after diagnosis should you consider an explicit persistent repair:

```text
automexia shell-integration install --force
```

Do not treat persistent installation as the default fix for every shell issue; normal launch is designed to be session-only.

## Recipe 10: Customize safely in small steps

1. Create a starter:

   ```text
   automexia --write-config
   ```

2. Open it with `Ctrl+,`, `Ctrl+Shift+,`, or `Cmd+,` depending on platform.
3. Change one thing—such as font size or shell.
4. Restart or use a configured reload action.
5. Verify the result.
6. Add another change only if it solves a real need.

If a config reload fails, Automexia keeps the last known-good runtime configuration, so fix the parse/theme error rather than layering more changes on top.

## Recipe 11: Create a repeatable reviewed command

> Requires a build exposing the **locally implemented / release-gated** Quick Action feature.

For a command you run occasionally but do not want to memorize:

1. Create/import one typed action definition.
2. Preview it:

   ```text
   automexia actions put ONE_ACTION.toml
   ```

3. Review the exact tokens, placeholders, risk/scope, and revision.
4. Apply with the returned expected revision:

   ```text
   automexia actions put ONE_ACTION.toml --apply --expected-revision N
   ```

5. Use the UI to find the action and choose **Insert** or **Copy**.
6. Review the command in the shell before running it.

This is better than a global alias while the workflow is still evolving.

## Recipe 12: Promote a mature Quick Action to a shell alias

> Requires a build exposing the **locally implemented / release-gated** alias feature.

First inspect health:

```text
automexia aliases doctor
```

Preview the alias:

```text
automexia aliases enable ACTION_ID --name NAME --shell powershell
```

Review collisions, source revision, generation digest, tool/completion health, and policy decisions. Then apply using the exact reviewed compare-and-swap values:

```text
automexia aliases enable ACTION_ID --name NAME --shell powershell --apply --expected-revision N --expected-generation DIGEST
```

Use an alias only when the action is stable, frequent, and truly belongs in that shell's global command namespace.

## Recipe 13: Local + SSH side by side

1. Start a local project shell.
2. Create a fresh split for a clean remote launcher.
3. In the second pane run:

   ```text
   ssh host-alias
   ```

4. Keep the first pane local for code/files/builds.
5. Use geometric pane navigation to move between local and remote.

OpenSSH remains the authority for keys, agents, host-key policy, proxy jumps, and config. Automexia's planned Connection Hub is not required to use SSH today.

## Recipe 14: Use WSL without putting Linux build work on `/mnt/c`

On Windows, keep Linux-heavy repositories in a WSL-native location such as:

```text
~/src/my-project
```

Use NTFS for native Windows/MSVC work. Treat Windows and WSL as separate tool/config/cache/trust environments rather than trying to share Cargo target directories or persistent generated shell artifacts across the boundary.

This avoids the filesystem I/O penalty and reduces cross-environment state confusion.

## Recipe 15: Work on Automexia itself

From the Automexia repository:

```text
cargo xtask doctor
cargo dev
```

After the full gate has passed once, use:

```text
cargo automexia
```

Before a contribution/merge, use:

```text
cargo ready
```

This recipe is for developing the terminal project itself. Your own projects inside Automexia should use their own build/test commands.

## When a recipe stops fitting

Recipes are meant to explain the intended composition of features, not become another hidden specification. When you need an exact option/key/setting, jump to:

- [CLI reference](../reference/cli.md)
- [Keyboard reference](../reference/keyboard.md)
- [Configuration reference](../reference/configuration.md)

When behavior is failing rather than merely unfamiliar, use [Troubleshooting](../guide/troubleshooting.md).
