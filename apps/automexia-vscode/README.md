# Automexia Terminal for VS Code

**Automexia: Open Docked View** opens an Automexia panel in VS Code with its own
terminal interface and real shell sessions. The v0.4.0 docked preview adds a
per-pane information bar and dynamic tags alongside tabs, split panes, visible
output search, clipboard interaction, a panel command palette, and Liquid
Hacker and Arctic Day themes. It uses Automexia's `librio` terminal core and
native PTY. Platform-specific builds include the companion, the Automexia
desktop runtime, and its command-line tools.

Use desktop VS Code 1.85 or newer and the VSIX for the OS and architecture
where VS Code runs the extension.
In a WSL-connected window, install the Linux VSIX for that distribution's
architecture. The supported package targets are Windows, macOS, and Linux on
x64 and ARM64, with Linux packages for supported desktop and WSL environments.

## Open the docked view

1. In the window where you want the view, choose **Extensions → More Actions
   → Install from VSIX...** and select the matching platform package.
2. Open a trusted workspace. In the Command Palette, run **Automexia: Open
   Docked View**. The **Automexia** panel appears beside VS Code's other panels.
3. Select **New terminal** to open the first shell session.

The docked view uses the shell executable reported by VS Code and supported
startup arguments from an explicit default profile. It opens native shells
in the current extension host, so use a WSL-connected window for Linux shells
and Linux paths inside WSL.

The bundled companion and CLI are selected automatically. An explicit
`automexia.docked.hostExecutable` or `automexia.terminal.binaryDirectory`
setting keeps its override behavior. Clear those settings when upgrading a
hand-configured preview if you want the bundled defaults.

### Build the native companion

Use this section for a source-only development installation or to test a
custom companion. From the checkout root on the matching host, build it using
the repository's [isolated validation workflow](../../docs/TESTING.md).

```text
cargo build --locked --no-default-features -p librio --bin automexia-vscode-host --features vscode-host
```

Source-only installations look for `automexia-vscode-host.exe` on Windows
or `automexia-vscode-host` on macOS, Linux, and WSL in the configured CLI
directory and the extension host's `PATH`. To select an exact binary, set
`automexia.docked.hostExecutable` in **User
Settings (JSON)**, or in **Remote Settings (JSON)** for WSL. Use an absolute
path or a path beginning with `~/`.

```json
{
  "automexia.docked.hostExecutable": "~/src/Automexia checkout/target/debug/automexia-vscode-host"
}
```

For a Windows build, a JSON setting can use
`"C:\\Tools\\Automexia\\automexia-vscode-host.exe"`. Keep the companion at the
configured path before opening the panel. WSL needs a Linux build of the
companion even when VS Code itself runs on Windows.

### Information bar and context tags

Each pane shows the context of its own shell session. Available tags identify
the host system or WSL environment and the shell. The account appears when
Automexia shell integration reports it. The Git tag shows the branch or
abbreviated detached commit when the displayed directory matches a repository
that VS Code already knows about.

The bar updates its directory when the shell sends a valid OSC 7 working
directory report. Until a report arrives, **Started in** identifies the
launch directory and any Git tag describes that starting location. The
extension uses the reports provided by your shell and leaves its startup
files as configured.

In the docked view, use the **…** menu to choose **Information bar preset**.
The companion supplies the desktop presets, including Git-first developer
bar and Compact icon mode. The panel remembers the selection alongside its
theme. It displays the available tags in the preset's order and layout.

### Browse Quick Actions and saved Workspaces

Use **Automexia: Browse Quick Actions** or **Automexia: Browse Saved
Workspaces** from the Command Palette or the docked view's **…** menu. Use the
Quick Pick to search each catalog's summaries. These commands use the bundled
Automexia executable by default and also support a configured custom directory
or a source installation on `PATH`.

Both commands show metadata from the existing desktop catalogs. Use
**Automexia: Open Desktop App** to open the full desktop application in a
separate window. It uses the same executable lookup and starts in the current
workspace or home directory. A WSL-connected window needs a Linux desktop
build and a graphical display available to WSL. The desktop app manages
action execution, remote connections, and Workspace restore.

### Docked preview scope

VS Code supplies the panel location and window controls. Hiding the panel
pauses frame updates and keeps its sessions. Closing a session ends its PTY.
When the shell stops, the pane shows its exit status when available and keeps
the last terminal output. The information bar and command input apply to live
sessions.
Automexia supplies the terminal grid and session controls inside the panel.
Search covers the displayed terminal output. Prompt-attached tags, Kubernetes
and cloud tags, inline image rendering, Connection Hub workflows, Quick Action
execution, and Workspace restore remain later porting work. Native verification
of the packaged preview on Windows, macOS, Linux, and WSL remains pending.

## Use the CLI

The existing integrated terminal profile runs the Automexia CLI with your
VS Code default shell. Platform packages make `amx` and `automexia` available
in the **Automexia CLI** terminal profile and the docked view. In a source-only
installation, the profile can use the default shell's normal `PATH` or a
custom binary directory containing both tools.

### Use the CLI in WSL

Use Microsoft's WSL extension to connect VS Code to the distribution that has
the Linux CLI. A local Windows window whose default profile launches `wsl.exe`
still uses Windows extension settings. A WSL-connected window lets the
extension use Linux paths and the distribution's default terminal profile.

If you need to build the two executables from source, run these commands in
WSL after substituting the path to your checkout on the Linux filesystem.

```bash
cd "/path/to/your/Automexia checkout"
cargo build --locked -p automexia-terminal --bin automexia --bin amx
```

### Use a custom binary directory

If the commands are outside your shell's `PATH`, open **Preferences: Open User
Settings (JSON)** in a native VS Code window or **Preferences: Open Remote
Settings (JSON)** in a WSL-connected window. Set
`automexia.terminal.binaryDirectory` to the directory containing both
executables. A source build normally puts them in `target/debug` under the
checkout. With a custom Cargo target directory, use its `debug` subdirectory.

```json
{
  "automexia.terminal.binaryDirectory": "~/src/Automexia checkout/target/debug"
}
```

On macOS, Linux, and WSL, use an absolute path or a path beginning with `~/`.
An installed Mac app can use `/Applications/Automexia Terminal.app/Contents/MacOS`
if the bundle is in Applications. On Windows, use a fully qualified drive or
UNC path, or start with `~/` or `~\` to use your home directory. For example,
a Windows directory can be written as `"C:\\Tools\\Automexia"` in JSON. Spaces
and Unicode are allowed. If the directory name contains the OS's `PATH`
separator (`;` on Windows or `:` on macOS and Linux), control characters, or
the literal sequence `${`, use a link whose own path avoids those characters.
Open a new Automexia terminal after changing the setting.

VS Code or your shell startup files can replace `PATH` after the extension
prepares the launch environment. If `amx` is missing, add the Automexia
directory to the `PATH` defined by the default profile or shell startup file
that supplies the final value.

Existing explicit settings from v0.1.1 carry over. To keep using Bash in WSL,
select Bash as the default terminal profile for that window. Version 0.1.0 used
`~/automexia/target/debug` when the setting was omitted. If you depended on that
default and the binaries are outside `PATH`, set that directory explicitly.

## Install the VSIX

For command-line installation, run `code --install-extension` in the VS Code
window where the extension will run. Replace the example file name with the
matching package. In WSL, use the Linux path to the package file.

```bash
code --install-extension "/path/to/automexia-0.6.0-win32-x64.vsix"
```

You can also open Extensions, select **More Actions → Install from VSIX...**,
choose the file, and reload if prompted. [Microsoft's remote installation guide](https://code.visualstudio.com/api/advanced-topics/remote-extensions#_installing-a-development-version-of-your-extension)

### Build platform packages

Build on a verified disposable builder with the repository's target toolchain.
Use the existing Automexia release staging for the desktop input. Supply the
Windows or Linux `portable` directory, or the complete macOS
`Automexia Terminal.app`. Build the companion from the same source candidate.

| VSIX target | Rust target |
| --- | --- |
| `win32-x64` | `x86_64-pc-windows-msvc` |
| `win32-arm64` | `aarch64-pc-windows-msvc` |
| `darwin-x64` | `x86_64-apple-darwin` |
| `darwin-arm64` | `aarch64-apple-darwin` |
| `linux-x64` | `x86_64-unknown-linux-gnu` |
| `linux-arm64` | `aarch64-unknown-linux-gnu` |

Example from the checkout root for Windows x64. Use the normal desktop
packaging prerequisites documented in [packaging](../../packaging/README.md),
or supply the verified portable stage from that build.

```powershell
cargo xtask package --target x86_64-pc-windows-msvc
cargo build --locked --release --target x86_64-pc-windows-msvc -p librio --no-default-features --features vscode-host --bin automexia-vscode-host
python tools/ci/package_vscode.py --target win32-x64 --desktop target/package/x86_64-pc-windows-msvc/portable --companion target/x86_64-pc-windows-msvc/release/automexia-vscode-host.exe --out target/vscode/automexia-0.6.0-win32-x64.vsix
```

For Linux, the desktop input is also `target/package/<triple>/portable`.
For macOS, use the release staging bundle `dist/Automexia Terminal.app` or
the complete app extracted from the reviewed DMG.

The assembler checks the native architecture and required desktop resources,
preserves signed bytes and Unix executable permissions, and writes a
platform-targeted VSIX. Linux uses the portable package's system-library and
libc baseline. Opening the full desktop app from WSL also requires a graphical
display for the distribution. Verify the exact installed package in a clean
native guest before distribution. [VS Code platform packaging guide](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#platform-specific-extensions)

### Build a source-only development VSIX

In a verified disposable environment with Node.js 22 or newer, run from the
checkout root:

```bash
cd apps/automexia-vscode
mkdir -p ../../target/vscode
npx --yes @vscode/vsce@4.1.1 package --no-dependencies --allow-missing-repository \
  --out ../../target/vscode/automexia-0.6.0-source.vsix
```

Use the companion and CLI setup above with this development package.

## Open the CLI terminal profile

1. Open a trusted project folder. Use **Terminal: Select Default Profile** to
   choose the shell you want Automexia to use. Then open **View → Terminal**.
2. Select **Automexia CLI** from the arrow next to **+**.
3. At the prompt in that panel, run the version check.

   ```bash
   amx --version
   ```

`amx` prints the installed Automexia version, and the shell stays open in the
panel. If the command is missing, check the shell's `PATH` or set the custom
binary directory above. If you previously added the
**Automexia (WSL)** launcher to `terminal.integrated.profiles.linux`, you can
remove that entry from Remote Settings to keep a single dropdown option.

## Contributor checks

From `apps/automexia-vscode` in a verified disposable Linux guest with Node.js 22 or
newer, run the provider checks.

```bash
npm test
npm run bench
```

The tests and benchmark cover the profile provider, native runtime lookup,
docked terminal bridge, and context validation and Git matching. Run the
assembler fixture in the same type of environment from the checkout root.

```sh
python -m unittest discover -s tools/ci -p test_package_vscode.py
```

Install each platform VSIX in a clean desktop profile without Automexia on
`PATH` or in user settings. In WSL, use a separate WSL 2 test distribution and
the matching Linux package. Check `amx --version`, docked shell input and
resizing, independent information bars and presets, catalog browsing, desktop
launch, and process cleanup. Check executable permissions and, where signing
applies, native signatures after VS Code extracts the package. Repeat with a
source-only extension and explicit binary overrides to cover migration.

In a disposable Windows guest, build the native companion, then set
`AUTOMEXIA_VSCODE_TEST_HOST` to the absolute path of the resulting `.exe`.
Run `npm run test:windows-companion` from `apps/automexia-vscode`. The probe
compares the normal Windows PowerShell launch with `-NoProfile`, checks for a
prompt and context tags, and submits a test command through the companion's
typed input protocol before verifying the shell exit status. If both launches
pass, repeat the interaction through the installed VSIX in the guest to check
webview focus and rendering.
