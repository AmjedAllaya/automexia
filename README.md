# Automexia Terminal

Automexia Terminal is a fast, hardware-accelerated desktop terminal focused on
modern development workflows. Version 0.4.0 is a standalone downstream of Rio
with a separate executable, application identity, configuration root, and
release process.

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
metadata, workspace checks, warning-denied Clippy, all tests, and `cargo deny`;
builds Automexia; verifies `automexia --version`; and launches the terminal.
The first run can take several minutes. Exhaustive checks use a dedicated,
non-incremental verification target that is removed whether the gate passes or
returns an ordinary failure; only the reusable application build remains in the
normal Cargo target.

The complete gate requires Python 3 with PyYAML and `cargo-deny`. If either is
missing, `cargo dev` reports it before starting the expensive build. Install
them with your platform package manager or:

```text
python -m pip install PyYAML
cargo install --locked cargo-deny --version 0.20.2
```

For normal day-to-day launches after the repository is known to be healthy:

```text
cargo automexia
```

It rebuilds only changed code, performs a version smoke, and launches Automexia.
Both launch commands return after starting the Automexia process, so the
terminal remains usable and Cargo's build output stays unlocked. Each launch
uses a generation-specific copy under `target/automexia-runtime`; stale copies
are reclaimed automatically on later launches.
Pass terminal arguments after `--`, for example:

```text
cargo automexia -- --working-dir D:\work
```

Before opening a pull request, run the same complete gate without launching a
window:

```text
cargo ready
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

The native liquid-hacker interface, persistent context bar, tab/window
interactions, one-line shell prompt, command timing, semantic output styling,
and focused regression commands are documented in
[docs/LIQUID-HACKER-UX.md](docs/LIQUID-HACKER-UX.md).

On Windows, install or refresh the bundled PowerShell and WSL integrations once
after building, then restart Automexia:

```powershell
.\shell-integration\install-windows.ps1
```

Native PowerShell gains icon-aware `ls` output through a bundled, pipeline-safe
format view and does not require `eza`. In Bash and Zsh, the integration uses an
installed `eza` for icon-aware `ls`, `ll`, and `tree` output and falls back
cleanly when `eza` is unavailable; see
[docs/LIQUID-HACKER-UX.md](docs/LIQUID-HACKER-UX.md#file-and-folder-icons) for
the shortcuts and opt-out.

## Configuration

Automexia uses these roots by default:

- Windows: `%LOCALAPPDATA%\Automexia\Terminal`
- macOS: `~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal`
- Linux/BSD: `$XDG_CONFIG_HOME/automexia`, or `~/.config/automexia`

`AUTOMEXIA_CONFIG_HOME` overrides the root. See
[docs/CONFIGURATION.md](docs/CONFIGURATION.md) for migration and compatibility
details.

## Project status

- v0.4 keeps attributed private `rio-*`, `librio`, Sugarloaf, and related
  engine crate names while all product-facing identity is Automexia.
- v0.5 will extract Automexia-owned application modules and then consider
  grouping inherited engines beneath `engine/`.
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
