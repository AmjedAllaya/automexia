# Automexia Terminal

Automexia Terminal is a fast, hardware-accelerated desktop terminal focused on
modern development workflows. Version 0.4.0 is a standalone downstream of Rio
with a separate executable, application identity, configuration root, and
release process.

> Stable v0.4.0 publication is intentionally blocked until the final Automexia
> brand kit, Windows signing certificate, Apple signing/notarization credentials,
> and a private conduct-reporting contact are configured. Development builds do
> not reuse Rio artwork.

## Build

Install the Rust toolchain declared in `rust-toolchain.toml`, then run:

```text
cargo xtask doctor
cargo xtask check
cargo build -p automexia-terminal
```

The executable is `target/debug/automexia` (`automexia.exe` on Windows). A
normal build or test must leave tracked files unchanged.

## Verify changes

```text
cargo xtask verify architecture
cargo xtask verify identity
cargo xtask verify provenance
cargo xtask test conformance
cargo xtask ci
cargo xtask package --check
```

The complete command contract and platform prerequisites are documented in
[CONTRIBUTING.md](CONTRIBUTING.md) and [docs/TESTING.md](docs/TESTING.md).

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
