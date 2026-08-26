# Commands and shell workflows

Automexia organizes command-driven work across software, operations,
automation, data, media, and other toolchains without replacing the shell. The
most important command rule is simple: **normal commands belong to your real
shell; Automexia commands control the workspace around it.**

## Understand the command layers

| Layer | Who owns parsing/behavior? | Use it for |
|---|---|---|
| Shell command line | PowerShell, CMD, Bash, Zsh, Fish, etc. | `git`, `cargo`, `ssh`, scripts, pipes, redirection, environment expansion |
| Automexia application CLI | Automexia | Launch directory/shell, starter config, logging, shell-integration maintenance |
| Command palette | Automexia UI | Discoverable terminal/window actions |
| Quick Actions | Automexia typed action model | Reviewed templates that are inserted/copied rather than silently executed |
| Persistent aliases | Shell-native generated files | Frequent reviewed actions you intentionally want as short shell names |
| Cargo/xtask | Automexia repository | Building, testing, and running Automexia from source |

## 1. Use your shell for normal work

Inside an Automexia pane, run third-party tools exactly as you normally would in that shell:

```text
git status
```

```text
cargo test
```

```text
ssh my-server
```

The same rule applies to tools such as Python, FFmpeg, database clients,
data-processing commands, build tools, and project scripts: use their normal
syntax in the shell.

Automexia does not reparse these commands from rendered terminal cells. PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE, Fish, and other tools continue to own quoting, history, cursor movement, environment expansion, pipelines, and normal completion.

### Why this matters

If a command uses shell syntax such as a pipeline, redirect, glob, shell function, alias, command substitution, or environment expansion, type it in the shell. Do not try to express a complex shell language expression through Automexia's launch-time `-e` option.

## 2. Use the Automexia CLI for launch-time behavior

The installed application syntax is:

```text
automexia [OPTIONS] [COMMAND]
```

The user-facing launch options are:

| Option | Use it when… |
|---|---|
| `-w, --working-dir <PATH>` | The new session should start in a known directory. |
| `-e, --command <PROGRAM> [ARGS...]` | You want a specific shell/program instead of the configured default. Keep it last. |
| `--write-config [PATH]` | You need a non-overwriting starter config. |
| `--enable-log-file` | You are diagnosing a launch/runtime problem. |
| `--title-placeholder <TEXT>` | You need a different initial title before live title metadata takes over. |
| `--app-id <ID>` | You need to override Wayland `app_id` / X11 `WM_CLASS` on Linux/BSD. |
| `-h, --help` | You need the exact syntax supported by the current binary. |
| `-V, --version` | You need the current binary version. |

Example:

```text
automexia --working-dir D:\work -e pwsh -NoLogo
```

Remember that everything after `-e <PROGRAM>` is a program argument, not another Automexia option.

## 3. Use the command palette for UI actions

Open the palette with `Ctrl+Shift+P` on Windows/Linux/BSD or `Cmd+Shift+P` on macOS.

The palette is ideal when:

- you know the operation but not the shortcut;
- an action is used too rarely to justify a custom binding;
- a pane/tab rail is hidden because the layout is compact;
- you want a keyboard-driven way to invoke a registered terminal action without editing configuration.

Use shortcuts for frequent muscle-memory operations and the palette for discoverability. They invoke the same product action model rather than creating two competing behaviors.

## 4. Understand session-only shell integration

**Available now.** Automexia can provide prompt lifecycle metadata, context/path presentation, icon-aware listings, and related shell-aware behavior while still hosting the real shell.

Normal application launch is session-only by default. It does not need to rewrite your shell profile just to make the Automexia child session work.

Check persistent/integration health with:

```text
automexia shell-integration doctor
```

This command is read-only.

Only use persistent installation when you intentionally need Automexia integration in nested/other shells outside the normal session-only launch path:

```text
automexia shell-integration install
```

Repair an Automexia-owned installation explicitly with:

```text
automexia shell-integration install --force
```

Remove only the Automexia-owned persistent integration with:

```text
automexia shell-integration uninstall
```

On Windows, these commands respect the effective PowerShell execution policy. Automexia does not bypass enterprise policy.

### Which approach should I choose?

| Need | Approach |
|---|---|
| Normal Automexia sessions | **Session-only integration** — do nothing extra |
| Diagnose missing context/listing integration | `shell-integration doctor` |
| Nested shells outside normal Automexia launch need integration | Explicit persistent `install` |
| Persistent integration looks damaged/stale | Preview/diagnose, then `install --force` |
| You no longer want persistent profile hooks | `uninstall` |

Do not install persistent integration simply because it exists; session-only behavior is the preferred default boundary.

### Jump between completed commands

When command output is long, use `Ctrl+Shift+Up` and `Ctrl+Shift+Down` on
Windows/Linux/BSD, or `Cmd+Shift+Up` and `Cmd+Shift+Down` on macOS, to move the
selected pane to the previous or next command boundary. The same actions are
available as **Jump to Previous Command** and **Jump to Next Command** in the
command palette.

This is viewport navigation, not shell history. It never changes the editable
command line, presses Enter, reruns a command, or moves another pane. Repeating
Up at the oldest retained command or Down at the live prompt is a no-op. The
feature follows OSC 133 prompt marks from session-only shell integration and
therefore continues to work with wrapped prompts and retained scrollback. A
custom shell that emits no supported marks is left unchanged instead of being
parsed heuristically.

## 5. Use Quick Actions for reviewed repeatable commands

**Implemented locally / release-gated.** A Quick Action is a typed command template with explicit placeholders, scope, risk metadata, and revision information. The intended user flow is:

1. Search for the action.
2. Review the exact command tokens, placeholders, context requirements, collision state, and risk.
3. Fill placeholders.
4. Choose **Insert** or **Copy**.
5. Review/edit the command in the shell before running it.

The action itself does not silently press Enter or gain arbitrary process authority merely because it was saved.

Use a Quick Action when a command is worth remembering as a named workflow but still deserves review before execution. Use a normal shell command for one-off work, and use an alias only after the action is stable enough that a short persistent shell name is genuinely helpful.

See [Command productivity](productivity.md) for the management workflow.

### Provider-aware Quick Actions

**Product-integrated / provider refresh not activated.** A validated cached
SSH or provider product is synchronized to the selected pane when you open
the same Quick Actions surface with
`Ctrl+Shift+O` on Windows/Linux/BSD or `Cmd+Shift+O` on macOS. No account is
queried and no provider command runs when the surface opens or while you type.
The retained capsule must exactly match the pane session and revision; otherwise
the old rows are cleared. Until an approved provider refresh/capsule producer
publishes that validated product, contextual rows do not appear.

A contextual row uses the connection icon and a concise label such as
`AWS · Account 123456789012 · Current · Production`. Review repeats the exact
public target, provider state, and environment risk in text and in its accessible
name. Current read-only observations may be inserted into the shell without
Enter. Production requires a second confirmation. Actions that need the D3
broker or private provider environment show **Broker required** and cannot be
copied or inserted through ambient CLI state.

If a row says **Refreshing**, **Stale**, **Expired**, **Offline**,
**Unavailable**, **Error**, or **Context changed**, refresh the owning provider
outside the Quick Actions surface and review the newly published row. The old
row is rejected immediately before copy or insertion even if it was already
open. Closing the pane or disabling/uninstalling the provider removes the
in-memory contextual snapshot; it does not alter persisted Quick Actions,
provider configuration, credentials, shell profiles, or cloud state.

OpenBao is not included in this source slice. Real-provider activation, native
provider/account testing, and controlled screen-reader/release evidence remain
release gates; this section does not claim those workflows are available in the
v0.4 product.

## 6. Use persistent aliases for high-frequency reviewed actions

**Implemented locally / release-gated.** Persistent aliases are generated from canonical typed actions. They are projections, not the source of truth.

This is a good fit when:

- the action is used frequently;
- the alias name is memorable and collision-free;
- you understand the action's risk;
- native shell ownership can be preserved;
- you want the alias only in explicitly selected shells.

It is a poor fit when the command changes every day, contains secrets, should be project-local, or collides with an existing native alias/function.

Alias management is dry-run first. For example:

```text
automexia aliases list
```

```text
automexia aliases preview --shell powershell
```

```text
automexia aliases test --shell powershell
```

An `enable`, `disable`, `rename`, `regenerate`, or `disable-all` operation previews by default. Applying a source-changing operation requires the expected revision and generation returned by the reviewed preview. This prevents a stale review from mutating newer state.

See [Command productivity](productivity.md) for the full practical flow and [CLI reference](../reference/cli.md) for exact flags.

## 7. Use Cargo/xtask only when working on Automexia itself

These commands are for the source repository, not general terminal use:

| Command | Purpose |
|---|---|
| `cargo dev` | Full contributor gate, build, smoke, then launch |
| `cargo automexia` | Incremental build/smoke/launch after the full gate has passed |
| `cargo ready` | Full gate without launching |
| `cargo ci` | Full non-launching CI gate alias |
| `cargo qa` | Deeper Phase 0 evidence profile |
| `cargo storage` | Inspect Automexia build-storage use |
| `cargo purge` | Remove workspace build artifacts after Automexia windows close |
| `cargo xtask doctor` | Check Rust/tools/host shell/packaging/storage/WSL placement |

For ordinary work inside Automexia, use the commands owned by your project or chosen tool. `cargo dev` here specifically means “develop Automexia Terminal.”

## 8. A simple decision tree

Use this order when deciding how to perform a task:

1. **Is it a normal tool/shell command?** Type it in the shell.
2. **Is it an Automexia window/session action?** Use a shortcut or the command palette.
3. **Does Automexia need a launch-time option?** Use `automexia ...` from the invoking shell/shortcut.
4. **Is it a repeatable command that should stay reviewable?** Use a Quick Action if your build exposes that release-gated feature.
5. **Is that reviewed action frequent and stable enough to deserve a shell name?** Publish an explicit alias.
6. **Are you building/testing Automexia itself?** Use the Cargo/xtask layer.

This separation keeps normal shell semantics intact while still giving Automexia useful workflow controls around them.
