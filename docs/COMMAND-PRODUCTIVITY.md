# Command Productivity: Completion and Quick Actions

Status: CP0, CP1, CP2.0, and CP2.1 are complete at their defined source
boundaries. CP2.2 search, placeholder review, dry-run administration,
import/export, and explicit insert/copy are implemented locally. Its stable
release gate remains partial until hosted native Windows/Linux/macOS and
controlled screen-reader/performance evidence pass. Exact launch, trusted
workspace activation, secret expansion, generated aliases, and provider-aware
execution remain disabled or planned. CP5 source components exist but its
suggestion surface and shell bridge remain preview-disabled and unpublished.

The complete command-first product vocabulary that consumes this track is
specified in [Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md).
That document does not widen CP authority or turn planned commands into shipped
behavior.

In this document and ADR 0025, **editor** means the native shell's editable
command line. It is separate from the proposed
[Automation Studio](AUTOMATION-STUDIO-ARCHITECTURE.md) file editor; CP5 neither
implements nor authorizes that document surface.

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
The complete CP2/CP3 product contract for user-created aliases, first-party
DevOps packs, persistence, projection compilers, UX, and verification is
[DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md).

Technology placement follows the canonical
[build, wrap, and adopt boundary](BUILD-WRAP-ADOPT-ARCHITECTURE.md):

- core owns the operation/action schema, static completion generation,
  replacement-span/escaping contracts, bounded search service, editor bridge,
  persistence/projection, collision policy, and insert-versus-execute review;
- first-party packs contribute immutable typed records and explicit refresh
  adapters, but receive no shell-editor, renderer, PTY, process, network, or
  ambient credential authority;
- PSReadLine, Readline, ZLE, Fish, and CMD retain native editing; Carapace is an
  optional external compatibility bridge;
- `clap_complete`, `clap_mangen`, and `schemars` are planned generated-
  artifact dependencies, while `nucleo` is accepted only after the large-list
  benchmark, Unicode, cancellation, memory, binary-size, and fallback gate;
- provider CLIs are never invoked at startup or per keystroke. Explicit refresh
  runs through the core ExternalToolRunner and publishes a bounded immutable
  last-known-good snapshot.

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
- **Local suggestion**: a non-executing candidate derived locally from the
  active shell/editor, its history, the current directory, executable names, or
  an already-refreshed public provider cache. It is never inferred from painted
  terminal cells.
- **Editor bridge**: an opt-in, versioned, session-scoped protocol through which
  the shell/editor reports bounded buffer, cursor, replacement-span, quoting,
  candidate, and generation state without giving Automexia ownership of editing.

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

Automatic setup is detection-first and idempotent. It inspects the current
PSReadLine version/options, an already installed `bash-completion`, the user's
initialized Zsh compsys/`compinit` state, Fish's native completion paths, and
CMD/DOSKEY availability before proposing any change. It does not install a
package, rerun expensive initialization on every shell start, or replace a user
profile, keybinding, completer, predictor, function, abbreviation, or style.
Existing user configuration wins; unsupported or partial setups produce one
actionable diagnostic and retain the native fallback.

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

CP1 status (2026-08-17): implemented. PowerShell, Bash, Zsh, Fish, CMD, and WSL
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
terminates at 750 ms, bounds stdout/stderr, validates UTF-8 plus C0 and
bidirectional controls, and publishes private fixed-name files using
same-directory atomic replacement. The child environment is cleared and rebuilt
from OS process essentials, a local absolute-only `PATH`, deterministic locale/
color controls, and no home, provider configuration, proxy, credential, or
arbitrary application variables. Hostile provider diagnostics are bounded and
terminal-control sanitized.
Provider commands run inside a POSIX process group or Windows Job Object, so a
timeout, output overflow, or early leader exit terminates descendants that still
hold output pipes; the command never leaves detached capture threads or provider
children behind. The executable is held open and its stable file identity is
revalidated after version discovery and generation, rejecting replacement races.
Windows refresh accepts only native `.exe`/`.com` images and never implicitly
routes a provider through `.cmd`/`.bat` shell parsing. Windows completion state
and the canonical provider image must remain on a local drive; UNC roots and
remote canonical redirects fail before startup or execution I/O. Shell adapters
reject a linked or non-directory component anywhere in their managed parent
chain and enforce the 4096-byte configuration-root ceiling.
Windows installer and completion integrity checks use the platform SHA-256 API
directly and do not depend on `Microsoft.PowerShell.Utility` auto-loading during
clean installation or shell startup.

Refresh publishes a bounded two-digest transition before replacing an existing
artifact, then collapses it to one digest. Bash, Zsh, Fish, and PowerShell can
therefore verify the previous or candidate artifact at every interruption point
without sourcing unverified text. All persistence overrides must be absolute.
macOS uses
`~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal`; Linux
and BSD use `${XDG_CONFIG_HOME:-~/.config}/automexia`. Installers repair a stale
owned profile block in place; uninstallers preflight every exact owned target
before their first mutation and never recursively remove an installation root.
`doctor` performs bounded artifact, digest, metadata, provenance, override, and
parent-chain validation. No provider is invoked during shell startup, typing,
rendering, or `doctor`.

The schema-1 CP1 authority is
[`cp1-contract-v1.json`](../tests/fixtures/command-productivity/cp1-contract-v1.json).
Its validator keeps the 12-file activation allowlist, provider policy, five
shell outcomes, six hard resource limits, and zero network/secret/grid/provider-
startup capability machine-enforced. CP2.0 and CP2.1 remain the model and
persistence foundations; CP2.2 adds only reviewed Quick Action search,
administration, and insert/copy. CP1 itself does not add Quick Actions,
generated aliases, provider authentication, a custom popup, or exact launch.

### CP2 — persistent typed Quick Actions

**CP2.0 complete:** `automexia-devops::actions` now owns the capability-free,
bounded schema-1 TOML parser, typed model, deterministic validator, validated-
only wrapper, exact architecture allowlist, and versioned hostile corpus. It has
no filesystem, watcher, process, network, secret, UI, PTY, shell-profile, alias,
or execution authority.

**CP2.1 persistence foundation complete:** the app-owned exact five-file boundary
implements private bounded no-follow storage, same-directory atomic primary plus
one previous revision, cross-process nonblocking lock/CAS, immutable fingerprinted
last-known-good snapshots, exact parent-directory watch filtering, bounded burst
coalescing, periodic reconciliation, CRUD, and explicit recovery. It has no
provider/process/network/secret/profile/clipboard/PTY/UI or execution authority
and grants no provider, process, network, secret, profile, PTY, UI, or execution
authority by itself.

**CP2.2 implemented locally:** one application-owned worker starts the store,
publishes immutable last-known-good snapshots, coalesces the latest query per
route, admits at most 32 live routes, publishes every admitted pane fairly, and
is joined on shutdown. A capability-free layered index revalidates each layer
and applies distinct deterministic session, capsule, trusted-workspace,
shell-user, global-user, and built-in precedence. Workspace entries remain
fail-closed in the CP2.2-only boundary; CP3.3 activates only an exact digest/
revision-trusted workspace source and rechecks it before insertion. The Command
Center provides a keyboard-complete responsive search, placeholder, visible
risk/source/conflict/health, explicit empty and unavailable states,
exact-command review, and explicit
**Insert without Enter** or copy flow. Insertion uses the shell-owned editor's bracketed-paste
path and never synthesizes Enter. Exact launch and secret-reference expansion
remain unavailable and are rejected before placeholder collection.

The versioned `automexia actions` interface provides read-only `list`, `show`,
and `doctor`; dry-run-by-default `put`, `actions import`, `remove`, and
`recover`; and bounded digest-checked `export`. Mutations require `--apply` and
revision compare-and-swap; import conflicts and machine-specific fixed paths
require separate explicit consent.

Exit still required for a stable cross-platform claim: the pushed hosted native
Windows/Linux/macOS matrix, controlled Narrator/NVDA/VoiceOver/Orca interaction,
native shell insertion evidence, and the named-hardware 30-day latency/resource
baseline. Renderer-neutral layout/accessibility labels, quoting/Unicode,
privacy, worker lifecycle, storage/recovery, mutation, and short benchmark gates
are automated now.

### CP3 — projection, aliases, and first-party DevOps packs

**CP3.0 complete at its pure boundary:** `automexia-devops::actions` compiles
validated aliases deterministically for PowerShell, Bash, Zsh, Fish, and CMD.
Portable eligibility, exact user override consent, native ownership, completion
and tool health, rollback/source/artifact metadata, hard observation/file limits,
tamper verification, hostile quoting, and typed argument policies are enforced.
The compiler independently recomputes canonical source identity, requires
complete collision/completion/tool inventories, and accepts a same-action owner
only when its deterministic fingerprint matches. Missing and unsupported tool
details remain in decisions for actionable UI. The compiler returns in-memory
artifacts with activation disabled and owns no profile, filesystem, process,
environment, network, secret, or execution capability. Thirteen focused tests,
native syntax/capture checks, mutation gates, an all-five-shell libFuzzer target,
and a 256-binding Criterion target own this boundary.

**CP3.1 fully done at the source/local boundary:** explicit opt-in aliases use
private immutable content-addressed generations, SHA-256 manifests, source and
generation compare-and-swap, a crash-recovery journal, a pointer-last commit,
and one verified rollback generation. The existing managed hook validates the
ordered exact compiler/source/shell manifest and activates one file per shell;
native definitions win, authenticated exact-owner consent is rechecked, reload
is last-known-good, and uninstall preserves saved actions. Dry-run-first
management reports directly reusable CAS values plus full collision/completion/
tool detail; read-only doctor verifies active/rollback topology and permissions
and returns stable malformed-source/unsafe-root health. Unique executable and
completion observations are cached across shells. Native Windows and full local
WSL Bash/Zsh/Fish lifecycle tests, configured nightly/release WSL gates, a
256-alias benchmark, fuzz/property coverage, and mutation/contract ratchets own
the boundary.

**CP3.2 fully done locally:** the capability-free schema-1 registry provides
Git, Docker/Compose, Kubernetes, OpenShift, Helm, Terraform, OpenTofu, AWS,
Azure, Google Cloud, and OpenSSH packs: 11 manifests and 33 disabled-by-default
`TypedArgv` actions. Bounded caller observations drive version/provider-absence/
completion health without starting tools. Effect/risk floors deny aliases for
context, authentication, destructive, and privileged actions. A reviewed digest
freezes the complete registry payload; version-only provenance updates remain
unchanged while functional metadata changes are reported as updates. Dry-run/CAS
enablement previews exact argv/effect/risk/documentation and rejects stale
revisions before store creation. Fourteen pack unit/integration cases, five CLI
parser/rendering/preflight cases, Criterion, nightly fuzz, an exact-payload
contract, and eight mutations own CP3.2.

**CP3.3 fully done locally:** capability-free bounded parsers consume only an
explicitly supplied PowerShell CSV, Bash/Zsh alias, Fish abbreviation,
CMD/DOSKEY, or Git inventory. They reject controls/bidi, duplicates, likely
secrets, machine paths, substitutions, pipelines, redirection, metacharacters,
Git shell aliases, and unsupported kinds. Import requires explicit unique names,
is dry-run first, supports portable ID rename, previews conflict/replace, and
applies once through Quick Action revision CAS without editing native sources.

Exact named just, Task, and mise bridges persist in `.automexia/actions.toml` as
Mutating, Insert, WorkspaceRoot, WorkspaceTask actions with no aliases. Private
path-free receipts bind workspace identity, canonical source digest, and exact
revision; trust/revoke/put/remove are bounded, no-follow, staged, lock/CAS
mutations. Any source/link/malformed/revoked mismatch removes the trusted layer.
The background worker bounds ancestor walking, cache entries, reconciliation,
and route authorization; the review and insert/copy boundaries recheck trust and
show a textual refresh-and-review state when stale. Automexia never discovers/
lists tasks, parses recipes, starts providers/runners, reads credentials, uses
the network, executes tasks, synthesizes Enter, or projects workspace aliases.

Fourteen named regressions, Unix no-follow cases, the schema-1 contract and
mutations, aggregate source ratchets, nightly fuzzing, parser/trust benchmarks,
CI/xtask wiring, ADR 0021, and synchronized documentation own CP3.3. Hosted
native/accessibility and named-hardware 30-day measurements remain release
evidence, not missing CP3.3 source implementation.

### CP4 — capsule/provider-aware productivity

Status: **Fully done locally at the product-integrated nonactivating boundary; partially done overall.**
The existing Quick Actions surface can consume an explicitly published,
immutable public capsule snapshot for SSH, AWS, Azure, Google Cloud,
Kubernetes, OpenShift, and Teleport. OpenBao remains absent pending ADR 0024,
the Connection Hub-to-Action Center handoff is implemented, and no approved
provider refresh/capsule producer currently creates these snapshots.

The implementation:

- projects exact cached targets plus account/subscription/project, region/zone,
  cluster/context/namespace, infrastructure, provenance, freshness, state, and
  environment risk without provider, network, filesystem, credential, startup,
  renderer, or keystroke-time work;
- publishes at most 32 route-owned snapshots and 256 actions, with 16 actions
  per provider, 32 presentation fields, and 128 search results;
- retains the validated capsule behind a redacted immutable product publication
  and synchronizes it on Action Center open only to the exactly matching
  selected route/session/revision; identical generations are idempotent, while
  mismatch, revocation, or absence clears candidates before cached search;
- discards stale requests, rejects non-monotonic generations, isolates route,
  session and capsule revision, removes snapshots on route cleanup, and
  revalidates the binding immediately before copy or bracketed insertion;
- gives capsule candidates precedence over same-ID persisted actions without
  duplicate rows, while preserving deterministic ranking and shadow counts;
- shows provider, exact target, current/stale/refreshing/expired/offline/
  unavailable/error/replaced state, provenance-backed context, and development/
  staging/production risk in compact visible and accessible text;
- keeps current observation actions insert-without-Enter. Kubernetes,
  OpenShift, and Teleport operations that need a private environment or D3
  launch remain visibly broker-required and never fall back to ambient state;
- requires a second confirmation for production even when the command is
  read-only, and never treats color as the only production or failure signal.

The versioned CP4 contract, static authority checker, seven mutation cases,
25 named regressions, hostile-capsule fuzz target, and cached snapshot/search
benchmark own the local source claim. Exact process execution, OpenBao, real
provider accounts/CLIs/clusters, native Linux/macOS execution, screen-reader
inspection, controlled resource baselines, packaging, and release evidence
remain external gates. Disabling CP4 means clearing the route snapshot or not
publishing one; persisted CP2/CP3 actions and provider-owned state are unchanged.

Exit is met locally at the nonactivating authority boundary: provider-aware
discovery does not widen provider, credential, process, or session authority.
The combined product/release exit remains partial until the external gates above
actually run.

### CP5 — Shell Completion and Suggestions

Status: CP5.0 research is fully done. ADR 0025 and the six-threat machine
contract are accepted. CP5.1 protocol/endpoints, CP5.2 bounded sources, CP5.3
deterministic ranking, and CP5.4 UI/publication are fully done at their source/
local model boundaries. CP5.5 has a complete inert helper/adapter source bridge
but not its reviewed launcher, signed artifact, WSL relay, live composition, or
activation. CP5.6 assurance remains partial and preview/stable activation is
false. CP5 remains optional and is not a v0.5.0 blocker; CP1 remains the default
and complete fallback.

#### CP5.0 — research, baselines, and dependency decision

1. Record native completion, prediction, startup, typing, cancellation, memory,
   accessibility, and resize baselines for supported PowerShell/PSReadLine,
   Bash/Readline, Zsh/ZLE/compsys, Fish, and CMD versions.
2. Prototype the smallest supported editor-state bridge per shell without
   changing existing profiles or keybindings. A shell with no safe persistent
   bridge retains native completion; CMD must not claim rich parity.
3. Benchmark the in-tree deterministic prefix/token matcher against the focused
   `nucleo-matcher` crate using realistic 32/128/512-candidate corpora, Unicode,
   long common prefixes, stale-generation cancellation, and low-end hardware.
   The upstream project recommends this smaller crate when a managed picker is
   unnecessary. Adopt its MPL-2.0 dependency only if legal/license policy,
   advisories, features, binary/compile cost, maintenance, and measurements show
   a user-visible benefit; do not import the higher-level picker by default.
4. Treat Reedline as a UX, history, hint, Unicode, and menu test reference only.
   Do not embed Reedline or Rustyline because they are complete line editors and
   would replace the shell-owned editor contract.
5. Evaluate Carapace as an explicitly installed external adapter only. Never
   bundle it silently, invoke it on every keystroke, or let it outrank an
   existing native/provider completion. Official CLI generators and native
   shell definitions remain the primary path.
6. Reuse existing `unicode-segmentation`, width/layout primitives, bounded worker
   lifecycle, cancellation, theme contrast, and accessibility infrastructure.
   Do not add another UI toolkit, async runtime, cache framework, or filesystem
   watcher merely for this feature.

Exit: a decision report records benchmark inputs/results, adopted/rejected
components and licenses, shell/version support, binary/startup cost, privacy
changes, and the native-fallback proof. No runtime dependency is added only
because it is popular.

##### CP5.0 research decision

CP5.0 is complete. The machine contract
<code>tests/fixtures/command-productivity/cp50-research-contract-v1.json</code>
fixes a seven-family native-shell matrix, the bounded replacement-only
prototype, false runtime capabilities, 32/128/512-candidate benchmark inputs,
and a reviewed decision to retain CP1 and defer P2. The full evidence and
reproduction commands are in
[CP5.0 native autocomplete research](research/CP5-AUTOCOMPLETE-RESEARCH.md).

The existing in-tree matcher is retained. <code>nucleo-matcher 0.3.1</code>
remains pinned only in a standalone research workspace because it provided no
measured benefit on the bounded product corpus; it is absent from the root
manifest, root lockfile, runtime, and release binaries. Reedline remains a UX
reference, while Carapace remains an explicitly installed external adapter
candidate. No profile, keybinding, shell process, editor transport, history,
terminal-grid inference, cache, worker, or product UI was introduced.

##### CP5.1-CP5.6 accepted source decision

[ADR 0025](adr/0025-authenticated-native-editor-suggestion-bridge.md) and the
schema-1
[`cp51-bridge-threat-contract-v1.json`](../tests/fixtures/command-productivity/cp51-bridge-threat-contract-v1.json)
are accepted for source implementation. Six stable threats cover endpoint/
replay, privacy, stale replacement, candidate spoofing, input/occlusion, and
resource amplification. Contract and source mutation tests prevent authority,
transport, peer, replay, privacy, span, source, ranking, keyboard, Fish
`Ctrl+Space`, fallback, limit, lifecycle, activation, and release-gate
downgrades. The contract records `accepted: true` and
`runtime_activation: false`.

The [implementation audit](research/CP51-CP56-IMPLEMENTATION-AUDIT.md) records
source owners and exact remaining gates. Authenticated bidirectional codecs,
platform endpoints, broker, six local sources, deterministic ranking, pane UI,
publication mailbox, application route exchange, helper binary target, four
native response/replacement adapters, fuzz, benchmarks, and local native shell
harnesses exist. Normal shell integration does not source those inert adapters.
No reviewed launcher, signed/attested artifact, WSL host relay, public setting,
default shortcut, live screen composition, or activated user-facing suggestion
surface ships.

#### CP5.1 — versioned editor bridge and ownership

The bridge is local, opt-in, session scoped, capability authenticated, and
version negotiated. Prefer a private named pipe on Windows and a mode-0600 Unix
domain socket under the protected runtime directory. The endpoint is never a
TCP listener, never placed in a world-readable directory, and is removed when
the pane closes. A persistent shell-side adapter owns one connection; no helper
process is spawned per keypress.

Every request carries:

- schema version; application/window/tab/pane/session route; shell/editor kind
  and version; prompt generation; monotonically increasing buffer generation;
- the bounded editor buffer supplied by the editor API, cursor byte offset and
  grapheme boundary, optional selection/replacement span, quoting/token context,
  working directory, and active completion mode;
- source revision, request reason (`explicit`, `typing`, or `refresh`), and a
  cancellation token.

Every candidate carries a stable request-local ID, insertion value, display
label, short description, semantic kind, source/provenance, freshness, exact
replacement span, quoting/insertion mode, and optional non-secret public context.
Messages are framed, length checked before allocation, schema validated, and
rejected on route, generation, cursor, span, shell, endpoint, or capability
mismatch. Buffer and candidate payloads are memory-only, excluded from logs,
telemetry, crash reports, clipboard history, diagnostics, and extension APIs,
and dropped on cancellation, prompt completion, pane rebind, tab close, or
application shutdown.

Automexia never reconstructs a buffer from terminal cells, writes directly into
shell memory, or invents shell escaping. Acceptance is an explicit bridge call
back to the active editor, which revalidates the generation and replacement
span, performs native quoting/insertion, and leaves the command unexecuted.

Exit: protocol conformance, downgrade/reject behavior, restrictive endpoint
permissions, peer/session authentication, generation isolation, payload
redaction, exact cleanup, and native fallback pass before any popup is enabled.

#### CP5.2 — local-only source broker

Candidate sources are ordered and independently controllable:

1. current shell-native registered completers;
2. shell-owned history predictions, returned by the shell without Automexia
   opening or parsing history files;
3. current-directory files/directories and executable names exposed through the
   shell/editor completion API, without recursive filesystem walks;
4. opt-in frequency ranking based only on accepted candidate IDs and decayed
   counters—never raw command lines, arguments, environment values, or secrets;
5. already generated CP1 provider artifacts and last-known-good CP4 public
   context, with source and freshness labels;
6. later CP2/CP3 typed Quick Actions and enabled aliases, as insert-only
   candidates with visible provenance and risk.

History and frequency learning are separately opt-in and local-only. There are
no server, telemetry, extension, AI, remote-output, provider-authentication, or
secret-store sources. Terminal output, OSC payloads, clipboard contents, remote
host output, and untrusted workspace text cannot create candidates.

Shell-native candidates are immediate. Other allowed local work is debounced,
asynchronous, bounded, cancellable, and performed off PTY input, VT parsing,
rendering, resize, and shell-output threads. Only one latest request per pane is
queued; a newer buffer generation cancels and replaces older work. Temporarily
slow sources return the last-known-good public snapshot with an explicit stale
label or disappear without delaying native completion.

Exit: source precedence, opt-in state, offline behavior, stale labels, no-network
and no-secret negative tests, cancellation, queue saturation, multi-pane
isolation, and cleanup pass for every activated shell.

#### CP5.3 — deterministic matching, ranking, and insertion safety

Ranking is explainable and stable: exact prefix, shell-native rank, token/word
boundary, case-aware prefix, recently accepted candidate ID, then optional fuzzy
score. Risk, freshness, and provenance never disappear during sorting. Ties use
source priority, normalized display value, then stable candidate ID. There is no
AI or remote ranking.

The existing hard ceilings remain authoritative: at most 512 returned
candidates, 1 KiB per rendered candidate, 512 KiB aggregate response, and 8 MiB
for the process-wide action/completion cache. CP5 additionally freezes a bounded
buffer/message limit, one latest queued request per pane, a visible-row limit,
and per-source deadlines in its activation contract before implementation.
Candidate text is untrusted display data: sanitize control/bidi-confusing text,
preserve grapheme boundaries, never interpret markup, and insert only the
editor-returned escaped value into the revalidated replacement span.

Initial measurement gates are:

| Interaction | CP5 gate before activation |
|---|---:|
| Native candidate availability | unchanged from the shell baseline |
| Warm local results visible | <= 50 ms p95 after debounce |
| Popup update/render work | <= 8 ms p95 and no missed input frame |
| Stale generation cancellation | <= 50 ms p95 |
| Explicit local source deadline | <= 250 ms, then stale/native fallback |
| Provider process/network/auth work | 0 during startup and typing |
| Completion cache | <= 8 MiB process-wide |

Exit: deterministic/property tests, hostile Unicode/control tests, shell-native
round trips for spaces/quotes/metacharacters, cancellation storms, and criterion
benchmarks meet the frozen limits without renderer or input-thread blocking.

#### CP5.4 — premium pane-owned UI/UX

The renderer receives an immutable renderer-neutral projection; it never owns
candidate production or insertion. The popup belongs to exactly one pane and is
clipped to that pane. Its z-order is above terminal content and below
application modals; it cannot cover pane tabs, the footer, another pane, the
active cursor, the IME candidate window, or a confirmation/security dialog.
It chooses above/below placement from measured free space and repositions on
resize without moving terminal cells or the shell cursor.

The default surface shows:

- completion value with matched graphemes emphasized;
- redundant icon and text kind: command, file, directory, option, host, cluster,
  cloud profile, Quick Action, or alias;
- a concise description plus source and freshness (`Shell`, `History`,
  `Cached · 2m`, or `Action`); and
- risk/production state for action-backed candidates using text and icon, never
  color alone.

Use the terminal's design tokens, semantic colors, contrast correction, corner
radius, and scale factor. Candidate text is slightly smaller than command text
but never below the platform-accessible minimum. Keep rows dense but touch
friendly, align icons on a shared optical box, and use no decorative animation;
when reduced motion is off, an optional opacity transition is capped at 120 ms.
At narrow/short sizes, hide description, then freshness, then switch to a compact
single-line native hint. If the pane cannot fit the surface without obscuring
input, dismiss it and retain native completion rather than overlaying content.

Keyboard and pointer behavior:

- the shell's existing Tab/Shift+Tab behavior remains unchanged by default;
- an explicit configurable `Ctrl+Space` action may open the Automexia surface
  only when it does not collide with a user binding;
- Up/Down, PageUp/PageDown, Home/End navigate while open; Escape dismisses;
- Tab or Right Arrow accepts according to the active shell's native contract;
  accepting never also submits the command; Enter is not captured unless the
  shell bridge explicitly declares safe insert-only acceptance;
- pointer hover changes only the visual highlight and never moves keyboard focus
  or the editor cursor; one click selects/inserts and never executes;
- Enter dismisses the popup and retains the shell's native submit behavior for
  the current buffer; it never implicitly accepts a highlighted candidate;
- focus loss, typing that invalidates the generation, pane change, prompt
  completion, or modal opening dismisses the surface.

Expose listbox/option semantics through the existing accessibility model. Each
option's accessible name includes value, type, description, source, freshness,
and risk. Announcements are coalesced and rate limited. Validate keyboard-only,
screen-reader, high-contrast, 100–300% scale, RTL/bidi containment, IME,
multiline, split-pane, and extreme-resize behavior with renderer-neutral goldens
plus native assistive-technology checks.

Exit: UX review and automated layout/accessibility evidence prove the popup is
clearer than the native baseline, does not occlude or move the input/cursor, and
fully disappears without residue.

#### CP5.5 — shell-specific activation

- **PowerShell 7.2+ / supported PSReadLine 2.2.2+**: prefer PSReadLine Predictive
  IntelliSense and its public `ICommandPredictor` contract. Respect the user's
  `PredictionSource`, `PredictionViewStyle`, key handlers, and other predictors.
  Windows PowerShell 5.1 keeps history prediction/native completion and does not
  claim plugin parity.
- **Bash**: Readline and Bash programmable completion remain authoritative.
  Use `COMP_LINE`, `COMP_POINT`, `COMP_WORDS`, and `COMPREPLY` only inside a
  supported completion/widget invocation; do not run `complete -C` processes on
  ordinary typing or replace existing compspecs.
- **Zsh**: ZLE/compsys owns context and insertion. Reuse the initialized user
  completion system; never repeatedly call `compinit` or replace user styles.
- **Fish**: Fish already provides asynchronous autosuggestions, completion
  descriptions, and a pager. Do not draw a duplicate surface unless the user
  explicitly selects Automexia UI and a supported Fish bridge can suppress
  duplication without mutating user configuration.
- **CMD**: retain DOSKEY/native editing and CP1 diagnostics. Rich suggestions
  remain unavailable until a supported editor-state API exists.
- **WSL**: activate only after a host/distribution-local authenticated transport
  is proven without translated profile writes or ambient cross-distribution
  access. Otherwise the WSL shell keeps native completion.
- **SSH/container/remote shells**: remain native-only until a separately reviewed
  authenticated sideband exists. Do not tunnel editor buffers through OSC,
  terminal output, a TCP listener, or an implicit port forward merely to obtain
  feature parity.

Existing user completion frameworks, profiles, aliases, functions, bindings,
prediction settings, and Fish abbreviations always take priority. Install,
update, disable, rollback, and uninstall own exact Automexia files/blocks and
must leave user/provider state byte-for-byte intact.

Exit: each supported shell/version has a native owner test, compatibility entry,
install/update/remove proof, and explicit unsupported fallback. One shell's
failure cannot disable completion in another pane.

#### CP5.6 — release, security, performance, and rollback gate

Ship behind an explicit preview flag first, then staged opt-in. Do not silently
turn on history/frequency sources. The settings surface explains that all data
stays local, lists every enabled source, shows storage/memory use and freshness,
provides reset/disable controls, and previews the exact native fallback. A kill
switch must disable CP5 without restart, profile edits, command loss, or changing
Tab/history behavior.

Required evidence includes:

- exact PowerShell, CMD, Bash, Zsh, Fish, WSL, Linux PTY, macOS PTY, and Windows
  ConPTY/native-GUI tests on supported hosts;
- generation, routing, split/tab/window, clone, resize/reflow, prompt/output,
  worker-loss, shutdown, sleep/resume, and rapid-typing storms;
- Unicode/grapheme/IME/RTL, spaces, quotes, multiline buffers, selections,
  shell modes, malformed frames, hostile labels, and exact replacement tests;
- no implicit network, provider, authentication, credential, history-file,
  clipboard, telemetry, extension, or terminal-output access;
- endpoint ACL/permission, peer authentication, replay/cross-session rejection,
  fuzzing, property tests, dependency-policy, SBOM, license, and advisory gates;
- bounded CPU/memory/cache/queue/message growth; cancellation latency; process,
  task, socket/pipe, file, watcher, GPU, and storage leak cycles;
- renderer-neutral layout goldens at tiny, normal, 4K, 8K, 100–300% scale,
  light/dark/high-contrast themes, and multi-pane modal/menu z-order states;
- keyboard-only and automated accessibility checks on every PR plus controlled
  NVDA/Narrator, VoiceOver, and Orca evidence before stable activation;
- a 30-day opt-in performance baseline, regression budgets, rollback drill,
  migration/disable/uninstall documentation, and last-known-good recovery.

Exit: all machine gates pass, external native accessibility evidence is linked,
and maintainers record that the Automexia surface improves a measured workflow.
Otherwise CP1 native completion remains the shipped solution.

### CP6 — ecosystem integration

Status: partially done at the proposal-only policy boundary. Proposed
[ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md), the
[schema-1 machine contract](../tests/fixtures/ecosystem/d7-cp6-ecosystem-contract-v1.json),
15 mutation tests, and the
[D7/CP6 execution audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md) are complete.
No pack or AI runtime, public SDK, download, sandbox, provider request, tool
call, or execution authority exists.

- Signed third-party action packs may map only into existing typed CP2/CP3
  actions after package digest, publisher, signature/provenance, compatibility,
  current revocation, and exact capability/data-flow review. A pack never owns
  execution; preview, collision, stale revision, production confirmation,
  insert/copy, and final revalidation remain host-owned.
- Optional AI is a separate opt-in and per-request consent flow. It may receive
  only exact selected bounded text after redaction preview and provider/locality/
  model/destination/purpose/retention/size/risk disclosure. Ambient terminal,
  history, clipboard, files, environment, credentials, agents, provider caches,
  capsules, connections, other panes, logs, telemetry, and support data remain
  unavailable.
- AI output is a bounded typed explanation or suggestion, independently risk
  classified and offered as copy/insert without Enter. Tool calls, MCP
  passthrough, background or typing-triggered requests, and automatic execution
  are outside CP6.
- Runtime work requires explicit ADR/contract acceptance followed by the package,
  custom WIT/Wasmtime, distribution/revocation, privacy, UX, native,
  accessibility, performance/resource, rollback, and release gates in the
  execution audit. CP1-CP3 and private first-party extensions remain fallback.

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
- [PowerShell predictor plug-in contract](https://learn.microsoft.com/powershell/scripting/dev-cross-plat/create-cmdline-predictor)
- [Fish interactive autosuggestions and completion pager](https://fishshell.com/docs/current/interactive.html)
- [Fish responsiveness design](https://fishshell.com/docs/current/design.html#the-law-of-responsiveness)
- [Reedline editor/menu reference](https://github.com/nushell/reedline)
- [Nucleo / `nucleo-matcher` evaluation candidate (MPL-2.0)](https://github.com/helix-editor/nucleo)
- [Carapace optional multi-shell adapter candidate](https://github.com/carapace-sh/carapace-bin)
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
