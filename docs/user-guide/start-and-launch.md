# Start and launch sessions

Starting in the right shell, directory, and task context removes repeated
setup and makes the purpose of a session clear. This guide explains the
practical launch choices for a packaged executable and for work on the Automexia
source repository.

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

With no custom shell configured, Automexia uses the user's default shell (PowerShell on Windows and the login shell on Unix). A normal launch uses **session-only shell integration** where supported; it does not rewrite your PowerShell/Bash/Zsh/Fish profile merely because the application opened.

Useful discovery commands are:

```text
automexia --help
automexia --version
```

Use these before assuming a command from development documentation is present in the particular binary you have.

## 2. Start in a specific directory

Use `--working-dir` / `-w` when the directory is part of the session you want to create:

```text
automexia --working-dir D:\work\my-project
```

```text
automexia --working-dir ~/work/my-project
```

Automexia validates the path. An invalid working-directory override is rejected with a warning and the safe default is used rather than silently launching in an untrusted/nonexistent location.

### When to use this instead of `cd`

Use `--working-dir` when you are launching from a desktop shortcut, script, file manager, or project launcher and already know the desired project root. Use a normal shell `cd` when you are already working interactively and the directory change is temporary.

For repeated project launchers, prefer a shortcut/script that passes `--working-dir` over hard-coding project paths into your global Automexia configuration.

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

This is useful for a dedicated REPL, monitor, or tool that already owns its own interactive interface. It is less appropriate for a complicated shell command line that depends on pipes, redirection, quoting, environment expansion, or shell aliases: in those cases, start the shell normally and run the command inside it.

Automexia passes an exact program/argument vector; it does not reinterpret the command using shell-string parsing.

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

Start with only the values you need; omitted settings retain tested defaults. See [Configuration and customization](customization.md) for a practical workflow and [Configuration reference](../reference/configuration.md) for the complete schema.

## 6. Optional logging for one launch

To write an Automexia log file for the current launch:

```text
automexia --enable-log-file
```

Logs live under the Automexia configuration root. Treat them as potentially sensitive because paths and process diagnostics can reveal local machine information. Use this option when diagnosing a problem, not as a requirement for normal use.

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

Do not treat these Cargo aliases as commands an ordinary installed-user package needs. They are repository-owned development automation.

## 8. Confirm that the session started correctly

A healthy first session should show:

1. A tab title that identifies the actual shell/profile or WSL distribution.
2. Prompt context and the complete working path before you type the first character.
3. The terminal input row below that context.
4. A pane footer containing operational information such as encoding, newline convention, grid size, and local time when the pane is large enough to show it.
5. Normal shell behavior: history, quoting, completion, scripts, and tools continue to be owned by the shell.

If icon-aware `ls`/`ll`, prompt metadata, or other shell-enhanced behavior is missing, continue with [Commands and shell workflows](commands-and-shell.md) and [Troubleshooting](../guide/troubleshooting.md).

## 9. What to learn next

Once you can open a session, the most important concept is the workspace hierarchy. Read [Workspaces, tabs, and panes](workspace.md) before building multi-pane layouts; it explains the difference between a top-level tab, a pane-local tab, a fresh split, and a cloned split.
