# DevOps Quick Actions and persistent aliases

Status: planned for CP2 (persistent typed Quick Actions) and CP3 (native-shell
alias projections and first-party packs). CP1 native completion is shipped;
the action store, predefined packs, and generated aliases described here are
not shipped in v0.4.

This document is the implementation authority for predefined DevOps shortcuts
and user-created alias persistence. The broader
[Command Productivity](COMMAND-PRODUCTIVITY.md) document owns CP0-CP6, the
[compatibility baseline](COMMAND-PRODUCTIVITY-COMPATIBILITY.md) owns native
shell precedence, the [threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md) owns
the accepted trust boundaries, and
[ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md) owns the
decision to persist typed actions instead of shell code.

## Product outcome

Users should be able to save a useful command once, optionally give it a short
name, and use it in every future matching Automexia session without editing five
different shell profiles or recreating it after every tab, pane, window, WSL
distribution, or restart.

The feature should make frequent DevOps work faster while preserving the facts
that:

- the shell owns parsing, editing, history, completion, execution, exit status,
  signals, and job control;
- a visible command is easier to review than an opaque abbreviation;
- production, destructive, privileged, and identity-changing operations need
  more friction than a read-only status command;
- PowerShell, Bash, Zsh, Fish, and CMD have different alias semantics; and
- user definitions, organization policy, and provider-native tools remain
  authoritative.

Automexia therefore stores a **typed Quick Action** as the durable source and
generates a small, reversible, shell-correct projection only when the user
enables an alias. Generated shell files are caches, never the database.

## Problems this feature solves

1. **Repeated setup:** a user no longer recreates the same alias in every new
   shell or terminal session.
2. **Cross-shell drift:** one action can project safely to selected supported
   shells without pretending their syntax is identical.
3. **Unreviewed copy/paste:** curated actions show purpose, arguments, context,
   risk, source, and expansion before activation.
4. **Collision surprises:** the UI detects native commands, aliases, functions,
   abbreviations, macros, and completers before installing anything.
5. **Multi-environment mistakes:** later CP4 actions bind to the current
   immutable Environment Capsule instead of changing global provider state.
6. **Broken completion:** every alias records whether and how completion follows
   the underlying official provider integration.
7. **Unrecoverable profile edits:** uninstall removes exact Automexia-owned
   projections and leaves user profiles and canonical action data intact.

## Non-negotiable principles

- **Nothing short is enabled by default.** Built-in packs are discoverable
  actions; the user selects every alias name and shell.
- **Native definitions win.** Automexia does not silently shadow an executable,
  builtin, cmdlet, alias, function, Fish abbreviation, DOSKEY macro, or native
  completion.
- **Review before enable.** The user sees the complete shell-specific expansion,
  argument forwarding, executable identity, risk, completion status, and files
  that will change.
- **Typed before textual.** Built-in and portable actions store executable plus
  argument tokens. Arbitrary shell snippets stay insert-only and cannot become
  an executable alias projection.
- **No secret aliases.** A projected alias cannot contain a token, password,
  private key, environment secret, inline kubeconfig credential, browser code,
  or secret placeholder.
- **No hidden context mutation.** A pack never changes the global Kubernetes
  context, Azure subscription, Google configuration, AWS profile, Terraform
  workspace, or working directory as an invisible side effect.
- **No startup/provider work.** Shell startup loads one already-generated local
  file. It performs no provider invocation, network call, authentication,
  package installation, source rewrite, or secret-store access.
- **No automatic execution.** Palette actions insert without Enter by default.
  A native alias executes only after the user types it and submits it through
  their shell normally.
- **One owner per artifact.** Canonical user data, built-in pack data, generated
  shell files, completion artifacts, and user profiles have separate owners.
- **Disable is complete.** Disabling integration restores native behavior even
  if canonical actions remain saved for later.

## Terminology

- **Quick Action:** persistent, typed command intent with stable ID, metadata,
  scope, risk, and insert/copy behavior.
- **Alias projection:** generated shell-native alias, wrapper function, Fish
  abbreviation, or DOSKEY macro referring to one Quick Action.
- **Built-in pack:** versioned first-party collection of disabled-by-default
  actions for one tool family.
- **User action:** action authored or copied/customized by the user.
- **Workspace action:** action from a trusted `.automexia/actions.toml`; it does
  not become a global alias automatically.
- **Action overlay:** user-owned changes to a built-in action, stored separately
  so a pack update cannot overwrite them.
- **Projection compiler:** pure, shell-specific serializer from a validated
  action and alias policy to a generated artifact.
- **Health observation:** local state describing tool availability, collision,
  artifact digest, completion linkage, reload requirement, and source version.
- **Transparent expansion:** presentation that lets the user see the real
  command before execution, naturally provided by Fish abbreviations and by
  Quick Action insertion.

## Architecture and ownership

### Placement in the current repository

| Concern | Existing owner to extend | Must not depend on |
|---|---|---|
| Quick Action, alias, pack, risk, scope, precedence, and validation models | `automexia-devops` pure modules | Renderer, GPU, PTY, shell process, provider SDK, secret store |
| Action/alias list, editor, review, conflict, and health projections | `automexia-ui-model` | Shell syntax, filesystem mutation, provider process |
| User-private store, atomic update, watcher, and last-known-good snapshot | application-owned service in `apps/automexia-terminal` using existing persistence seams | Renderer/input/VT threads, generated file as authority |
| PowerShell/Bash/Zsh/Fish/CMD projection compilers | shell-integration/command-productivity adapter layer | Provider network/authentication, terminal-grid inference |
| CP1 completion linkage and managed startup hook | existing shell-integration completion adapters | A second profile block or second completion system |
| Built-in pack manifests | versioned data owned by `automexia-devops`, validated and shipped with the signed application | Remote marketplace or mutable downloaded code |
| Contributor validation/generation | `tools/xtask` and `tools/ci` | User home mutation during checks |

Do not create one crate per provider merely for static action data. Provider
extensions become relevant at CP4 when cached Environment Capsule context and
reviewed exact launch are introduced. CP2/CP3 models remain renderer-, PTY-,
GPU-, network-, and provider-SDK-independent.

### Data flow

```text
built-in pack + canonical user source + trusted workspace source
  -> bounded parser and schema validation
  -> deterministic scope/precedence merge
  -> collision/tool/completion/risk validation
  -> immutable last-known-good action snapshot
  -> user reviews and enables one alias projection
  -> pure shell-specific compiler builds a candidate artifact
  -> syntax/native semantic test + digest/permission validation
  -> same-directory atomic replacement under generated/aliases
  -> existing managed shell hook verifies and loads it in future sessions
```

Opening a terminal does not repeat the parse/generation pipeline. Startup reads
and verifies the small pre-generated artifact once. A missing, stale, linked,
oversized, permission-unsafe, or digest-mismatched artifact fails closed to the
native shell and is repaired only through an explicit application/CLI update.

### Why typed actions instead of plain persisted aliases

| Choice | Result |
|---|---|
| Persist separate shell snippets | Rejected: inconsistent quoting, parameters, collisions, updates, security review, and removal |
| Persist only one POSIX alias string | Rejected: PowerShell, Fish, CMD, and parameterized commands do not share that grammar |
| Replace the user's line editor | Rejected: loses mature history, completion, accessibility, IME, and shell semantics |
| Store typed actions and compile native projections | Recommended: one durable model, shell-correct output, reviewable risk, disposable generated files |
| Use a general task runner as Automexia's database | Rejected: valuable project runners have their own execution/config semantics and cannot represent every interactive shell/capsule rule |

## Canonical data model

The CP2 source schema rejects unknown security-sensitive fields. CP3 extends the
accepted Quick Action model with a separate alias projection; it does not store
generated code.

```text
QuickActionDocument {
  schema_version: 1
  revision
  actions[]
}

QuickAction {
  id
  display_name
  description
  tags[]
  scope: Session | Capsule | TrustedWorkspace | ShellUser | GlobalUser | BuiltinDisabled
  shells: Powershell | Bash | Zsh | Fish | Cmd
  template: TypedArgv(executable_id, ArgumentToken[]) | RawInsertOnly(shell, text)
  placeholders[]
  working_directory_policy: Inherit | WorkspaceRoot | Fixed(path)
  risk: ReadOnly | Mutating | Destructive | Privileged
  execution: Insert | Copy | ExactLaunch
  provenance: User | BuiltIn(pack_id, version) | Imported(source_digest)
  enabled
  alias_projection?
}

ArgumentToken = Literal(value) | Placeholder(name)

AliasProjection {
  requested_name
  shells[]
  mode: Auto | CommandAlias | WrapperFunction | FishAbbreviation | DoskeyMacro
  argument_policy: None | ForwardAll | TypedBindings
  completion: Required | BestEffort | Disabled
  override_policy: NativeWins | ExplicitExactOverride
  mutating_acknowledged
  enabled
}

PackManifest {
  schema_version
  pack_id
  pack_version
  display_name
  description
  tool_family
  supported_tool_versions
  actions[]
  source_revision
  license_and_provenance
}
```

`TypedArgv` is an executable ID plus ordered tokens. It is not a command string.
`RawInsertOnly` can be stored for a specific shell and inserted for manual
review, but can never be projected as an alias, wrapper, macro, or exact launch.
Action IDs remain stable when display names or aliases change.

### Argument forwarding

An alias projection is eligible only when its arguments have one of these
contracts:

- **None:** the command takes no user-supplied arguments.
- **ForwardAll:** all remaining shell arguments are forwarded as independent
  arguments after fixed tokens, preserving empty values and boundaries.
- **TypedBindings:** named action fields map to bounded positional/named
  arguments with type, required/default rules, and shell-native completion.

String interpolation, `eval`, command substitution, redirection, pipelines,
backgrounding, multi-command separators, inline environment assignment, and
shell parsing are not argument policies. An action needing those constructs is
insert-only or becomes a separately reviewed built-in wrapper executable.

Secret placeholders, interactive credential values, and environment-value
capture make an action ineligible for alias projection. A fixed public flag
such as `--namespace` is a token; a token obtained from a provider cache is not.

### Portable alias names

The default portable policy is intentionally stricter than every individual
shell:

```text
[a-z][a-z0-9-]{1,31}
```

- Names are 2-32 ASCII characters, case-stable, visible, and free of whitespace,
  control characters, quotes, slashes, dots, equals signs, shell metacharacters,
  bidi controls, and confusable Unicode.
- One-letter names are never suggested by a built-in pack. A user may request
  one only through an advanced warning and exact collision review.
- Names beginning with `-`, `_automexia`, `automexia-internal`, or a reserved
  shell/provider prefix are rejected.
- A shell-specific extended name is allowed only in a shell-scoped action after
  that shell's parser/property/native tests; it is never exported as portable.
- Display names and descriptions may use Unicode; executable alias identifiers
  use the stricter policy.

## Persistence and cross-session behavior

### Paths and ownership

- Canonical user source:
  `<config-root>/actions/actions.toml`.
- One bounded prior revision for recovery:
  `<config-root>/actions/actions.previous.toml`.
- Immutable built-in packs: signed application resources owned by the
  `automexia-devops` package; not copied into user state unless customized.
- Trusted workspace source: `.automexia/actions.toml`, disabled until the user
  trusts that exact workspace revision.
- Generated aliases:
  `<config-root>/generated/aliases/<shell>/automexia-aliases.<ext>`.
- Generated completion remains under the existing
  `<config-root>/generated/completion/<shell>/` CP1 root.

User data and generated data never share a file. Deleting `generated/aliases`
removes active projections but does not delete saved actions. Uninstall removes
the exact managed hook/projections and preserves `actions.toml` unless the user
separately chooses `Delete my action data` with a preview.

### Transaction contract

1. Read the current version/revision through bounded no-follow checks.
2. Parse a complete candidate with deny-unknown-fields and all hard limits.
3. Resolve scopes, conflicts, tools, completion linkage, and projection
   eligibility without executing an action.
4. Compile all selected shell artifacts into private same-directory staging
   files.
5. Run syntax and semantic fixture validation; calculate schema/generator/source
   digests and file metadata.
6. Serialize writers. Across application processes use an exact lock and
   compare-and-swap source revision so one process cannot silently overwrite a
   newer edit.
7. Flush staged source/artifacts, rotate at most one private previous source,
   atomically replace, then publish one immutable application snapshot.
8. On any failure retain the complete last-known-good source and generated set;
   never publish half of a multi-shell change.

Temporary/previous files use the same user-only permissions and size ceilings as
the source. Disk-full, permission, antivirus lock, process crash, stale revision,
and concurrent-window failures return an actionable result and leave the old
state usable.

### New and active sessions

- Every new matching shell sources the already-generated file through the
  existing CP1 managed profile block, so aliases survive tabs, panes, windows,
  application restarts, and machine reboots.
- Shell startup performs no canonical rewrite, pack merge, provider call, or
  per-alias subprocess.
- Existing sessions are never injected silently. `Reload aliases in this
  session` is an explicit, bounded shell-native action when safe; otherwise the
  UI says `Open a new shell to apply`.
- Reload replaces only Automexia-owned definitions. It cannot remove a native
  definition that appeared after generation; that collision disables the
  projection and reports it.
- Session-scoped actions are memory-only. Workspace/capsule aliases are not
  globally projected in CP3; a user can copy one into a global/shell action, or
  wait for CP4's reviewed session-aware adapter.

### WSL, remote hosts, containers, and multiple machines

- Windows and each WSL distribution have separate canonical roots, permissions,
  tools, paths, shells, and generated files. Automexia never sources a Windows
  path inside Linux.
- `Copy selected actions to WSL...` previews a metadata-only transfer, validates
  the distribution/user and Linux destination, regenerates Linux projections in
  that environment, and never copies machine paths or credentials.
- SSH/remote/container installation is separate from connection. It is opt-in,
  names the destination, files, owner, and removal command, and cannot piggyback
  on opening a session.
- Initial releases provide explicit export/import rather than a proprietary
  cloud sync service. Users may version/sync `actions.toml` with their chosen
  dotfile tool, but machine-specific observations and generated files are
  excluded and concurrent revisions require review.

## Scope, precedence, and collision handling

Quick Action layers remain:

1. session-only;
2. active Environment Capsule;
3. explicitly trusted workspace;
4. shell-specific user;
5. global user;
6. disabled-by-default built-in pack.

Native shell definitions sit outside and above this order. They win unless the
user selects an exact, reversible override for that name and shell. Built-ins
cannot request an override.

### Collision inventory

Before activation, inspect without executing bodies:

| Shell | Names checked |
|---|---|
| PowerShell | aliases, functions, cmdlets, scripts, applications, keywords, and existing registered completion ownership where observable |
| Bash | aliases, functions, builtins, keywords, PATH commands, and completion specifications |
| Zsh | aliases including global aliases, functions, builtins/reserved words, commands, widgets, and completion definitions |
| Fish | abbreviations, functions, builtins, PATH commands, bindings, and completions |
| CMD | internal commands, PATH/PATHEXT commands, DOSKEY macros for `cmd.exe`, and reserved batch syntax |

The UI shows the requested name, existing owner/type/source, proposed owner,
affected shells, and options: `Choose another name`, `Keep native definition`,
`Disable existing Automexia projection`, or—only for a user action—`Override
this exact native definition`. Override is never preselected and can be revoked
without editing the native definition.

If a collision appears later, the next verified load disables the Automexia
projection rather than racing or shadowing it. Generated-file tampering is not
imported as a customization; the UI offers `Regenerate from source`, `Inspect
diff`, or `Keep disabled`.

## Shell-specific projections

One compiler cannot safely emit all shells. Every compiler consumes the same
validated tokens but owns its native quoting, argument forwarding, collision,
completion, reload, and uninstall tests.

### PowerShell 5.1 and PowerShell 7+

- Use `Set-Alias` only when the action is exactly one command name with no fixed
  arguments or logic. Microsoft documents that aliases cannot contain command
  parameters.
- Use a generated namespaced function for fixed tokens or forwarding. Invoke
  the resolved executable without `Invoke-Expression`, script-block creation
  from strings, or concatenated commands; forward remaining arguments as
  PowerShell objects/strings while preserving boundaries.
- Do not change PSReadLine keys, predictor settings, history, error preferences,
  output formatting, current directory, or global provider state.
- Preserve the underlying command's success/error stream and exit semantics.
- Register completion only through the reviewed CP1 PowerShell consent/override
  contract; never claim completion ownership that PowerShell cannot inspect
  safely.
- Load from the existing unique Automexia profile block; never append one line
  per alias to `$PROFILE`.

### Bash

- Use an alias only for a literal command name with no arguments. GNU Bash
  recommends functions when arguments are required and documents that aliases
  expand while input is read.
- Generate one function per parameterized action, with fixed token literals and
  quoted `"$@"` forwarding. Do not use `eval`, `bash -c`, `source` on action
  data, command substitution, or multi-command bodies.
- Preserve `set`/`shopt`, traps, positional parameters, current directory,
  standard streams, signals, and the child exit status.
- Register a completion wrapper only after CP1's underlying definition exists;
  it translates the fixed prefix and preserves `COMP_WORDS`, `COMP_CWORD`,
  quoting, and native fallback.
- Definitions are interactive only. Noninteractive scripts, `BASH_ENV`, and
  unrelated SSH command execution must not inherit Automexia aliases implicitly.

### Zsh

- Prefer functions for parameterized actions. Do not use global aliases (`alias
  -g`); their expansion outside command position is too broad for a safe pack.
- Keep functions under an Automexia namespace internally while exposing only
  the reviewed public alias/function name.
- Avoid alias/function parse-time collisions documented by Zsh; generate
  definitions as separate syntactic units and use native `unalias`/`unfunction`
  only for Automexia-owned names.
- Reuse the user's existing ZLE/compsys/`compinit`; do not rerun `compinit`,
  reorder global `fpath`, or change styles/options.
- Completion prefix translation preserves `$words`, `$CURRENT`, context, and
  user functions. Unsupported versions retain the action without projection.

### Fish

- Prefer a **global abbreviation defined by the managed startup file** for safe
  interactive expansions. Fish shows the expanded command before execution and
  stores the expanded command in history, which is the most transparent UX.
- Do not use universal abbreviations by default: they mutate Fish-owned
  persistent universal-variable state and complicate exact uninstall. Universal
  scope requires separate explicit consent and ownership proof.
- Use a Fish function only when typed argument forwarding cannot be represented
  as an abbreviation. Do not create a string-evaluation path.
- Preserve Fish bindings, autosuggestions, pager, completion, variables, and
  current directory. `Ctrl+Space` remains the user's native literal-abbreviation
  escape.
- Completion follows the expanded provider command where possible; a wrapper
  completion is generated only from the existing CP1 artifact.

### CMD

- Generate one private DOSKEY `/macrofile` for `cmd.exe`; load it through the
  existing Automexia CMD bootstrap, not the registry `Command Processor\AutoRun`
  keys.
- Support only one fixed command plus fixed tokens and optional `$*`/bounded
  positional forwarding. Reject `$T`, `$B`, `$G`, `$L`, redirection, pipelines,
  chained commands, delayed-expansion dependence, and free-form macro syntax.
- DOSKEY macros are interactive and cannot be used from batch programs; the UI
  states this limitation and never claims PowerShell/Bash parity.
- CMD has no equivalent rich programmable-completion contract. Keep native file
  completion and expose completion as `Unavailable in CMD` rather than adding a
  hidden line editor.
- Do not set or edit global registry AutoRun values; Microsoft documents that
  those values execute whenever `cmd` starts and incorrect registry edits can
  damage the system.

## Completion linkage

Enabling an alias produces one of four explicit health states:

- `Linked`: native CP1 completion is adapted and version/digest valid;
- `Native after expansion`: Fish or a simple alias expands to the real command;
- `Unavailable`: shell/provider cannot support correct completion;
- `Blocked`: an existing completer/collision or unsafe provider generator
  prevents activation.

Completion never runs a provider because an alias is enabled or a key is typed.
It reuses CP1's pre-generated official artifact. Fixed-prefix actions require a
tested shell adapter that presents the underlying provider as if the fixed
tokens were already typed, then maps the replacement back without changing the
user's buffer or submitting Enter.

If correct offset translation is unavailable, alias activation can proceed only
after the UI clearly shows `Completion unavailable`; built-in pack quality gates
may make completion mandatory for selected actions. Kubernetes aliases follow
Kubernetes' documented alias completion registration. Dynamic Helm/plugin
completion remains separately opt-in because it can execute plugin code.

## User experience

The management surface is `Command Center > Quick Actions & Aliases`, a
window-level application page/modal above terminal content. It does not resize a
PTY, inspect terminal cells, or belong to only one pane.

### Wide layout

```text
+--------------------------------------------------------------------------------+
| Quick Actions & Aliases                                Import  Export  Settings X|
| [ Search actions, aliases, tools, tags...                                  ]    |
+----------------------+--------------------------------------+------------------+
| Overview             | BUILT-IN PACKS                       | kubernetes.get-pods|
| My actions        18 | Git                 8 actions        | Read only          |
| Enabled aliases   11 | Docker              9 actions        | kubectl get pods   |
| Conflicts          2 | Kubernetes         12 actions        |                    |
| Needs reload       1 | Terraform/OpenTofu  8 actions        | Alias: kgp         |
|                      | AWS / Azure / GCP                   | Shells: Bash Zsh   |
| Packs                |                                      | Completion: linked |
|  Git             3/8 | MY ACTIONS                           | Collision: none    |
|  Docker          2/9 | > show staging logs   alias: slog   |                    |
|  Kubernetes     3/12 |   release status      no alias      | [Preview shells]   |
|  Helm            1/7 |                                      | [Enable alias]     |
|  Terraform       0/8 |                                      | [Insert command]   |
+----------------------+--------------------------------------+------------------+
| CP1 completion healthy | 11 aliases active | PowerShell requires reload         |
+--------------------------------------------------------------------------------+
```

- Left navigation is 220-280 logical pixels and exposes counts in text.
- The center list is virtualized, locally searchable, and grouped by source or
  tool. Selection never executes an action.
- The 320-440 pixel inspector shows complete tokens, placeholders, risk, shells,
  completion, provenance, source version, scope, and health.
- At medium width the inspector becomes a drill-in page. Below 700 logical
  pixels or high text scale, the surface is single-column with Back/Close and a
  sticky primary action. No font shrink forces the wide layout.

### Create/save workflow

1. Choose `New Quick Action`, `Copy built-in`, or `Import simple alias`.
2. Enter a stable display name/description and select a fixed executable from
   the allowed local tool inventory.
3. Build ordered argument tokens and typed public placeholders. A shell-specific
   raw snippet is visibly downgraded to insert-only.
4. Choose scope. CP3 alias projection is available only for global or one-shell
   user scope; workspace/capsule/session actions remain palette actions.
5. Select risk. The model proposes a minimum risk from tokens/tool metadata; the
   user may increase but cannot lower the built-in minimum.
6. Preview the inserted command and every selected shell projection using
   synthetic values containing spaces, Unicode, quotes, and leading dashes.
7. Request an alias name. Live local checks show collisions and completion
   health for each shell without executing definitions.
8. Save the canonical action atomically. Enable the alias in a separate final
   confirmation that lists exact generated files and reload behavior.

Saving and enabling are separate so a useful palette action is not blocked by
one shell's alias collision.

### Explain, preview, and test

- `Explain` shows source layer, pack/version, final precedence, risk rationale,
  executable/tool health, fixed tokens, argument forwarding, working-directory
  rule, completion source, generated file/digest, and why a projection is
  disabled.
- `Preview shells` shows syntax-highlighted, non-editable generated output plus
  a token-by-token semantic view. The token view is authoritative; display code
  is never copied back into the model.
- `Test projection` runs a **repository-owned capture fixture**, not the real
  DevOps command. It proves exact argv, stdin/stdout/stderr, exit status, CWD,
  environment allowlist, and argument forwarding for the selected native shell.
- `Insert command` places the expanded command at the active editor cursor and
  does not press Enter.
- `Copy command` is explicit and disabled for secret-bearing actions.

### Health and recovery

Health states include `Ready`, `Disabled`, `Missing tool`, `Unsupported tool`,
`Collision`, `Completion unavailable`, `Stale source`, `Reload required`,
`Tampered artifact`, `Unsafe permissions`, `Malformed source`, and `Generation
failed`. Each has one primary recovery action and preserves last-known-good
behavior.

`doctor` is read-only. It checks source/artifact bounds, versions, digests,
permissions, exact parent chains, tool resolution, names, collisions, completion
linkage, and reload state. It never invokes providers, evaluates definitions,
reads history/secrets, or repairs automatically.

### Planned product CLI

The CP2/CP3 implementation should expose scriptable, noninteractive management:

```text
automexia actions list [--scope <scope>] [--json]
automexia actions show <id> [--json]
automexia actions add --file <candidate.toml> --dry-run
automexia actions remove <id> --dry-run
automexia aliases list [--shell <shell>] [--json]
automexia aliases enable <action-id> --name <name> --shell <shell> --dry-run
automexia aliases disable <action-id> [--shell <shell>] --dry-run
automexia aliases doctor [--json]
automexia aliases regenerate [--shell <shell>] --dry-run
automexia aliases export --output <file>
automexia aliases import --input <file> --dry-run
```

Mutating commands support `--dry-run`, stable structured output, nonzero failure
codes, no prompts in JSON mode, optimistic source revision, and no provider
execution. `--force` cannot bypass security ceilings, secret rules, path safety,
native ownership, destructive alias policy, or unknown schema versions.

## Built-in first-party DevOps packs

Packs optimize frequent, understandable workflows—not the shortest possible
spelling. Every action has a stable ID, description, minimum tool version,
tokens, placeholders, risk, context requirements, completion rule, tests, and
documentation link. Suggestions below are offered only when collision-free and
are never activated automatically.

### Pack policy

- Read-only actions may offer a short suggestion.
- Mutating actions default to insert/review; only reversible, bounded actions
  may offer a descriptive alias after an extra warning.
- Destructive or privileged actions have **no alias projection**. They remain
  reviewed Quick Actions and cannot be made one-letter shortcuts.
- Login/logout, context/subscription/project/workspace changes, force pushes,
  pruning, delete/destroy/apply, secret display, and broad `--all` mutations do
  not receive built-in aliases.
- A target, namespace, profile, region, subscription, project, or workspace is a
  visible token/placeholder. No pack hides it in mutable global state.
- CP3 packs are static. Provider/capsule-aware variants wait for CP4/D5/D6 and
  show freshness/risk before insertion.

### Curated baseline

| Pack | Example action ID | Token intent | Risk | Suggested alias |
|---|---|---|---|---|
| Git | `git.status` | `git status --short --branch` | ReadOnly | `gst` |
| Git | `git.log.graph` | `git log --graph --decorate --oneline --all` | ReadOnly | `glg` |
| Git | `git.diff.staged` | `git diff --staged` | ReadOnly | `gds` |
| Git | `git.switch` | explicit branch argument | Mutating | `gsw` after review |
| Docker | `docker.ps` | running container summary | ReadOnly | `dps` |
| Docker | `docker.compose.ps` | Compose service summary | ReadOnly | `dcps` |
| Docker | `docker.logs` | logs for explicit container/service | ReadOnly | `dlog` |
| Docker | `docker.inspect` | inspect explicit object | ReadOnly | `dins` |
| Kubernetes | `kubernetes.get-pods` | pods in explicit/current capsule namespace | ReadOnly | `kgp` |
| Kubernetes | `kubernetes.get-services` | services in explicit/current capsule namespace | ReadOnly | `kgs` |
| Kubernetes | `kubernetes.describe` | explicit resource kind/name | ReadOnly | `kdesc` |
| Kubernetes | `kubernetes.logs` | explicit pod/container | ReadOnly | `klog` |
| OpenShift | `openshift.get-pods` | pods in explicit/current capsule project | ReadOnly | `ogp` |
| OpenShift | `openshift.current-project` | show current project only | ReadOnly | `oproj` |
| Helm | `helm.list` | releases in explicit namespace | ReadOnly | `hls` |
| Helm | `helm.status` | status for explicit release | ReadOnly | `hst` |
| Terraform | `terraform.fmt-check` | formatting check only | ReadOnly | `tfmtc` |
| Terraform | `terraform.validate` | validate current reviewed directory | ReadOnly | `tfval` |
| Terraform | `terraform.plan` | explicit reviewed plan path/options | Mutating/network | no built-in short alias |
| OpenTofu | `opentofu.validate` | validate current reviewed directory | ReadOnly | `toval` |
| OpenTofu | `opentofu.plan` | explicit reviewed plan path/options | Mutating/network | no built-in short alias |
| AWS | `aws.identity` | show current caller identity | ReadOnly/network | `awho` after CP4 context |
| Azure | `azure.account-show` | show selected account/subscription | ReadOnly | `azwho` after CP4 context |
| Google Cloud | `gcp.config-list` | show selected configuration/project | ReadOnly | `gcctx` after CP4 context |
| SSH | `ssh.agent-public-list` | list public agent fingerprints | ReadOnly | `sshkeys` |
| SSH | `ssh.connect` | selected reviewed OpenSSH alias | Remote/session | no static CP3 alias; CP4/D5 action |

The actual manifest tokens must be verified against supported tool versions;
the table is the product baseline, not permission to hard-code unstable output
parsing. Pack documentation explains whether an action may contact a daemon,
cluster, provider, backend, or remote even when it is logically read-only.

### Pack update behavior

- Built-in manifests are tied to the signed application version and validated
  in CI. There is no background pack download in CP3.
- A pack update creates a new immutable version. Unmodified user enablement can
  adopt it after compatibility checks; token/risk/name changes require review.
- Customized actions are forks/overlays. They never get overwritten; the UI
  shows old/new semantic diff and offers keep, merge, or create another action.
- Removed/deprecated tool commands disable the affected action with an
  explanation. They never fall back to a similarly named command.
- Tool absence disables only that pack. Other actions and the terminal remain
  usable.

## User-created aliases and import/export

### Lifecycle

Users can create, preview, test with a capture fixture, save, enable, reload,
disable, rename, rescope, duplicate, export, and delete. Every change is an
atomic canonical-source revision and a complete multi-shell projection update.

Disabling removes only generated ownership and preserves the action. Deleting
requires a dependency preview listing aliases, workspace references, capsule
references, completion links, and custom overlays; it never deletes provider
config, a native alias, history, credential, or task file.

### Importing existing shell aliases

Import is explicit and inventories definitions without executing their bodies.
Only simple one-command aliases with fixed literal tokens and safe argument
forwarding can become typed actions.

Unsupported examples remain visible but unimported:

- PowerShell functions/script blocks or aliases requiring parameters;
- Bash/Zsh functions, global aliases, command substitution, pipelines,
  redirection, multiple commands, or `eval`;
- Fish functional/regex abbreviations or functions with arbitrary bodies;
- DOSKEY `$T`/redirection/pipeline macros and macros scoped to another program;
- Git `alias.*` entries beginning with `!` because they execute shell commands;
- any definition containing a secret, inline credential, control character,
  machine-only private path, or unknown encoding.

The preview shows original shell/source, parsed tokens, portability, risk,
collisions, lost semantics, and the proposed canonical action. Import never
rewrites or removes the original definition. Native remains the winner until the
user removes it themselves or chooses a reversible exact override.

### Export, backup, and synchronization

- Default export contains canonical user actions and alias choices only.
- It excludes generated files, built-in pack copies, session actions, cached
  health, resolved executable paths, environment values, secrets, credentials,
  provider caches, history, and machine-specific opaque references.
- Private paths/account/cluster/host labels are redacted unless individually
  selected and clearly marked nonportable.
- Export is a bounded versioned TOML document with source digest and conflict
  policy; import is dry-run first and never silently replaces a newer revision.
- Automexia does not initially operate a cloud sync service. Users may back up
  the canonical file with an organization-approved encrypted/dotfile system.
  Recovery documentation states that aliases can be recovered from that source;
  external credentials cannot.

## Existing tools and open-source projects

The feature should reuse supported interfaces and learn from established tools
without importing their execution engines into terminal core.

| Tool/project | Recommended relationship | Boundary |
|---|---|---|
| Native PowerShell/Bash/Zsh/Fish/DOSKEY mechanisms | Use as projection/runtime | One generated managed file per shell; no editor replacement |
| Existing CP1 provider completion | Reuse for alias completion | No new provider call at startup/typing; preserve native precedence |
| Git `alias.*` | Respect and optionally import simple entries | Do not edit `.gitconfig`; reject shell aliases beginning with `!` |
| `just` | Detect as an optional trusted-workspace task source/reference | Do not bundle or copy recipes; after workspace trust, create an action invoking one explicit recipe |
| Task (`go-task`) | Same optional trusted-workspace integration | Taskfiles can contain commands/variables/includes; never scan/execute automatically |
| `mise run` tasks | Same optional trusted-workspace integration | Use explicit `mise run <task>`; avoid ambiguous shorthand that future mise commands can shadow |
| `kubectl-aliases` and shell-framework packs | UX/import reference only | Large opaque alias sets, collisions, version drift, and missing cross-shell completion make default installation inappropriate |
| `kubectx`/`kubens` | Optional external tools/actions | Context mutation must be visible and CP4 capsule-aware; never silently change all panes |
| `direnv` | Environment ownership reference, not alias store | Automexia does not evaluate `.envrc` or copy its environment into other sessions |
| Dotfile managers such as chezmoi | User-selected backup/sync | Automexia supplies portable export but does not take over encryption, Git, or remote sync |
| Carapace/Reedline/Rustyline | CP5 references, not CP3 dependencies | Completion/editor replacement is outside alias persistence |

### Dependency recommendation

Add no runtime crate merely to generate aliases. Reuse the existing TOML/Serde
stack, SHA-256/digest utilities, atomic/private-file helpers, bounded worker and
watcher lifecycle, typed UI model, and CP1 shell installers. Keep each shell
serializer small and pure.

If a new locking, diff, or matcher dependency is proposed, require a written
comparison of correctness, advisory/license history, transitive/binary/compile
cost, Windows/macOS/Linux behavior, maintenance, and measured benefit. The
1,024-action search path can initially use the project's existing deterministic
matcher; CP5's separate matcher evaluation must not expand CP3's dependency
surface by accident.

## Security and privacy

### Risk and alias eligibility

| Risk | Palette behavior | Alias policy |
|---|---|---|
| ReadOnly | Insert by default; explicit normal shell execution | Eligible after collision/completion review |
| Mutating | Review-before-insert with target/effect | Eligible only for bounded reversible actions and descriptive names after extra confirmation |
| Destructive | Blocking target/effect review | No alias projection |
| Privileged | Capability/OS/provider review | No alias projection |

An action cannot lower the risk inherited from its built-in pack, tokens,
provider operation, workspace trust, or capsule. Unknown imported actions start
at the stricter risk and remain insert-only until reviewed.

### Required controls

1. Parse actions as data with deny-unknown-fields; never `source` TOML or imported
   text.
2. Compile from tokens using separate PowerShell, POSIX/Bash, Zsh, Fish, and CMD
   serializers; no shared generic escaping function claims parity.
3. Reject NUL, control/bidi characters, invalid names, oversized values,
   traversal, devices, unsafe shares, symlinks/reparse points, nonregular files,
   and unsafe ownership/permissions.
4. Never emit `eval`, `Invoke-Expression`, `sh -c`, `cmd /c`, dynamic script
   blocks, command substitution, pipelines, redirects, or chained command text.
5. Resolve tools from the CP1 fixed policy, record version/file identity in
   health metadata, and regenerate on a reviewed identity change.
6. Detect all native name owners before activation and at verified reload;
   built-ins never override.
7. Keep tokens, passphrases, environment values, history, terminal text,
   provider caches, private keys, kubeconfig credentials, and secret references
   out of sources, artifacts, diagnostics, logs, crashes, telemetry, exports,
   clipboard history, screenshots, and AI surfaces.
8. Workspace actions require exact workspace trust and cannot project globally.
   Trust/digest loss immediately removes them from the active index.
9. Generated files include schema, generator, source, shell, tool-version, and
   digest headers; the managed hook verifies the exact private regular file.
10. Install/update/uninstall preflight every target before the first mutation,
    own one marked profile block, and preserve arbitrary surrounding Unicode
    content.
11. No background marketplace/download exists in CP3. First-party pack changes
    pass repository signing, provenance, dependency, and protected review.
12. No analytics records command text or alias usage. Optional aggregate product
    metrics, if ever added, require a separate privacy decision and cannot
    include identifiers or providers.

### Misuse-resistant behavior

- Never suggest aliases named `rm`, `del`, `sudo`, `ssh`, `kubectl`, `docker`,
  `git`, another real tool, or a confusing one-character lookalike.
- Never offer a destructive convenience pack such as force push, recursive
  delete, global prune, `kubectl delete`, `helm uninstall`, `terraform destroy`,
  `tofu destroy`, or provider resource deletion.
- Do not hide `--context`, `--namespace`, profile, subscription, project,
  region, workspace, host, or production intent when it affects target scope.
- Do not print a real expanded secret command for preview/testing.
- A failed projection does not fall back to a raw snippet or another shell.
- A missing provider does not trigger installation or network access.

## Performance, resilience, and resource budgets

These provisional targets become ratchets after a named 30-day baseline; they
are not current v0.4 claims.

| Operation | Initial target/invariant |
|---|---|
| Warm load of last-known-good 1,024-action snapshot | <= 25 ms p95 off renderer/input/PTY paths |
| Search/filter 1,024 actions | <= 16 ms p95; deterministic and allocation-bounded |
| Collision check for 256 aliases | <= 25 ms p95 from cached shell inventory; no definition body execution |
| Compile one shell's 256 projections | <= 50 ms p95 background target |
| Verify/source one generated alias file at shell startup | <= 50 ms p95 post-warmup, no subprocess/network |
| Regenerate all five shell artifacts | Bounded/cancellable; atomic all-or-old publication |
| Active in-process action/health cache | <= 8 MiB |
| Canonical source/generated file | <= existing 1 MiB per-file ceiling |
| Watchers | Exact canonical source, exact generated files, and reliable parents only; no recursive workspace/home watch |

Hard existing limits remain: 1,024 actions across active layers, 256 enabled
alias bindings, 64 arguments/action, 32 placeholders/action, 64 tags/action, 4 KiB/string, and
1 MiB/generated adapter. CP3 must define lower pack/name/token limits in its
machine contract; it may never raise CP0 ceilings.

Parsing, merging, collision inventory, generation, and health run outside the
renderer, input, VT, PTY, resize, and shell-output paths. Events coalesce to the
latest source generation. Late work cannot publish across application instance,
shell, workspace, capsule, or source revision. Shutdown cancels workers and
releases files, locks, watchers, tasks, process handles, and staging files.

Last-known-good behavior covers malformed edits, partial writes, antivirus
locks, disk full, permission changes, tool replacement, watcher storms, process
crash, concurrent application instances, sleep/resume, and uninstall. No retry
loop rewrites files continuously; backoff is bounded and repair is explicit.

## Accessibility and responsive UX

- Expose action/alias rows, pack groups, search, filters, risk, source, enabled,
  collision, completion, reload, and primary actions through the existing
  renderer-neutral AccessKit plan.
- Use text plus icon for risk/health; never color alone. Generated code preview
  has an equivalent token list and screen-reader description.
- Full keyboard navigation supports search, list movement, open details, Back,
  enable/disable, conflict options, and close without executing an action.
- A final enable confirmation receives initial focus on its descriptive title or
  Cancel for destructive-looking changes, never silently on Enable.
- Focus is trapped in the application modal and returns to the exact invoking
  pane/control. The underlying terminal is visually dimmed and accessibility-
  inert; opening the UI does not resize PTYs or change selection.
- At narrow sizes and 200%/400% text scale, navigation becomes a drawer and the
  inspector a separate page. Alias, full action name, risk, collision, and
  primary action remain visible; details scroll rather than truncate silently.
- Support high contrast, reduced motion/transparency, magnification, Narrator/
  NVDA, VoiceOver, and AT-SPI/Orca controlled evidence.

## Verification plan

The planned boundary is protected by the versioned
[`CP2/CP3 alias specification fixture`](../tests/fixtures/command-productivity/cp2-cp3-alias-spec-v1.json),
[`check_devops_alias_spec.py`](../tools/ci/check_devops_alias_spec.py), and its
mutation suite. CP2.0 additionally provides the capability-free
`automexia-devops::actions` model, bounded in-memory TOML parser, deterministic
validator, readable valid fixture, and an eleven-case hostile TOML corpus. The
machine gates freeze the exact three-file source allowlist and reject filesystem,
process, network, environment, UI, PTY, shell-profile, async-runtime, and unsafe
code authority in that boundary.

The overall CP2/CP3 status remains planned and non-activated. There is no action
store, watcher, runtime alias, generated shell file, provider call, UI, or exact
launch in CP2.0. Those capabilities still require the later phase-specific
implementation and native evidence below; a valid model is not evidence that
aliases are shipped.

### Pure model and persistence tests

- Schema versions, deny-unknown-fields, all hard limits, stable IDs, pack
  versions, overlays, scope precedence, deterministic ordering, and risk floor.
- Alias portable/shell-specific name properties with reserved names, Unicode
  confusables, bidi/control input, case rules, length boundaries, and collisions.
- Exact source revision/CAS behavior across multiple windows/processes; atomic
  save, previous revision, crash between every write step, disk full, read-only,
  malformed source, partial file, permissions, symlink/reparse swaps, and
  last-known-good recovery.
- Import/export round-trip, redaction, portability, unsupported constructs,
  native-wins behavior, built-in update/custom overlay merge, and exact delete.
- Watcher coalescing, late-generation rejection, repeated enable/disable,
  suspend/resume, shutdown, and no handle/task/file/storage growth.

### Projection compiler tests

For every shell, property/golden/native tests cover:

- zero/fixed/forwarded/typed arguments;
- empty strings, spaces, tabs, Unicode/graphemes, quotes, backslashes, leading
  dashes, percent/dollar/exclamation/caret characters, glob/metacharacters,
  maximum lengths, and hostile input;
- exact argv captured by a harmless fixture executable;
- unchanged stdin/stdout/stderr, exit code, signal/Ctrl+C, CWD, environment,
  prompt/editor options, history owner, and shell state;
- syntax validation on Windows PowerShell 5.1, current PowerShell, CMD, supported
  Bash/Zsh/Fish on Linux and macOS, and Bash/Zsh/Fish inside WSL;
- no `eval`/shell command construction/raw snippet projection;
- native command/function/alias/abbreviation/macro/completion collisions;
- digest/version/tool mismatch, tamper, unsafe permission/link, missing tool,
  and disabled integration fallback;
- exact reload/unload/uninstall with arbitrary surrounding profile content.

Semantic equivalence is asserted against token arrays and the capture fixture,
not by comparing generated strings between shells.

### Pack tests

- Every manifest/action has documentation, stable ID, unique name, supported
  tool range, exact tokens, placeholders, minimum risk, context rule, completion
  rule, provenance/license, and expected output independence.
- Read-only/mutating/destructive/privileged classification mutations fail.
- No destructive/privileged/login/global-context-changing action has an alias.
- Official tool fixtures prove command/version behavior without contacting a
  real provider on PRs. Controlled provider runs verify selected read actions
  against least-privilege test environments without public logs.
- Missing/old/new tools, feature/plugin absence, offline, auth-required, expired,
  and changed output leave truthful health and never rewrite actions.
- Pack update/custom overlay/deprecation and rollback preserve user choices.

### Completion tests

- Each enabled alias reports linked/native/unavailable/blocked truthfully.
- Fixed-prefix translation preserves shell buffer/cursor/token indices and
  native results without sending Enter or executing a provider.
- Kubernetes documented alias completion works where supported; Helm dynamic
  plugins remain disabled unless explicitly enabled under CP1 policy.
- User completers win, duplicate surfaces are absent, stale provider artifacts
  fall back, and alias disable removes only Automexia linkage.

### UI and accessibility tests

- Renderer-neutral wide/medium/narrow/extreme layouts at 100%, 150%, 200%, and
  400% scale; zero/one/1,024 actions; zero/one/256 aliases; long Unicode labels;
  light/dark/high-contrast/reduced-motion; every health/risk/conflict state.
- Keyboard-only create/preview/save/enable/disable/rename/import/export/recovery
  journeys; modal z-order, focus trap/return, screen-reader tree, announcements,
  touch targets, and no PTY geometry mutation.
- Preview token/code parity, exact file list, collision owner, completion status,
  reload requirement, and no accidental execution from Enter while browsing.

### Security, fuzz, and mutation tests

- Fuzz action/import parsers, alias names, placeholder bindings, pack manifests,
  scope merge, each projection serializer, DOSKEY substitutions, and generated
  metadata headers.
- Mutation-test native-wins, destructive alias denial, secret-field rejection,
  workspace trust, path/permission/no-follow checks, digest/version checks,
  atomic publication, redaction, and no-provider-startup policy.
- Canary tokens/private paths/environment/history values must be absent from
  source artifacts, generated files, logs, crashes, diagnostics, screenshots,
  exports, clipboard history, telemetry, and AI surfaces.
- Static scans reject added evaluation primitives, provider calls in startup,
  recursive home/workspace watches, cloud sync, remote pack downloads, and new
  runtime authority before their own ADR/gate.

### Performance and leak tests

- Criterion targets for parse/merge 1/256/1,024 actions, collision 1/64/256,
  compile/verify each shell, search, reload, and pack update diff.
- Native post-warmup startup measurement compares CP1 baseline, CP1+empty CP3,
  and 256 aliases for each shell/OS.
- 1,000 save/regenerate/reload cycles plus concurrent-window, watcher-storm,
  antivirus-lock, sleep/resume, shell churn, and application shutdown assert
  bounded memory, CPU, files, storage, handles/descriptors, watchers, locks,
  processes, tasks, and temporary artifacts.
- ASan/TSan/Miri/Loom run on suitable pure/concurrent modules; long fuzz and
  native macOS/Windows/Linux resource tools remain nightly/controlled evidence.

### Planned contributor commands

```text
python tools/ci/check_devops_alias_spec.py
python tools/ci/test_devops_alias_spec.py
cargo xtask verify actions
cargo xtask verify aliases
cargo xtask test actions
cargo xtask test aliases --shell <powershell|bash|zsh|fish|cmd>
cargo xtask bench aliases
cargo xtask qa --full --bundle
```

Checks are non-mutating. Generation tests use isolated temporary roots and fake
profiles; they never install aliases into a contributor's real shell. Controlled
native smoke uses a disposable test account/profile or exact backup/restore
preflight and records redacted evidence.

## Delivery phases

### CP2.0 - contract and fixtures (implemented)

- Schema 1, revision metadata, scopes, limits, typed argv/raw-insert templates,
  placeholders, risk, execution, provenance, and alias eligibility are frozen in
  the versioned machine fixture and pure Rust model.
- Parsing rejects sources above 1 MiB before TOML decoding, unknown fields,
  unbounded nested collections, duplicate IDs/shells/tags/placeholders, controls
  and bidi text, unsafe command IDs, invalid references, secret/default leakage,
  raw alias projection, risky aliases, and unreviewed mutating aliases.
- The eleven-case hostile TOML corpus, 34 alias-policy mutations, 19 command-
  productivity mutations, and exact three-file architecture allowlist fail
  closed without adding process, network, filesystem, secret, profile, UI, PTY,
  async-runtime, or unsafe-code authority.

Exit: satisfied at the pure source/model boundary. The focused commands are:

```text
cargo test -p automexia-devops --all-targets --locked
cargo clippy -p automexia-devops --all-targets --locked -- -D warnings
python tools/ci/check_devops_alias_spec.py
python tools/ci/test_devops_alias_spec.py
python tools/ci/check_command_productivity.py
python tools/ci/test_command_productivity.py
```

No runtime store, watcher, alias, shell projection, UI, or execution path exists;
CP2.1 is the next activation boundary.

A short 2026-08-16 local Windows Criterion smoke measured median validated TOML
parsing at 7.46 us for one action, 3.37 ms for 256 actions, and 18.36 ms for the
1,024-action ceiling. The maximum case remains below the provisional 25 ms warm-
load budget. This deliberately short, noisy run proves the benchmark works; it
is not the required controlled 30-day cross-host baseline or a release ratchet.

### CP2.1 - user-private action store

- Implement bounded parse, atomic source/one previous revision, serialized
  compare-and-swap writes, last-known-good snapshot, exact watcher, and CRUD.
- No provider process, alias generation, workspace activation, or exact launch.

Exit: corruption/concurrency/crash/permission/storage/leak tests pass on
Windows, Linux, and macOS.

### CP2.2 - action search, editor, and insert/copy

- Build renderer-neutral list/editor/review, scope merge, placeholders, risk,
  local search, explicit import/export, and shell-native insert without Enter.
- Workspace source remains disabled until exact trust; secret expansion is not
  stored/copied.

Exit: UI/accessibility/responsive, Unicode/quoting/insertion, privacy,
performance, and native shell evidence passes.

### CP3.0 - projection compiler baseline

- Freeze portable names, alias eligibility, shell model, collision inventory,
  completion health, generated metadata, and rollback/uninstall contract.
- Implement pure PowerShell/Bash/Zsh/Fish/CMD serializers plus capture-fixture
  semantic equivalence; keep activation disabled.

Exit: property/fuzz/native syntax and exact-argv tests pass for all supported
shells without touching real profiles.

### CP3.1 - persistent opt-in user aliases

- Extend the existing CP1 managed hook to verify/load one exact alias file.
- Add preview/test/enable/disable/rename/reload/doctor/regenerate with atomic
  all-or-old generation and native-wins conflicts.
- Keep aliases limited to global/shell user scopes and eligible risks/templates.

Exit: aliases survive new sessions/restarts on Windows/Linux/macOS/WSL; active
reload is truthful; startup budgets, completion linkage, uninstall, and repeated
resource cycles pass.

### CP3.2 - first-party static DevOps packs

- Ship reviewed Git, Docker/Compose, Kubernetes/OpenShift, Helm,
  Terraform/OpenTofu, AWS, Azure, Google Cloud, and SSH manifests.
- Enable no alias by default. Validate tool versions, risk floors, completion,
  documentation, update/overlay/deprecation, and provider absence.

Exit: every action/pack test passes and destructive/privileged/context-changing
actions cannot acquire an alias.

### CP3.3 - explicit import and trusted task-runner bridges

- Import only simple native aliases after dry-run.
- After workspace trust, allow an action to invoke one explicit named
  `just`/Task/`mise run` task through normal shell insertion; do not parse/copy
  recipe bodies or execute listing during automatic discovery.

Exit: malicious workspace/task/alias fixtures, revocation, rename/conflict,
portable export, and clean removal pass. No remote pack marketplace ships.

### CP4 - capsule-aware actions

- Bind selected cached public connection/provider/Kubernetes/IaC context with
  freshness and exact session/capsule generation.
- Add provider-aware pack variants and reviewed D3 exact launch only after D5/D6
  gates. Static aliases never mutate a shared global context.

Exit: multi-pane multi-cloud isolation, stale/offline/expired context, production
risk, capability revocation, cancellation, audit redaction, and resource tests
pass.

## Acceptance criteria

CP2/CP3 are complete only when:

- one saved global/shell action appears in every future matching Automexia
  session without rewriting source at startup;
- PowerShell, Bash, Zsh, Fish, CMD, WSL, Linux, and macOS projections preserve
  exact arguments, shell state, completion ownership, exit status, and clean
  uninstall within their declared capability;
- users can preview, fixture-test, enable, disable, rename, import, export,
  diagnose, regenerate, reload, and delete without editing profiles manually;
- built-in packs enable no alias automatically and destructive/privileged/
  login/context-changing operations cannot receive a short alias;
- native definitions always win unless the user grants one exact reversible
  override;
- generated files are private, bounded, digest/version verified, atomic,
  disposable, and reconstructable only from canonical data;
- malformed/tampered/partial state retains last-known-good behavior and never
  falls back to raw execution;
- no startup or typing path performs provider/network/authentication/secret work;
- secrets, history, terminal text, private provider state, and machine-only data
  remain absent from persistence, observability, export, and AI surfaces;
- responsive/keyboard/screen-reader/high-contrast behavior passes renderer-
  neutral and controlled native evidence;
- the frozen startup/search/compile/memory/storage/lifecycle ratchets pass; and
- disabling/uninstalling restores the exact native shell behavior while keeping
  user action data only when the user requests it.

## Primary references

- [PowerShell aliases](https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_aliases)
  and [PowerShell profiles](https://learn.microsoft.com/powershell/module/microsoft.powershell.core/about/about_profiles)
- [GNU Bash aliases](https://www.gnu.org/software/bash/manual/html_node/Aliases.html),
  [functions](https://www.gnu.org/software/bash/manual/html_node/Shell-Functions.html),
  and [startup files](https://www.gnu.org/software/bash/manual/html_node/Bash-Startup-Files.html)
- [Zsh alias grammar](https://zsh.sourceforge.io/Doc/Release/Shell-Grammar.html#Aliasing)
  and [functions](https://zsh.sourceforge.io/Doc/Release/Functions.html)
- [Fish abbreviations](https://fishshell.com/docs/current/interactive.html#abbreviations)
- [Microsoft DOSKEY macros](https://learn.microsoft.com/windows-server/administration/windows-commands/doskey)
  and [CMD startup/AutoRun behavior](https://learn.microsoft.com/windows-server/administration/windows-commands/cmd)
- [Kubernetes alias completion](https://kubernetes.io/docs/reference/kubectl/quick-reference/#kubectl-autocomplete)
- [Git aliases](https://git-scm.com/book/en/v2/Git-Basics-Git-Aliases.html)
- [`just` command runner](https://github.com/casey/just),
  [Task](https://taskfile.dev/), and
  [`mise run` tasks](https://mise.jdx.dev/tasks/running-tasks.html)
- [`kubectl-aliases`](https://github.com/ahmetb/kubectl-aliases) and
  [`kubectx`/`kubens`](https://github.com/ahmetb/kubectx) as ecosystem references
