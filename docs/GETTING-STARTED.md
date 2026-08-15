# Getting started

This tutorial builds, verifies, and opens Automexia Terminal from a clean
checkout. It is for contributors and source users; signed stable installers are
not published until the release prerequisites in [Releasing](../RELEASING.md)
are satisfied.

## 1. Prepare the host

Install Git, Python 3, and the Rust toolchain pinned by
`rust-toolchain.toml`. Install PyYAML and the pinned dependency-policy tool:

```text
python -m pip install PyYAML
cargo install --locked cargo-deny --version 0.20.2
```

Platform builds also need their native SDK and display dependencies:

- **Windows:** Visual Studio Build Tools with the MSVC and Windows SDK
  workloads, PowerShell, and—only for ARM64 MSI work—the .NET SDK.
- **Ubuntu/Debian:** `build-essential`, `pkg-config`, `libasound2-dev`,
  `libfontconfig1-dev`, `libxkbcommon-dev`, `libwayland-dev`,
  `libx11-xcb-dev`, and `glslang-tools`.
- **macOS:** current Xcode Command Line Tools. Signing and notarization are
  needed only for release artifacts.

Run the non-mutating environment report before compiling:

```text
cargo xtask doctor
```

On WSL, stop if it reports a source or target below `/mnt/c` or `/mnt/d`.
Follow [Windows and WSL development](WSL-DEVELOPMENT.md) and use a Linux-native
checkout such as `~/src/automexia-terminal`.

## 2. Verify, build, and open

From the repository root, run:

```text
cargo dev
```

This one command performs the complete contributor gate, builds the debug
application, checks `automexia --version`, installs or repairs the host shell
integration, and opens Automexia. A cold build can spend several minutes in
WGPU, shader, and workspace tests. The window opens only after verification
passes.

After one successful complete gate, use the incremental launch path:

```text
cargo automexia
```

Use `cargo ready` when you need the complete gate without opening a window.
The command and its isolated build-artifact cleanup are identical on Windows,
Linux, and macOS.

## 3. Confirm the first session

In the opened terminal, check:

1. The tab title names the real shell or WSL distribution.
2. The semantic context row appears before input, without a first keystroke.
3. The complete working path appears below the context tags.
4. `ls` uses icon-aware presentation where the installed shell integration
   supports it.
5. The footer remains attached to the pane bottom and reports encoding,
   newline convention, grid size, and local time.

PowerShell, CMD, WSL, Bash, and Zsh behavior is explained in
[Shell integration](SHELL-INTEGRATION.md). If a check fails, use the focused
diagnosis in [Troubleshooting](TROUBLESHOOTING.md).

## 4. Try panes, tabs, and selection

On Windows/Linux, try these default interactions:

- `Ctrl+T`: new window-level tab.
- `Ctrl+Shift+T`: new independent tab inside the selected pane.
- `Ctrl+Shift+R` / `Ctrl+Shift+D`: fresh right/lower split.
- `Ctrl+R` / `Ctrl+D`: clone the active session into a right/lower split.
- `Alt+Arrow`: select a neighboring pane.
- `Shift+Arrow`: start terminal selection at the insertion cursor and extend
  it; add `Ctrl` on horizontal motion to jump by Unicode word boundary. Press
  an Arrow without `Shift`, or type/paste text, to exit selection and resume
  normal shell input.
- `Ctrl+Shift+P`: command palette.

macOS uses its native `Cmd` variants. The authoritative complete table and
custom action names are in [Keyboard and input reference](KEYBOARD.md).

## 5. Create a personal configuration

Create a non-overwriting starter file with a pointer to the canonical reference:

```text
automexia --write-config
```

Then use [Configuration reference](CONFIGURATION.md) to add only the settings
you want to change. Automexia works with no configuration; omitted values use
tested defaults. Restart, or bind the stable `ReloadConfig` action for an
explicit reload. Malformed or oversized configuration is rejected and the last
known-good runtime configuration remains active.

## 6. Prepare a contribution

Create a focused branch, add tests and a `changes/` fragment, update the
feature assurance ledger and relevant docs, then run:

```text
cargo ready
```

Read [Contributing](../CONTRIBUTING.md) before opening a pull request. Every
commit requires a DCO sign-off.
