# Terminal-first remote operations

## Status and purpose

This document is the canonical product specification for replacing the useful
remote-access, inventory, automation, collaboration, and governance workflows
found in GUI-oriented products such as Termius with a keyboard-first Automexia
experience.

It is a **planned product contract**, not a claim that these commands ship in
Automexia v0.4. Current implementation status remains authoritative in the
[feature catalog](FEATURES.md) and
[phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md). Delivery reuses
the existing D3-D7 SSH/multi-cloud phases and CP2-CP6 command-productivity
phases; it does not create a second process, credential, or extension model.

The outcome is a terminal that can manage many hosts, clouds, clusters, and
environments at Vim speed. Common work takes one command, one fuzzy selection,
or one leader-key sequence. A mouse remains optional.

## Product decision

Automexia exposes every supported operation through three consistent surfaces:

1. The canonical `automexia` CLI for discoverability, scripting, and recovery.
2. A renderer-owned leader-key command mode for low-latency interactive use.
3. Pane- or window-owned keyboard overlays only when browsing, comparison,
   preview, or explicit security confirmation is clearer than line output.

Examples use `ax` as an optional shorthand for `automexia`. Automexia must not
install or project `ax` when an executable, alias, function, or abbreviation
with that name already exists. Enabling the shorthand is explicit, reversible,
and follows the CP3 collision and native-precedence rules.

`Ctrl+Shift+P` remains the universal default for the existing command palette.
A user may additionally configure a command-mode `<leader>` after Automexia
checks application, OS, shell, editor, IME, and user-binding collisions. There
is no unconditional cross-shell leader default: for example, Fish owns
`Ctrl+Space` as its literal-abbreviation escape. A configured leader enters
Automexia command mode and never injects text into the PTY. User configuration
wins and the product must offer an accessible remapping path. Bare
`Ctrl+C`, `Ctrl+D`, `Ctrl+R`, `Tab`, shell history, editing, completion, quoting,
and cursor movement remain owned by the active shell except while an explicit
Automexia overlay or terminal selection owns focus.

The technology boundary is equally explicit. Core owns the typed registry,
interaction and review state, generic capabilities, exact-launch policy,
Capsules, pane/workspace composition, UI semantics, redaction, and resource
limits. First-party extensions own safe SSH/provider/file/collaboration domain
adapters. OpenSSH, provider CLIs, agents/vaults, Git, Mosh, Upterm, policy
services, and model endpoints retain their mature authority and are reached
only through the core runner. The complete decision matrix is
[Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md).

## Non-goals

- Recreate Termius screens, account model, pricing, or proprietary protocols.
- Make every operation graphical or hide the exact command that will run.
- Parse terminal-grid cells to infer an editable command or trusted target.
- Store private keys, passphrases, cloud tokens, or recovered secrets in normal
  Automexia configuration, logs, snapshots, crash bundles, or synchronization.
- Replace OpenSSH, official cloud CLIs, native shell editors, remote IAM/RBAC,
  or organization-managed identity systems.
- Execute remote output, autocomplete text, imported inventory, or AI output.
- Make AI, direct provider APIs, shared PTYs, or third-party extensions ambient
  authorities.
- Claim mobile support before a separate platform, lifecycle, and accessibility
  program exists.

## Shared interaction grammar

### Canonical commands

The stable public form is:

```text
automexia <domain> <verb> [target] [typed options]
```

The planned domains are:

```text
automexia connect      connections, protocols, routes, and destinations
automexia inventory    hosts, groups, tags, queries, and imports
automexia identity     agents, keys, certificates, and public auth status
automexia known-host   inspect and explicitly maintain SSH host trust
automexia route        jump chains, proxies, validation, and explanation
automexia run          Quick Actions, aliases, recipes, and hooks
automexia workspace    panes, tabs, restoration, and broadcast
automexia files        remote files, transfers, synchronization, and preview
automexia logs         history, bookmarks, handover, and audit views
automexia tunnel       local, remote, dynamic, and persistent tunnels
automexia context      cloud, cluster, namespace, and infrastructure context
automexia share        explicit collaboration and control handoff
automexia vault        non-secret personal/team inventory change review
automexia serial       explicitly selected local serial-device sessions
automexia backup       versioned export, verification, restore, and recovery
automexia assist       optional explain, suggest, and diagnose workflows
automexia admin        local/team policy, membership, SSO, and audit operations
```

Every applicable command must support predictable automation options:

```text
--help             human-readable usage and examples
--json             versioned machine-readable result
--quiet            suppress non-result presentation
--dry-run          validate and show the complete plan without mutation
--non-interactive  fail rather than opening a prompt or overlay
--timeout <value>  bounded operation deadline
```

Unknown options, duplicate security-sensitive options, unsupported schema
versions, and values outside resource limits fail before capability approval
or process launch. Machine output is never mixed with decorative terminal UI.

### Leader-key map

The default command-mode vocabulary is mnemonic and scope-aware:

| Sequence | Operation |
|---|---|
| `<leader> h` | Search hosts and inventory |
| `<leader> c` | Connect or reconnect |
| `<leader> w` | Search/open workspaces |
| `<leader> s` | Search Quick Actions and snippets |
| `<leader> a` | Search user aliases |
| `<leader> f` | Open the file navigator for the active route |
| `<leader> t` | Inspect and manage tunnels |
| `<leader> i` | Inspect or choose an identity reference |
| `<leader> l` | Search session logs and bookmarks |
| `<leader> p` | Inspect or switch public environment context |
| `<leader> b` | Enter reviewed broadcast-target selection |
| `<leader> ?` | Show contextual keys and available actions |
| `<leader> :` | Open the complete action palette |

The typed action registry is the single source for the CLI, palette, shortcuts,
help, accessibility actions, and future website reference. Labels and shortcuts
must not be independently hand-maintained.

### Fuzzy picker contract

Hosts, actions, identities, contexts, workspaces, logs, and files use the same
navigation rules:

| Key | Behavior |
|---|---|
| Arrows or `j`/`k` | Move one item without changing the shell buffer |
| `/` | Focus bounded search |
| `Enter` | Select the default safe action |
| `Space` | Toggle an item in a bounded multi-selection |
| `Tab` | Show public details/preview without executing |
| `Ctrl+Enter` | Open in a new split when supported |
| `Alt+Enter` | Open in a pane-local tab when supported |
| `?` | Show accessible contextual help |
| `Esc` | Cancel and restore exact previous focus |

Search is incremental, deterministic, Unicode-aware, cancellable, and bounded.
It uses immutable cached public records and never starts a provider, network,
authentication, or secret operation on a keystroke. Virtualization prevents a
large inventory from allocating one visual row per record.

### Risk and confirmation language

Every operation has one of the existing typed risk classes. Presentation uses
text, icon, color, and accessibility state together:

- `read-only`: inspect without mutation;
- `local mutation`: change local Automexia state;
- `remote mutation`: change a remote non-production target;
- `privileged`: request elevated local or remote authority;
- `production`: affect a production-scoped target;
- `destructive`: delete, overwrite, rotate, revoke, or irreversibly change.

The default Quick Action behavior is **review and insert without Enter**.
Execution requires the reviewed exact-argv broker and exact capability. A
production or destructive multi-target action requires target review and a
non-trivial confirmation; an extension cannot downgrade its risk label.

## Capability replacement matrix

This matrix maps the complete target surface to the existing Automexia roadmap.
`Foundation` means usable lower-level behavior exists but the named public
workflow does not yet ship.

| GUI-oriented capability | Terminal-first Automexia replacement | Delivery owner | Current status |
|---|---|---|---|
| SSH connection | `automexia connect <alias>` plus host picker | D5 | D4 inventory foundation only |
| Local terminal | Existing native session/tab/pane actions | v0.4 core | Shipped |
| Mosh | Typed `connect --transport mosh` adapter | D7 review | Not implemented |
| Telnet | Explicit insecure `connect --transport telnet` adapter | D7 review | Not implemented |
| Serial | `automexia serial open` and exact device parameters | D7 review | Not implemented |
| Recent/favorite hosts | `inventory list --recent`, pin, fuzzy picker | D5.0-D5.1 | Not implemented |
| Hosts and groups | Declarative records, paths, tags, saved queries | D4-D5 | D4 bounded aliases only |
| Inherited group settings | Layered configuration plus `inventory explain` | D5-D6 | Not implemented |
| Keychain | External identity references and `identity doctor` | D5 | Not implemented |
| Password storage | OS secret service only when unavoidable | D5 protected slice | Not implemented |
| SSH certificates/FIDO2 | System OpenSSH and agent/provider status | D5 | Not implemented |
| SSH ID | Interoperable device/certificate adapters, not proprietary custody | D6-D7 | Not implemented |
| Post-quantum SSH | System OpenSSH negotiation and truthful public status | D5 native evidence | Not implemented as UI |
| Known Hosts | `known-host` inspect/verify workflow; OpenSSH remains authority | D5 | Not implemented |
| Jump hosts/host chains | Reusable typed routes and route explanation | D5 | Not implemented |
| HTTP/SOCKS proxy | Typed route step with visible provenance | D5/D7 by transport | Not implemented |
| Port forwarding | `tunnel local|remote|socks`, status, owner, stop | D5 | Not implemented |
| Agent forwarding | Per-connection explicit grant; off by default | D5 | Not implemented |
| Snippets | Typed persistent Quick Actions | CP2 | Model/store foundation only |
| Shared snippets | Reviewed signed/team action catalogs | CP6/D7 | Not implemented |
| Startup snippets | Typed connection lifecycle hooks | D5A-D5E | Specification only |
| Multi-host execution | Bounded target query and result isolation | CP4/D6 | Not implemented |
| Autocomplete | Native completion first; optional editor bridge later | CP1/CP5 | CP1 shipped, CP5 planned |
| DevOps aliases | Optional collision-checked projections of actions | CP3 | Not implemented |
| Workspaces | Declarative session/layout templates and restore | D5 plus core UI | UI primitives shipped; persistence planned |
| Focus/split modes | Existing panes/tabs plus workspace focus commands | v0.4/D5 | Primitives shipped |
| Broadcast input | Explicit reviewed target set and visible armed state | D5/CP4 | Not implemented |
| SFTP | `files` TUI plus scriptable transfer commands | Post-D5/D7 | Not implemented |
| Session logs | Local bounded logs, search, bookmarks, comments | D7 | Not implemented |
| Terminal multiplayer | Expiring read-only share and explicit control handoff | D7 | Not implemented |
| Personal/team vaults | Versioned non-secret inventory plus external secret custody | D6-D7 | Not implemented |
| Roles/permissions | Inspectable policy, grants, revocation, explanations | D5-D7 | Capability foundation partial |
| Command Palette | Generated action registry and fuzzy command mode | Core/CP2/D5 | Generic palette shipped; operations planned |
| Cloud integrations | Official CLI/config adapters and Environment Capsules | D6.1-D6.5 | Not implemented |
| API Bridge | Local typed import/reconcile interface, diff before apply | D6-D7 | Not implemented |
| Ansible integration | Static inventory import/export; reviewed explicit refresh | D6 | Not implemented |
| AI command generation | Explain/suggest/insert; never ambient execute | CP6/D7 | Not implemented |
| Enterprise SSO/admin | External identity provider plus `admin` policy/audit CLI | D7 | Not implemented |
| Import/export/backup | Transactional versioned commands with dry run | D5-D7/CP3 | Partial subsystem foundations |

## Connections and inventory

### Connect

Users may address a host by exact alias, path, tag query, recent entry, or URI:

```text
automexia connect production/api-01
automexia connect --tag production --pick
automexia connect ssh://deploy@example.invalid:2222
automexia connect production/api-01 --destination split-right
```

The Connection Review shows the resolved public intent before authority:

```text
Target       production/api-01 (api.example.invalid:22)
Identity     work-fido2 (external OpenSSH agent)
Environment  production / AWS / eu-west-3
Route        local -> corp-bastion -> api-01
Host key     known, last verified 2026-08-10
Capabilities session.launch once; filesystem/network owned by OpenSSH
```

One action can choose a new pane, pane-local tab, workspace tab, or OS window.
Every destination creates an independent PTY, route, process, capsule, cache,
and teardown lifecycle. A failed clone/connection leaves layout unchanged.

### Inventory records, paths, tags, and queries

The canonical non-secret record is human-readable and versioned. For example:

```toml
[hosts.prod-api]
address = "api.example.invalid"
user = "deploy"
port = 22
identity = "work-fido2"
route = "production-bastion"
tags = ["production", "aws", "eu-west-3"]
startup_profile = "production-observer"
```

Common operations are:

```text
automexia inventory list
automexia inventory show prod-api
automexia inventory add --dry-run ...
automexia inventory edit prod-api
automexia inventory clone prod-api prod-api-02
automexia inventory remove prod-api
automexia inventory test prod-api
automexia inventory doctor prod-api
```

Group paths model organizations, environments, and providers without becoming
authorization boundaries:

```text
production/aws/eu-west-3/api-01
staging/azure/westeurope/web-01
```

Saved queries provide smart groups:

```toml
[queries.production_kubernetes]
expression = "tag:production AND capability:kubernetes"
```

### Inheritance and explanation

Configuration layers are explicit and deterministic:

```text
global -> organization -> provider -> environment -> group -> host -> one-time override
```

`automexia inventory explain prod-api` reports the winning value and source for
every field. Secrets never participate in printable explanation output.

## Identity, certificates, and trust

### External custody first

`automexia identity` discovers and references OpenSSH agents, Windows OpenSSH
Agent, macOS Keychain, Linux Secret Service, hardware FIDO2 tokens, encrypted
OpenSSH files, 1Password/KeePassXC agents, Teleport, Smallstep, Vault/OpenBao,
and organization-specific certificate authorities. It stores opaque references
and bounded public status, not private key material or tokens.

```text
automexia identity list
automexia identity discover
automexia identity add-agent work
automexia identity add-fido2 yubikey-main
automexia identity test yubikey-main
automexia identity assign prod-api yubikey-main
automexia identity doctor
```

Password capture, when policy permits it, uses protected input and the native OS
secret store. A password must never enter argv, environment, shell history,
logs, telemetry, snapshots, configuration, QA bundles, clipboard history, or
AI input.

### Certificates and devices

Certificate operations are provider-neutral:

```text
automexia identity certificate request --issuer corp-ca --identity operator
automexia identity certificate status
automexia identity certificate renew
automexia identity device list
automexia identity device revoke old-laptop
```

Automexia integrates established OpenSSH certificate, FIDO2, Smallstep,
Teleport, and Vault/OpenBao flows before considering a proprietary identity
protocol. Post-quantum algorithm negotiation remains system OpenSSH behavior;
Automexia may report the negotiated public algorithm only when the native
client exposes it reliably.

### Known-host workflow

OpenSSH stays the verification authority. Automexia adds accessible explanation
and explicit operations without silently modifying trust:

```text
automexia known-host list
automexia known-host show api.example.invalid
automexia known-host verify api.example.invalid
automexia known-host remove api.example.invalid
```

First-use review includes the exact target, algorithm, full fingerprint,
provenance, DNS SSHFP result when explicitly checked, and accept-once/save/
reject choices. A changed fingerprint stops the connection and never offers a
one-key silent replacement.

## Routes, proxies, and tunnels

Reusable routes are typed ordered steps, not concatenated command strings:

```toml
[routes.production]
steps = [
  { host = "corp-bastion" },
  { host = "production-jump" },
]
```

```text
automexia route explain production
automexia route test production
automexia connect prod-db --route production
```

The explanation shows every hop, public identity intent, timing, freshness, and
the exact failed step. Preview/indexing never evaluates `Match exec`,
`ProxyCommand`, `LocalCommand`, or configuration commands.

Tunnel commands are typed:

```text
automexia tunnel local 5432 prod-db:5432 --through production
automexia tunnel remote 8080 localhost:3000 --host prod-api
automexia tunnel socks 1080 --host corp-bastion
automexia tunnel list
automexia tunnel stop database
```

Local/dynamic listeners bind loopback by default. Public binding, background
persistence, and agent forwarding require separate exact grants. The footer
shows active tunnel count/health without becoming an action bar. Session-owned
tunnels terminate with their process tree; persistent tunnels have bounded
restart/backoff and an always-visible owner/lifetime.

## Quick Actions, aliases, and connection automation

### Typed actions instead of opaque snippets

The CP2 Quick Action is the canonical reusable-command format:

```toml
[actions.health-check]
description = "Inspect system and application health"
command = "uptime && df -h && systemctl --failed"
risk = "read-only"
shells = ["bash", "zsh"]
```

Parameterized actions declare placeholders, types, validation, shell, target,
risk, and insertion/execution mode. Before execution Automexia displays the
fully resolved command, exact targets, environment risk, required capability,
and freshness. Raw snippets remain shell-scoped and insert-only.

```text
automexia run search health
automexia run preview health-check --target staging/api-01
automexia run health-check --target staging/api-01
```

### Aliases

Aliases are optional generated projections, never the source of truth:

```text
automexia run alias add kpods kubernetes-pods
automexia run alias list
automexia run alias conflicts
automexia run alias disable kpods
automexia run alias export
```

Native definitions win. No short alias is enabled by default. Projection must
be reversible, syntax-checked for PowerShell/Bash/Zsh/Fish/CMD, paired with
completion where supported, and omitted rather than overwritten on collision.

### Lifecycle hooks and recipes

Connection automation uses the stage/risk/retry model in
[SSH connections and automation](SSH-CONNECTION-AUTOMATION.md):

```toml
[profiles.production-observer]

[[profiles.production-observer.hooks]]
phase = "after-authentication"
action = "detect-environment"

[[profiles.production-observer.hooks]]
phase = "after-shell-ready"
action = "open-tmux-session"
required = false
```

Supported stages are `before-connect`, `after-transport`,
`after-authentication`, `after-shell-ready`, `before-disconnect`,
`after-disconnect`, and `on-reconnect`. Hooks have explicit order, deadline,
retry/backoff, cancellation, failure policy, redacted audit, dry run, and
capability review. `--no-hooks` is always available as a safe recovery path.

### Multi-target execution

Targets come from a reviewed bounded query:

```text
automexia run health-check --tag staging --parallel 4
```

The preview separates reachable, expired, denied, and unverified targets.
Concurrency is capped; each host owns independent output, deadline,
cancellation, result, and cleanup. Production is excluded by default when a
query did not explicitly request it.

## Workspaces and broadcast

A workspace is a versioned declarative layout and connection-intent template:

```toml
[workspaces.payments-incident]
layout = "incident-quad"

[[workspaces.payments-incident.sessions]]
host = "payments-api"
action = "follow-service-log"

[[workspaces.payments-incident.sessions]]
host = "payments-db"
action = "database-activity"
```

```text
automexia workspace open payments-incident
automexia workspace save current incident-2026-08
automexia workspace inspect payments-incident
automexia workspace close
```

Restoration preserves public intent: host/transport, destination type, layout,
current directory, pane-local tabs, tunnels, context, and approved safe startup
actions. It never resumes an interrupted destructive command automatically.

Broadcast begins with target selection and an unmistakable armed state. It
shows the exact command and target count before send, isolates results, and
requires stronger confirmation when production is included. Escape immediately
disarms it. No visual state may make broadcast indistinguishable from normal
single-pane input.

## Remote files and transfers

`automexia files <host>` opens a pane-owned, keyboard-first dual-location
navigator only after a separately reviewed file capability exists. The local
and remote locations remain clearly labeled. The planned keys are movement,
search, selection, upload, download, rename, delete, preview, hidden-file
toggle, and cancel; all have generated accessible labels.

Non-interactive commands remain available:

```text
automexia files upload ./build.zip prod-api:/tmp/ --dry-run
automexia files download prod-api:/var/log/app.log .
automexia files sync ./dist prod-api:/var/www --dry-run
```

The file milestone requires no-follow/path-containment policy, permission and
symlink handling, overwrite review, partial-file naming, atomic finalize,
resume integrity, byte/file/depth/concurrency ceilings, cancellation, disk-
space preflight, checksum policy, cleanup, hostile filename handling, and
transfer-rate/resource benchmarks. Initially, system `sftp` may provide a
normal PTY recovery path; a structured navigator must not parse screen cells or
pretend that process output is a trusted file API.

## Session memory and logs

Planned commands are:

```text
automexia logs recent
automexia logs host prod-api
automexia logs search "connection refused"
automexia logs bookmark --name database-failover
automexia logs export incident-2026-08
automexia logs pause
```

Logs are local, bounded, private, encrypted where platform policy permits, and
disabled or limited by policy. They store public session identity, time,
working-directory/context transitions, exit outcomes, redaction events,
bookmarks, and comments. Command text/output is captured only through an
explicit shell/session contract; secure-input regions, secrets, private paths,
tokens, credential prompts, clipboard data, and excluded commands are never
persisted. Retention, maximum bytes, rotation, deletion, export, and recovery
are visible and testable.

## Collaboration, team state, and policy

### Shared sessions

`automexia share session` creates an expiring read-only invitation by default.
Control requires a visible request and time-bounded grant. The pane always
shows participant count, owner, recording state, and who controls input.
Revocation is immediate. Shared PTYs remain a D7 capability requiring encrypted
transport, peer identity, replay/order controls, redaction, rate and queue
limits, disconnect recovery, malicious-peer tests, and independent external
security review.

### Team inventory without secret custody

Team state uses versioned non-secret objects such as hosts, routes, actions,
workspaces, policies, public identity references, and known-host metadata:

```text
automexia vault status
automexia vault diff
automexia vault propose
automexia vault review
automexia vault apply
```

This name is a user concept, not authorization for Automexia to hold private
keys or cloud tokens. Organization-managed credential systems remain the
custodian. Changes are diffable, attributable, transactional, conflict-aware,
and policy-reviewed.

### Permissions and administration

```text
automexia admin access explain operator production
automexia admin access grant alice operator --scope staging
automexia admin access revoke bob --scope production
automexia admin policy test
automexia admin audit export
automexia admin sso test
```

Denials explain the rule, scope, safe alternatives, and remediation without
revealing secrets. Local Automexia policy is defense in depth; remote IAM,
RBAC, SSH CA, host, network, and identity-provider policy remain authoritative.

## Cloud, Kubernetes, and infrastructure context

One provider-neutral command displays immutable per-pane public context:

```text
automexia context show
automexia context switch
```

Provider operations delegate authentication to official tools:

```text
automexia context aws login production
automexia context azure login platform
automexia context gcp login engineering
automexia context kubernetes use prod-eu --namespace payments
automexia context terraform use production --directory ./infra
```

This reuses D6 Environment Capsules. AWS IAM Identity Center/STS, Microsoft
Entra, Google federation, Kubernetes exec plugins, OpenShift login, Teleport,
and OpenBao retain their existing security contracts. A switch in one pane
never mutates another pane or an unrelated global CLI context.

### Inventory synchronization

Automexia replaces a proprietary API bridge with typed import/reconcile
adapters:

```text
automexia inventory import openssh ~/.ssh/config
automexia inventory import ansible inventory.yml --dry-run
terraform output -json | automexia inventory import terraform-json --dry-run
automexia inventory sync aws --explicit-refresh
automexia inventory reconcile --dry-run
```

Local config and static files come first. Official CLIs are invoked only by an
explicit refresh with exact argv, null stdin where possible, stable executable
identity, deadline, output cap, process-tree cleanup, redaction, and immutable
last-known-good results. Direct SDK/API inventory remains lazy, optional,
separately permissioned, and isolated from renderer/PTY code.

## Completion and suggestion behavior

PSReadLine, Readline, ZLE, Fish, and CMD remain editor authorities. CP1 supplies
native completion for Automexia domains, host aliases, tags, workspaces,
actions, tunnels, contexts, and public identity references. No provider runs
at startup or per keystroke.

The optional CP5 popup may show value, type, source, freshness, risk, and short
description only after the separate authenticated editor bridge provides exact
buffer, cursor, replacement span, and generation. It remains inside its pane,
avoids the cursor/IME/footer/tabs/modals, cancels obsolete generations, and
falls back completely to the native shell experience.

## AI assistance

AI is a late optional action provider, not a terminal owner:

```text
automexia assist explain --command "kubectl get pods -A"
automexia assist suggest "show pods using the most memory"
automexia assist diagnose --from-selection
```

The default result is explanation or insertion, never execution. The review
shows the full suggested command, data provenance, model/locality, target,
environment, risk, and requested capability. Remote terminal output is
untrusted data and cannot supply instructions. No AI receives ambient history,
credentials, agent access, cloud tokens, filesystem, process, network, capsule,
or production authority. Remote AI is opt-in and receives bounded explicitly
selected/redacted input only.

## Import, export, backup, and recovery

```text
automexia inventory import openssh ~/.ssh/config --dry-run
automexia inventory export openssh
automexia backup create
automexia backup verify
automexia backup restore --dry-run
```

Imports are bounded, non-executing, schema-versioned, idempotent, and show a
diff. Apply is transactional and conflict-aware. Sensitive exports require an
explicit encrypted format and must never imply that externally owned
credentials can be recovered. Backup verification checks schema, checksums,
permissions, identity references, and required external providers without
contacting them implicitly.

## Renderer and responsive UI contract

Terminal-first does not mean forcing comparisons and security reviews into an
unreadable byte stream. Renderer-owned overlays are appropriate for fuzzy
selection, route graphs, host-key review, multi-target confirmation, diffs,
files, images, logs, layouts, permissions, and command risk.

Every overlay must:

- remain above all pane chrome with a real modal scrim and modal input routing;
- never resize or overwrite the PTY grid;
- preserve/restores exact focus, selection, prompt, IME, and scroll position;
- remain inside the intended pane unless it is explicitly window-modal;
- support keyboard-only use, focus visibility, screen readers, 200% text, high
  contrast, reduced motion, Unicode, and right-to-left isolation;
- virtualize large lists and remain usable from the minimum viewport to 8K;
- expose renderer-independent semantics and deterministic interaction goldens;
- close with `Esc` without mutation unless a committed operation already owns
  an explicit cancellation contract.

## Security and privacy invariants

1. Shell text, terminal output, imported labels, completion candidates, and AI
   output are untrusted display data until a reviewed typed action is selected.
2. Process launch uses stable executable identity and exact argv; no `sh -c`,
   `cmd /c`, PowerShell evaluation, or string concatenation is introduced.
3. Every filesystem, process, network, clipboard, secret, log, sharing, or
   provider action requires the narrow capability owned by its extension and
   exact session/route/capsule.
4. Credentials stay in native agents, hardware, OS stores, official caches, or
   organization-managed vaults. Automexia retains opaque references only.
5. Startup, rendering, resizing, prompt, and keystroke paths perform no network,
   provider, authentication, inventory refresh, secret read, or disk scan.
6. Persistent state is bounded, private, no-follow, versioned, atomic,
   last-known-good, rollback-aware, and removable.
7. Logs and diagnostics are allowlist-structured and redact secret-adjacent
   identifiers before storage. Public QA evidence never contains live hosts,
   usernames, paths, commands, tokens, or terminal contents.
8. Imported and remote data cannot grant itself authority. Capability changes,
   trust changes, provider activation, and team policy require explicit review.
9. Cancellation terminates descendants, leases, tunnels, transfers, refreshes,
   and stale UI generations without affecting another session.
10. Local policy supplements rather than claims to replace server-side access
    control and audit.

## Performance and resource invariants

- Input, parsing, rendering, resizing, and active shell completion never wait
  on an inventory, provider, file, log, collaboration, or AI worker.
- All queues, messages, strings, records, selections, caches, files, recursion,
  concurrency, output, retries, and retention have reviewed hard ceilings.
- Fuzzy search operates on immutable snapshots and cancels obsolete queries.
- Provider and inventory data uses stale-while-revalidate with truthful
  freshness; no blanking a usable last-known-good snapshot during refresh.
- Repeated open/close/connect/cancel cycles return process, handle, descriptor,
  thread, task, channel, GPU, cache, temporary-file, and disk usage to baseline.
- Benchmark gates cover cold/warm command mode, 1/1,000/10,000 inventory search,
  action insertion, connection review, process start-to-prompt, tunnels,
  1/10/50 sessions, transfer throughput, log search, collaboration backpressure,
  cancellation, and idle resource use.
- The 30-day baseline precedes hard regression thresholds. Thereafter, the
  established latency and memory waiver policy applies.

## Verification matrix

Every delivered slice needs evidence proportional to its authority:

| Area | Required evidence |
|---|---|
| Model | Unit, boundary, hostile-input, schema round-trip, unknown-version, redaction, and deterministic-order tests |
| State machines | Property tests for selection, review, grants, retry, reconnect, cancellation, lifecycle, and rollback |
| Concurrency | Loom/pure model checks where suitable; saturation, stale generation, shutdown, and cross-session isolation tests |
| Parsing | Fuzz OpenSSH/import/action/provider/protocol/log frames with bounded memory and corpus regression |
| Native shells | PowerShell 5.1/7, CMD, Bash, Zsh, Fish, WSL quoting, insertion, aliases, completion, and uninstall |
| Native OS | Windows, Linux, macOS OpenSSH/process/permissions/agents/PTY tests; controlled hardware where required |
| UX | Renderer-neutral semantics, keyboard/focus, tiny-to-8K, 100-300% scale, modal stacking, contrast, and screen-reader evidence |
| Security | Capability mutation tests, exact argv, path containment, secret canaries, peer identity, permissions, supply chain, and threat review |
| Performance | Criterion plus controlled end-to-end latency, throughput, memory, CPU, disk, handles/descriptors, and long-run baseline |
| Resilience | Offline/expired/denied/cancelled/corrupt/partial/full-disk/slow-provider/process-crash/restart and last-known-good recovery |
| Cleanup | Repeated lifecycle tests, process-tree termination, tunnel/transfer cleanup, temp-file removal, bounded logs, and uninstall |
| Documentation | User guide, exact CLI/config/shortcut reference, ADR, troubleshooting, migration, recovery, and feature-ledger evidence |

PR tests use deterministic fake providers and servers. Nightly tests add fuzz,
sanitizers, Miri, longer resource cycles, packages, and native matrices. Release
evidence adds real system clients/servers, controlled agents/hardware, GPU,
screen readers, signing, packages, network failure, and human security/UX review.
No mocked, compile-only, Windows-only, or local-only result may claim the full
cross-platform release gate.

## Delivery sequence

### v0.5.0: command-first production SSH

Reuse D5 and CP2/CP3 to deliver:

1. The generated operation registry and canonical `automexia` command grammar.
2. CP2.2 action search/review/insert and CP3 collision-safe optional aliases.
3. Read-only host/group/tag/recent/favorite inventory over D4.
4. Connection Review, quick connect, destination selection, independent PTYs,
   cancellation, reconnect, route explanation, jump hosts, and typed tunnels.
5. External identity references, agent/certificate public status, strict
   host-key explanation, and platform setup/doctor journeys.
6. Declarative workspace intent and safe restoration using existing pane/tab
   primitives; reviewed broadcast only after its visible armed-state tests.
7. Native completion for all shipped operation domains.

Exit is the existing D5.0-D5.2/CP2-CP3 gate. SFTP, shared sessions, cloud API
inventory, proprietary identity, AI execution, and arbitrary remote scripts are
not v0.5.0 blockers and remain disabled.

### v0.5.1: multi-cloud and context-aware operations

Reuse D6 and CP4 to deliver:

1. Immutable per-pane Environment Capsules and `context show/switch`.
2. AWS, Azure, Google Cloud, Kubernetes, OpenShift, Teleport, and OpenBao slices
   independently, each with exact official-CLI/authentication behavior.
3. Static imports for OpenSSH, Ansible, Kubernetes, and infrastructure outputs;
   explicit bounded provider refresh only where needed.
4. Capsule-aware actions and multi-target selection using cached public context.
5. Provider-native remote transports only through visible official commands.

Each provider passes its independent native, offline, expiry, MFA, cancellation,
redaction, isolation, accessibility, and resource gate before release.

### v0.6 and later: files, memory, collaboration, ecosystem, and AI

Deliver independent protected slices rather than one broad entitlement:

1. Structured SFTP/file transfer after its separate file-write threat model.
2. Bounded local session memory, bookmarks, comments, retention, and export.
3. Team inventory synchronization, review, policy, and external-vault adapters.
4. Read-only-first terminal sharing and explicit expiring control handoff.
5. Mosh, serial, and explicitly warned Telnet only after platform/security
   evaluation; system OpenSSH remains the SSH compatibility authority.
6. Public/signed extension packs only after sandbox, provenance, revocation,
   quota, compatibility, and migration gates.
7. CP5 optional completion UI and CP6/D7 AI only after their separate bridge,
   privacy, capability, performance, and rollback approvals.

## Acceptance criteria

The terminal-first product direction is complete only when:

- every shipped remote operation is accessible by canonical CLI, keyboard
  command mode, generated help, and accessibility action;
- the common connect/action/context/workspace workflows take no more than one
  mnemonic leader sequence or one predictable command;
- users can inspect the exact target, identity reference, environment, route,
  risk, command/argv, and capability before authority is exercised;
- the system remains fully usable with every DevOps/provider extension disabled;
- no secret custody, shell ownership, network, process, file, or AI authority is
  added implicitly for convenience;
- all overlays preserve PTY state and remain responsive, correctly stacked,
  keyboard-operable, screen-reader meaningful, and usable at extreme sizes;
- operations are bounded, cancellable, cross-session isolated, leak-tested,
  benchmarked, recoverable, and truthful when stale/offline/expired/denied;
- Windows, Linux, macOS, WSL, supported shells, real OpenSSH, and applicable
  provider/native flows pass their declared PR/nightly/release evidence;
- current behavior, configuration, commands, shortcuts, architecture choices,
  security recovery, and limitations are documented without presenting roadmap
  examples as shipped functionality.

## Research baseline

The capability comparison was reviewed on 2026-08-16 from Termius's current
product and pricing surfaces, security documentation, and first-party feature
announcements. These links establish product context only; Automexia does not
depend on Termius code, services, protocols, accounts, or availability:

- [Termius product overview](https://www.termius.com/)
- [Termius plans and feature matrix](https://termius.com/pricing)
- [Termius security overview](https://support.termius.com/hc/en-us/articles/4402480134169-Termius-Security-Overview)
- [Termius Vaults](https://termius.com/blog/meet-vaults)
- [Termius Workspaces](https://termius.com/blog/workspaces-focus-without-losing-context)
- [Termius session-log bookmarks](https://termius.com/blog/remember-what-was-done-with-bookmarks-for-session-logs)
- [Termius API Bridge](https://termius.com/blog/keep-connection-details-up-to-date-with-api-bridge)
- [Termius Ansible integration](https://termius.com/blog/termius-integration-with-ansible)
- [Termius AI Agent beta](https://termius.com/blog/ai-agent)

The mapping captures material workflows rather than copying commercial plan
boundaries or treating a launch announcement as proof that a feature remains
generally available. Automexia's release status is determined solely by its own
feature ledger and evidence gates.

## Related specifications

- [Roadmap](ROADMAP.md)
- [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md)
- [SSH, DevOps, and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md)
- [Connection Hub](CONNECTION-HUB.md)
- [SSH connections and automation](SSH-CONNECTION-AUTOMATION.md)
- [Command Productivity](COMMAND-PRODUCTIVITY.md)
- [DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md)
- [Exact-argument session launch broker](SESSION-LAUNCH-BROKER.md)
- [OpenSSH inventory](SSH-INVENTORY.md)
- [Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md)
- [ADR 0018](adr/0018-terminal-first-remote-operations.md)
- [ADR 0020](adr/0020-hybrid-build-wrap-adopt-boundary.md)
