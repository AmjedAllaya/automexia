# Connection Hub product, security, and delivery specification

Status: planned for D5 (production OpenSSH) and D6 (multi-cloud); no managed
connection UI or credential custody is shipped in v0.4.

This document is the implementation authority for Automexia's Connection Hub.
The [roadmap](ROADMAP.md) owns release order, the
[stabilization roadmap](STABILIZATION-ROADMAP.md#early-devops-and-ssh-delivery-track)
owns phase gates, and
[ADR 0012](adr/0012-first-party-ssh-and-session-launch-boundary.md) owns the
process, credential, and network boundary. This specification turns those
boundaries into a complete user journey and renderer-neutral UI contract.
Reusable connection profiles, connection-scoped context, automatic action
recipes, remote initialization, and their detailed delivery/test contract are
specified in [SSH connections and automation](SSH-CONNECTION-AUTOMATION.md).
The canonical command-first vocabulary and its relationship to actions,
workspaces, files, logs, and provider contexts are specified in
[Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md).

## Product outcome

The Connection Hub should let a developer, operator, or platform engineer find
the correct environment, understand the identity and route that will be used,
authenticate through the organization's existing tools, and open an isolated
terminal session without reconstructing commands or switching global state.

It solves five recurring multi-environment problems:

1. finding a host, account, subscription, project, cluster, or environment;
2. seeing whether identity is ready before a command fails;
3. avoiding accidental production or wrong-account connections;
4. keeping simultaneous panes pinned to independent contexts; and
5. explaining a failed connection without hiding the authoritative provider or
   OpenSSH diagnostic.

The Hub is an inventory and orchestration surface, **not a credential vault**.
System OpenSSH, OS or organization agents, hardware devices, certificate
authorities, and official provider CLIs retain credential custody. Automexia
stores public metadata, local preferences, immutable session intent, and opaque
references only.

## Non-negotiable product principles

- **One connection intent, one review, one independent PTY.** A launch never
  attaches a second view to an existing PTY.
- **Provider-native authentication.** Browser, device-code, MFA, certificate,
  agent, and token-cache behavior remains owned by the official tool.
- **No surprise work.** Opening the Hub, typing in search, changing a filter, or
  moving selection performs no network request, login, plugin execution, or
  credential-cache read.
- **Truth before convenience.** Stale, offline, denied, locked, missing, and
  expired states are distinct and never collapsed into a green `Ready` state.
- **Review changes, not rituals.** A previously reviewed non-production route
  can reconnect with one click when its identity, target, transport,
  fingerprint policy, capability set, and source revision are unchanged.
  Production, changed-host-key, new capability, non-loopback tunnel, and
  changed-route cases always return to review.
- **Session isolation.** A provider or Kubernetes selection creates an
  immutable Environment Capsule for the new session; it never mutates another
  pane's global CLI state.
- **Safe failure.** A failed login or connection leaves the current layout and
  existing sessions unchanged, returns focus predictably, and offers a
  provider-specific recovery action.
- **Terminal independence.** With every DevOps extension disabled, ordinary
  `ssh`, `aws`, `az`, `gcloud`, `kubectl`, and `oc` shell behavior is unchanged.

## Ownership and data model

### Crate and process placement

The implementation adapts to the existing structure:

| Concern | Owner | Forbidden dependencies/authority |
|---|---|---|
| Connection, filter, review, authentication-state, and capability-view models | `automexia-ui-model` and provider-neutral `automexia-devops` types | Renderer, PTY, GPU, OpenSSH parser, provider SDK, secret values |
| OpenSSH inventory and public metadata | private `automexia-devops-ssh` extension | Process launch, direct network, raw keys, `ssh -G`, executable config evaluation |
| Provider inventory and capsule templates | independently enabled D6 provider extensions | Renderer access, ambient environment, another provider's cache |
| Capability decision and exact launch | application-owned D3 broker | Shell command strings, wildcard executables, extension-owned PTYs |
| External process execution | one application-owned `ExternalToolRunner` used by every adapter | Provider-specific launchers, ambient environment, unbounded output, render/resize/startup/keystroke invocation |
| Modal composition, virtualized rows, focus, responsive layout | `apps/automexia-terminal` adapter over renderer-neutral models | Provider-specific business logic or credentials |
| Authentication and connection process | system OpenSSH or reviewed official CLI in a normal Automexia PTY | Hidden password capture, token parsing, silent fallback |

GPU drawing stays in the frontend adapter. The model must be fully testable
without a window system, provider account, PTY, or GPU.

### Core records

The Hub uses versioned, bounded records:

```text
ConnectionDefinition
  id, display_name, description, tags, favorite, last_used_at
  source_kind + source_reference + source_revision
  provider_kind + transport_kind
  public_destination
  environment_template_reference
  identity_reference
  jump_references[]
  tunnel_templates[]
  risk

ConnectionObservation
  connection_id + generation
  auth_state + observed_at + expires_at? + stale_after
  tool_state + transport_state
  public_identity_summary
  diagnostic_code + recovery_action?

ConnectionIntent
  connection_id + source_revision
  exact destination and selected transport
  ordered jump chain and typed tunnels
  identity reference and Environment Capsule template
  destination surface: pane | pane-tab | workspace-tab | window
  requested capabilities

ConnectionReview
  normalized intent
  policy decisions and warnings
  known-host/fingerprint observation
  changed fields since last approval
  redacted exact executable/argv preview

ConnectionReceipt
  operation_id + session_id + capsule_id
  approved intent digest + source revision
  process/route/tunnel ownership references
  started_at + terminal outcome
```

No record contains a private key, passphrase, cloud token, Kubernetes
`ExecCredential`, agent protocol message, browser cookie, full inherited
environment, terminal content, or arbitrary command string. Labels, paths,
counts, nesting, payloads, persistence, and operation lifetimes inherit the
fixed D3/D4 ceilings and must receive explicit lower UI limits before D5 ships.

### Connection groups

Grouping is a projection, not a second database. Built-in views are:

- **All connections**;
- **Favorites**;
- **Recent**;
- **Needs attention** (`Missing`, `Locked`, `Expired`, `MFA required`,
  `Offline`, `Denied`, `Unsupported`, or `Error`);
- **SSH**;
- **AWS**, **Azure**, and **Google Cloud**;
- **Kubernetes** and **OpenShift**;
- **Workspaces**, grouping saved cross-provider environment recipes; and
- user-defined tags.

The default result grouping is environment risk, then provider/account, then
display name. Production is never hidden inside an unlabeled group. Sorting is
stable and deterministic: favorite, exact search match, recently used,
display name, then stable ID. A user can switch to alphabetical, recent, or
provider grouping without changing the underlying connection.

## Exact Connection Hub layout

The Hub is a window-level application modal above tabs, panes, footers, image
previews, and terminal cells. It dims and makes the underlying terminal inert;
opening it does not resize the PTY grid or mutate terminal selection. Closing
it restores focus to the exact pane and control that opened it.

### Wide layout (at least 1,100 logical pixels)

```text
+--------------------------------------------------------------------------------+
| Connection Hub                                       + Add  Refresh Settings X |
| [ Search hosts, accounts, clusters, tags...                     ]  Ctrl/Cmd+F   |
+------------------+--------------------------------------+----------------------+
| All          142 |  [Environment: All] [State: All]     | payments-prod       |
| Favorites     12 |  [Provider: All] [Risk: All] [More]  | PRODUCTION           |
| Recent        18 +--------------------------------------+----------------------+
| Needs attention 3|  PINNED                              | Identity: AdminRole  |
|                  | > payments-prod  AWS/SSM    Ready    | AWS 123... / eu-west |
| SSH           46 |   build-bastion  SSH       Locked   | Target: i-0123...    |
| AWS           22 |                                      | Transport: SSM       |
| Azure         17 |  DEVELOPMENT                         | Route: local -> SSM  |
| Google Cloud  11 |   dev-cluster    Kubernetes Ready   | Host policy: n/a     |
| Kubernetes    29 |   lab-vm         SSH        Offline  | Capabilities: launch |
| OpenShift      6 |                                      |                      |
| Workspaces     8 |                                      | [Review & connect]   |
| Tags             |                                      | [Open terminal only] |
+------------------+--------------------------------------+----------------------+
| 142 cached | observed 14:32 | offline-safe                         Esc to close |
+--------------------------------------------------------------------------------+
```

- Left navigation is 224-280 logical pixels and can collapse to icons plus
  accessible labels.
- The center result region is flexible with a 420-pixel usable minimum and a
  virtualized list; row height grows for text scaling rather than clipping.
- The inspector is 320-440 pixels. It shows only public data and never becomes
  the sole location of a blocking warning.
- Search and filters remain visible while results scroll.
- Status uses icon, label, and accessible state; color is redundant.

### Medium layout (700-1,099 logical pixels)

Navigation becomes a drawer. Results occupy the window. Selecting a result
pushes a full-height review page with a visible Back action; it does not place a
narrow inspector over terminal content.

### Narrow layout (below 700 logical pixels or at 200%+ text scale)

The Hub becomes a single-column route:

```text
+--------------------------------------+
| < Connection Hub                  X  |
| [ Search...                         ] |
| [All] [Needs attention] [Provider v] |
+--------------------------------------+
| payments-prod                 READY  |
| AWS / Production / eu-west-1         |
| AdminRole via SSM                     |
|                                      |
| build-bastion                 LOCKED  |
| SSH / Production / jump: corp        |
+--------------------------------------+
```

No minimum font reduction is used to force the desktop layout. At impossible
sizes the title, search, selected result, state, Back/Close, and primary action
remain reachable through scrolling; lower-priority details collapse into
labeled disclosure sections.

### Search, filters, and refresh

- Search is local, fuzzy, diacritic-aware, Unicode-safe, cancellable, and
  bounded. It matches display name, concrete alias, public target hint, tag,
  account/subscription/project label, cluster/context, and region.
- It never matches a secret, token cache, private-key content, terminal output,
  or shell history.
- Filter chips cover environment/risk, provider, transport, auth state,
  region, tag, favorite, and freshness. Active filters have a text summary and
  one `Clear filters` action.
- Normal refresh reparses granted local public configuration and cached status.
  `Deep refresh` is an explicit reviewed action that may invoke approved local
  tools or provider inventory and shows its operations before execution.
- Refresh is generation-scoped. Older results cannot replace a newer query,
  filter, source revision, session, or extension generation.
- Last-known-good results remain visible with `Stale`/`Offline`; failure never
  replaces a useful list with an empty success state.

### Create, import, edit, and diagnose

`Add connection` is available beside Refresh and in the command palette. It
starts with a source choice rather than a blank credential form:

1. **Use an OpenSSH alias** (recommended): select a concrete D4-indexed alias
   and store only tags, favorite, risk, capsule template, and display metadata.
2. **Create an explicit SSH destination:** enter host, optional public user/port,
   reviewed jump references, and typed tunnels. The preview maps these fields to
   exact argv; it never accepts a free-form option string. Identity remains an
   agent/certificate/hardware/encrypted-file reference owned by OpenSSH.
3. **Import public metadata:** preview a bounded provider/config inventory,
   select records, show conflicts and skipped secret-bearing/unsupported fields,
   then atomically save only Automexia-owned metadata. Source files remain
   unchanged.
4. **Create an environment workspace:** group connection references and public
   capsule templates for a project without combining their credentials or
   granting batch connection authority.

Editing shows the same review diff used at launch. Changing target, transport,
identity reference, jump chain, tunnel, risk, provider, or source revision
invalidates the old approval. Delete removes Automexia metadata only and offers
no misleading `delete credentials` action.

`Diagnose` is explicit and tiered: static configuration explanation first,
local tool/agent status second, network/provider check only after review. It
shows executed operation IDs, redacted exact argv, source provenance, elapsed
step, typed result, and recovery guide. `Copy diagnostic summary` excludes
private paths and identifiers by default and never includes terminal output,
credentials, provider caches, or environment values.

## First-run discovery and setup

First run is progressive. It must produce a useful local-terminal experience
even when no remote tool or account exists.

### Discovery tiers

| Tier | Trigger | Allowed work | Explicitly forbidden |
|---|---|---|---|
| 0: passive | Hub first open | Check fixed executable locations/PATH result, granted config-file presence, and agent-socket/service presence; read D4 public cache | Process execution, network, login, token/cache reads, plugin execution, file mutation |
| 1: local scan | User selects `Scan local tools` and reviews scope | Bounded static parse of OpenSSH/provider config names and kubeconfig public context metadata; version probes with exact executables | Reading credentials, evaluating `Match exec`, running kubeconfig exec plugins, provider API calls |
| 2: status check | User selects `Check status` for an identity/provider | Bounded official status command or agent public-key listing with timeout, cancellation, redaction, and visible progress | Authentication, hidden refresh, token output retention, cross-provider mutation |
| 3: authenticate/refresh | User selects Login/Unlock/Renew | Visible official CLI/agent flow in a dedicated PTY or approved browser origin | Password/token capture by Automexia, background retries, command-string evaluation |
| 4: remote inventory | User selects provider deep refresh and approves capability | Lazy provider operation through the reviewed D6 adapter | Implicit network on open/search, renderer-thread work, broad SDK/network authority |

Static kubeconfig parsing treats the file as executable-capable untrusted input.
It may display `exec.command`, arguments, API version, and source, but it never
runs the plugin. Kubernetes 1.35+ credential-plugin allowlist policy is surfaced
when available; unfamiliar executables remain denied or require an exact
reviewed grant.

### First-run screen

```text
+------------------------------------------------------------------+
| Set up Connection Hub                                             |
| Automexia found public configuration only. No credentials copied. |
|                                                                   |
| OpenSSH       Installed  config: 18 aliases       [Review]        |
| SSH agent     Not checked                          [Check agent]   |
| AWS CLI       Installed  4 named profiles         [Review]        |
| Azure CLI     Installed  configuration present    [Review]        |
| Google Cloud  Missing                              [Install guide] |
| Kubernetes    Installed  6 contexts, 1 exec rule  [Review]        |
| OpenShift     Missing                              [Install guide] |
|                                                                   |
| [Scan local tools]                         [Continue without setup]|
+------------------------------------------------------------------+
```

Missing tools link to official installation documentation. Automexia never
downloads, upgrades, executes a package manager, edits shell startup files, or
changes an OS service without a separate explicit workflow and OS confirmation.

### Windows OpenSSH and agent journey

1. Detect the supported Windows OpenSSH client through the D3 canonical
   executable resolver and report its actual path/version.
2. Detect `%USERPROFILE%\.ssh\config` only through the D4 exact grant.
3. Inspect the Windows `ssh-agent` service state through a fixed platform API or
   approved read-only query; do not require elevation for detection.
4. If disabled, explain that Windows disables the service by default. Offer
   `Copy reviewed setup commands` and `Open elevated system setup`; never start
   or change the service silently.
5. Add a key only through visible `ssh-add` in a PTY so its passphrase prompt is
   owned by OpenSSH. Show only public fingerprint/comment status afterward.
6. Treat compatible organization or third-party agents through the standard
   OpenSSH agent interface. Do not add a vendor credential SDK merely to list
   keys.

### macOS agent/keychain journey

1. Detect the system/user-selected OpenSSH executable, `~/.ssh/config`, and
   `SSH_AUTH_SOCK` without reading keychain secrets.
2. Query public agent identities only after `Check agent`.
3. Prefer an existing system, organization, hardware, or user-selected agent.
   Do not create a competing Automexia agent.
4. If the installed OpenSSH supports a platform keychain option, show a
   version-aware reviewed snippet or launch the installed `ssh-add` visibly.
   Do not silently edit `~/.ssh/config`, launch agents, or change login items.
5. Keep Keychain access under macOS and the selected agent. Automexia stores
   only the opaque identity reference and public observation.

### Linux agent/keychain journey

1. Detect the canonical `ssh`, granted config, and `SSH_AUTH_SOCK`.
2. Query public identities only after `Check agent`.
3. When no agent exists, explain distribution/desktop choices and link to
   official package or session documentation. Never append to `.profile`,
   `.bashrc`, `.zshrc`, or a systemd user unit silently and never leave an
   orphan `ssh-agent` process.
4. Integrate with GNOME Keyring, KWallet, gpg-agent, 1Password, organization
   agents, or hardware through their standard OpenSSH-compatible socket or
   provider configuration; no desktop-specific secret database is copied.
5. Run `ssh-add` visibly for an explicit user action and retain only public
   status.

## Authentication state machine

`AuthState` is provider-neutral but preserves provider diagnostic codes. Every
observation includes generation, provenance, observation time, optional expiry,
and freshness. `Ready` without evidence is invalid.

| State | Meaning | Primary action | Permitted automatic transition |
|---|---|---|---|
| `Unknown` | No trustworthy observation exists | Check status | `Checking` only |
| `Checking` | A bounded user-authorized observation is running | Cancel | To any observed terminal state |
| `Ready` | External owner reports usable identity at the observation time | Review & connect | `Expired` when an observed expiry passes; `Unknown` when source revision changes |
| `Locked` | Key/agent/store exists but needs user presence or unlock | Unlock | None; user action starts `Authenticating` |
| `Missing` | Required tool, profile, identity, key reference, or context is absent | Set up / Choose another | `Unknown` after source change |
| `Expired` | Previously valid external session/certificate is no longer usable | Log in / Renew | None |
| `MfaRequired` | Provider requires interactive MFA or browser/device approval | Continue sign-in | None |
| `Authenticating` | Visible provider/agent interaction is active | Cancel | `Ready`, `Cancelled`, `Offline`, `Denied`, or `Error` |
| `Cancelled` | User cancelled the current operation | Retry | None |
| `Offline` | Required remote service is unreachable; cached metadata may remain | Retry when online | `Unknown` after network/source change, never `Ready` by timer |
| `Denied` | Provider, policy, capability, or local user decision refused access | View policy / Choose another | None; no retry loop |
| `Unsupported` | Installed version or auth/transport method is not supported | View requirements | `Unknown` after tool/source change |
| `Error` | Bounded operation failed for another truthful reason | Details / Retry | None |

State transitions are event-driven and operation-scoped. Authentication is
never retried in the background. A timeout cancels descendants and becomes
`Offline` or `Error` based on typed evidence, not error-string guessing. Closing
the Hub may leave an explicitly launched login PTY open only after the user
chooses that behavior; otherwise it cancels the operation tree.

### One-click login, retry, and reconnect

- **Login/Unlock/Renew** launches the exact official flow, foregrounds its PTY
  or approved system browser, observes only typed exit/status evidence, then
  performs a fresh status check. Device codes are shown by the provider tool;
  Automexia does not copy them unless the user uses normal selection/copy.
- **Retry** repeats only the failed step with a new operation/generation. It
  never replays a password, token, host-key acceptance, or destructive setup.
- **Reconnect** creates a new PTY/process/route/capsule after fresh policy and
  expiry checks. It never reuses a dead PTY and never silently switches an SSH
  failure to PowerShell or another transport.
- **Quick connect** is one click only for a non-production intent whose review
  digest is unchanged. Otherwise it opens Connection Review with changed fields
  emphasized.
- All actions are idempotent at the UI operation layer, disable while their
  generation is active, and offer cancellation without blocking input/render.

## Capability approval and revocation

The Hub displays application-owned capability decisions; an extension cannot
draw, word, or approve its own prompt.

An approval contains:

- verified extension ID, publisher, package digest/version, and trust status;
- structured operation and reason;
- exact target connection, environment, and risk;
- executable identity and a redacted ordered-argv view;
- working-directory reference and environment **names** to be set, never secret
  values;
- transport, network destination class, jump chain, tunnels, and lifetime;
- destination surface and owning session; and
- scope: once, this session/capsule, or a narrowly persisted exact operation.

Default is deny. Wildcard executables, wildcard destinations, arbitrary shell
text, inherited environment access, raw secret access, and unbounded lifetime
cannot be approved. A publisher, package digest, manifest, executable identity,
source revision, route, risk, or capability change invalidates the old grant.

`Connection Hub > Settings > Capabilities` lists active and stored grants by
extension, environment, operation, target, and last use. Revocation is immediate:
it cancels in-flight operations, stops owned background tunnels according to
their reviewed contract, prevents new launches, and records a redacted local
audit event. Revoking one provider must not disable another provider or kill
unrelated ordinary shell sessions.

## Connection Review

Every first launch and every material change uses a review page:

```text
+--------------------------------------------------------------------+
| < Back                    Review connection                         |
| payments-prod                         PRODUCTION / approval required |
+--------------------------------------------------------------------+
| Identity   AWS AdminRole via corp-sso     Ready, expires 16:45      |
| Context    account 123456789012 / eu-west-1 / production            |
| Target     i-0123456789abcdef0 (inventory revision 81)              |
| Transport  AWS Systems Manager                                    |
| Route      local -> aws CLI -> SSM managed instance                 |
| Jump chain none                                                     |
| Tunnels    local 127.0.0.1:5432 -> database.internal:5432           |
| Host trust managed by transport (no SSH host key)                   |
| Capability launch exact `aws` operation; network owned by child     |
| Destination new pane-local tab                                      |
|                                                                    |
| Changed since last review: role, local tunnel                       |
| [Cancel]                         [Connect to PRODUCTION]             |
+--------------------------------------------------------------------+
```

Required sections are:

1. **Identity:** public user/role/profile/certificate/agent label, external
   owner, authentication state, freshness, and expiry.
2. **Environment:** environment label, risk, account/subscription/project,
   region/zone, cluster/context/namespace, and capsule source.
3. **Target:** concrete alias and public target summary with source/revision.
4. **Transport:** system OpenSSH, SSM, Bastion, IAP/OS Login, Teleport, or
   another separately approved exact adapter.
5. **Jump chain:** every ordered hop, user-visible alias, transport, and risk.
6. **Tunnels:** direction, bind address, listen port, destination, ownership,
   and lifetime. Non-loopback listeners receive a blocking risk warning.
7. **Host trust:** first-use/known/changed status, full SHA-256 fingerprint and
   source when OpenSSH can provide it without weakening its authority. Changed
   host keys block; the Hub never edits `known_hosts`.
8. **Capabilities:** exact operation, executable, environment names, working
   directory, and persistence scope.
9. **Destination:** new pane, pane-local tab, workspace tab, or window.
10. **Changes:** semantic difference from the previously approved digest.

The exact argv disclosure is structured and redacted. It is never a copyable
shell command assembled through concatenation. Production uses a clearly named
primary action; organization policy may require reason, ticket, step-up MFA, or
additional confirmation. Host-key changes and public/non-loopback listeners
cannot be waived by a generic `Always allow` control.

## Provider setup journeys

Each journey begins with passive/static discovery, keeps authentication visible,
creates a new Environment Capsule, and never mutates an unrelated session.

### AWS

1. Detect the official AWS CLI and bounded named-profile/`sso-session` metadata
   from granted configuration. Do not read SSO token or credential cache files.
2. Present profile, SSO session, account/role when public configuration provides
   them, region, source, and freshness. Unknown account/role remains unknown.
3. `Check status` may run a reviewed bounded official identity command and may
   contact AWS; it is never invoked on open/search/keystroke.
4. For `Missing`, launch visible `aws configure sso` or guide the user to an
   organization-managed profile. Do not synthesize static access keys.
5. For `Expired`/`MfaRequired`, launch exact `aws sso login --profile <name>`.
   Prefer the AWS CLI's default PKCE browser flow and expose a deliberate device
   code option when required by the user's environment.
6. Review account, role, region, expiry, and transport. The capsule contains
   only `AWS_PROFILE`, reviewed region intent, and opaque provenance.
7. Connect through a normal shell/SSH session or prefer SSM Session Manager for
   eligible instances. EKS uses the supported exec credential flow and the
   exact provider/plugin review.
8. Logout is an explicit provider-wide action because `aws sso logout` can
   affect multiple profiles; show its scope before launch.

### Azure

1. Detect the official Azure CLI and public configuration presence; never parse
   refresh/access tokens.
2. Show tenant/subscription labels only from a bounded static source or a
   user-requested official status operation.
3. `Log in` launches visible `az login`. Preserve Windows Web Account Manager
   or provider browser behavior and offer `--use-device-code` only as a reviewed
   alternative. MFA remains owned by Microsoft Entra.
4. Require explicit tenant/subscription selection when ambiguous. Do not use a
   hidden global `az account set` that changes other panes.
5. Use an identity-scoped `AZURE_CONFIG_DIR` where isolation requires it and
   pass `--subscription` for extension-launched sensitive operations.
6. Review tenant, subscription, identity kind, region, expiry/freshness, and
   transport. Human sessions use interactive Entra/MFA; automation should use
   workload identity rather than a password in Automexia.
7. AKS uses a supported `kubelogin`/exec flow under the Kubernetes executable
   policy. Prefer Azure Bastion for eligible remote administration.

### Google Cloud

1. Detect the official Google Cloud CLI and bounded named-configuration
   metadata. Do not read token databases or service-account private keys.
2. Keep Google Cloud CLI authorization separate from Application Default
   Credentials. The Hub never silently runs `gcloud auth application-default
   login` when a user asked to log in to the CLI.
3. Setup uses `gcloud init` or a named `gcloud config configurations create`
   flow in a visible PTY. Login uses visible `gcloud auth login` for the selected
   configuration or the organization's Workforce/Workload Identity flow.
4. Require explicit account/configuration/project selection. The capsule pins
   `CLOUDSDK_ACTIVE_CONFIG_NAME`, reviewed config-root policy, project, and
   region/zone intent without copying credentials.
5. GKE uses the supported credential plugin after exact executable review.
   Prefer IAP and OS Login for eligible VM access.
6. Service-account key import is not a Hub workflow; recommend workload identity
   federation and explain organization-managed alternatives.

### Kubernetes

1. Parse only granted, bounded kubeconfig sources. Respect platform-specific
   `KUBECONFIG` list separators and deterministic merge precedence while keeping
   source provenance.
2. Show context, cluster, namespace, user reference, server origin, TLS policy,
   and exec-plugin declaration. Never show token/client-key data.
3. Warn that untrusted kubeconfig can execute code. Do not run any exec plugin
   during discovery, search, preview, or static validation.
4. Review every unfamiliar exec executable by canonical path, file identity,
   API version, arguments, declared environment names, interactive mode, and
   source. Prefer Kubernetes 1.35+ `kuberc` `Allowlist`/`DenyAll` policy when
   supported; older clients use Automexia's exact local grant as defense in
   depth.
5. Build a per-session kubeconfig overlay/source list and pin context and
   namespace. Do not call `kubectl config use-context` against a shared file.
6. An explicit status action may run an official read-only identity/cluster
   check and network request. Failure leaves cached metadata stale/offline.
7. Launch the user's shell with capsule context so normal `kubectl`/Helm tools
   remain authoritative. Provider-specific EKS/AKS/GKE authentication stays in
   its provider adapter.

### OpenShift

1. Reuse the Kubernetes source, context, plugin, TLS, and session-isolation
   contract; add OpenShift API, project, cluster, and identity labels.
2. Detect official `oc` and bounded public context metadata without contacting
   the cluster.
3. Login runs visibly with the installed supported browser/web flow when
   available. Never place a password or token in argv, shell history, logs, or
   Automexia state. If only a sensitive manual flow exists, leave it in the PTY
   under `oc` ownership.
4. Review loopback callback behavior and approved origins before opening a
   browser.
5. Pin the selected kubeconfig/context/project in a new capsule rather than
   mutating a shared current context/project.
6. Domain APIs and direct cluster watches require a later explicit network
   grant; shell access and official CLI behavior do not depend on them.

## Organization-managed identity integrations

External integrations are built-in reviewed adapters, not arbitrary command
templates and not a generic plugin escape hatch.

### Adapter contract

```text
ExternalIdentityAdapter
  descriptor() -> provider ID, executable ID, supported versions/capabilities
  discover(grant, limits, cancellation) -> bounded public profiles
  observe_status(profile_ref, operation) -> AuthObservation
  login(profile_ref, approved browser/PTY policy, operation) -> terminal outcome
  renew(profile_ref, operation) -> terminal outcome
  logout(profile_ref, reviewed scope, operation) -> terminal outcome
  prepare_public_artifact(intent, operation) -> expiring opaque artifact ref
  launch_plan(intent, artifact_ref?) -> exact reviewed LaunchRequest
```

All calls are typed, generation-scoped, cancellable, deadline/output bounded,
redacted, and attributable. The adapter receives no terminal history, arbitrary
environment, raw agent messages, or another provider's cache. It cannot return
a general command string or secret. Adding an executable, browser origin,
callback, direct network endpoint, token helper, or credential type requires a
manifest/ADR/threat-model review.

### Teleport

- Detect the user/organization-installed `tsh`; do not bundle a second
  untracked binary.
- Read only bounded public profile/status data after explicit scan/status.
- Login uses visible `tsh login --proxy <reviewed-proxy>` and Teleport-owned
  browser/MFA/hardware-key behavior. `tsh` retains its profiles and short-lived
  certificates.
- SSH uses exact `tsh ssh`; Kubernetes uses an explicit `tsh kube login` and
  selected cluster. Public roles, cluster, proxy, and expiry may be shown.
- Automexia never copies Teleport private keys, certificates, cookies, identity
  files, or profile secrets. It does not start `tshd`, VNet, or local proxies
  without a separately reviewed lifetime/network contract.
- Logout and access-request operations display their Teleport-wide scope and
  do not pretend a local UI can override Teleport RBAC or approval policy.

### OpenBao

- OpenBao remains the token/lease/policy owner. Login runs visibly through
  `bao login` and its configured token helper; Automexia never accepts a token
  in argv, captures CLI stdout containing a token, reads `BAO_TOKEN`, or copies
  the token-helper store.
- Initial SSH support is certificate issuance for an existing local public key.
  A built-in adapter may submit the public key and reviewed signer role through
  exact `bao` argv/stdin/file semantics and receive a short-lived **public SSH
  certificate**. The private key never crosses the boundary.
- The public certificate is treated as private metadata: stored in a
  user-private, session-owned bounded location only when OpenSSH requires a
  file; expiry/ownership are tracked; atomic creation and cleanup are mandatory;
  it never enters logs, snapshots, telemetry, or AI surfaces.
- The Hub shows signer, principals, lease/expiry, public-key fingerprint, and
  target policy. It does not expose OpenBao administration, unseal, root-token,
  secret browsing, arbitrary path writes, or a generic Vault/OpenBao console.
- If a safe CLI contract cannot avoid token-bearing output for the installed
  version, integration stays `Unsupported` and the user continues in a normal
  PTY. Convenience never justifies secret capture.

### Dependency/tool recommendation

| Tool/project | Recommendation | Reason |
|---|---|---|
| System OpenSSH | Adopt as D5 transport | Mature config, agents, host keys, hardware, certificates, jumps, and tunnels with no new protocol engine |
| Official `aws`, `az`, `gcloud`, `kubectl`, and `oc` | Adopt first in D6 | Preserve supported authentication/MFA and minimize provider dependencies in core |
| AccessKit | Reuse for the renderer-neutral accessibility tree | Already aligned with the project accessibility plan; avoid a second accessibility model |
| `nucleo` | Adopt for measured large host/action/context lists after the D5 benchmark gate | Concurrent bounded search and immutable completed snapshots without blocking UI; preserve the current matcher as deterministic fallback |
| `schemars`, `clap_complete`, and `clap_mangen` | Adopt with the typed operation registry | Generate schemas, static native-shell completion, and manuals from one source instead of hand-maintained copies |
| Existing Automexia bounded worker/cache/runtime | Reuse | Generation cancellation, session isolation, and no UI-thread I/O are already architectural requirements |
| Teleport `tsh` | Optional built-in external adapter after D5 | Short-lived SSH/Kubernetes identity remains organization-owned |
| OpenBao `bao` SSH signer | Optional built-in external adapter after D5 | Signs a public key while OpenBao owns tokens/policy and OpenSSH owns the key/connection |
| `keyring-core` plus exact platform stores | Defer until a custody ADR proves a required opaque item cannot stay with an agent/CLI | Cross-platform abstraction is useful, but backend behavior and secret custody expand the trusted boundary; never enable a broad default backend set |
| `secrecy` and `zeroize` | Defense in depth only if a future approved adapter must transiently hold a secret | They reduce accidental formatting/lifetime but do not create secure custody |
| Rust SSH libraries or embedded Termix/Electron | Reject for D5 | Duplicate protocol/renderer/session behavior and increase credential/web attack surface |
| Provider Rust SDKs / `kube-rs` | Defer to lazy, out-of-process inventory proven impossible through config/CLI | Avoid core dependency growth, hidden network, auth incompatibility, and cross-provider coupling |

Search should first reuse the project's deterministic local matcher and measured
10,000-entry inventory path. A new fuzzy-search crate is accepted only when a
benchmark, binary-size/license/security review, Unicode corpus, and cancellation
test demonstrate a material benefit.
The complete ownership matrix and protected dependency order are in
[Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md).

## Recovery, backup, migration, and export

Automexia cannot recover externally owned credentials. This statement appears
in setup, migration, recovery, and support documentation—not only in a legal or
architecture page.

- If a private key or hardware credential is lost, the user or administrator
  must generate/reissue it and update authorized systems. An agent cannot be
  treated as a recoverable private-key backup.
- If a cloud/Teleport/OpenBao session expires or its cache is removed, the user
  signs in through that provider again. Automexia cannot decrypt, reconstruct,
  reset, or bypass it.
- Backups cover Automexia favorites, tags, grouping, connection references,
  public capsule templates, and preferences. Provider/OpenSSH configuration is
  backed up according to its owner and organization policy.
- Import accepts public aliases/metadata and opaque references only. A legacy
  record containing a key, passphrase, token, cookie, credential blob, or
  unknown executable content is rejected with a migration report; it is never
  silently copied.
- Migration is previewed, bounded, atomic, idempotent, source-preserving, and
  reversible for Automexia-owned metadata. It never edits or deletes OpenSSH,
  Rio, provider, kubeconfig, Teleport, or OpenBao source data.
- Export is metadata-only by default and redacts private paths, hostnames, user
  names, account IDs, and organization labels unless the user explicitly
  selects each field. It never exports credentials or a reusable auth session.
- Diagnostics distinguish `configuration missing`, `identity unavailable`,
  `credential externally owned`, `policy denied`, and `offline`; they never
  promise a recovery action Automexia cannot perform.

## Keyboard, accessibility, and input contract

The first release exposes `Connection Hub: Open` in the application menu and
command palette. A default global shortcut is assigned only through the typed
keybinding registry after cross-platform collision testing; user bindings can
always target the action. This avoids stealing shell/editor or existing pane
shortcuts merely to match another application.

Within the Hub:

| Input | Behavior |
|---|---|
| `Ctrl/Cmd+F` or `/` outside a text control | Focus search |
| `Up`/`Down` | Move one result; never connects |
| `Home`/`End` | First/last result in the current filtered list |
| `Page Up`/`Page Down` | Move by visible page while preserving selection |
| `Enter` | Open Connection Review for the selected result |
| `Space` | Toggle favorite when focus is on a result and no text input owns Space |
| `Tab`/`Shift+Tab` | Move among major controls with a bounded, visible focus order |
| `Escape` | Close a disclosure/review level, then the Hub; never cancels an unrelated PTY |
| Context/menu key or `Shift+F10` | Open the selected item's accessible action menu |

There is no unmodified key that immediately connects from the result list.
Connection Review owns the final primary action, preventing an arrow/Enter
navigation mistake from reaching production.

Accessibility requirements:

- Model the Hub as an application-owned modal dialog: background is visually
  dimmed, input-inert, and absent/inert in the accessibility tree; focus remains
  inside until close and returns to the opener.
- Give the dialog, navigation, search, filters, result collection, each result,
  inspector/review, status, warnings, progress, and actions stable roles, names,
  descriptions, states, and actions through AccessKit.
- Use a list/grid pattern with one managed focus stop rather than thousands of
  tab stops. Arrow, Home/End, and paging behavior is renderer-neutral and
  consistent at every scale.
- Announce selection and terminal auth-state changes through a rate-limited
  live region. Never announce a token, private path, device code, terminal
  output, or every progress tick.
- Preserve visible focus at all times. Status and risk use text plus icon; color
  alone is insufficient. Meet at least 4.5:1 text contrast and support platform
  high-contrast, reduced-motion, reduced-transparency, 200%/400% text, and
  screen magnification.
- Hover-only content has the same keyboard/focus action. Touch targets are at
  least 44 logical pixels when touch mode is active without inflating desktop
  density.
- Validate Narrator and NVDA on Windows, VoiceOver on macOS, and AT-SPI/Orca on
  controlled Linux. Renderer-neutral tests do not replace those checks.

## Performance, resilience, and privacy budgets

These are provisional D5 acceptance targets to freeze against the project's
30-day baseline; they are not v0.4 guarantees:

| Path | Initial target and invariant |
|---|---|
| Open Hub from a warm 10,000-entry cache | First usable frame <= 50 ms p95 on baseline hardware; no process/network work |
| Search/filter 10,000 entries | Updated result model <= 16 ms p95; obsolete generations cancelled |
| Move selection/open cached inspector | <= 16 ms p95 without allocation proportional to total inventory |
| Cached auth/context lookup | Sub-millisecond typical; no lock shared with renderer/input/PTY |
| Passive discovery | Background, bounded, cancellable; cannot delay first editable prompt |
| Non-interactive local probe | Default one-second ceiling unless a stricter provider limit applies |
| Interactive login | No fixed user-interaction timeout; cancellation and child-tree teardown are mandatory |
| Result rendering | Virtualized; work proportional to visible rows plus small overscan |
| Storage | Versioned user-private metadata only, fixed entry/byte limits, atomic replacement, no cache growth per refresh generation |
| Failure | Last-known-good public metadata remains with stale/offline/error state |

Provider/network/process work never occurs on input, VT, render, window-resize,
or accessibility callback threads. Identical refreshes use singleflight;
provider concurrency is bounded; obsolete generations cancel; output is capped;
large inventories page; negative tool detection is cached; and closing the Hub,
session, extension, or application deterministically releases workers,
processes, browser callback listeners, watches, routes, and tunnels.

Privacy defaults:

- no telemetry of connection names, hosts, accounts, projects, clusters,
  identities, fingerprints, regions, tags, searches, or provider errors;
- logs use stable redacted IDs and typed diagnostic codes;
- screenshots/goldens use synthetic fixtures only;
- clipboard receives data only through an explicit copy action;
- crash/QA bundles use canary-tested redaction and never include provider
  config/cache content or Connection Review values.

## Renderer-neutral interaction goldens

Goldens are structured snapshots of model, geometry, semantics, focus, z-order,
and accessible output. Pixel screenshots supplement them on controlled native
runners; OCR is not an assertion mechanism.

Required fixture axes:

- wide 1,440x900, medium 1,024x768, narrow 700x800, and extreme 360x640;
- 100%, 150%, 200%, and 400% text/UI scale; dark, light, high contrast, reduced
  transparency, and reduced motion;
- zero, one, 100, and 10,000 results; Unicode, emoji, combining characters,
  long aliases, long provider labels, RTL/bidirectional-hostile display data,
  and duplicate public labels with distinct IDs;
- every authentication state and freshness state;
- SSH direct/jump/multi-jump, each tunnel type, agent-forwarding enabled,
  first-use/known/changed host key, and certificate expiry;
- AWS/Azure/GCP/Kubernetes/OpenShift, Teleport, and OpenBao review variants;
- empty, initial setup, loading, partial failure, stale, offline, denied,
  unsupported, extension crash, and revoked capability;
- background terminal with tabs, four panes, pane-local tabs, footer, command
  palette, close confirmation, image preview, search overlay, and IME to prove
  the Hub is the top modal and restores focus/z-order;
- keyboard-only route, screen-reader tree, focus return, filtered-result removal,
  live update while selected, and extension disable while review is open.

Suggested fixture location:

```text
tests/fixtures/connection-hub/
  records/
  provider-observations/
  auth-transitions/
  reviews/
  hostile/
  goldens/layout/
  goldens/accessibility/
```

## Verification plan

### Deterministic PR coverage

1. **Model/property tests:** stable IDs/sort/grouping, filter algebra, Unicode
   search, source revision/digest invalidation, state-transition legality,
   capsule/session isolation, and no secret fields in serializable types.
2. **Discovery security:** symlink/reparse races, unsafe permissions, cycles,
   path escapes, oversized/deep/count-heavy configs, malformed encodings,
   option-confused aliases, hostile kubeconfig exec declarations, and zero
   executable/network activity during passive/static discovery.
3. **Capability tests:** allow once, exact persisted scope, deny, cancel,
   revocation, publisher/package/executable/source change, production policy,
   tunnel bind risk, replay, PID reuse, and cross-session denial.
4. **Auth-state tests:** every table transition, expiry clock boundaries,
   cancelled MFA, denied policy, offline/stale recovery, concurrent operations,
   late result discard, status-source disagreement, and no automatic login.
5. **Exact launch tests:** Windows and Unix argv semantics; spaces, Unicode,
   leading dashes, metacharacters, long values, fixed executables, environment
   name/value policy, CWD validation, jump chains, and tunnels.
6. **Provider contract tests:** recorded/synthetic official CLI outputs for
   supported versions; tokens/cookies/private keys injected as redaction
   canaries; AWS SSO, Azure MFA/subscription, Google configurations/ADC
   separation, Kubernetes exec allowlist, OpenShift web login, Teleport expiry,
   and OpenBao certificate lease behavior.
7. **UI tests:** all responsive/interaction/accessibility goldens, modal
   z-order/inert background, focus trap/return, keyboard routes, virtualization,
   selection preservation, filter count, empty/error states, and no PTY resize.
8. **Concurrency/resilience:** watcher/provider/login storms, rapid open/close,
   10,000-entry refresh/search, cancellation at every await, extension crash,
   application close, and last-known-good recovery under Loom where pure state
   permits.
9. **Fuzz/mutation:** provider/public-config parsers, auth/event protocol,
   search labels, review serialization, hostile CLI output, and security-policy
   mutations. Secret/redaction and allowlist checks receive mutation coverage.
10. **Documentation/architecture:** link/status validation, no shipped claim
    before activation, provider-dependency boundary, no raw-secret type, no
    renderer/PTY/provider coupling, and exact D5/D6 ownership.

### Native and controlled evidence

| Platform | Required D5/D6 evidence |
|---|---|
| Windows | Microsoft OpenSSH client, disabled/running agent, encrypted key prompt, Windows paths/Unicode, WAM/device flow, ConPTY cancellation/process-tree cleanup, Narrator/NVDA, AppVerifier/WPR, signed packaged build |
| macOS | system/user OpenSSH, agent/keychain/hardware where available, browser callback, universal packaged app, VoiceOver, Instruments/leak/energy, signed/notarized build |
| Linux | OpenSSH with representative desktop/agent sockets, Bash/Zsh/Fish, X11/Wayland, browser/device flow, AT-SPI/Orca, ASan/TSan/Valgrind-supported suites, DEB/RPM/tar |
| Providers | Controlled least-privilege development/staging accounts plus explicitly isolated production-policy fixtures; real expiry/MFA/offline/denial without public logs |
| SSH | Deterministic mock server for PR plus real controlled OpenSSH servers for host keys, certificates, jumps, tunnels, disconnect, latency, and teardown |

Release evidence covers 1/10/50 concurrent managed sessions and tunnels,
account/context mixtures across four panes, provider/login slowdown while local
input and resize remain responsive, suspend/resume, network changes, process
crash, extension disable, OS-window close, and application shutdown. Track
startup, first usable Hub frame, search/selection latency, login-to-ready,
connect-to-first-prompt, CPU, GPU, memory, handles/descriptors, sockets, child
processes, browser listeners, watchers, cache/storage, and cleanup convergence.

The full release gate uses `cargo xtask qa --full --bundle`; private provider
and hardware logs stay out of public artifacts and the redacted result lists
every skipped case. A deterministic mock or renderer-neutral golden never
substitutes for controlled native evidence, and controlled native evidence
never substitutes for deterministic PR tests.

## Delivery phases and exit gates

### D5.0 - contract, threat model, and UX baseline

- Accept ADR 0012/replacement capability decision.
- Freeze the records, state machine, discovery tiers, capability/review model,
  resource ceilings, privacy rules, keyboard/accessibility tree, and goldens.
- Add synthetic fixtures for all states/providers without enabling process or
  network authority.

Exit: architecture, mutation, hostile-fixture, model, layout, and accessibility
tests pass; no managed process exists in production.

### D5.1 - read-only Hub and first-run detection

- Connect the completed D4 inventory to the virtualized Hub.
- Implement passive discovery, explicit local scan, favorites/tags/recent,
  search/filter/grouping, stale/last-known-good behavior, and platform setup
  guidance.
- Keep status/login/connect operations disabled or clearly non-activated.

Exit: 10,000-entry, hostile-config, three-platform static discovery, responsive,
screen-reader-model, privacy, performance, and storage tests pass.

### D5.2 - reviewed OpenSSH launch and lifecycle

- Activate the application-owned exact launch broker after its native gate.
- Add identity/agent/certificate public status, Connection Review, destination
  choice, cancellation/reconnect, jumps, typed tunnels, host-key explanations,
  capability approvals, revocation, and local audit.

Exit: D5 native/security/performance matrix passes on Windows/macOS/Linux;
disabled extension and ordinary manual SSH remain unchanged.

### D6.0 - provider-neutral capsule and authentication framework

- Activate immutable provider capsules, auth observations/state transitions,
  visible official CLI login, approved browser origins/callbacks, provider
  isolation, and public status contributions.

Exit: two simultaneous providers cannot cross-contaminate environment, caches,
operations, status, logs, or UI; all cancellation/offline/expiry states pass.

### D6.1-D6.5 - provider slices

1. **D6.1 AWS:** IAM Identity Center/STS, profiles, SSM, and EKS.
2. **D6.2 Azure:** Entra/MFA/workload identity, subscriptions, Bastion, and AKS.
3. **D6.3 Google Cloud:** named configurations, Workforce/Workload Identity,
   IAP/OS Login, and GKE.
4. **D6.4 Kubernetes/OpenShift:** trusted kubeconfig sources, exec allowlists,
   contexts/namespaces/projects, and visible login.
5. **D6.5 organization adapters:** Teleport first; OpenBao public-key signing
   only after its token-output/certificate-file contract passes security review.

Each slice is independently enabled, revoked, tested, and releasable. A provider
does not wait for or inherit another provider's capability.

### Later work

Direct provider SDK inventory, `kube-rs`, SFTP, persistent background tunnels,
native SSH, third-party adapters, public extension SDKs, and organization policy
engines remain separate capability decisions. They do not block a useful,
secure CLI-first Connection Hub.

## Acceptance criteria

The Connection Hub is complete only when:

- a user can find and review SSH and provider environments without triggering
  network/authentication work;
- public state is visible immediately from bounded cache and turns stale rather
  than disappearing;
- Windows, macOS, and Linux setup guides integrate existing OpenSSH/agents
  without silent service, startup-file, keychain, or key changes;
- every required authentication state has a truthful action and deterministic
  transition coverage;
- unchanged non-production routes support safe one-click reconnect while
  production/material changes always review;
- the review names identity, environment, risk, target, transport, jumps,
  tunnels, host trust, capability, destination, and changes;
- AWS, Azure, Google Cloud, Kubernetes, and OpenShift sessions remain isolated
  across windows, panes, tabs, clones, failures, refresh, and shutdown;
- Teleport/OpenBao remain external credential owners and no token/private key
  reaches Automexia state or observability;
- users are explicitly told that Automexia cannot recover externally owned
  credentials;
- keyboard-only, screen-reader, high-contrast, text-scale, and extreme-size
  workflows pass renderer-neutral and controlled native checks;
- search, open, selection, refresh, login, connect, cancel, and cleanup meet the
  frozen performance/resource ratchets; and
- disabling every DevOps extension restores the generic terminal with no
  behavior, storage, process, or network residue.

## Primary references

- [OpenSSH client configuration](https://man.openbsd.org/ssh_config),
  [ssh-agent](https://man.openbsd.org/ssh-agent), and
  [ssh-add](https://man.openbsd.org/ssh-add)
- [Microsoft OpenSSH key management](https://learn.microsoft.com/en-us/windows-server/administration/openssh/openssh_keymanagement)
- [Apple Keychain data protection](https://support.apple.com/guide/security/keychain-data-protection-secb0694df1a/web)
- [AWS IAM Identity Center authentication for the AWS CLI](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)
- [Azure CLI interactive authentication](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli-interactively)
- [Google Cloud CLI configurations](https://cloud.google.com/sdk/docs/configurations)
  and [authentication](https://cloud.google.com/sdk/docs/authenticate)
- [Kubernetes kubeconfig organization and trust warning](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/),
  [exec credential authentication](https://kubernetes.io/docs/reference/access-authn-authz/authentication/),
  and [kuberc credential-plugin policy](https://kubernetes.io/docs/reference/kubectl/kuberc/)
- [Teleport `tsh` reference](https://goteleport.com/docs/reference/cli/tsh/)
- [OpenBao signed SSH certificates](https://openbao.org/docs/secrets/ssh/signed-ssh-certificates/)
  and [token helpers](https://openbao.org/docs/commands/token-helper/)
- [WAI-ARIA modal-dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  and [interactive grid pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)
- [`keyring-rs`](https://github.com/open-source-cooperative/keyring-rs)
