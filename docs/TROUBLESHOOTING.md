# Troubleshooting

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
4. Run the native checks in [Shell integration](SHELL-INTEGRATION.md#verification).

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
[Image previews](IMAGE-PREVIEWS.md). A rejected or stale decode must remove the
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
below their documented minimum. See [Liquid Hacker UX](LIQUID-HACKER-UX.md).

## WSL reports poor I/O performance

This is expected when Linux Cargo reads a checkout under `/mnt/c` or `/mnt/d`.
Move Linux work to a WSL-native checkout such as `~/src/automexia-terminal` and
keep Windows/MSVC work in the NTFS checkout. Do not share `target/` between
them. The complete workflow is in [Windows and WSL development](WSL-DEVELOPMENT.md).

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
example in [Configuration](CONFIGURATION.md).

## Fullscreen appears dimmer on Windows

Automexia does not change brightness. It pins the surface to SDR sRGB and holds
a route-scoped display-required request while fullscreen. Content-adaptive
brightness/contrast, local dimming, HDR SDR-content brightness, or monitor Eco
mode can still react to a mostly dark screen. Follow the OS/display steps in
[Support](../SUPPORT.md#windows-fullscreen-brightness).

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

Use [Support](../SUPPORT.md) for public defects and [Security](../SECURITY.md)
for private vulnerability reporting.
