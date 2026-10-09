# Shell integration

Automexia integrates with supported shells without replacing their editor,
history, completion, quoting, aliases, pipelines, or execution.

## Session-only integration by default

Normal application launch loads bounded integration only into the child shell.
It does not need to rewrite shell profiles, change execution policy, provision a
WSL distribution, or create persistent files.

If integration resources are missing or invalid, Automexia starts the ordinary
shell and reports a bounded redacted diagnostic.

PowerShell integration is dot-sourced into the interactive child session. Its
prompt, input callback, completion state, and nested CMD helpers must remain
available after startup returns. Both development and packaged launches use this
same session bootstrap; execution-policy denial still leaves the native shell.

Linux's unset shell configuration starts an ordinary interactive, non-login
shell. It inherits the desktop/login environment and reads the shell's normal
interactive rc file. To retain login startup explicitly, configure
`args = ["--login"]` in the `[shell]` section and source integration from that
startup file.
macOS keeps native login startup, including its default-shell `login(1)` policy.
Ordinary Bash, Zsh and Fish login sessions load integration after native startup;
non-login sessions use the same adapters as Linux. Bash login startup uses a
one-shot prompt hook because Bash ignores `--rcfile` in login mode. A custom
profile that replaces `PROMPT_COMMAND` entirely should explicitly source the
shipped Bash integration after configuring its prompt. No profile is rewritten.

The macOS system Bash is supported without requiring a Homebrew replacement.
Bash before 4.4 uses a session-local command-boundary hook that preserves an
existing DEBUG hook and command exit status; newer Bash uses its native PS0.
Neither path changes history settings or Readline bindings.

On Linux, ordinary Bash, Zsh and Fish sessions also load the packaged integration
automatically. Bash retains the user's `.bashrc`; Zsh retains `.zshenv`, `.zshrc`
and `ZDOTDIR`; Fish retains its native configuration and editor. New panes and
tabs use independent PTYs with the same validated launch descriptor, including
profile environment overrides and the starting directory.

Explicit commands, scripts, custom startup files and startup opt-out flags retain
their native arguments. Explicit Linux login flags also remain unchanged. To integrate such a custom launch, source
the matching shipped integration in the startup file you already own. Setting
`AUTOMEXIA_SHELL_INTEGRATION=0` disables automatic activation. The shipped
scripts also respect this switch when sourced from user startup files.

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

The terminal owns stable context/path rows; the native shell editor owns its
editable command and cursor. Context decoration does not insert command text.

CMD publishes a short identity reference on every prompt to stay below its
511-character prompt-format limit. Long user and executable names are registered
once per identity in the terminal's bounded metadata map. Nested shells restore
their own identity on return; missing or invalid registrations remain unavailable.
Clear-window and terminal reset preserve these session-local registrations. See
[ADR 0083](adr/0083-bounded-cmd-prompt-identity-reference.md) for the versioned
wire contract and compatibility behavior.

For passive Kubernetes context, Bash, Zsh, Fish and PowerShell publish local HOME
and exported KUBECONFIG paths with byte limits, cached encoding, explicit clearing
and snapshot commit markers. Native Windows PowerShell additionally publishes
HOMEDRIVE, HOMEPATH and USERPROFILE so the background worker can select the same
local default configuration as Kubernetes. Prompts do not probe these paths.
These values are not credentials or process authority.
Set `AUTOMEXIA_CONTEXT_PATH_HINTS=0` to clear them on the next prompt; remove that
setting to restore publication. CMD retains application-inherited discovery
because its native PROMPT cannot encode changing environment values. See
[prompt context assurance](PROMPT-CONTEXT-ASSURANCE.md) for supported behavior,
privacy, native tests, benchmarks and external verification requirements.

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

The Linux application regression runs the unconfigured default and explicit
Bash, Zsh and Fish PTYs on CPU and Vulkan at 100% and 150% X11 scaling. It checks real prompt/result metadata, tags,
tables, keyboard menus, theme cancellation, pane/tab cloning, working directories,
profile environment, resize and child cleanup:

```sh
cargo build --locked -p automexia-terminal --bin automexia --features visual-test-hooks
xvfb-run -a -s "-screen 0 1600x1200x24 -nolisten tcp" \
  python3 tests/integration/unix-session-ui.py \
  --binary target/debug/automexia --captures artifacts/native-linux
```

The driver requires Xvfb, xauth, xdotool, ImageMagick, and all three shells.
CI uses Mesa lavapipe for Vulkan; it rejects an unexpected CPU fallback.
Add `--backend webgpu` to a build with the `wgpu` feature to check WGPU separately.
These scenarios do not certify every Wayland compositor, hardware GPU or physical
display scale.

On macOS, build with `--all-features` and run the same driver directly without
Xvfb. It uses the existing AX/Quartz driver, requires preauthorized Accessibility
access, and stops on focus loss. CPU and WGPU/Metal cases use system Bash, system
Zsh, Fish and the unset shell configuration. Real display scaling is recorded;
controlled raster tests separately cover synthetic scales and pixels. The native
workflow runs on both Intel and Apple Silicon. Missing native permissions fail
the scenario and never count as successful validation.

The driver's bounded startup/command timing samples are diagnostics with 50 ms
polling resolution, not performance baselines. The native workflow also retains
Criterion responsive-layout samples; hosted machines do not establish controlled
application performance. See [benchmark methodology](../tools/renderer-benchmarks/README.md).

See [Shell productivity](guide/shell-productivity.md) and
[Commands and shell](user-guide/commands-and-shell.md).

## CP3.1 alias integration boundary

CP3.1 generated aliases are optional native-shell artifacts, not a replacement
editor or parser. Native definitions win, activation is explicit, and removal
restores the unchanged native fallback. See
[DEVOPS-ALIASES.md](DEVOPS-ALIASES.md).
