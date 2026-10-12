# Start and launch sessions

Choose a shell, starting directory, or program for an installed build or source checkout.

## Choose a launch approach

| Situation | Recommended approach | Why |
|---|---|---|
| Open a normal terminal | `automexia` | Uses your configured/default shell and normal settings. |
| Start directly in a task or project directory | `automexia --working-dir <PATH>` | Avoids an extra `cd` and makes the initial context explicit. |
| Start one specific program/shell | `automexia [other options] -e <PROGRAM> [ARGS...]` | Launches that program instead of the configured shell. |
| Generate a starter config | `automexia --write-config` | Creates a non-overwriting user config. |
| Diagnose an Automexia shell-integration problem | `automexia shell-integration doctor` | Read-only health information. |
| First run from a source checkout | `cargo dev` | Runs the full contributor gate, builds, then opens Automexia. |
| Normal edit/build/run after the full gate passed | `cargo automexia` | Faster incremental source-development loop. |
| Verify source without opening a window | `cargo ready` | Full contributor gate with no launch. |

## 1. Normal application launch

With the Automexia executable on your `PATH`, start it with:

```text
automexia
```

By default, Automexia starts PowerShell on Windows or the login shell on Unix.
Supported integration applies only to that session; launch does not rewrite shell profiles.

Useful discovery commands are:

```text
automexia --help
automexia --version
```

Check which commands your installed version provides.

## 2. Start in a specific directory

Use `--working-dir` / `-w` when the directory is part of the session you want to create:

```text
automexia --working-dir D:\work\my-project
```

```text
automexia --working-dir ~/work/my-project
```

An invalid directory produces a warning and uses the safe default.

### When to use this instead of `cd`

Use `--working-dir` in shortcuts, scripts, or project launchers.
Use shell `cd` to change directories in an open session.

For repeated launches, pass `--working-dir` from a shortcut or script instead of
storing project paths in global settings.

## 3. Start a specific shell or command

Use `-e` / `--command` to replace the configured shell for that launch:

```text
automexia -e pwsh -NoLogo
```

```text
automexia --working-dir D:\work -e pwsh -NoLogo
```

```text
automexia --working-dir ~/work -e zsh -l
```

**Important:** `-e` must be the final Automexia option because every value after the program name is passed to that program as an argument.

### Temporary override or permanent setting?

- Use `-e` when you need a different shell/program **for one launch**.
- Use `[shell]` in `config.toml` when you want that shell **as your normal default**.
- Use a separate desktop/script launcher when you regularly switch between several fixed launch profiles.

Permanent shell configuration is covered in [Configuration and customization](customization.md).

## 4. Launch for a single task

`-e` can launch a program directly rather than opening the normal interactive shell. For example:

```text
automexia --working-dir ~/work/project -e python
```

or:

```text
automexia --working-dir D:\work\project -e pwsh -NoLogo
```

Use this for an interactive interpreter such as Python, a monitor, or another
interactive tool. For pipelines, redirection, expansion, or aliases, start a
shell and enter the command there.

Automexia passes arguments directly to the program. It does not parse them as shell code.

## 5. Create the first configuration

Automexia works with no config file. Create a starter without overwriting an existing file:

```text
automexia --write-config
```

You can also choose an explicit file:

```text
automexia --write-config D:\configs\automexia.toml
```

Then open the config using the normal shortcut:

- Windows: `Ctrl+,`
- Linux/BSD: `Ctrl+Shift+,`
- macOS: `Cmd+,`

The config root is:

| Platform | Default root |
|---|---|
| Windows | `%LOCALAPPDATA%\Automexia\Terminal` |
| macOS | `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` |
| Linux | `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia` |

Start with only the values you need; omitted settings retain tested defaults. See [Configuration and customization](customization.md) for a practical workflow and [Configuration reference](../CONFIGURATION.md) for the complete schema.

## 6. Optional logging for one launch

To write an Automexia log file for the current launch:

```text
automexia --enable-log-file
```

Logs are saved under the configuration root. Enable them for diagnosis and
redact paths and process details before sharing.

## 7. First run from source

If you are using the repository rather than a packaged build, run the environment report first:

```text
cargo xtask doctor
```

Then use:

```text
cargo dev
```

`cargo dev` performs the complete contributor verification path, builds the debug executable, checks its version, provides session-only integration, and opens the terminal only after the gate passes.

After one successful complete gate, the normal incremental loop is:

```text
cargo automexia
```

When you want verification without opening the app:

```text
cargo ready
```

These Cargo commands are for source development only.

## 8. Confirm that the session started correctly

A healthy first session should show:

1. A tab title that identifies the actual shell/profile or WSL distribution.
2. Prompt context and the complete working path before you type the first character.
3. The terminal input row below that context.
4. A pane footer containing operational information such as encoding, newline convention, grid size, and local time when the pane is large enough to show it.
5. Normal shell behavior: history, quoting, completion, scripts, and tools continue to be owned by the shell.

If icon-aware `ls`/`ll`, prompt metadata, or other shell-enhanced behavior is missing, continue with [Commands and shell workflows](commands-and-shell.md) and [Troubleshooting](../TROUBLESHOOTING.md).

## 9. What to learn next

Read [Workspaces, tabs, and panes](workspace.md) to arrange sessions and choose
between fresh and cloned splits.
