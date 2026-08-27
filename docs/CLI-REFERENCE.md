# CLI and automation reference

Automexia exposes two command layers: the installed `automexia` application and
repository-owned `cargo`/`cargo xtask` contributor automation.

## Application command

```text
automexia [OPTIONS] [COMMAND]
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

## Ghostty compatibility inspection and migration

Inspection exits before GUI initialization and never launches Ghostty:

~~~text
automexia --list-actions [--aliases] [--unavailable] [--json]
automexia --list-keybinds [--profile automexia|ghostty|ghostty-1.3]
  [--platform linux-bsd|macos|windows] [--origin ORIGIN]
  [--effective] [--shadowing] [--explain TRIGGER_OR_ACTION] [--json]
~~~

The synthetic macOS Ghostty profile fails closed until its native fixture is
reviewed. Windows output is labeled as Automexia's deterministic adaptation.
Stable JSON includes profile identity, origins, policies, diagnostics, and
registry statistics.

Ghostty migration is dry-run by default and reads only bounded keybinding and
include directives:

~~~text
automexia migrate ghostty [--input PATH] [--dry-run] [--json]
automexia migrate ghostty [--input PATH] [--output PATH] --apply --confirm
~~~

Apply requires explicit confirmation, refuses an existing typed section,
validates the complete result, creates a backup, and publishes atomically. It
never evaluates Ghostty configuration as code or invokes a shell or Ghostty
process. See [Ghostty keyboard compatibility](GHOSTTY-KEYBOARD-COMPATIBILITY.md).

Non-GUI shell maintenance commands are explicit:

| Command | Mutation |
|---|---|
| `automexia shell-integration doctor` | None. Reports the validated session resource root and persistent state. |
| `automexia shell-integration install [--force] [--quiet]` | Installs/repairs only the marked persistent integration after direct user invocation. |
| `automexia shell-integration uninstall [--quiet]` | Removes only Automexia-owned persistent blocks and files. |

On Windows these commands honor the effective PowerShell execution policy;
Automexia never supplies an execution-policy bypass.

Quick Action administration is bounded and dry-run first:

| Command | Mutation and disclosure |
|---|---|
| `automexia actions list [--json]` | None. Lists metadata and revision; command templates are omitted. |
| `automexia actions show ACTION_ID [--json]` | None. Explicitly reveals the selected reviewed template. |
| `automexia actions put ONE_ACTION.toml` | None. Validates and previews exactly one global-user or shell-user action. |
| `automexia actions put ONE_ACTION.toml --apply --expected-revision N [--replace]` | CAS-protected create/update after explicit review. |
| `automexia actions import EXPORT.toml` | None. Validates size, schema, digest, scopes, paths, and conflicts. |
| `automexia actions import EXPORT.toml --apply --expected-revision N [--replace-conflicts] [--allow-machine-paths]` | CAS-protected import with separate conflict/path consent. |
| `automexia actions export DESTINATION [--overwrite] [--include-machine-paths] [--json]` | Writes one private atomic portable transfer; machine paths are excluded by default. |
| `automexia actions remove ACTION_ID [--apply --expected-revision N]` | Preview by default; CAS-protected removal only with both apply and revision. |
| `automexia actions recover PREVIOUS_REVISION [--apply]` | Preview by default; explicitly restore only the validated private previous generation. |
| `automexia actions doctor [--json]` | None. Reports redacted health, revision, and count. |

Persistent user aliases are explicit and dry-run first:

| Command | Mutation and disclosure |
|---|---|
| `automexia aliases list [--shell SHELL] [--json]` | None. Lists saved projections and activation state without revealing generated source. |
| `automexia aliases preview [--shell SHELL] [--show-source] [--json]` | None. Compiles all shells from a strictly read-only source; source disclosure requires `--show-source`. |
| `automexia aliases test [--shell SHELL] [--json]` | None. Verifies compiler invariants and each installed native parser in an isolated temporary root. |
| `automexia aliases enable ACTION_ID --name NAME --shell SHELL... [policy flags]` | Preview only. Reports revision, generation, source, collisions, completion/tool health, and decisions. |
| `automexia aliases enable ... --apply --expected-revision N --expected-generation DIGEST` | Source-CAS and generation-CAS protected activation after explicit review; mutating risk and exact-owner override have separate consent flags. |
| `automexia aliases disable ACTION_ID [--shell SHELL] [apply/CAS flags]` | Preview by default; applied changes republish all five shells atomically. |
| `automexia aliases rename ACTION_ID NAME [apply/CAS flags]` | Preview by default; applied changes republish all five shells atomically. |
| `automexia aliases regenerate [--apply --expected-generation DIGEST] [--json]` | Preview by default; verifies and publishes one immutable generation without changing canonical actions. |
| `automexia aliases disable-all [--apply --expected-generation DIGEST]` | Changes only the activation pointer; saved actions and generations remain. |
| `automexia aliases rollback CURRENT_GENERATION [--apply] [--json]` | Preview by default; swaps only the authenticated current and retained previous generation. |
| `automexia aliases doctor [--json]` | Strictly read-only. Reports source/generation/integrity/transaction health and performs no repair. |
| `automexia aliases reload --shell SHELL` | None. Prints the explicit shell-native reload command; CMD requires a new session. |

Supported `SHELL` values are `powershell`, `bash`, `zsh`, `fish`, and `cmd`.
Applied operations fail fast on stale revisions/generations and never execute an
action or provider. Native definitions win unless authenticated consent for the
same still-observed owner is revalidated. Shell aliases/functions whose exact
restoration cannot be proven are never overridden.
The Command Center's **Quick Actions** entry provides search, placeholders,
risk/conflict review, and explicit **Insert without Enter** or copy. It never
executes a command. Workspace actions, secret expansion, and exact launch are
disabled in CP2.2.

Example:

```text
automexia --working-dir D:\work -e pwsh -NoLogo
```

The executable has no network-management subcommands in v0.4. SSH and cloud
sessions use the selected shell and system tools; first-party managed SSH is a
v0.5 roadmap item.

## Managed workspace review commands (M6)

`automexia workspaces` is the public, nonexecuting manager for the private
Connection Library. Reads are bounded and reject links; writes are preview-first
and require explicit `--apply` plus the current library/entity revisions. JSON
input is strict, migration and recovery are reviewed, and no subcommand can
start a process, create a PTY, connect, broadcast, or send Enter.

| Command | Result |
|---|---|
| `automexia workspaces list [--json]` | List public workspace metadata and the current library revision. |
| `automexia workspaces show <id> [--json]` | Show one saved declarative topology and its public bindings. |
| `automexia workspaces put <file> [--json]` | Preview a strict workspace JSON edit. Add `--apply --expected-revision <library> --expected-entity-revision <workspace>` to commit it; use entity revision `0` only for creation. |
| `automexia workspaces remove <id> --entity-revision <workspace> [--json]` | Preview removal. Add `--apply --expected-revision <library>` to commit the exact reviewed revision. |
| `automexia workspaces restore <id> --generation <n> [--json]` | Review exact current profile bindings for a fresh restore generation. Reconnect and resume stay off. |
| `automexia workspaces recipe-plan --profile <id> --generation <n> [--no-hooks] [--context <file>] [--json]` | Review the authoritative ordered typed plan. `--no-hooks` is explicit recovery intent; context JSON is bounded and strict. |
| `automexia workspaces broadcast <id> --command-file <file> [--arm-duration-ms <n>] [--json]` | Review one bounded single-line exact command and exact targets from a regular file. The command is never accepted as an argument and execution stays off. |
| `automexia workspaces migrate [--json]` | Preview an in-memory migration of the primary schema-1 document; add `--apply --expected-revision <library>` to persist by CAS. A previous-generation recovery candidate fails with `recovery_required`. |
| `automexia workspaces recover <previous-revision> [--json]` | Explicitly preview an available previous generation; add `--apply` only when the primary is absent or rejected. This is the only command that consumes previous-generation recovery state. |
| `automexia workspaces doctor [--json]` | Report load/recovery state and the D3/M5 activation blockers without probing the network or tools. |

There is still no public managed `connect`, `run`, or `tunnel` command. Continue
using system OpenSSH in the shell for actual connections. Accepted ADR 0023 does
not bypass ADR 0012/D3/M5: managed recipe, restore, and broadcast execution stays
fail-closed until protected attestation and native lifecycle evidence pass.
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
| `cargo xtask visual-diff --expected PATH --actual PATH --config PATH --diff PATH --report PATH` | Compare bounded same-size PNG evidence with the reviewed tolerance/mask policy, then atomically write a heatmap and path-free JSON report. The command fails when dimensions, masks, limits, or changed-pixel ratio violate policy. |
| `cargo xtask check` | Locked metadata, formatting, repository contracts, workspace checks, Clippy, tests, dependency policy, build, and smoke without launch. |
| `cargo xtask ci` | Complete CI gate. |
| `cargo xtask qa --full [--bundle]` | Deep bounded evidence run; optional privacy-reviewed report bundle. |
| `cargo xtask verify architecture` | Enforce dependency, threading, prompt metadata, renderer, shell, and capability boundaries. |
| `cargo xtask verify identity` | Reject non-allowlisted user-facing Rio identity. |
| `cargo xtask verify provenance` | Protect licenses, notices, fork attribution, and private crate publication policy. |
| `cargo xtask verify keybindings` | Verify the checked-in Ghostty 1.3.1 provenance, generated manifests, profiles, and reference tables without changing them. |
| `cargo xtask generate keybindings <--version 1.3.1\|--check>` | Regenerate the pinned Ghostty 1.3.1 artifacts from reviewed native fixtures, or byte-check them with `--check`. Native macOS generation fails closed until its external fixture is available. |
| `cargo xtask test keybindings` | Run the focused keybinding compiler, registry, dispatch, migration, UI-model, topology-history, bounded-selection, and generated-artifact checks. |
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

## Reviewed DevOps packs (CP3.2)

```text
automexia packs list [--json]
automexia packs show PACK_ID [--json]
automexia packs doctor [--json]
automexia packs doctor PACK_ID [--missing | --tool-version TEXT] [--completion-shell SHELL]... [--json]
automexia packs enable PACK_ID ACTION_ID [--json]
automexia packs enable PACK_ID ACTION_ID --apply --expected-revision N [--json]
```

`list`, `show`, and `doctor` are read-only and never start a provider. With
no pack selected, doctor reports `registry-ready` only; it does not imply that
any provider is installed or healthy. Pack health uses only the supplied
Missing/Detected/Unobserved observation and text output names every missing
completion shell.

`enable` previews by default and shows the exact argv token array, display text,
effect, risk, documentation URL, alias eligibility, registry digest, and reusable
revision. Apply rejects a stale revision before creating the store, refuses to
overwrite an existing action, and uses the same private compare-and-swap store
as other Quick Actions. It always leaves the alias disabled. Eligible inspection
actions require a separate explicit `automexia aliases enable` review; context-
changing, authentication, destructive, and privileged pack actions are rejected
there.

## Native alias imports and workspace task bridges (CP3.3)

```text
automexia actions import-aliases --source SOURCE --input INVENTORY --name NAME [--name NAME] [--action-id NAME=ACTION_ID] [--json]
automexia actions import-aliases --source SOURCE --input INVENTORY --name NAME --apply --expected-revision N [--replace-conflicts] [--json]
automexia actions task-put --workspace PATH --runner RUNNER --task TASK --id ACTION_ID --display-name TEXT --shell SHELL [--shell SHELL] [--description TEXT] [--risk RISK] [--json]
automexia actions task-put --workspace PATH --runner RUNNER --task TASK --id ACTION_ID --display-name TEXT --shell SHELL --apply --expected-revision N [--replace] [--json]
automexia actions task-remove --workspace PATH --id ACTION_ID [--json]
automexia actions task-remove --workspace PATH --id ACTION_ID --apply --expected-revision N [--json]
automexia actions workspace-trust --workspace PATH [--json]
automexia actions workspace-trust --workspace PATH --apply --expected-trust-revision N [--json]
automexia actions workspace-revoke --workspace PATH [--json]
automexia actions workspace-revoke --workspace PATH --apply --expected-trust-revision N [--json]
automexia actions workspace-doctor --workspace PATH [--json]
```

`SOURCE` is one of `powershell`, `bash`, `zsh`, `fish`, `cmd`, or `git`.
`RUNNER` is `just`, `task`, or `mise`; `SHELL` is repeatable and uses the
supported Quick Action shell names. Workspace task risk defaults to `mutating`
and cannot be lowered to read-only.

All mutating CP3.3 commands preview by default. Apply requires the revision
printed by that preview. Alias import additionally requires at least one exact
`--name`; repeated names and missing/rejected aliases fail. `--action-id` is an
explicit native-name-to-portable-ID rename. Existing IDs fail unless
`--replace-conflicts` is supplied with apply. The input is a bounded regular
file opened without following a link. Automexia never generates the inventory
or starts a shell/Git/provider command.

`task-put` writes one exact named insert-only bridge to
`.automexia/actions.toml`; `--replace` is required for rename/conflict
replacement. `task-remove` deletes one stable bridge. Neither command lists
tasks, parses recipes, runs a task, reads credentials, or accesses the network.

`workspace-trust` binds the exact current workspace identity, source digest, and
source revision in the private path-free trust store. Any source mutation makes
the receipt stale until the new source is reviewed and trusted. Revoke removes
the receipt immediately through trust-store CAS. `workspace-doctor` is read-only
and reports redacted identity/digest/revision/trust status without a workspace
path. Read-only runtime lookup does not create trust storage.

Trusted workspace actions remain insert-only and unaliased. Search uses bounded
background ancestor/cache reconciliation; review and insert/copy recheck
short-lived route authorization. Changed, linked, malformed, revoked, expired,
or unresolvable WSL guest state fails closed with a refresh-and-review message.
