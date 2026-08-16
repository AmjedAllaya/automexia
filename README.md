# Automexia Terminal

Automexia Terminal is a fast, hardware-accelerated desktop terminal focused on
modern development workflows. Version 0.4.0 is a standalone downstream of Rio
with a separate executable, application identity, configuration root, and
release process.

Start with the [documentation home](docs/index.md) for a guided path through
installation/building, features, configuration, shortcuts, shells,
troubleshooting, platform support, architecture decisions, testing, and
release operations.

> The supplied Automexia raster mark is integrated for development and nightly
> packages. Stable v0.4.0 publication remains blocked until its vector variants
> and rights approval, Windows signing certificate, Apple signing/notarization
> credentials, and a private conduct-reporting contact are configured. No build
> reuses Rio artwork.

## Build, verify, and run

Install the Rust toolchain declared in `rust-toolchain.toml`, then use one
command for the complete local workflow:

```text
cargo dev
```

This checks required tools and repository formats; verifies identity,
architecture, provenance, packages, and brand assets; runs rustfmt, locked
metadata, native shell-integration validation, workspace checks, warning-denied
Clippy, all tests, and `cargo deny`; builds Automexia; verifies
`automexia --version`; installs or refreshes the shell integration; and launches
the terminal. On Windows that automatic phase prepares PowerShell, Command
Prompt, and every detected user WSL distribution. On macOS/Linux it prepares Bash, Zsh,
and user-local terminfo. No separate integration command or restart is needed.
The first run can take several minutes. Exhaustive checks use a dedicated,
non-incremental verification target that is removed whether the gate passes or
returns an ordinary failure; only the reusable application build remains in the
normal Cargo target.

No Automexia window appears until those checks pass. The workflow prints its
launch and verification phases and keeps compiler/build-script progress live,
including long WGPU and native shader compilation. This distinguishes active
work from a stalled process.

The complete gate requires Python 3 with PyYAML and `cargo-deny`. If either is
missing, `cargo dev` reports it before starting the expensive build. Install
them with your platform package manager or:

```text
python -m pip install PyYAML
cargo install --locked cargo-deny --version 0.20.2
```

On Windows, `cargo xtask doctor` also reports the installed PowerShell host,
newest available PSReadLine module, and PowerShell 7 availability. Its history
advisory is informational: it never installs or updates shell software.

When developing inside WSL, keep the Linux checkout and Cargo target under the
WSL filesystem (for example `~/src/automexia-terminal`), not under
`/mnt/c` or `/mnt/d`. Keep this NTFS checkout for Windows/MSVC,
ConPTY, GPU, and packaging work. `cargo xtask doctor` reports the effective
workspace I/O mode, and compilation-heavy project workflows fail early on a
cross-filesystem WSL checkout instead of spending hours in avoidable metadata
I/O. The supported two-checkout workflow and diagnostic override are documented
in [Windows and WSL development](docs/WSL-DEVELOPMENT.md).

For normal day-to-day launches after the repository is known to be healthy:

```text
cargo automexia
```

It rebuilds only changed code, performs a version smoke, exposes the
repository-owned integration to the new child shell, and launches Automexia.
It does not repeat the exhaustive isolated gate and is the recommended command
for normal launches after `cargo ready` or `cargo dev` has passed once.
Both launch commands return after starting the Automexia process, so the
terminal remains usable and Cargo's build output stays unlocked. A normal
launch never writes profiles, runs an integration installer, changes PowerShell
execution policy, or provisions WSL. Each launch
uses a generation-specific copy under `target/automexia-runtime`; stale copies
are reclaimed automatically on later launches. Missing session resources leave
the user's shell unmodified. Persistent integration for nested shells outside
Automexia is an explicit
`automexia shell-integration install` operation and can be inspected or
removed with `doctor`/`uninstall`.
Pass terminal arguments after `--`, for example:

```text
cargo automexia -- --working-dir D:\work
```

Before opening a pull request, run the same complete gate without launching a
window:

```text
cargo ready
```

For the deeper Phase 0 evidence profile, including pinned Nextest/JUnit,
property/model checks, hard subprocess deadlines with process-tree cleanup,
privacy-bounded host/resource/coverage evidence, and explicit external-gate
status, run:

```text
cargo qa
cargo qa --bundle
```

The executable remains available at `target/debug/automexia`
(`automexia.exe` on Windows). Advanced scoped `cargo xtask` commands are
documented in [CONTRIBUTING.md](CONTRIBUTING.md) and
[docs/TESTING.md](docs/TESTING.md).

The workflow refuses to start an exhaustive gate with less than 12 GiB free or
an application build with less than 4 GiB free on the target filesystem. Check
where build storage is being used with `cargo storage`. To remove all Cargo
build artifacts, close running Automexia windows and run `cargo purge`.
Brand-source and platform-export rules are in
[docs/BRANDING.md](docs/BRANDING.md).
The latest plan-by-plan implementation evidence and explicit external release
blockers are recorded in [docs/READINESS-AUDIT.md](docs/READINESS-AUDIT.md).

The native liquid-hacker interface, responsive pane-local tab rail,
passive session footers, per-command operational context, tab/window
interactions, shell prompt, command timing, semantic output styling, and focused regression commands are documented in
[docs/LIQUID-HACKER-UX.md](docs/LIQUID-HACKER-UX.md).

Both `cargo dev` and `cargo automexia` use repository-owned shell resources
only in the child session. `cargo ready`, `cargo check`, CI, and normal
launch remain non-mutating with respect to user profiles. Signed release
resources are loaded from the installed package; persistent profile support is
available only through the explicit application maintenance command.

Native PowerShell gains icon-aware `ls` output through a bundled, pipeline-safe
format view and does not require `eza`. Typing bare `cmd` or `cmd.exe` from an
integrated PowerShell pane opens Command Prompt inside that same Automexia pane;
CMD receives the branded full-path/lambda prompt, live shell/user/path context,
and icon-aware `ls`/`ll`. Identity is reasserted on every CMD prompt, and
PowerShell restores its own metadata immediately after `exit`; built-in `dir`
and explicit `cmd /c` behavior stay native. In Bash and Zsh, the integration
uses an
installed `eza` for icon-aware `ls`, `ll`, and `tree` output and falls back
cleanly when `eza` is unavailable. A bundled compatibility layer gives older
Ubuntu/WSL eza 0.18.x releases the same colored composite folder badges as
PowerShell without changing filenames or piped output; see
[docs/LIQUID-HACKER-UX.md](docs/LIQUID-HACKER-UX.md#file-and-folder-icons) for
the shortcuts, sensitive/config/log/source/test/build category vocabulary, and
opt-out.

Clone the active PowerShell, Command Prompt, Bash, Zsh, or WSL session into an
independent right/lower split with the original Automexia shortcuts
`Ctrl`+`R` / `Ctrl`+`D`. `Ctrl`+`Alt`+`R` sends history search to the shell and
`Ctrl`+`Alt`+`D` sends EOF/logout. Fresh default-shell splits use
`Ctrl`+`Shift`+`R` / `Ctrl`+`Shift`+`D` on Windows/Linux/BSD and `Cmd`+`D` /
`Cmd`+`Shift`+`D` on macOS.

On Windows, Linux, and BSD, `Ctrl`+`T` adds a window-level tab and
`Ctrl`+`Shift`+`T` adds an independent tab inside the selected split/session.
macOS uses `Cmd`+`T` and `Cmd`+`Shift`+`T` for those two scopes. Pane-local tabs preserve the selected
shell/profile, WSL identity, and working directory while owning independent
PTYs. When a pane has multiple local tabs, their controls live inside that pane
and do not resize its siblings. Navigate panes geometrically with
`Alt`+Arrow on Windows/Linux/BSD (`Cmd`+`Alt`+Arrow on macOS), or cycle them
with `F6` / `Shift`+`F6` (`Cmd`+`]` / `Cmd`+`[` on macOS).
`Alt`+`PageDown` / `Alt`+`PageUp` switches tabs only inside the selected pane;
macOS uses `Cmd`+`Alt`+`]` / `Cmd`+`Alt`+`[`.
`Ctrl`+`Tab` remains reserved for window-level tabs. See
[configuration](docs/CONFIGURATION.md). Ghostty compatibility is a
future opt-in profile tracked separately in the
[compatibility roadmap](docs/GHOSTTY-COMPATIBILITY-ROADMAP.md); it is not the
implicit Automexia default.

Select terminal text without reaching for the mouse on every supported OS:
`Shift`+Arrow extends by one visible cell or row, and
`Ctrl`+`Shift`+Left/Right extends by a Unicode-aware word boundary. The first
press anchors at the live terminal cursor; later presses grow or reverse the
same selection. Search, Vi mode, image-preview browsing, and explicit user
bindings retain their established ownership.

## Image previews

Automexia renders application-driven Sixel, Kitty Graphics (including Unicode
placeholders), and iTerm2 inline images. It also provides local quick look for
paths printed by ordinary commands: hover a filename, click it to pin, then use
the arrow keys to browse other visible images. `Esc` closes the card. Selecting
a path and pressing `Ctrl`+`Alt`+`I` (`Cmd`+`Alt`+`I` on macOS) and the
**Preview Selected Image** palette action remain keyboard alternatives.
Decoding is local-only, bounded, asynchronous, route-scoped, and responsive
across pane sizes. See
[image previews](docs/IMAGE-PREVIEWS.md) for supported tools, formats, security
limits, and focused tests.

## Configuration

Automexia uses these roots by default:

- Windows: `%LOCALAPPDATA%\Automexia\Terminal`
- macOS: `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal`
- Linux/BSD: `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia`

`AUTOMEXIA_CONFIG_HOME` overrides the root. See
the [configuration reference](docs/CONFIGURATION.md) for the complete
schema, defaults, limits, platform overrides, reload behavior, migration, and
compatibility details.

## Project status

- v0.4 keeps attributed private `rio-*`, `librio`, Sugarloaf, and related
  engine crate names while all product-facing identity is Automexia.
- v0.5.0 will perform the smallest behavior-preserving API/runtime/UI-model
  extraction required to ship an optional first-party `devops-ssh` extension
  through the system OpenSSH client. v0.5.1 then adds separately enabled AWS,
  Azure, Google Cloud, Kubernetes, OpenShift, and infrastructure extensions with
  per-PTY environment isolation. These are planned, not current v0.4 features;
  see the [roadmap](docs/ROADMAP.md) and
  [SSH/DevOps/multi-cloud architecture](docs/SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md).
- The v0.5 [Command Productivity](docs/COMMAND-PRODUCTIVITY.md) track has
  completed CP1 shell-native completion health and bounded explicit provider
  refresh. Persistent typed Quick Actions, opt-in non-colliding aliases, and
  reviewed DevOps packs remain later phases; built-in short aliases remain
  disabled by default.
- Grouping inherited engines beneath `engine/` remains lower priority than the
  release-critical v0.5 extension/session boundary.
- Third-party extension downloads, a public extension SDK, and Wasm sandboxing
  remain out of scope until the documented v0.6 milestone.

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md),
[SUPPORT.md](SUPPORT.md), and [GOVERNANCE.md](GOVERNANCE.md). All commits must
carry a DCO `Signed-off-by` line.

## License and provenance

Automexia Terminal is MIT licensed. It preserves Rio's full Git history and
copyright notice. See [NOTICE.md](NOTICE.md) and [UPSTREAM.md](UPSTREAM.md) for
the exact fork point and upstream-port policy.
