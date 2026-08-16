# Command Productivity: Completion and Quick Actions

Status: CP0 architecture baseline and CP1 native-completion activation accepted; CP2-CP6 capabilities remain
planned for v0.5.x and later. This document does not claim that autocomplete,
Quick Actions, generated aliases, or provider-aware candidates currently ship.

## Purpose

Automexia should make frequent DevOps commands fast to discover and reuse
without replacing the mature command-line editors that already own interactive
input. This track covers two related capabilities:

1. reliable, shell-native command completion; and
2. persistent, reviewable Quick Actions that may optionally expose short shell
   aliases.

The implementation must preserve normal PowerShell, Bash, Zsh, Fish, and CMD
semantics, remain useful when Automexia integration is disabled, and never turn
completion into an implicit network, authentication, or secret-reading channel.

## Executive decision

- PSReadLine, GNU Readline, ZLE, Fish, and DOSKEY remain the owners of the
  editable command buffer, completion invocation, history, and cursor.
- Automexia provisions and diagnoses official completion integrations; it does
  not scrape the rendered terminal grid to infer a command.
- Reusable commands are stored as typed Quick Actions in an Automexia-owned,
  versioned source document. Per-shell files are generated artifacts.
- A Quick Action inserts a reviewed command into the active editor by default.
  Exact execution is allowed only for typed, first-party operations already
  authorized by the D3 launch broker.
- Built-in DevOps packs ship without short aliases enabled. Users opt into an
  alias after collision and safety validation.
- Native user aliases, functions, and completion definitions win over generated
  Automexia definitions unless the user explicitly changes precedence.
- Completion and alias expansion perform no provider calls, authentication, or
  network access on a keystroke or during terminal startup.

The architectural decision is recorded in
[ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md).
The accepted native-shell/provider matrix is
[Command Productivity Compatibility](COMMAND-PRODUCTIVITY-COMPATIBILITY.md);
security and privacy boundaries are in the
[Command Productivity Threat Model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md).

## Terminology

- **Native completion**: candidates and insertion behavior owned by the active
  shell/editor or an official CLI completion generator.
- **Completion adapter**: small shell-specific code that registers official
  providers, reports health, and exposes Automexia's static action names.
- **Quick Action**: a typed, persistent command template with scope, safety,
  provenance, and presentation metadata.
- **Alias projection**: an optional shell-native alias or function generated
  from a Quick Action.
- **Provider pack**: reviewed actions and completion registration for one tool,
  such as Git, Docker, Kubernetes, Helm, Terraform/OpenTofu, AWS, Azure, GCP,
  or OpenSSH.

## Goals

1. Preserve each shell's expected Tab, menu-completion, history, quoting, and
   accessibility behavior.
2. Detect installed CLIs and completion support lazily and report actionable
   health without silently installing tools.
3. Let users create, edit, disable, import, export, and delete reusable commands
   once and use them in later tabs, panes, windows, and application starts.
4. Support global, shell, capsule, workspace, and session scopes with explicit
   precedence and conflict reporting.
5. Make dangerous commands visually explicit and never execute a generated
   command merely because it was selected.
6. Keep startup, typing, completion, and action search bounded, cancellable, and
   independent of renderer or PTY latency.
7. Work on Windows, macOS, and supported Linux environments, including WSL and
   remote shells where installation policy allows integration.

## Non-goals

- Replacing PSReadLine, Readline, ZLE, Fish's editor, or the CMD input model.
- Sending command buffers, history, paths, aliases, or candidates to a service.
- AI-generated commands or semantic command correction in this track.
- Installing provider CLIs, shell plugins, or credentials automatically.
- Executing arbitrary shell text through the extension process broker.
- Claiming identical completion behavior where a shell lacks an equivalent API.

## User experience contract

### Completion

`cargo automexia` and `cargo dev` install or refresh only the Automexia-owned
adapter for the detected local shell. `cargo xtask doctor` reports:

- active shell and editor version;
- whether Automexia integration is enabled and current;
- official provider generators that are available;
- stale generated files, collisions, unsupported providers, and remediation;
- whether the current shell/profile is writable, without editing user-owned
  profile content during a read-only doctor run.

The user's normal completion key remains unchanged. Candidate ordering,
selection, quoting, and insertion belong to the shell. Automexia may later show
a renderer-owned candidate surface only through a versioned editor bridge that
provides buffer, cursor, replacement span, generation, and cancellation state;
terminal-cell scraping is forbidden.

### Quick Actions

The command palette and a dedicated action search expose the same action model.
Creating an action requires a name and command template; scope, shell support,
tags, description, working-directory policy, risk class, and alias are optional.

Selecting an action normally inserts the expanded command at the editor cursor
and leaves it unexecuted. Placeholders are presented in a review form before
insertion. Secret placeholders hold only a secret reference and are never
persisted as expanded values or included in telemetry, logs, crash reports, or
palette history.

### Scope and precedence

The deterministic precedence is:

1. session-only action;
2. selected Environment Capsule;
3. trusted workspace action;
4. shell-specific user action;
5. global user action;
6. disabled-by-default built-in pack.

Within a scope, duplicate IDs are invalid. A lower layer never silently
overwrites a higher layer. Native user aliases/functions/completers remain in
control unless an explicit, reversible override is configured. Conflicts appear
in `doctor` and the action editor with both sources and a rename/disable choice.

### Execution modes

| Mode | Default | Contract |
|---|---|---|
| Insert | Yes | Expand safely, paste at the editor cursor, do not press Enter. |
| Copy | Optional | Copy expanded non-secret text after an explicit action. |
| Exact launch | No | Typed executable/argv/cwd request through the D3 broker and capability policy. |
| Raw shell snippet | No | Shell-scoped, visibly marked, insert-only, never broker-executed. |

## Typed data model

The source schema is versioned and rejects unknown security-sensitive fields:

```text
QuickAction {
  schema_version
  id
  display_name
  description
  tags[]
  scope: Global | Shell | Capsule | Workspace | Session
  shells[]
  template: TypedArgv | ShellFunction | RawInsertOnly
  placeholders[]
  working_directory_policy
  risk: ReadOnly | Mutating | Destructive | Privileged
  execution: Insert | Copy | ExactLaunch
  alias?: { name, enabled, shells[] }
  provenance: BuiltIn | User | Imported
  enabled
}
```

`TypedArgv` stores an executable ID and argument tokens, not a concatenated
command. Complex shell behavior uses a generated function for that shell.
`RawInsertOnly` is never treated as safe argv. Action IDs are stable and do not
depend on display names or aliases.

## Persistence and generated artifacts

- Canonical user source: `<config-root>/actions/actions.toml`.
- Optional trusted workspace source: `.automexia/actions.toml`, disabled until
  the user trusts that workspace and separately grants any exact-launch action.
- Capsule source: capsule metadata, containing action IDs and public parameters,
  never credentials.
- Generated shell files: `<config-root>/generated/completion/<shell>.*` and
  `<config-root>/generated/aliases/<shell>.*`.

Writes use a temporary file in the destination directory, flush, atomic replace,
and restrictive user-only permissions where supported. Source files are the
authority; generated files contain a schema/tool version and source digest and
may be deleted and rebuilt. Startup never rewrites them unless their digest is
stale. Watchers are exact-file, debounced, bounded, cancellable, and retain the
last-known-good snapshot on malformed input.

Hard ceilings:

| Resource | Ceiling |
|---|---:|
| One source file | 1 MiB |
| Actions across active layers | 1,024 |
| Enabled alias bindings | 256 |
| Arguments per typed action | 64 |
| Placeholders per action | 32 |
| One string value | 4 KiB |
| Candidates returned per request | 512 |
| One rendered candidate | 1 KiB |
| Aggregate candidate response | 512 KiB |
| One generated adapter/alias file | 1 MiB |

Imports validate schema, limits, scope, shell compatibility, alias syntax, and
conflicts before replacing state. Export excludes session actions, generated
files, secret values, and machine-specific credential references by default.

## Shell adapter strategy

### PowerShell

Use PSReadLine's public completion and prediction contracts. The adapter may
register native argument completers or reviewed predictor integration only on a
supported PowerShell/PSReadLine combination. Prediction remains opt-in and
respects the user's view/style settings. Generated aliases use `Set-Alias` only
for a simple command name; arguments or logic require a function. Never rewrite
the user's profile—source a single managed file from the existing idempotent
Automexia integration block.

### Bash

Use programmable completion and Readline. Source one generated adapter from the
managed integration block. Prefer an official CLI completion generator or a
packaged static completion file. Functions, not aliases, represent commands
with arguments. Preserve `COMP_WORDS`, `COMP_CWORD`, quoting, exit status, and
user completion definitions.

### Zsh

Integrate with `compsys`/`compinit` and add one Automexia-owned directory to
`fpath`. Do not run `compinit` repeatedly or alter user options globally. Keep
generated functions namespaced and preserve ZLE ownership of the command line.

### Fish

Add a first-class Fish adapter using Fish completion and abbreviation APIs.
Prefer abbreviations for interactive expansion and functions for complex
actions. Do not modify universal variables without explicit user consent.

### CMD

Use a generated DOSKEY macro file for safe, compatible aliases. CMD has no
equivalent rich, context-aware programmable completion API, so this phase
provides action search, insertion, and diagnostics without claiming parity.

### WSL, SSH, and containers

Install adapters inside the actual environment only after explicit user consent
and using that environment's paths and permissions. Never project a Windows
path into a Linux shell file. Remote installation is separate from connecting
and must show the destination and exact files before modification.

## Official completion providers

Prefer documented provider generators and cache their generated static output:

- Git and system shell completion where packaged by the platform;
- `docker completion`;
- `kubectl completion` and the documented completion registration for an
  optional user-created `kubectl` alias;
- `helm completion`;
- Terraform's documented autocomplete installation contract and equivalent
  reviewed OpenTofu support;
- `aws_completer` where installed with AWS CLI;
- Azure CLI's supported shell-specific completion contract;
- Google Cloud CLI completion installed by its official SDK;
- OpenSSH host completion from native shell tooling and Automexia's bounded,
  non-executable alias inventory.

Generator discovery is lazy and versioned. Generated output is treated as
untrusted text: bound time/output, validate its destination, write atomically,
and never evaluate it inside the Automexia process. Helm/plugin and other
third-party generators may execute provider plugin code; they require explicit
enablement and inherit the user's normal shell trust boundary.

## Built-in DevOps packs

Built-in packs provide descriptive actions, not opaque abbreviations. Initial
packs cover Git, Docker/Compose, Kubernetes/OpenShift, Helm,
Terraform/OpenTofu, AWS, Azure, GCP, and SSH. They follow these rules:

- no short alias is enabled by default;
- read-only discovery actions are separated from mutating/destructive actions;
- namespace, context, subscription, project, region, capsule, and target are
  visible parameters rather than hidden globals;
- production or privileged actions always remain review-before-insert;
- provider-aware suggestions use cached public context only after D6 capsule
  plumbing exists;
- missing tools disable only their pack and produce one actionable diagnostic;
- pack updates are versioned and never overwrite a customized user action.

Suggested examples such as `k` for `kubectl` may be offered after proving the
name is free and registering the matching completion function. Automexia does
not reserve common short names globally.

## User alias lifecycle

A user-created alias is saved once in the typed action source and regenerated
for every matching future session, tab, pane, and window. The UI provides create,
preview, test, enable, disable, rename, change scope, export, and delete actions,
plus a dry-run showing the exact shell file change and collision result. Deleting
or disabling an alias removes only Automexia's generated definition and never
touches a user-owned profile definition.

An optional import assistant may inventory names from the active shell without
executing their bodies. It may import only simple aliases whose expansion can be
represented safely; shell functions, DOSKEY macros with complex substitution,
PowerShell script blocks, command substitution, and arbitrary profile code are
shown as unsupported and left untouched. Import is explicit, previews the typed
result, and records provenance. Automexia does not automatically copy every
existing alias or synchronize action files through a cloud service.

On session startup the managed adapter loads the current generated digest once;
it does not recreate aliases interactively or rewrite the canonical store.
Changes are compiled atomically in the background and become visible to new
sessions immediately. An active shell receives a bounded explicit reload action
where its native contract permits it; otherwise the UI truthfully says that a
new shell session is required.

## Security and privacy contract

1. Parse typed data; never source `actions.toml` as shell code.
2. Keep secret values out of action storage, generated files, logs, diagnostics,
   crash reports, clipboard history, and completion candidates.
3. Treat workspace and imported actions as untrusted. They cannot exact-launch
   until the user grants the exact action identity/capability.
4. Reject control characters, NUL, invalid identifiers, path traversal, device
   paths, and unsupported encoding before generation.
5. Quote using a tested shell-specific serializer. Never reuse quoting rules
   across PowerShell, POSIX shells, Fish, and CMD.
6. Detect alias/function/completion collisions before activation and preserve a
   reversible backup/managed-block marker.
7. Redact public parameters according to their schema; unknown placeholders are
   private by default.
8. No network, authentication, provider command, or secret-store read occurs on
   terminal startup or every keystroke.
9. Exact launch uses the reviewed D3 executable registry, argv validation,
   capsule boundary, cancellation, audit record, and least-privilege grant.
10. Imported packs require provenance and digest display; remote marketplaces
    remain deferred to the signed/sandboxed v0.6 extension ecosystem.

## Performance and resilience budgets

Before the 30-day baseline, these are measurement targets rather than release
claims:

| Operation | Target |
|---|---:|
| Search 1,024 cached actions | <= 16 ms p95 |
| Load a warm last-known-good action snapshot | <= 25 ms p95 |
| Register an already-generated shell adapter | <= 50 ms p95 |
| Show a pending completion affordance | <= 100 ms |
| Owned dynamic-provider deadline | <= 750 ms, then cancel/fallback |
| Cancellation acknowledgement | <= 50 ms p95 |
| In-process action/completion cache | <= 8 MiB |

Parsing, indexing, search, and generation run off the renderer and PTY read
paths. Requests carry session ID, prompt generation, buffer generation, and
cancellation. Stale results are discarded. Queues are bounded and latest-state
coalesced; shutdown drains or cancels workers without retaining processes,
watchers, temporary files, or provider handles.

## Delivery phases

### CP0 — decisions, threats, and compatibility

- Accept ADR 0015, schema, trust model, shell matrix, ownership boundaries, and
  terminology.
- Inventory native completion/profile behavior and existing user definitions on
  each supported platform without mutation.
- Add architecture rules forbidding terminal-grid command inference and provider
  calls on input/render threads.

Exit: decision, threat model, versioned fixtures, and conflict/precedence matrix
are approved. This work may proceed in parallel with D5.

CP0 status (2026-08-15): complete at the non-runtime policy boundary. ADR 0015
is accepted; the native shell/provider/discovery and conflict/fallback matrix,
16-threat model, schema-1 fixtures, 14 resource ceilings, canonical
fingerprints, whole-workspace pre-activation scanner, seventeen policy tests
(including a versioned 11-case hostile corpus), repository-policy integration,
and architecture gate are implemented.
The CP0 baseline remains immutable. Its activation gate now permits only the
exact CP1 files listed by the CP1 machine contract; provider code is still
forbidden in the renderer, input, VT, PTY, and extension runtime paths.

### CP1 — completion health and shell adapters

- Add Fish as a first-class integration target.
- Implement read-only doctor diagnostics and idempotent managed adapters for
  PowerShell, Bash, Zsh, Fish, CMD, and WSL.
- Register and cache reviewed official provider completions behind explicit
  enablement; preserve native definitions and shell-disabled behavior.

Exit: clean install/update/uninstall, quoting, cursor, history, exit-status,
startup-time, disabled-integration, and collision tests pass natively.

CP1 status (2026-08-16): implemented. PowerShell, Bash, Zsh, Fish, CMD, and WSL
retain their native editors. Managed Bash/Zsh/Fish adapters inventory existing
definitions before sourcing a fixed, digest-verified artifact; native entries
win. Because PowerShell exposes no supported read-only argument-completer
registry, a cached PowerShell provider additionally requires the explicit
`--allow-native-override` marker. CMD truthfully retains native DOSKEY behavior
and does not claim programmable-completion parity.

The read-only health and explicit lifecycle commands are:

```text
cargo xtask completion doctor
cargo xtask completion refresh --provider kubernetes --shell bash
cargo xtask completion refresh --provider kubernetes --shell powershell --allow-native-override
cargo xtask completion remove --provider kubernetes --shell bash
cargo xtask completion disable
cargo xtask completion enable
```

Only Docker, Kubernetes, OpenShift, and Helm official generators are cacheable,
and only for shells their installed CLI supports. Git, AWS, Azure, GCP, and
OpenSSH remain package/provider-owned. Terraform and OpenTofu profile-mutating
installers require their own reviewed manual consent and are never invoked by
Automexia. Refresh resolves one executable, passes exact argv with null stdin,
terminates at 750 ms, bounds stdout/stderr, validates UTF-8/control bytes, and
publishes private fixed-name files using same-directory atomic replacement.
Provider commands run inside a POSIX process group or Windows Job Object, so a
timeout or output overflow terminates descendants that still hold output pipes;
the command never leaves detached capture threads or provider children behind.
Windows refresh accepts only native `.exe`/`.com` images and never implicitly
routes a provider through `.cmd`/`.bat` shell parsing. Shell adapters reject a
linked or non-directory component anywhere in their managed parent chain.
No provider is invoked during shell startup, typing, rendering, or `doctor`.

The schema-1 CP1 authority is
[`cp1-contract-v1.json`](../tests/fixtures/command-productivity/cp1-contract-v1.json).
Its validator keeps the 12-file activation allowlist, provider policy, five
shell outcomes, six hard resource limits, and zero network/secret/grid/provider-
startup capability machine-enforced. CP2 remains the next phase; CP1 does not
add Quick Actions, generated aliases, provider authentication, a custom popup,
or exact command launch.

### CP2 — persistent typed Quick Actions

- Implement the bounded schema, atomic store, last-known-good reload, exact-file
  watcher, in-memory index, palette/search surface, and insert/copy modes.
- Add CRUD, import/export, layered scope, placeholder review, and conflict UI.
- Keep exact launch disabled until D3 activation is accepted.

Exit: restart persistence, concurrent-window reload, corruption recovery,
Unicode/hostile input, deterministic ordering, accessibility, and leak tests
pass on Windows, Linux, and macOS.

### CP3 — aliases and first-party DevOps packs

- Generate reversible shell functions/aliases from enabled user choices.
- Deliver versioned Git, Docker, Kubernetes/OpenShift, Helm,
  Terraform/OpenTofu, AWS, Azure, GCP, and SSH packs.
- Add risk labeling, tool/version health, collision resolution, and completion
  linkage for every enabled alias.

Exit: no default collision, native aliases win, every projection round-trips,
and pack actions remain review-before-insert across all supported shells.

### CP4 — capsule/provider-aware productivity

- After D5/D6, filter and parameterize actions with the selected SSH target,
  capsule, cluster/context, account/subscription/project, region, and workspace.
- Use only bounded cached public context; refresh explicitly or through the
  extension freshness contract, never synchronously from a keystroke.
- Permit reviewed exact-launch actions through D3 with capability and audit.

Exit: multi-pane/session isolation, stale-context labeling, production safety,
revocation, cancellation, offline behavior, and provider-native tests pass.

### CP5 — optional rich completion surface

- Specify a versioned editor bridge before drawing any custom candidate popup.
- Preserve shell candidate semantics, replacement spans, accessibility, IME,
  Unicode graphemes, composition, cancellation, and native fallback.
- Ship only when it is measurably faster or clearer than shell-native UI and can
  be disabled without changing command behavior.

Exit: renderer-neutral/native accessibility, latency, resize, IME, shell parity,
and fallback evidence is recorded. This is not a v0.5.0 blocker.

### CP6 — ecosystem integration

- Consider signed third-party action packs only with v0.6's manifest,
  capability, provenance, revocation, and sandbox policy.
- AI suggestions remain a separate opt-in capability with explicit data-flow
  consent and are not treated as completion.

## Verification matrix

Required deterministic tests include:

- schema versioning, unknown fields, every ceiling, malformed/partial writes,
  rollback, concurrent readers, reload coalescing, and last-known-good behavior;
- action scope/precedence, duplicate IDs, native collision, deterministic sort,
  enable/disable, import/export, pack upgrade, and customized-pack preservation;
- per-shell quoting and tokenization using spaces, Unicode, quotes, metacharacters,
  option-like values, multiline/control input, reserved names, and long paths;
- real returned shell objects/semantics where formatting or integration is
  involved; integration disabled and unsupported-version behavior;
- command-buffer insertion at the editor cursor, selection replacement,
  placeholder navigation, cancellation, prompt-generation isolation, and no
  automatic Enter;
- secret-reference redaction and negative tests proving no secrets/history are
  written to actions, adapters, cache, logs, diagnostics, or crash artifacts;
- exact-launch denial by default, capability approval/revocation, audit redaction,
  and hostile argv through the D3 broker;
- queue saturation, stale completion, worker loss/restart, repeated reload,
  startup/shutdown cycles, watcher/temp-file/process/handle cleanup, and bounded
  memory/storage growth;
- benchmarks for cold/warm load, 1,024-action search, candidate formatting,
  adapter generation, reload, cancellation, and sustained completion storms;
- fuzzing for TOML/schema parsing, template/placeholder parsing, shell serializers,
  provider-output tokenizer, and import/export;
- native PowerShell/CMD/ConPTY, Bash/Zsh/Fish/PTY, WSL, macOS, X11, and Wayland
  tests. Representative distro/package-manager tests complement, but do not
  replace, native shell contracts.

CI must also assert that every declared shell/OS support entry has a native job,
that adapters are absent when integration is disabled, and that generated files
leave the normal build/test working tree clean.

## Product acceptance

The track is complete only when:

- completion works through each supported shell's native editor without command
  scraping, input lag, history regression, cursor corruption, or profile loss;
- a saved Quick Action survives restart and is available in every matching
  pane/window while remaining isolated from nonmatching capsules/workspaces;
- optional aliases have matching completion, never override native definitions
  silently, and can be removed without residue;
- built-in packs expose clear tool/capsule/target/risk context and no short alias
  is enabled by default;
- no completion/action path reads secrets or performs network/auth/provider work
  on startup or per keystroke;
- performance/resource budgets and Windows/Linux/macOS functional, security,
  accessibility, uninstall, and recovery gates pass;
- documentation explains installation, configuration, precedence, security,
  troubleshooting, migration, and removal for every supported shell.

## Why this approach

Shell editors already solve quoting, cursor ownership, accessibility, IME,
history, and tool-specific completion. Replacing them from rendered terminal
cells would be fragile and would recreate shell-specific security bugs. A typed
source plus generated adapters gives users one durable Automexia model while
keeping shell behavior native and generated state disposable. Review-before-
insert avoids surprising execution; the D3 broker remains the only route for
operations that genuinely need typed execution authority.

Provider refresh keeps separate bounded stdout/stderr readers to avoid pipe
deadlock. It uses the maintained `process-wrap` abstraction for POSIX process
groups and Windows Job Objects instead of maintaining two security-sensitive
OS process-tree implementations inside `xtask`.

## Primary references

- [PowerShell predictive IntelliSense and PSReadLine predictors](https://learn.microsoft.com/powershell/scripting/learn/shell/using-predictors)
- [PowerShell about aliases](https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_aliases)
- [GNU Bash programmable completion](https://www.gnu.org/software/bash/manual/html_node/Programmable-Completion.html)
- [Zsh completion system](https://zsh.sourceforge.io/Doc/Release/Completion-System.html)
- [Fish completions](https://fishshell.com/docs/current/completions.html)
- [kubectl shell completion](https://kubernetes.io/docs/tasks/tools/install-kubectl-linux/#enable-shell-autocompletion)
- [Docker completion](https://docs.docker.com/engine/cli/completion/)
- [Helm completion](https://helm.sh/docs/helm/helm_completion/)
- [Terraform shell tab completion](https://developer.hashicorp.com/terraform/cli/commands#shell-tab-completion)
- [AWS CLI command completion](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-completion.html)
- [Azure CLI command completion](https://learn.microsoft.com/cli/azure/azure-cli-completion)
- [Google Cloud CLI shell completion](https://cloud.google.com/sdk/docs/install-sdk#installing_the_latest_version)
- [DOSKEY macros](https://learn.microsoft.com/windows-server/administration/windows-commands/doskey)
- [Git aliases](https://git-scm.com/book/en/v2/Git-Basics-Git-Aliases)
- [process-wrap cross-platform process lifecycle](https://docs.rs/process-wrap/latest/process_wrap/)
