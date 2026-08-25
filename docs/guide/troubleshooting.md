# Troubleshooting

## Antivirus reports PowerShell or shell-integration behavior

Current normal launches do not invoke an installer, change execution policy,
write a PowerShell profile, or provision WSL. Confirm the session resource
boundary without changing state:

```text
automexia shell-integration doctor
```

Persistent integration is optional. If
`automexia shell-integration install` is blocked, continue using the
session-only integration and do not add an antivirus exclusion or pass
`-ExecutionPolicy Bypass`. For a protected release, verify the MSI/ZIP
checksum, Authenticode publisher/timestamp, and GitHub attestation as described
in [Release trust](../developer/testing-release.md), then submit only that exact public
artifact through the antivirus vendor's official false-positive process.
Developer builds and checked-out scripts are intentionally unsigned and should
not be redistributed.

If a RemoteSigned startup error names a local path beginning with the Windows
verbatim-path prefix, update to a build containing the Windows shell-path
normalization fix and restart Automexia. Do not weaken the execution policy:
Automexia keeps the canonical resource validation while passing PowerShell a
normal local path. Use Get-ExecutionPolicy -List only to diagnose the effective
policy. Under an organization-enforced AllSigned policy, use a signed Automexia
release; unsigned developer integration intentionally falls back to the native
shell.

A OneDrive-redirected Documents profile is no longer touched by normal launch.
The explicit persistent installer allows Microsoft Cloud Files reparse tags but
still rejects junctions, symbolic links, and unknown redirect types.

Start with the smallest diagnostic that preserves the failure. Do not delete a
configuration, profile, or Cargo target until its location and ownership are
known.

## `cargo dev` finishes tests but no window opens

`cargo dev` runs the complete gate before launch. A line such as
`cargo test --workspace --locked` means verification is still active, not that
the launcher forgot the window. Wait for the explicit launch phase. For normal
relaunches after one clean gate, use:

```text
cargo automexia
```

If the command exits, read the first failed phase and run `cargo xtask doctor`.
Provisioning errors intentionally stop launch rather than opening a terminal
without prompt/context/listing features.

## Context or path appears only after the first keypress

1. Run `cargo automexia` once to repair shell integration.
2. Inside the session, verify `TERM_PROGRAM=Automexia` and
   `AUTOMEXIA_SHELL_INTEGRATION=1`.
3. Confirm the shell is PowerShell, CMD, Bash, or Zsh; unsupported shells remain
   usable but may not emit enhanced metadata.
4. Run the native checks in [Shell integration](shell-productivity.md#verification).

Do not add prompt escape sequences manually before testing the repository-owned
integration; duplicate hooks can create repeated semantic generations.

## `ls` has no icons or category colors

- PowerShell/CMD: rerun `cargo automexia`; check that
  `AUTOMEXIA_PLAIN_LS` is not `1`.
- Bash/Zsh: install `eza` for enhanced listings. Without it, Automexia
  deliberately falls back to native `ls`.
- Use `command ls` on Unix or set `AUTOMEXIA_PLAIN_LS=1` before shell startup
  when you intentionally want plain output.

Formatting never changes PowerShell pipeline object types. If a script parses
display text, replace it with object/property access or an explicit native
command.

## Image preview is empty, black, or does not open

1. Use a local PNG, JPEG, GIF, BMP, ICO, WebP, or TIFF regular file. SVG, PDF,
   URLs, arbitrary UNC paths, symlinks, devices, and pipes are rejected.
2. Hover the printed path for at least 100 ms, click to pin, or select it and
   press `Ctrl+Alt+I` (`Cmd+Alt+I` on macOS).
3. For spaces or parentheses, print/select the complete path; quoted paths are
   supported.
4. In a mouse-reporting TUI, hold `Shift` for host-UI pointer ownership.
5. Run `cargo xtask test image-rendering`; on a controlled Windows display use
   `cargo xtask test image-rendering --native-gui`.

Review the file/dimension/cache limits and WSL path rules in
[Image previews](terminal-experience.md). A rejected or stale decode must remove the
card rather than paint uninitialized pixels.

## Prompt, cursor, or footer moves during resize

Run the deterministic suite:

```text
cargo xtask test resize-stress
```

On a controlled Windows desktop, also run:

```text
cargo xtask test resize-stress --native-gui
```

The footer is a real pane-bottom reservation; it must not float in unused grid
space. At an extremely short pane it intentionally folds away below 112 logical
pixels and returns when the pane grows. Pane-local tab rails similarly fold
below their documented minimum. See [Liquid Hacker UX](terminal-experience.md).

## WSL reports poor I/O performance

This is expected when Linux Cargo reads a checkout under `/mnt/c` or `/mnt/d`.
Move Linux work to a WSL-native checkout such as `~/src/automexia-terminal` and
keep Windows/MSVC work in the NTFS checkout. Do not share `target/` between
them. The complete workflow is in
[Windows and WSL development](#windows-and-wsl-development-workflow).

## C: or D: is filling up

First inspect rather than delete:

```text
cargo storage
```

Close Automexia windows, then remove workspace build artifacts with:

```text
cargo purge
```

The complete gate uses an isolated target and removes it after ordinary success
or failure. If `AUTOMEXIA_KEEP_VERIFY_TARGET=1` was set for diagnosis, unset it
and purge afterward. Also inspect separate Windows and WSL targets; cleaning one
checkout does not clean the other.

## Configuration reload fails

Automexia keeps the last known-good runtime configuration. Check the reported
TOML error, correct the file, then invoke a user binding for `ReloadConfig` or
restart the application. The main config is limited to 4 MiB and a theme to
1 MiB; non-regular or invalid UTF-8 files are rejected. Start from the minimal
example in [Configuration](../reference/configuration.md).

## Fullscreen appears dimmer on Windows

Automexia does not change brightness. It pins the surface to SDR sRGB and holds
a route-scoped display-required request while fullscreen. Content-adaptive
brightness/contrast, local dimming, HDR SDR-content brightness, or monitor Eco
mode can still react to a mostly dark screen. Follow the OS/display steps in
the repository `SUPPORT.md` fullscreen-brightness guidance.

## A new OS window closes every window

The native/custom close control and `CloseWindow` action close only the current
OS window. `Quit` intentionally exits the application. If current `main`
reproduces cross-window teardown, capture the exact shortcut/control, shell,
and `automexia --version`, then run the native window/resize gate and report a
platform regression.

## Report a reproducible issue

Include:

- `automexia --version`;
- OS version, architecture, shell, display server, and GPU/driver where relevant;
- the smallest reproduction and whether it happens with default config;
- focused command output with secrets and private paths removed;
- a screenshot only when visual state is material.

Use the repository `SUPPORT.md` process for public defects and follow the
current private vulnerability-reporting route in `SECURITY.md`.

## Windows and WSL development workflow

Automexia supports Windows and Linux development on the same machine, but each
toolchain must build from the filesystem native to that operating system.

### Why WSL reports slow I/O

Microsoft's
[WSL file-storage guidance](https://learn.microsoft.com/windows/wsl/setup/environment#file-storage)
recommends keeping files on the same operating-system filesystem as the tools
that operate on them. Linux Cargo work in `/mnt/c` or `/mnt/d` crosses the
Windows/WSL filesystem boundary for every metadata read, dependency scan, and
object write. A Rust workspace amplifies that cost across thousands of small
files. The WSL performance notification is therefore expected when Linux tools
operate on this Windows checkout; it is not evidence that Automexia runtime I/O
is leaking or consuming storage.

The inverse rule also applies: keep Visual Studio, Windows Cargo/MSVC, MSI
packaging, and native ConPTY/GPU tests in the NTFS checkout.

### Supported dual-native layout

Keep two Git checkouts and exchange source changes through commits:

```text
Windows / MSVC / ConPTY:
C:\src\automexia-terminal

WSL / Linux Cargo / Unix PTY:
~/src/automexia-terminal
```

Create the WSL checkout from a WSL shell:

```bash
mkdir -p ~/src
git clone https://github.com/AmjedAllaya/automexia-terminal ~/src/automexia-terminal
cd ~/src/automexia-terminal
git switch <branch>
cargo xtask doctor
cargo ready
```

Before a Linux build, `pwd` must not begin with `/mnt/`. Leave
`CARGO_TARGET_DIR` unset or point it to a location under the Linux filesystem,
such as `$HOME/.cache/automexia-target`. Do not share `target/`, a Cargo
registry, or compiler caches between Windows and WSL.

Use Git to synchronize the checkouts:

```bash
git status
git add <files>
git commit -s -m "type(scope): summary"
git push
```

Then fetch and switch or pull that branch in the other native checkout. Never
copy build artifacts between the two environments.

### Workflow safeguards

`cargo xtask doctor` reports one of:

- `workspace I/O host-native/ok` for a normal Windows or Unix checkout;
- `workspace I/O WSL-native/ok` for a WSL checkout under the Linux filesystem;
- an actionable advisory when either source or `CARGO_TARGET_DIR` is on a
  mounted Windows drive.

Compilation-heavy project workflows fail before building when invoked from WSL
with source or target storage under `/mnt/<drive>`:

- `cargo dev`;
- `cargo automexia`;
- `cargo ready`;
- `cargo ci`;
- `cargo qa`;
- `cargo xtask check`.

Focused read-only diagnosis remains available, and raw Cargo commands keep their
standard behavior. For a one-off diagnostic only, the guard can be acknowledged:

```bash
AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT=1 cargo xtask check
```

Do not use the override for routine work, CI, benchmarks, fuzzing, or release
evidence. It only acknowledges the known performance penalty; it cannot remove
it.

### Windows-triggered fuzzing

`cargo xtask test image-decoder-fuzz --seconds N` remains a supported Windows
command. The runner translates the current working tree once, copies its source
into a disposable WSL-native directory under `/tmp`, and performs all Cargo,
libFuzzer, corpus, and target I/O there. The copy excludes `.git`, the
workspace target, and generated fuzz target, corpus, and artifact directories
while retaining current tracked and untracked source edits.

The shell uses `pipefail`, explicit nightly Rust, bounded time/RSS, and a cleanup
trap. The disposable source, corpus, and build artifacts are removed on normal
exit, error, or interruption.

### Windows and WSL recovery steps

If the guard reports a mounted Windows source:

1. Run `pwd` and confirm it starts with `/mnt/c` or `/mnt/d`.
2. Commit or stash source work in the Windows checkout.
3. Clone or update the same branch under `~/src`.
4. Run `cargo xtask doctor` and require `WSL-native/ok`.
5. Re-run the Linux command.

If only the Cargo target is mounted, unset `CARGO_TARGET_DIR` or move it to
the Linux filesystem. Use `cargo storage` and `cargo purge` independently
inside each checkout; neither command deletes source or the other environment's
artifacts.
