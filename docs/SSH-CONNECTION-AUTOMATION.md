# SSH access, multi-cloud connections, and automation recipes

Status: D5.1's read-only product Hub is fully implemented locally. D5.2 is
**partially done** only at a pure, non-activated M3 review boundary: exact direct
destination grammar, stale plan/revision binding, and a complete disabled
renderer-neutral review exist. Product connection wiring, OpenSSH/PTY launch,
reconnect/receipts/cleanup, native evidence, and D6 multi-cloud delivery are not
implemented. Automexia v0.4 does not ship managed connections or automatic
remote actions; ordinary user-entered system-tool commands remain unchanged.
This document is the implementation authority for connection profiles and
connection-scoped automation. The [Connection Hub](CONNECTION-HUB.md) owns the
overall inventory, authentication, review, and responsive UI contract. The
[session-launch broker](SESSION-LAUNCH-BROKER.md) owns exact process authority,
and [ADR 0012](adr/0012-first-party-ssh-and-session-launch-boundary.md) owns the
SSH credential, process, and network boundary. If these sources conflict, the
accepted ADR and its protected review take precedence.
The user-facing `automexia connect`, `run`, `workspace`, `tunnel`, and `context`
grammar is planned in
[Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md); those examples
remain non-shipped until their D/CP activation gates pass.

## Product outcome

Automexia should let an operator select an environment, review exactly who and
what will be used, connect through the organization's existing tools, and
arrive in a correctly scoped working session without reconstructing a runbook.

The product is composed of two reusable objects:

- a **connection profile** describes the destination, transport, public
  identity, environment capsule, risk, and authentication owner; and
- an **automation recipe** describes bounded, ordered, reviewable actions to
  prepare, verify, initialize, and clean up that connection.

This model must support direct SSH, jump hosts, cloud-native access, Kubernetes
and OpenShift shells, Teleport, and local containers without turning Automexia
into a credential vault or embedding another SSH protocol implementation.

## Current implementation boundary

The current repository provides:

- the disabled, bounded D4 OpenSSH inventory in `automexia-devops-ssh`;
- provider-neutral environment and extension contracts;
- an exact-argument launch validation model that is intentionally disabled;
- independent PTY, pane, tab, clone, context, and cancellation ownership; and
- the complete Connection Hub and security specifications; and
- the F2/D5.0 bounded public-only records, strict validation, authentication/
  result reducers, deterministic non-executing planner/fingerprints, pure Hub/
  review/planner projections, fixtures, fuzz, mutation, and benchmark owners;
- the F3 bounded 10,000-record catalog, local filtering/grouping, source
  revisions, and renderer-neutral virtualization;
- the F3 explicit-grant application composition worker with generation
  supersession, redacted stale last-known-good health, and static platform
  guidance; and
- the F3 private transactional Connection Library for validated profiles,
  recipes, and Hub preferences, with CAS/recovery and redacted fresh-ID
  transfer; and
- the M3 pure direct-OpenSSH review binding: one inventory-typed concrete alias or
  one bounded literal host, exact `ssh`/`session.launch` identity, profile/
  source/capsule/observation/trust invalidation, redacted argv shape, and a
  renderer-neutral disabled review with no added authority.

It now provides the D5.1 rendered read-only Connection Hub in v0.5 source
builds, but does **not** provide a profile/recipe editor, automatic SSH launch,
recipe executor, cloud transport adapters, or remote action handshake. D5.2/D6
UI examples remain planned contracts, not instructions for v0.4.

### F2/D5.0 implementation ledger (2026-08-17)

- **Fully done locally:** strict schema-1 ConnectionDefinition, Observation,
  Intent, Review, Receipt, Profile, Recipe, Step, Tunnel, document, state, and
  ResolvedConnectionPlan owners with fixed ceilings, sealed validated wrappers,
  fallible plan sequencing, and redacted references.
- **Fully done locally:** hostile controls/bidi, option confusion, duplicates,
  cycles, missing dependencies, oversize, secret-bearing fields, command-string
  shapes, and invalid policy/retry combinations fail closed.
- **Fully done locally:** canonical fingerprints and dry-run resolution cover
  all material target/identity/route/tunnel/recipe/executable/capability/source
  changes while returning an all-false authority ceiling.
- **Fully done locally:** exhaustive authentication/result reducers reject late
  operation generations, and the pure responsive Hub, Connection Review, and
  64-step planner projections preserve roving focus, live progress, route-aware
  modal cycles, and value-redacted human labels. Thirty required regressions and
  structured provider/auth/layout/accessibility fixtures have property,
  mutation, architecture, fuzz, and benchmark ownership.
- **Not done externally:** ADR 0012 protected acceptance.
- **Fully done separately in D5.1:** private transactional profile/recipe/
  preference persistence, explicit recovery, fault-preserving writes, and
  redacted fresh-ID transfer.
- **Fully done separately in D5.1:** catalog, explicit reviewed grants, app-owned
  joined runtime, read-only modal, and favorite/tag CAS UI are implemented.
- **Not done in D5.2/D6:** every connection/provider execution path and the full
  profile/recipe editor remain separately gated.

Therefore F2/D5.0 is **Partially done** overall despite its complete local
non-executing exit. No account, filesystem, process, network, credential, PTY,
listener, window, or GPU is required to test this baseline.

## Non-negotiable principles

1. **System tools remain authoritative.** The first implementation launches
   system OpenSSH or an explicitly reviewed official provider CLI. Automexia
   does not implement SSH crypto, host-key storage, MFA, or cloud token flows.
2. **One reviewed intent creates one independent session.** A connection gets
   its own route, PTY, environment capsule, action state, and cleanup owner.
3. **No hidden global mutation.** A profile must not silently change another
   pane's AWS profile, Azure subscription, GCP configuration, Kubernetes
   context, working directory, or environment.
4. **Typed actions before scripts.** Normal workflows use validated action
   types and structured parameters. Arbitrary scripts are an advanced,
   separately approved capability.
5. **No credential custody.** Profiles store public metadata and opaque
   references only. Passwords, private keys, passphrases, cloud tokens, agent
   responses, and MFA data never enter Automexia persistence or logs.
6. **No terminal-output automation.** Untrusted local or remote output cannot
   create, edit, approve, reorder, or trigger an action.
7. **Review behavioral changes.** A changed destination, jump chain, identity,
   executable, provider, capability, tunnel, recipe hash, or source revision
   invalidates prior approval.
8. **Failure is visible and bounded.** Each action has a deadline, cancellation
   behavior, result state, and explicit recovery. No infinite retry exists.
9. **Manual recovery always works.** Disabling every DevOps extension leaves a
   complete terminal where the user can run the native tools directly.
10. **Accessibility and keyboard operation are product requirements.** Every
    action and state has text, focus semantics, and a non-color identifier.

## User journeys

### Quick connect without automation

1. Open the Connection Hub.
2. Search by alias, host, provider, account, subscription, project, cluster,
   environment, tag, favorite, or recent use.
3. Select a result and inspect the Connection Review.
4. Choose a destination: new pane, pane-local tab, window tab, or OS window.
5. Approve the exact capability when required.
6. Authenticate through OpenSSH or the official provider CLI in a normal PTY.
7. Receive an independent terminal with truthful context tags.

Opening, searching, filtering, or selecting a result performs no login,
network request, executable configuration evaluation, or process creation.

### Connect with a reusable recipe

1. Select a connection profile.
2. Select one or more compatible recipes.
3. Review the resolved action plan, variables, risk, expected mutations, and
   exact external tools.
4. Fill required non-secret variables or choose opaque credential references.
5. Connect.
6. Watch the ordered progress timeline: preflight, authentication, transport,
   remote initialization, verification, ready.
7. On failure, choose only an allowed recovery: retry, skip, edit, disconnect,
   or open the authoritative diagnostic.
8. On disconnect, Automexia cancels pending work and closes session-owned
   listeners and processes.

### Create a profile and recipe

The default editor is form-driven. An advanced text representation may be
provided after the versioned schema is stable, but editing structured fields
must remain the safest and easiest path.

1. Choose a transport template.
2. Choose a discovered target or enter a public destination.
3. Select the remote user, provider scope, and environment classification.
4. Add actions from a searchable catalog.
5. Reorder actions within valid stage constraints.
6. Resolve validation errors inline.
7. Preview the exact plan without running tools.
8. Save, test explicitly, and approve the resulting fingerprint.

## Connection profile model

The provider-neutral source model should live in `automexia-devops`; the UI
projection belongs in `automexia-ui-model`.

```text
ConnectionProfileV1
  schema_version
  id + revision
  display_name + description
  tags + favorite
  environment_classification
  provider_kind
  transport
  public_target
  identity_reference
  environment_capsule_template
  recipe_references[]
  tunnel_definitions[]
  destination_preference
  source_reference + source_revision
  approval_fingerprint
  created_at + updated_at + last_used_at
```

### Required profile fields

| Field | Contract |
|---|---|
| ID | Random opaque identifier; never derived from a host, user, or secret |
| Revision | Monotonic; stale edits and launches are rejected |
| Display name | User-controlled, bounded, Unicode-safe, sanitized for UI only |
| Environment | Local, development, test, staging, production, or custom text plus risk |
| Provider | None, AWS, Azure, GCP, Kubernetes, OpenShift, Teleport, or future reviewed provider |
| Transport | Typed transport descriptor; never a free-form command line |
| Public target | Alias or provider resource reference safe to show in review |
| Identity | Public user/account/profile/certificate kind or opaque external reference; never secret material |
| Capsule | Session-only public environment and context intent |
| Recipes | Ordered references resolved to immutable recipe revisions at launch |
| Source | User-created, OpenSSH inventory, provider inventory, imported, or organization-managed |

### Transport descriptors

```text
OpenSshAlias { alias }
OpenSshExplicit { host, port?, user?, proxy_jump[] }
AwsSessionManager { target, profile, region, document? }
AzureBastion { bastion, resource_group, vm_resource_id, subscription }
GcpIap { instance, project, zone, configuration }
KubernetesExec { context, namespace, workload, container?, shell }
OpenShiftRsh { context, namespace, workload, container? }
TeleportSsh { proxy?, cluster?, target, login? }
LocalContainer { runtime, container, user?, shell }
```

Every descriptor compiles to an exact executable path and argument vector only
after a visible capability decision. Provider versions determine supported
flags. An unavailable or incompatible official tool produces a truthful
`Missing` or `Unsupported` state; it never falls back to a different transport.

## Automation recipe model

```text
AutomationRecipeV1
  schema_version
  id + revision
  display_name + description
  compatible_providers[]
  compatible_transports[]
  variables[]
  steps[]
  approval_fingerprint
  created_at + updated_at

AutomationStepV1
  id
  stage
  action
  preconditions[]
  timeout
  failure_policy
  retry_policy
  risk
  confirmation_policy
  reconnect_policy
```

Recipes are reusable and immutable for an in-flight connection. Editing a
recipe creates a new revision; it cannot change a running session. A launch
captures the exact profile revision, recipe revisions, executable identities,
and public variable values in its session intent.

### Execution stages

Stages have a fixed partial order:

```text
Resolve
  -> Preflight
  -> Authenticate
  -> BeforeConnect
  -> Connect
  -> RemoteInitialize
  -> Verify
  -> Ready
  -> BeforeDisconnect
  -> Cleanup
```

- **Resolve** combines references and validates limits without external work.
- **Preflight** checks local files, tools, versions, agents, and public state.
- **Authenticate** starts only a reviewed official interactive authentication
  flow when the existing state is missing, locked, or expired.
- **BeforeConnect** applies session-local public environment and prepares
  session-owned resources.
- **Connect** starts the one authoritative transport process and PTY.
- **RemoteInitialize** runs only through an explicitly supported remote-shell
  contract; it never guesses readiness from arbitrary screen contents.
- **Verify** checks declared identity and context without broad discovery.
- **Ready** transfers normal ownership to the user.
- **BeforeDisconnect** runs only reviewed, bounded, safe actions while the
  route is still valid.
- **Cleanup** always cancels remaining work and releases Automexia-owned state.

### Action risk classes

| Class | Examples | Default approval |
|---|---|---|
| Observe | Check tool version, identity, directory, or context | Review with profile |
| Session-local | Set pane-only environment, directory, display preference | Review with profile |
| Authenticate | Start browser/device/MFA/agent interaction | User-visible each time required |
| Remote-session | Change remote directory, export a public value | Review with recipe |
| Privileged | `sudo`/`doas` user switch, protected target | Confirm every connection |
| Network-listener | Local/remote/dynamic tunnel | Exact endpoint review; non-loopback always confirm |
| Persistent mutation | Provider/config/workspace file change | Explicit opt-in; never default automatic |
| Custom code | User script or command template | Separate capability and changed-hash review |

### Failure and retry policies

Supported failure policies are `StopAndKeepDiagnostic`, `StopAndDisconnect`,
`WarnAndContinue`, and `OfferManualRecovery`. `Ignore` is not a valid policy.

Automatic retry is allowed only when an action declares all of the following:

- idempotent behavior;
- no privilege prompt or user interaction;
- no persistent mutation;
- a bounded attempt count;
- exponential backoff with jitter and a total deadline; and
- cancellation-safe resource ownership.

Authentication, host-key decisions, MFA, `sudo`, provider account changes,
context-file mutations, and custom scripts never retry silently.

### Reconnect semantics

- Reconnect creates a new PTY and new execution generation.
- `OncePerConnection` actions may run again in the new generation.
- `OncePerUserIntent` actions require a new explicit decision after failure.
- Persistent mutations default to `NeverAutomaticallyRepeat`.
- A cloned pane inherits the profile and capsule intent but does not inherit
  process handles or completed action state. The user chooses whether to run
  the recipe again.
- Obsolete generations cannot publish status into a replacement session.

## First-party action catalog

The initial catalog should contain typed actions rather than command strings.

### Local and capsule actions

- `SetSessionEnvironment { name, public_value }`
- `UnsetSessionEnvironment { name }`
- `SetLocalWorkingDirectory { path_reference }`
- `RequireExecutable { executable_id, version_requirement? }`
- `RequireFile { granted_path_reference, kind }`
- `CheckAgentState { agent_kind }`
- `SetAwsScope { profile, region? }`
- `SetAzureScope { subscription, config_reference? }`
- `SetGcpScope { configuration, project?, region?, zone? }`
- `SetKubernetesScope { kubeconfig_reference, context, namespace? }`
- `SetOpenShiftScope { kubeconfig_reference, context, namespace? }`
- `SetTerraformIntent { working_directory, workspace? }`

### Connection actions

- `ConnectOpenSshAlias`
- `ConnectOpenSshExplicit`
- `ConnectAwsSessionManager`
- `ConnectAzureBastion`
- `ConnectGcpIap`
- `ConnectKubernetesExec`
- `ConnectOpenShiftRsh`
- `ConnectTeleport`
- `ConnectLocalContainer`
- `StartLocalForward`
- `StartRemoteForward`
- `StartDynamicForward`

### Remote initialization and verification

- `SetRemoteWorkingDirectory`
- `SetRemotePublicEnvironment`
- `SwitchRemoteUser { method, user }`
- `StartRemoteLoginShell { shell }`
- `VerifyRemoteUser`
- `VerifyRemoteWorkingDirectory`
- `VerifyAwsIdentity`
- `VerifyAzureAccount`
- `VerifyGcpAccountAndProject`
- `VerifyKubernetesContext`
- `VerifyOpenShiftContext`
- `VerifyCommandSucceeds { reviewed_command_id, arguments }`

Verification actions must return bounded structured result classes. Raw
stdout/stderr remains in the PTY and is never interpreted as a new action.

## Correct handling of common workflows

### Select the SSH user

Prefer the `User` value in an OpenSSH alias or the typed `user` field in an
explicit destination. The Connection Review displays the resolved public user.
Automexia should not connect as one user and automatically run `su` when the
correct user can be selected before connection.

### Switch to a privileged remote user

Use a `SwitchRemoteUser` action with a reviewed method such as `sudo` or
`doas`. It is privileged, confirms every connection, and leaves password,
MFA, policy, and TTY interaction visible to the user. Automexia never stores,
retrieves, or types a privilege password and never weakens remote policy.

### Enter a remote working directory

Use a typed, absolute or home-relative directory value and a declared remote
shell adapter. Failure leaves the user in the authenticated shell with a clear
diagnostic or disconnects according to the reviewed policy. No local path is
silently translated into a remote path.

### Select a Kubernetes or OpenShift context

`kubectl config use-context` writes `current-context` in a kubeconfig and can
affect other sessions. Automexia therefore prefers explicit `--context` and
`--namespace` parameters, shell functions scoped to the pane, or a
user-selected isolated configuration. It must not rewrite a shared kubeconfig
as an automatic connection side effect.

### Select cloud context

- AWS uses pane-scoped `AWS_PROFILE`, `AWS_REGION`, and official profile/SSO
  behavior. Access-key values are never profile fields.
- Azure prefers explicit subscription arguments. A user-selected
  `AZURE_CONFIG_DIR` may isolate official CLI state, but Automexia stores only
  the path reference and does not parse or copy its tokens.
- GCP uses `CLOUDSDK_ACTIVE_CONFIG_NAME` and property environment overrides or
  explicit `--configuration`; it does not activate a global configuration.
- Kubernetes and OpenShift use explicit context/namespace values rather than a
  global current-context mutation.
- Terraform workspace selection can mutate project state and is therefore a
  visible persistent-mutation action, never a default automatic step.

## Remote execution contract

Remote automation is a separate security boundary from local exact-argument
launch. SSH transmits a remote command as text for the destination shell;
there is no portable remote argv channel. Automexia must not pretend otherwise.

The initial implementation supports remote initialization only when one of
these contracts is explicit:

1. the user's OpenSSH alias already defines `RemoteCommand`/`RequestTTY`;
2. the profile declares a supported remote shell and Automexia compiles only
   first-party typed actions through its reviewed shell adapter; or
3. destination-side Automexia shell integration performs a versioned
   capability handshake designed and reviewed in a later phase.

Automexia must never detect a prompt by scraping cells and then inject hidden
keystrokes. If no supported contract exists, remote steps appear as manual
Quick Actions after connection rather than running automatically.

Supported shell adapters start narrowly:

- POSIX `sh` for portable directory/environment/login-shell initialization;
- PowerShell with an exact encoded/script-block contract after security review;
- Bash and Zsh only where behavior differs from POSIX `sh`;
- Fish only after its quoting and startup semantics have dedicated tests; and
- CMD does not receive automatic remote scripting in the first release.

Parameters are encoded by the adapter, not interpolated into a user-provided
template. Newlines, NUL, control characters, invalid Unicode, shell
metacharacters, option-like destinations, and over-limit values fail closed.

### Custom scripts

Custom scripts are useful but cannot share the safe typed-action capability.
When eventually enabled they require:

- a separate `custom-code.execute` capability;
- explicit local or remote shell selection;
- exact source text and SHA-256 fingerprint in Connection Review;
- renewed approval after any byte changes;
- no implicit secrets or ambient environment export;
- bounded source, output, time, memory, and process tree;
- cancellation and descendant cleanup;
- disabled automatic retry;
- a visible production warning; and
- export/import provenance with untrusted imports disabled until reviewed.

## Provider and transport adapters

| Adapter | Official owner | Session-scoped intent | Automatic actions allowed initially |
|---|---|---|---|
| OpenSSH | System/user-selected `ssh` | Alias or exact host/user/port/jump/tunnels | Preflight, connect, supported typed remote initialization |
| AWS | AWS CLI and Session Manager plugin | Profile, account hint, region, target | SSO/login prompt, SSM connect, identity verification |
| Azure | Azure CLI and Bastion native client | Subscription, tenant/account hint, config reference, resource | Login prompt, Bastion connect, account verification |
| GCP | Google Cloud CLI and IAP | Configuration, account hint, project, zone, instance | Login prompt, IAP/SSH connect, project verification |
| Kubernetes | `kubectl` and kubeconfig owner | File reference, context, namespace, workload/container | Exec, scoped wrapper, context verification |
| OpenShift | `oc` and kubeconfig owner | File reference, context, project, workload/container | Login prompt, `rsh`/exec, context verification |
| Teleport | `tsh` | Proxy, cluster, login, target | Login prompt, SSH connect, identity verification |
| Container | User-selected runtime | Runtime, container, user, shell | Exact exec and working-directory initialization |

Each provider extension is independently enabled and granted. One provider
cannot read another provider's files or reuse its capability decision. Direct
SDK inventory remains later work and requires a new threat review; the first
release uses official CLIs and their native authentication/cache behavior.

## Connection Hub automation UI

### Wide layout

```text
+ Connection Hub ----------------------------------- Add  Refresh  Settings X +
| Groups          | Connections                    | Review / Automation       |
| All          84 | prod-bastion      Ready        | Production Bastion        |
| SSH          28 | staging-k8s       Expired      | AWS / eu-west-3           |
| AWS          16 | payments-iap      Locked       | deploy@bastion-prod        |
| Azure        12 | ...                            | Route: corp -> prod        |
| GCP          11 |                                | Recipe: Platform Operator  |
| Kubernetes   17 |                                | 1 Preflight                |
| Workspaces    9 |                                | 2 Authenticate if needed   |
|                  |                                | 3 Connect                  |
|                  |                                | 4 cd /srv/platform         |
|                  |                                | 5 Verify identity          |
|                  |                                | [Review & Connect]         |
+----------------------------------------------------------------------------+
```

Medium width collapses groups into a filter bar. Narrow width uses a
list-detail route with a stable Back action. The modal remains window-level,
topmost over application chrome, and never changes PTY dimensions.

### Recipe editor

The editor contains:

- name, description, compatibility, and risk summary;
- a staged vertical action timeline;
- Add Action search grouped by local, cloud, SSH, context, verification, and
  advanced actions;
- inline typed fields, validation, help, and source ownership;
- drag or keyboard reordering constrained by stage dependencies;
- variable definitions with preview values and secret-reference prohibition;
- exact plan preview and changed-fingerprint warning;
- Save Draft, Validate, Test Explicitly, and Enable actions; and
- accessible error summary and focus navigation.

### Connection progress

Every step displays `Pending`, `Running`, `WaitingForUser`, `Succeeded`,
`Warning`, `Failed`, `Cancelled`, or `SkippedByUser`. The active step has a
spinner only while real work exists. Provider-native prompts stay in the PTY;
the Hub displays a focus action rather than duplicating or obscuring them.

The user can cancel at every stage. Cancellation never reports success before
the owned process tree, listener, pending worker, and route are reconciled.

## Persistence and migration

Automexia-owned profile data should live beneath the Automexia configuration
root in a dedicated `connections/` directory:

```text
connections/
  profiles.v1.json
  recipes.v1.json
  preferences.v1.json
```

The existing D4 public OpenSSH metadata remains under
`extensions/devops-ssh/connections.v1.json`; it is a discovery source, not the
profile database.

Persistence requirements:

- versioned schemas with reject-unknown behavior at security boundaries;
- bounded files, counts, strings, tags, variables, actions, and aggregate
  serialized size;
- same-directory temporary write, file synchronization, atomic replacement,
  and directory synchronization where supported;
- user-only Unix mode and Windows current-user DACL;
- last-known-good state after malformed, oversized, interrupted, or read-only
  failures;
- no private source path in UI/logs unless explicitly revealed by the user;
- no provider cache, key, passphrase, token, terminal content, or environment
  value copied into these documents;
- export defaults to public profile/recipe metadata only and strips recent
  usage, local paths, account identifiers, and opaque secret references; and
- imports are untrusted, disabled until reviewed, and receive new local IDs.

Initial architecture ceilings:

| Resource | Ceiling |
|---|---:|
| Profiles | 10,000 |
| Recipes | 2,000 |
| Steps per recipe | 64 |
| Variables per recipe | 64 |
| Tunnels per profile | 32 |
| Tags per profile | 32 |
| One display value | 4 KiB |
| One custom script, when enabled | 256 KiB |
| Profiles document | 16 MiB |
| Recipes document | 16 MiB |
| Audit records retained locally | 10,000 or 30 days, whichever is smaller |

These are maximum architecture limits. UI defaults and organization policy may
set lower values but cannot raise them without a reviewed architecture change.

## Capability and approval model

Separate capabilities are required for:

- reading each exact inventory/config source;
- probing an exact executable/version;
- starting provider authentication;
- launching one exact connection transport;
- opening each typed listener;
- executing reviewed typed remote initialization;
- executing custom code; and
- persisting public profile/recipe metadata.

An approval binds the extension/package identity, version/digest, operation,
session, capsule revision, executable identity, target, route, recipe hash,
arguments, public environment names, tunnel endpoints, risk, timestamp, and
expiry. It cannot be replayed in another session or after revocation.

Production, privilege escalation, non-loopback listeners, agent forwarding,
changed host identity, changed routes, persistent mutations, and custom code
always return to visible review.

## Security and privacy threats

The implementation must explicitly test and mitigate:

- command, option, shell, environment, path, and control-character injection;
- malicious OpenSSH/provider configuration and executable directives;
- symlink/reparse swaps and check-to-spawn executable replacement;
- confused-deputy launches across sessions, providers, capsules, or users;
- stale approval, generation, inventory, executable, and recipe reuse;
- hostile remote output attempting to imitate readiness or approvals;
- terminal escape sequences, clipboard requests, graphics and oversized input;
- secret exposure through logs, crash reports, QA bundles, UI snapshots,
  completion, clipboard history, telemetry, or AI surfaces;
- unsafe host-key acceptance, agent forwarding, X11 forwarding, and wildcard
  listener binding;
- malicious imported profiles or recipe scripts;
- retrying a mutation or privileged action after an ambiguous result;
- orphaned transport, authentication, tunnel, helper, or shell processes; and
- cross-pane provider/context contamination.

Audit output contains stable IDs, public action kinds, result classes,
durations, and bounded redacted diagnostics. It excludes raw command output,
terminal contents, environment values, tokens, passphrases, key paths, private
hostnames, and authentication messages.

## Performance and resource budgets

No Connection Hub or recipe work runs on the renderer, PTY parse, or input
thread. Inventory, validation, provider probes, and action execution use
bounded cancellable workers and immutable latest-generation snapshots.

Initial budgets:

| Operation | Target/bound |
|---|---:|
| Open cached Hub | first useful model <= 50 ms |
| Search 10,000 profiles | p95 <= 16 ms after warm index |
| Local filter/group update | p95 <= 8 ms |
| Action-plan validation, 64 steps | p95 <= 10 ms |
| Input-to-render while connection work runs | no material regression from terminal baseline |
| Provider/executable probe | explicit, cancellable, <= 5 s default |
| One action | declared deadline, <= 30 s default except visible authentication/connect |
| Automatic attempts | <= 3 for eligible idempotent actions |
| Concurrent background profile operations | bounded global and per provider |
| Persistent audit/profile storage | bounded by the ceilings above |

Connection, authentication, and first-prompt latency are measured separately;
provider or network time is never reported as renderer latency. Benchmarks must
record cold/warm state and tool versions.

## Resilience and cleanup

- Adjacent inventory refreshes and obsolete plan validations coalesce.
- A newer generation actively cancels older work and rejects late results.
- Last-known-good inventory/profile/recipe state survives refresh failure.
- Authentication cancellation terminates only its owned process tree.
- A failed connection does not alter the existing pane layout.
- A failed recipe never silently opens a different transport or user.
- Session-owned tunnels bind loopback by default and close with the session.
- Persistent background tunnels are a separate future product mode with
  visible ownership, health, lifetime, and bounded restart policy.
- Shutdown cancels workers, prevents new launches, reconciles routes and
  listeners, flushes bounded public metadata, and times out before forced
  process-tree termination.
- Crash recovery may report an interrupted action but never resumes a remote
  mutation automatically.

## Accessibility and responsive behavior

- Complete keyboard navigation for group, result, review, stage, action, field,
  error, and confirmation surfaces.
- Predictable focus restoration after authentication, cancellation, errors,
  modal close, and responsive layout transitions.
- Screen-reader names include provider, target, identity, environment, state,
  risk, and action position.
- Status never relies only on icon, animation, or color.
- Production and privileged actions include explicit text and accessible state.
- At 200% text scale and narrow widths, fields reflow without clipping action
  names, errors, confirmation choices, or focus indicators.
- Motion respects reduced-motion settings; countdowns and spinners are not the
  only indication of progress.
- The model remains renderer-neutral so keyboard, focus, reading order, and
  responsive projections can be tested without OCR or a GPU.

## Placement in the existing architecture

| Concern | Existing owner |
|---|---|
| Profile, recipe, action, result, and provider-neutral state | `automexia-devops` |
| Renderer-neutral Hub/editor/focus/accessibility models | `automexia-ui-model` |
| Capability requests, bounded types, operation/session IDs | `automexia-extension-api` |
| Cancellation, queues, workers, latest generation, cache | `automexia-extension-runtime` |
| Static OpenSSH inventory and public metadata | `automexia-devops-ssh` |
| Exact executable/argv authorization and process attachment | application launch broker |
| PTY, route, pane/tab/window, lifecycle, notification | `apps/automexia-terminal` |
| GPU composition and input hit testing | frontend renderer adapter |
| Shell-owned prompt/completion and optional future handshake | `shell-integration` |

The renderer never parses provider files, launches a command, stores a token,
or owns action policy. Extensions never receive PTY output or renderer access.
The application remains the only owner allowed to attach a process to a route.

## Delivery plan

### D5A - contracts and golden fixtures

Status: **Partially done** overall.

- [ ] **Not done externally:** accept or supersede ADR 0012 through protected
  review.
- [x] **Fully done locally:** freeze bounded profile, recipe, step, connection,
  state, result, review, plan, and capability schemas.
- [x] **Fully done locally:** implement strict validation, revision fields,
  fingerprints, redacted debug, and synthetic all-state fixtures. Persisted
  schema migration remains D5B because F2 performs no storage.
- [x] **Fully done locally:** add renderer-neutral wide/medium/narrow Connection
  Hub, review, and recipe-planner models with keyboard/focus/accessibility
  goldens.
- [x] **Fully done locally:** keep all process, network, provider, credential,
  PTY, listener, renderer, and GPU authority disabled.

Exit status: hostile/mutation/property/record/state/model/layout/accessibility
tests pass and the synthetic slice is reviewable without an account, PTY,
network, window, or GPU. Protected ADR acceptance remains the overall blocker.

### D5B - read-only Connection Hub

Status: **Partially done**.

- [ ] **Partially done:** the bounded catalog and explicit-grant composition
  service connect public D4 records to pure search/grouping; product
  state-root/controller and rendered-modal wiring remain.
- [ ] **Partially done:** explicit scans, metadata-backed favorite/tag/recent
  values, source revision, stale last-known-good health, and static platform
  guidance exist; exact file-selection and favorite/tag mutation UI remain,
  while recent must stay read-only until managed D5.2 success receipts exist.
- [x] **Fully done locally:** private profile/recipe/preference persistence,
  strict transfer parsing, canary redaction, fresh local import IDs, revision
  CAS, concurrent-writer exclusion, atomic previous-generation recovery, and
  read-only/disk-full preservation pass.
- [ ] **Partially done:** pure projections expose no execution authority, but
  the product modal must visibly explain why Connect and automatic actions are
  disabled.

Exit gate: the 10,000-profile pure search budget and Windows-local Connection
Library persistence tests pass. Product rendering, application ownership,
native macOS/Linux permission runs, controlled screen readers, and the
remaining exact-selection/mutation flows are still required.

### D5C - recipe editor and dry-run planner

Status: **Partially done**; the pure model/planner is done and the product editor
is not done.

- [x] **Fully done locally:** add the typed first-party action catalog,
  variables, dependencies, risk, confirmation, deadlines, failure/retry/
  reconnect policies, and plan projection.
- [x] **Fully done locally:** add deterministic compilation to a non-executing
  `ResolvedConnectionPlan` with explicit all-false authority.
- [ ] **Partially done:** changed-fingerprint review data and reusable recipe
  references exist; persisted selection and the interactive editor belong to
  D5B/D5.1 and are not implemented.
- [x] **Fully done locally:** no tool is spawned and no connection is opened.

Exit status: supported model actions and hostile parameters have deterministic
validation, redaction, accessibility, and layout coverage. Editor interaction,
persistence/migration, and native UI evidence remain before D5C is fully done.

### D5D - managed system OpenSSH

Status: **Partially done** at the non-activated M3 review-model boundary.

- [ ] **Partially done:** the reviewed one-argument inventory-typed alias and
  literal-host grammar, exact F2/revision/executable/capability binding, hostile
  rejection, stale invalidation, redacted debug/argv shape, and complete
  renderer-neutral review exist; D4-record-to-F2 product composition remains.
- [ ] **Not done:** product controller/renderer connection action, accepted M2
  activation, independent PTY attachment, system OpenSSH launch, prompt and
  diagnostic passthrough, and native/manual compatibility evidence.
- [ ] **Partially done:** pure cancel/rebind/revocation already belongs to the
  test-only broker; reconnect, notifications, receipts, process cleanup, and
  route cleanup remain not done.
- [ ] **Not done:** explicit user/port/IPv6 destinations, config-defined jump
  chains, typed jumps, host-key detail, public identity readiness probes, and
  tunnels remain separately reviewed M4/M5 slices.

Exit gate: deterministic mock-server and controlled native OpenSSH evidence
passes on Windows, macOS, and Linux.

### D5E - safe automatic actions

- Execute local/session actions first.
- Add explicit known-shell remote initialization without cell scraping or
  hidden key injection.
- Add user switching, directory, public environment, and verification actions.
- Keep custom scripts disabled.

Exit gate: ordering, cancellation, reconnect, ambiguous failure, hostile
values, privilege prompts, resource cleanup, and cross-session isolation pass.

### D6 - independently enabled cloud adapters

Deliver AWS, Azure, GCP, Kubernetes/OpenShift, and Teleport one at a time. Each
adapter needs its own accepted threat model, exact CLI/version contracts,
authentication states, capsule isolation, grants, fixtures, native tests,
performance evidence, documentation, and disable/uninstall behavior.

### Later advanced actions

Custom scripts, organization-signed recipe packs, destination-side integration,
persistent tunnels, SFTP, direct provider SDK inventory, and a public extension
SDK remain separate future capabilities. None is required for the first secure
profile/recipe release.

## Verification plan

### Unit, property, and mutation coverage

- All schema fields, limits, unknown values, migration, and forward rejection.
- Canonical fingerprints and changed-behavior invalidation.
- Stage/dependency ordering, cycles, incompatible actions, and duplicate IDs.
- Unicode, spaces, quotes, leading dashes, NUL, control characters, long input,
  malformed encoding, metacharacters, and option confusion.
- Risk, confirmation, retry, reconnect, and failure-policy invariants.
- Redaction canaries in debug, errors, persistence, audit, crash, and QA models.
- Import provenance, hostile documents, interrupted writes, permissions,
  symlink/reparse swaps, and last-known-good recovery.
- Renderer-neutral focus, reading order, text scaling, responsive layout, and
  every action/authentication/result state.

### Deterministic integration coverage

- Fake executable registry proving exact executable and argument arrays.
- Direct SSH, config alias, explicit user, jump and multi-jump routes.
- Local, remote, and dynamic tunnels including collision and cleanup.
- Mock SSH server: first host key, accepted key, changed key, password, agent,
  encrypted key, certificate, disconnect, timeout, hostile output, and exit.
- Mock official provider CLIs: ready, missing, locked, expired, MFA, cancelled,
  offline, denied, malformed, slow, oversized, and version-incompatible.
- Remote POSIX and PowerShell action compilation and failure behavior.
- Multiple panes, tabs, windows, clones, reconnects, and stale generations with
  no context, approval, process, output, or result contamination.
- Cancellation at every lifecycle stage and application shutdown.

### Native platform matrix

| Platform | Required evidence |
|---|---|
| Windows | Microsoft OpenSSH, ConPTY, `ssh-agent` service, encrypted prompts, Unicode/space paths, process-tree cleanup, DACLs, Narrator/NVDA, AppVerifier/WPR, signed package |
| macOS | System/user OpenSSH, agent/keychain/hardware where available, universal app, VoiceOver, Instruments/leak/energy, signed/notarized package |
| Linux | Representative OpenSSH/agent sockets, Bash/Zsh/Fish adapters, X11/Wayland, AT-SPI/Orca, ASan/TSan/Valgrind-supported suites, DEB/RPM/tar |

Cloud release evidence uses controlled disposable tenants/projects/accounts and
non-production resources. Secrets and private identifiers never enter public
CI, snapshots, logs, artifacts, or screenshots.

### Performance and leak coverage

- Cold/warm inventory, Hub open, search, plan validation, provider probe,
  authentication start, connection start, first remote prompt, actions, and
  tunnel setup measured separately.
- 1, 10, and 50 parallel connections and tunnels.
- Rapid connect/cancel/reconnect/close storms.
- Handles, descriptors, sockets, listeners, threads, tasks, child processes,
  private bytes, working set, cache, queue, and storage growth.
- Long-lived idle sessions, high-throughput output, resize storms, hostile VT
  streams, and provider-worker saturation without input/render regressions.
- Thirty-day named-runner baseline before enforcing the project performance
  ratchet; retries remain failures for investigation, not hidden passes.

## Acceptance criteria

This feature is complete only when:

- a user can find, review, and connect to supported SSH/cloud targets without
  reconstructing commands or mutating another session;
- profiles and recipes are reusable, versioned, bounded, atomic, and portable
  after private fields are stripped;
- typed automatic actions can select a user, directory, cloud scope, cluster,
  namespace, and verified identity with clear review and failure behavior;
- no action runs from search, hover, selection, untrusted output, import, or a
  stale approval;
- authentication remains provider/OpenSSH-owned and no secret reaches
  Automexia persistence, logs, snapshots, telemetry, clipboard, or AI;
- exact launch, remote initialization, cancellation, reconnect, tunnel, and
  cleanup behavior passes deterministic and native tests;
- Windows, macOS, and Linux UI, accessibility, performance, and resource
  evidence is retained;
- disabling all integrations restores a complete ordinary terminal; and
- every current limitation and recovery procedure is documented.

## Authoritative references

- [OpenSSH client configuration](https://man.openbsd.org/ssh_config): user,
  jump, remote command, TTY, forwarding, agent, and host-key behavior.
- [AWS CLI environment variables](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html):
  profile and region precedence.
- [AWS Systems Manager Session Manager](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager.html):
  provider-managed sessions.
- [Azure CLI configuration](https://learn.microsoft.com/en-us/cli/azure/azure-cli-configuration):
  configuration directory and CLI behavior.
- [Azure Bastion native client](https://learn.microsoft.com/en-us/azure/bastion/connect-vm-native-client-windows):
  native SSH access through Bastion.
- [Google Cloud CLI configurations](https://cloud.google.com/sdk/docs/configurations):
  session-selectable named configurations.
- [Kubernetes `kubectl config use-context`](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_config/kubectl_config_use-context/):
  mutation of kubeconfig current context.
