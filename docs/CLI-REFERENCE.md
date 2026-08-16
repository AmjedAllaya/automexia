# CLI and automation reference

Automexia exposes two command layers: the installed `automexia` application and
repository-owned `cargo`/`cargo xtask` contributor automation.

## Application command

```text
automexia [OPTIONS]
```

| Option | Meaning |
|---|---|
| `-e, --command <PROGRAM> [ARGS...]` | Launch a command instead of the configured shell. It must be the final option because all remaining values are command arguments. |
| `-w, --working-dir <PATH>` | Start the shell in an existing directory. Invalid paths are rejected with a warning and the safe default is used. |
| `--write-config [PATH]` | Create a starter configuration at `PATH`, or at the platform configuration root when no path is supplied. Existing files are not overwritten. |
| `--enable-log-file` | Write logs under the Automexia configuration root for this launch. Review logs before sharing because terminal paths and process diagnostics may be sensitive. |
| `--title-placeholder <TEXT>` | Override the initial title placeholder. Shell/application title sequences can update the live title later. |
| `--app-id <ID>` | Override Wayland `app_id` / X11 `WM_CLASS` on Linux/BSD. |
| `-h, --help` | Print application help. |
| `-V, --version` | Print the Automexia version. |

Non-GUI shell maintenance commands are explicit:

| Command | Mutation |
|---|---|
| `automexia shell-integration doctor` | None. Reports the validated session resource root and persistent state. |
| `automexia shell-integration install [--force] [--quiet]` | Installs/repairs only the marked persistent integration after direct user invocation. |
| `automexia shell-integration uninstall [--quiet]` | Removes only Automexia-owned persistent blocks and files. |

On Windows these commands honor the effective PowerShell execution policy;
Automexia never supplies an execution-policy bypass.

Example:

```text
automexia --working-dir D:\work -e pwsh -NoLogo
```

The executable has no network-management subcommands in v0.4. SSH and cloud
sessions use the selected shell and system tools; first-party managed SSH is a
v0.5 roadmap item.

## Daily Cargo aliases

| Command | Mutates profiles? | Result |
|---|---:|---|
| `cargo dev [-- APP_ARGS...]` | No | Complete verification, debug build, version smoke, then detached launch with session-only integration. |
| `cargo automexia [-- APP_ARGS...]` | No | Incremental debug build, smoke, then detached launch with session-only integration. |
| `cargo ready` | No | Complete contributor gate without launching. |
| `cargo ci` | No | Alias for the full non-launching CI gate. |
| `cargo qa` | No | Full Phase 0 evidence profile. |
| `cargo storage` | No | Report target location, free space, aggregate size, and largest target children. |
| `cargo purge` | Yes | Cargo's built-in clean; removes workspace build artifacts after Automexia windows close. |

`cargo dev` is intentionally exhaustive and may be slow on a cold checkout.
Use `cargo automexia` for the ordinary edit/build/run loop after `cargo ready`
has passed.

## Focused xtask commands

| Command | Purpose |
|---|---|
| `cargo xtask dev [-- APP_ARGS...]` | Run the complete contributor preflight and then launch Automexia with optional application arguments. |
| `cargo xtask ready` | Run the release-readiness validation without launching the application. |
| `cargo xtask run [-- APP_ARGS...]` | Launch the already-built application with optional application arguments. |
| `cargo xtask doctor` | Report Rust/tools, host shell, PowerShell health, packaging prerequisites, target storage, and WSL filesystem placement. |
| `cargo xtask completion COMMAND [OPTIONS]` | Manage CP1 native-shell completion using one of the exact operations below. |
| `cargo xtask completion doctor` | Read-only provider/cache/shell support health; never invokes provider definitions. |
| `cargo xtask completion refresh --provider ID --shell SHELL [--allow-native-override]` | Explicitly generate one reviewed provider artifact within strict process/output limits. The override flag is PowerShell-only. |
| `cargo xtask completion remove --provider ID --shell SHELL` | Remove only the fixed files for one managed artifact. |
| `cargo xtask completion enable` / `disable` | Toggle managed artifacts globally; native shell behavior remains available. |
| `cargo xtask storage` | Same storage report as `cargo storage`. |
| `cargo xtask check` | Locked metadata, formatting, repository contracts, workspace checks, Clippy, tests, dependency policy, build, and smoke without launch. |
| `cargo xtask ci` | Complete CI gate. |
| `cargo xtask qa --full [--bundle]` | Deep bounded evidence run; optional privacy-reviewed report bundle. |
| `cargo xtask verify architecture` | Enforce dependency, threading, prompt metadata, renderer, shell, and capability boundaries. |
| `cargo xtask verify identity` | Reject non-allowlisted user-facing Rio identity. |
| `cargo xtask verify provenance` | Protect licenses, notices, fork attribution, and private crate publication policy. |
| `cargo xtask verify all` | Run all repository verification scopes plus Phase 0 assurance contracts. |
| `cargo xtask test conformance` | Run VT/Unicode/terminal conformance fixtures. |
| `cargo xtask test resize-stress [--native-gui]` | Deterministic prompt/reflow stress; optional real Windows GUI/ConPTY storm. |
| `cargo xtask test image-rendering [--native-gui]` | Decoder, preview, cache, renderer, and optional native GUI equivalence/lifetime checks. |
| `cargo xtask test image-decoder-fuzz [--seconds N]` | Run the image-decoder libFuzzer campaign through nightly Rust; Windows stages work onto WSL-native storage. |
| `cargo xtask test session-clone [--native-windows\|--native-wsl]` | Descriptor, route, quoting, and optional native clone lifecycle tests. |
| `cargo xtask package --check` | Validate package metadata and prerequisites without publishing. |
| `cargo xtask package --target TARGET` | Build the requested release target/package inputs. Replace `TARGET` with the Rust target triple. |
| `cargo xtask release --version VERSION` | Assemble changelog fragments or validate tagged release preflight. Replace `VERSION` with the release version; the command never bypasses signing/release gates. |

All verification commands are non-mutating unless their name explicitly
denotes launch, provisioning, generation, packaging, release assembly, or
cleanup. Full test ownership and expected duration are in
[Testing and verification](TESTING.md).

## Environment variables

| Variable | Scope |
|---|---|
| `AUTOMEXIA_CONFIG_HOME` | Override the complete writable product root. |
| `AUTOMEXIA_LOG_LEVEL` | Override configured log level. |
| `AUTOMEXIA_SHELL_INTEGRATION` | Marker injected into child shells; user configuration should not spoof it. |
| `AUTOMEXIA_SHELL_INTEGRATION_ROOT` | Internal validated package/development resource root. Release builds ignore arbitrary inherited/configured values. |
| `AUTOMEXIA_COMPLETION_DISABLED=1` | Disable managed completion for the current shell start without changing cached files. |
| `CARGO_TARGET_DIR` | Relocate Cargo artifacts; keep it native to the active OS. |
| `AUTOMEXIA_KEEP_VERIFY_TARGET=1` | Diagnostic-only retention of the isolated exhaustive target. |
| `AUTOMEXIA_VERIFY_MIN_FREE_GIB` | Override the 12 GiB exhaustive-gate minimum. |
| `AUTOMEXIA_BUILD_MIN_FREE_GIB` | Override the 4 GiB app-build minimum. |
| `AUTOMEXIA_TARGET_WARN_GIB` | Override the target-size warning. |
| `AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT=1` | Acknowledge, but does not fix, a one-off build under `/mnt/<drive>`. |
| `RIO_CONFIG_HOME`, `RIO_LOG_LEVEL` | Deprecated v0.4 read-only/value migration fallbacks; removed in v0.5. |

Do not lower storage guards in routine development or CI. See
[Configuration](CONFIGURATION.md) and [WSL development](WSL-DEVELOPMENT.md) for
precedence and lifecycle details.
