# Install Automexia

Automexia is currently available as a source build. Official signed stable
installers are not published yet, so the safest supported way to try the
project today is to build and launch it from this repository.

This page takes you from a clean computer to the first Automexia window. If you
already have a working source checkout, continue with the
[Getting started tutorial](GETTING-STARTED.md).

## Choose the path that matches your goal

| Goal | Recommended path |
|---|---|
| Try Automexia today | Follow the source-build steps on this page. |
| Use Automexia regularly from a source checkout | Complete one verified launch, then use `cargo automexia` for later launches. |
| Contribute code or documentation | Install the contributor tools, complete `cargo ready`, and read [Contributing](../CONTRIBUTING.md). |
| Install a signed desktop package | Wait for an official stable release. Do not treat an unverified third-party package as an Automexia release. |

## What you need

Every source build needs:

- Git;
- Python 3 with `pip`;
- the Rust toolchain selected by `rust-toolchain.toml`;
- PyYAML;
- `cargo-deny` version `0.20.2`;
- at least 12 GiB of free space for the first complete verification build.

The normal cached application build needs less space, but Rust and graphics
artifacts can grow over time. Automexia reports storage before an expensive
verification run and provides safe project-owned cleanup commands.

### Windows requirements

Install Visual Studio Build Tools with:

- **Desktop development with C++**;
- the current Microsoft C++ (MSVC) toolset;
- a Windows 10 or Windows 11 software development kit (SDK).

PowerShell 5 is included with supported Windows versions. PowerShell 7 is
optional. The .NET SDK is needed only by contributors who build the Windows
ARM64 installer; it is not needed for an ordinary source launch.

If you also use Windows Subsystem for Linux (WSL), keep the Windows/MSVC
checkout on an NTFS path and create a separate Linux checkout inside the WSL
filesystem. Do not build the Linux copy from `/mnt/c` or `/mnt/d`; that layout
is slow and the project doctor rejects it before an expensive build. See
[Windows and WSL development](WSL-DEVELOPMENT.md).

### Ubuntu and Debian requirements

Install the native compiler, display, audio, font, shader, Git, and Python
packages:

```text
sudo apt update
sudo apt install build-essential pkg-config libasound2-dev libfontconfig1-dev libxkbcommon-dev libwayland-dev libx11-xcb-dev glslang-tools python3 python3-pip git
```

Package names differ on Fedora, Arch, and other distributions. Use the
equivalent development packages supplied by your distribution. Automexia's
declared Linux release boundary and limitations are recorded in
[Platform support](PLATFORMS.md#linux).

### macOS requirements

Install the Xcode Command Line Tools:

```text
xcode-select --install
```

Signing and notarization tools are needed only for release work. They are not
needed for a normal local source build.

## 1. Get the source

Clone the official repository and enter its root directory:

```text
git clone https://github.com/AmjedAllaya/automexia-terminal.git
cd automexia-terminal
```

If you already have a checkout, do not copy these commands over local work.
Check `git status --short` first and preserve anything you have changed.

## 2. Install the repository tools

Install PyYAML and the exact dependency-policy tool used by the project:

```text
python -m pip install PyYAML
cargo install --locked cargo-deny --version 0.20.2
```

On a system where Python 3 is exposed only as `python3`, use
`python3 -m pip install PyYAML` instead.

The Rust version itself is selected automatically from `rust-toolchain.toml`
when Rust was installed through `rustup`. Do not replace the pinned version
just to make a build pass.

## 3. Check the computer before building

From the repository root, run:

```text
cargo xtask doctor
```

The doctor reports the toolchain, platform requirements, shell support, build
location, and available storage. Fix a reported requirement before continuing.
Optional packaging tools may be shown as missing; they are not required for an
ordinary application launch.

## 4. Complete the first verified launch

Run:

```text
cargo dev
```

This command validates the repository, builds the application, checks the
resulting executable, prepares session-only shell support, and opens Automexia.
A clean first build can take several minutes because the renderer and its
graphics dependencies are compiled locally.

The window opens only after the required checks pass. If the command stops,
read the first failure rather than repeatedly rerunning it. The focused fixes
for common problems are in [Troubleshooting](TROUBLESHOOTING.md).

## 5. Confirm the installation

The first window should show:

- a tab that identifies the real shell or WSL distribution;
- context and the complete working path above the command line;
- a responsive terminal area that accepts normal shell commands;
- a pane footer with text encoding, newline style, grid size, and local time.

The successful `cargo dev` output also reports the checked executable identity
and version. If you later place the executable on `PATH`, you can check it with:

```text
automexia --version
```

In a source-only checkout, the `cargo dev` version check is sufficient. Use the
repository launch command in the next section rather than copying a debug
executable away from its resource tree.

## 6. Start Automexia next time

After one successful complete verification, use the faster incremental path:

```text
cargo automexia
```

Use `cargo ready` when you want the complete contributor gate without opening
a window:

```text
cargo ready
```

The source-launch commands supply the checked-out shell resources only to the
new Automexia session. They do not silently rewrite your persistent shell
profiles.

## Optional persistent shell integration

Automexia works without persistent profile changes. If you explicitly want the
integration available when starting a packaged or directly invoked executable,
review its health first and then install it:

```text
automexia shell-integration doctor
automexia shell-integration install
```

Remove only the marked Automexia integration with:

```text
automexia shell-integration uninstall
```

Use `install --force` only to repair a known broken Automexia-managed block.
See the
[shell workflow guide](user-guide/commands-and-shell.md#session-only-shell-integration)
for when persistent integration is useful and when session-only support is the
better choice.

## Configuration location

Automexia works without a personal configuration file. To create a starter
file without overwriting an existing one, run:

```text
automexia --write-config
```

The default configuration folder is:

| Platform | Folder |
|---|---|
| Windows | `%LOCALAPPDATA%\Automexia\Terminal` |
| macOS | `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` |
| Linux/BSD | `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia` |

Set `AUTOMEXIA_CONFIG_HOME` before launch to use a separate folder, for example
when testing a clean setup. See [Configuration and customization](user-guide/customization.md)
before changing more than a few settings.

## Update a source checkout

First close or save work in Automexia and confirm that the checkout contains no
changes you could lose:

```text
git status --short
```

For a clean checkout that follows its current branch, update without rewriting
history and launch again:

```text
git pull --ff-only
cargo automexia
```

Run `cargo ready` after a large update, toolchain change, or before reporting a
problem. If `git status` is not clean, preserve or commit your work before
updating; this guide does not recommend discarding it.

## Reclaim build space

See how much space the project build directory uses:

```text
cargo storage
```

After closing every Automexia window launched from the checkout, remove only
the repository-owned Cargo build artifacts:

```text
cargo purge
```

The next build will take longer because dependencies must be compiled again.
Configuration, shell profiles, and files outside the project build directory
are not the intended cleanup target.

## Remove a source installation

1. Close Automexia windows started from the checkout.
2. Run `automexia shell-integration uninstall` if you explicitly installed the
   persistent integration.
3. Preserve any configuration, themes, or workflow files you want to keep.
4. Remove the source checkout using your normal file manager or repository
   management workflow.
5. Remove the Automexia configuration folder only if you also want to erase
   your personal settings and generated completion state.

Deleting the checkout does not intentionally delete the separate Automexia
configuration folder. Likewise, removing the configuration folder does not
remove unrelated shell, cloud, SSH, or provider configuration.

## About future stable packages

The repository owns packaging definitions for:

- Windows x86_64 and ARM64 Windows Installer (MSI) and ZIP artifacts;
- a universal macOS application in a disk image (DMG);
- Linux x86_64 and ARM64 DEB, RPM, and tar archives.

Those formats are part of the release plan, not proof that an official stable
download is available today. Stable publication remains blocked until the
remaining brand, signing, native, accessibility, and release checks are
complete. When packages are published, the installation page will add exact
download, signature/checksum verification, upgrade, rollback, and uninstall
steps for each format. See [Release trust](RELEASE-TRUST.md) for the required
artifact guarantees.

## Continue learning

After the first window opens:

1. complete the [Getting started tutorial](GETTING-STARTED.md);
2. follow [Getting started](GETTING-STARTED.md);
3. choose a practical task from the [workflow recipes](user-guide/recipes.md);
4. use the [FAQ](FAQ.md) or [Troubleshooting](TROUBLESHOOTING.md) when something
   is unclear.
