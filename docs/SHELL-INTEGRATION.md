# Shell integration

Automexia's shell integration makes prompt context, resize-safe semantic rows,
session cloning, and icon-aware listings work without changing the objects or
bytes that scripts consume.

## Automatic provisioning

`cargo dev` and `cargo automexia` provision integration immediately before
launch. The operation is source-fingerprinted, atomic, idempotent, and fails the
launch if it cannot establish the expected integration. Normal users and
contributors do not run installer scripts manually.

| Host/session | Provisioned behavior |
|---|---|
| Windows PowerShell / PowerShell 7 | A guarded profile hook, prompt lifecycle metadata, OSC 7 directory, shell/user identity, same-pane CMD interception, and a PowerShell formatting view. |
| Command Prompt | UTF-8 setup, full-path semantic prompt, identity metadata, and `ls`/`ll` wrappers while built-in `dir` and explicit `cmd /c` remain native. |
| WSL Bash/Zsh/Fish | Per-distribution user profile/conf.d integration, Linux-native files, semantic prompt metadata, native-first completion, and optional eza presentation. |
| Linux/macOS Bash/Zsh/Fish | User profile/conf.d integration, files under user configuration roots, native-first completion, and user-local terminfo. |

The integration loads only when `TERM_PROGRAM=Automexia`,
`AUTOMEXIA_SHELL_INTEGRATION=1`, or the corresponding WSL marker is present.
It uses load guards so repeated sourcing cannot stack prompt hooks.

## Native command completion

Automexia provisions shell adapters but never computes candidates from rendered
terminal cells. Tab behavior, quoting, cursor movement, history, menus,
autosuggestions, and accessibility remain owned by PSReadLine, Readline, ZLE,
or Fish. Existing native definitions win on Bash, Zsh, and Fish. PowerShell
requires an explicit override because it has no supported public read-only
completer registry; CMD retains native fallback without a parity claim.

```text
cargo xtask completion doctor
cargo xtask completion refresh --provider docker --shell bash
cargo xtask completion refresh --provider kubernetes --shell zsh
cargo xtask completion refresh --provider helm --shell fish
cargo xtask completion refresh --provider kubernetes --shell powershell --allow-native-override
```

Refresh is the only operation that starts a provider. It uses the installed
official CLI with exact arguments and no stdin, a 750 ms deadline, 1 MiB stdout
and 256 KiB stderr ceilings, fixed private destinations, SHA-256 sidecars, and
atomic replacement. Restart the shell after refresh, enable, disable, or remove.
`doctor` only resolves executable names and inventories cached regular files; it
does not run a provider, authenticate, read command history, or access secrets.

Disable or remove managed state without affecting native completion:

```text
cargo xtask completion disable
cargo xtask completion enable
cargo xtask completion remove --provider docker --shell bash
```

Generated files live below
`<Automexia-config-root>/generated/completion/<shell>/`. They are disposable;
the provider remains their source. Do not edit them. A digest mismatch, linked
artifact or managed parent, non-directory path component, oversized file,
unsupported shell/provider, missing tool, timeout, or malformed output fails
closed to native shell behavior.

## Prompt ownership

The integration emits OSC 7, OSC 133, and bounded OSC 1337 user variables for
the shell, user, distribution, prompt generation, and lifecycle. The renderer
owns context/path semantic rows; PSReadLine, Readline, or ZLE owns only the
lambda, editable input, and cursor. This split is why completed prompts can be
reflowed during resize without asking a shell editor to reconstruct scrollback.

Metadata is advisory and validated. An absolute, control-free OSC 7 directory
can update a pane's launch descriptor; malformed or relative values cannot.
Context discovery is asynchronous, bounded, session-scoped, and never blocks
the PTY/render path.

## Icon-aware listings

PowerShell keeps real `DirectoryInfo` and `FileInfo` pipeline objects and
changes presentation only. The final `Name` field contains the composite icon
beside the filename; there is no separate `Icon` column. CMD delegates its
interactive `ls` view through the same safe formatter. Bash/Zsh use `eza` when
installed and fall back to the native command when it is absent.

Folder categories such as secrets, configuration, logs, source, tests, build,
assets, packaging, infrastructure, caches, and documentation use a folder
shape plus a small semantic badge. The complete category/color contract is in
[Liquid Hacker UX](LIQUID-HACKER-UX.md#file-and-folder-icons).

Set `AUTOMEXIA_PLAIN_LS=1` before the integration loads to disable all listing
presentation. In Bash/Zsh, `command ls` bypasses the wrapper for one command.
In PowerShell, explicit object pipelines and property access remain native.
Set `AUTOMEXIA_PLAIN_CMD=1` to keep a typed `cmd`/`cmd.exe` launch completely
native instead of applying the same-pane CMD integration.

## Manual repair and removal

Use manual scripts only for diagnosis or uninstall:

```powershell
powershell -NoProfile -File shell-integration/install-windows.ps1 -Force
powershell -NoProfile -File shell-integration/uninstall-windows.ps1
```

```bash
bash shell-integration/install-unix.sh --force
bash shell-integration/uninstall-unix.sh
```

Before running a maintainer command, prefer `cargo automexia`; it repairs a
missing or stale install automatically. Installer tests use isolated profile
roots and verify first install, no-op repeat, deliberate damage, repair, and
uninstall boundaries.

## Verification

Check the environment inside a session:

```text
TERM_PROGRAM=Automexia
AUTOMEXIA_SHELL_INTEGRATION=1
```

Then confirm the context/path appears before a keypress and changes after
changing directory. Focused repository checks are:

```text
powershell -NoProfile -File tools/ci/test_powershell.ps1
bash tools/ci/test_shell_sources.sh
bash tools/ci/test_shell_integration.sh
zsh tools/ci/test_zsh_integration.zsh
fish tools/ci/test_fish_integration.fish
```

Run only the native-host command available to you; CI owns PowerShell/CMD on
Windows and Bash/Zsh on Linux and macOS. See [Troubleshooting](TROUBLESHOOTING.md)
when the first prompt lacks metadata or icons.

## Why injection instead of a second shell

Automexia hosts the user's real PowerShell, CMD, WSL, Bash, or Zsh process. It
does not replace the shell or parse commands itself. Small guarded hooks expose
metadata the PTY protocol cannot otherwise know, preserve native history and
profiles, and can be removed independently. Launch-time provisioning was
chosen over a manual prerequisite so a successful launch has a deterministic
feature set; see [ADR 0009](adr/0009-launch-time-shell-provisioning.md).
