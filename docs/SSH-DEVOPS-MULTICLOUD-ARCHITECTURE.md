# SSH, DevOps, and Multi-Cloud Extension Architecture

- Status: consolidated research and proposed target architecture
- Date: 2026-08-13
- Applies to: Automexia Terminal after the v0.4 stabilization boundary
- Current implementation status: D1/D2 and disabled D4 are implemented; F2/D5.0 non-executing connection/Hub/planner models are complete locally but overall partial pending ADR 0012; later product/execution/provider phases remain unimplemented unless explicitly identified

## Purpose

This document consolidates the documentation audit, product-boundary decision,
SSH design, multi-cloud design, extension model, security requirements,
performance requirements, Termix evaluation, reusable projects and libraries,
user experience, data model, and delivery plan discussed for Automexia.
The cross-feature command, leader-key, picker, workspace, file, log, and
collaboration experience that composes this architecture is specified in
[Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md).

The central decision is:

> Automexia core remains a fast, generic terminal. It provides PTYs, rendering,
> session isolation, generic extension APIs, capability enforcement, and hostile
> output protection. SSH, Kubernetes, OpenShift, cloud-provider, infrastructure,
> and AI orchestration behavior is delivered by separately enabled extensions.

This preserves a small and trustworthy terminal core while allowing Automexia
to become a particularly effective DevOps workstation when the relevant
first-party extension pack is installed.

## Executive decision

| Concern | Owner | Decision |
|---|---|---|
| A user types `ssh host` | Shell and PTY | Works with no extension, exactly like any other terminal command. |
| PTY, grid, input, scrollback, rendering, tabs, panes | Core terminal | Always core and provider-neutral. |
| Launching a trusted local program with exact arguments | Generic core capability broker | Exposed through a narrow `session.launch` capability; no shell-string evaluation. |
| Host list, tags, search, jump hosts, tunnels, quick connect | `devops-ssh` extension | First-party extension, separately enabled. |
| SSH private-key custody | Operating system agent, keychain, hardware token, or certificate authority | Automexia stores references and metadata, never raw private keys by default. |
| AWS, Azure, GCP, Kubernetes, OpenShift, and IaC context | Provider extensions | Separate first-party extensions with a common context contract. |
| Multi-cloud session isolation | Generic core session/context APIs plus provider extensions | Each PTY receives an immutable Environment Capsule; no window-global identity. |
| Cloud authentication | Official provider CLIs and identity systems | Prefer short-lived federated credentials; never invent another long-lived credential store. |
| Cloud-native remote access | Provider extensions | Prefer AWS SSM, Azure Bastion, and GCP IAP/OS Login over publicly exposed raw SSH. |
| Secrets | Provider caches, OS secret store, agents, or hardware | The renderer, logs, telemetry, and general configuration never receive secret values. |
| AI agents | AI orchestration extensions | Explicit, scoped tools only; they do not inherit a user's cloud or SSH authority. |
| Third-party extensions | Future sandboxed extension platform | Deny process, network, secret, and terminal-history access by default. |
| Termix | Reference or optional companion | Learn from its UX and data model; do not embed its Electron/Node application in the native Rust core. |

## Goals

The target system should let a DevOps user:

- open independent panes for production, staging, and development without
  credentials or context leaking between them;
- see exactly which identity, account, region, cluster, namespace, and
  infrastructure workspace belongs to each PTY;
- connect quickly through OpenSSH, jump hosts, tunnels, cloud bastions, or
  provider-native session services;
- use existing `~/.ssh/config`, `known_hosts`, `ssh-agent`, hardware keys,
  cloud CLI profiles, kubeconfigs, and organization identity policy;
- switch context by creating or explicitly rebinding a session, rather than
  silently mutating unrelated shells;
- recover quickly from expired credentials with visible, interactive login;
- work with multiple providers without Automexia pretending their identities
  and authorization models are interchangeable;
- keep terminal input and rendering responsive while discovery is slow,
  unavailable, or under load;
- enable only the DevOps features they want and disable the complete DevOps
  pack without weakening the terminal itself.

## Non-goals

Automexia is not:

- a cloud control plane;
- a replacement for AWS IAM Identity Center, Microsoft Entra ID, Google Cloud
  identity federation, Kubernetes authentication, or enterprise policy;
- a universal database of plaintext SSH and cloud credentials;
- a replacement for OpenSSH, `kubectl`, `oc`, `aws`, `az`, `gcloud`, Helm, or
  OpenTofu;
- a global context switcher that mutates all open shells at once;
- an infrastructure reconciler like Crossplane;
- an unrestricted command runner for downloaded extensions;
- an Electron application embedded inside the native WGPU terminal;
- a promise that provider context is an instantaneous live event stream.

## Current documentation audit

### What the project already defines correctly

The current documentation supplies several foundations required by this design:

1. Every pane-local tab owns an independent PTY, route, grid, input queue,
   history, launch descriptor, and extension state. This is the correct
   isolation boundary for cloud and environment identity. See
   [ADR 0007](adr/0007-pane-local-session-tabs.md).
2. Operational context is session-scoped and rendered beside the command that
   used it. A window-global cloud label would be false when panes point to
   different environments. See
   [ADR 0006](adr/0006-prompt-context-and-workspace-actions.md).
3. Extension work runs through bounded workers and cached snapshots. Rendering
   and PTY processing do not wait for extension I/O. See
   [ADR 0003](adr/0003-extension-capability-and-threading.md) and
   [Architecture](ARCHITECTURE.md).
4. Current DevOps discovery is local-only. It reads bounded configuration or
   invokes selected local CLIs; it does not contact a cloud or cluster API.
   See [Liquid Hacker UX](LIQUID-HACKER-UX.md#per-pane-operational-context).
5. The current extension contract is deliberately narrow: built-in,
   repository-reviewed, least-privilege local reads only. Arbitrary processes,
   network access, downloaded extensions, a public SDK, and Wasm sandboxing are
   outside v0.4.
6. The security roadmap treats every PTY producer as hostile, including
   SSH, containers, multiplexers, WSL, and local processes. Bounded
   OSC/APC/DCS/XTGETTCAP source hardening passed locally on 2026-08-14; hosted
   fuzz/sanitizer and native security evidence remain release blockers. See
   [Security debt](SECURITY-DEBT.md) and the
   [stabilization roadmap](STABILIZATION-ROADMAP.md#s0-bounded-control-strings).
7. The configuration root contains `config.toml`, `themes/`, `extensions/`, and
   `logs/`; it is not defined as a credential vault. That remains true.

### What the current documentation does not yet define

Before this document, the repository did not contain a complete strategy for:

- storing SSH hosts and opaque credential references;
- managing host keys, jump hosts, tunnels, certificates, or agent forwarding;
- connecting through AWS SSM, Azure Bastion, or GCP IAP;
- isolating multiple AWS, Azure, GCP, Kubernetes, and OpenShift identities;
- exposing provider-neutral status contributions from extensions;
- requesting reviewed process, browser-authentication, secret-reference, or
  network capabilities;
- signing, sandboxing, and distributing third-party DevOps extensions;
- protecting cloud access from AI orchestration extensions;
- measuring multi-cloud discovery latency and resource usage;
- deciding whether Termix should be embedded, reused, or treated as a reference.

This document fills the architectural design gap. It does not override current
accepted ADRs. Any implementation that introduces process or network access
must still pass the review and replacement-ADR requirements of ADR 0003.

## Product boundary

### Core terminal responsibilities

The core owns only generic terminal and extension-platform primitives:

- PTY creation, resize, input, output, grid, scrollback, selection, and search;
- OS windows, workspace tabs, split panes, and pane-local tabs;
- immutable launch descriptors and exact route/session ownership;
- rendering and a provider-neutral status/contribution model;
- generic session creation, cloning, rebinding, and program launch;
- extension lifecycle, signatures, compatibility, quotas, and cancellation;
- capability grant, denial, persistence, and audit metadata;
- safe inter-process communication with privileged first-party helpers;
- secure references to external secret or agent facilities;
- protection from untrusted terminal output;
- redaction, safe diagnostics, and resource ceilings.

The core must not contain AWS-, Azure-, GCP-, Kubernetes-, OpenShift-, Terraform-,
or SSH-specific business logic. It may understand generic concepts such as a
session, status segment, environment contribution, executable launch, secret
reference, authentication request, or remote transport.

### Extension responsibilities

Extensions own domain behavior:

- parsing provider profiles and configuration;
- detecting identities and environments;
- constructing exact CLI invocations;
- initiating official interactive authentication;
- providing status segments and context details;
- maintaining host and cluster inventories;
- opening tunnels and cloud-native remote sessions;
- optionally querying provider APIs after explicit permission;
- explaining stale, expired, unavailable, or conflicting state;
- implementing domain-specific commands, search, and workflows.

### Shared external-tool boundary

Provider ownership does not permit provider-specific process launch. System
OpenSSH, official cloud/Kubernetes CLIs, Mosh, Git, Upterm, SOPS/age, and later
external adapters all pass through one core-owned `ExternalToolRunner`. Its
request names the extension, operation, session, capsule, reviewed capability,
canonical executable, exact argv, validated working directory, and bounded
allowlisted environment. It provides protected/null stdin policy, startup/idle/
total deadlines, output byte and line caps, generation cancellation, descendant
termination, version compatibility, and redacted structured events.

Extensions construct typed requests and parse bounded versioned public results.
They do not call a shell, concatenate command text, spawn independently, retain
raw provider output without limits, or access an ambient environment. The
runner never executes on render, resize, terminal startup, or keystroke paths.
The complete split, technology matrix, and adoption order are in
[Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md).

### Why SSH is not core

Raw terminal use already supports the `ssh` executable. Making saved hosts,
key management, tunnels, SFTP, and cloud transports core would couple a generic
terminal to one remote-management domain, increase attack surface, and make the
terminal release cycle carry protocol-specific policy. These are valuable
features, but they belong in a separately enabled, first-party SSH extension.

Only the generic machinery required by many extension categories belongs in
core. `session.launch` is useful for SSH, databases, debuggers, container CLIs,
AI tools, and future workflows; therefore the capability is generic even when
`devops-ssh` is its first major user.

## Extension packaging

The recommended first-party packages are:

```text
Automexia Core
  generic terminal, sessions, renderer, extension runtime, capability broker

DevOps Pack (optional meta-package)
  devops-context
  devops-ssh
  devops-kubernetes
  devops-openshift
  devops-aws
  devops-azure
  devops-gcp
  devops-infrastructure

AI Pack (separate, optional)
  ai-agent-runtime
  ai-agent-orchestration
  provider-specific AI tool adapters, each requiring explicit grants
```

The DevOps Pack is a convenience bundle, not a monolith. Users can install or
enable only `devops-ssh` and `devops-kubernetes`, for example. Removing every
DevOps extension leaves a complete, normal terminal.

### Trust tiers

| Tier | Origin | Default authority |
|---|---|---|
| Core | Shipped terminal binary | Terminal primitives only. |
| First-party built-in | Repository-reviewed and built with the product | Current v0.4 local-read capabilities only. |
| First-party signed | Separately shipped by Automexia | Explicitly reviewed, narrowly scoped process/network/browser capabilities. |
| Enterprise-managed | Organization allowlist and signature policy | Capabilities bounded by administrator policy and extension manifest. |
| Third-party | Future public ecosystem | Sandboxed, no process, network, secret, clipboard, or terminal-history authority by default. |

An extension's UI visibility is not evidence of authority. Every sensitive
operation is checked at the time it occurs, against the exact session and
resource.

## Environment Capsule: the multi-cloud isolation unit

Each PTY has one immutable Environment Capsule created from explicit launch
intent. It is the portable description of the environment assigned to that
session. A capsule may contain:

```text
capsule_id
display_name
risk_classification                  development | test | staging | production

identity
  provider                           aws | azure | gcp | kubernetes | openshift | ...
  profile_or_configuration
  account_subscription_or_project
  tenant_or_organization
  role_or_principal_reference

location
  region
  zone

orchestrator
  kubeconfig_source_reference
  context
  cluster
  namespace

infrastructure
  working_directory
  tool                              opentofu | terraform | pulumi | ...
  backend_reference
  workspace

remote
  connection_reference
  transport                         ssh | aws-ssm | azure-bastion | gcp-iap

policy
  allowed_capabilities
  confirmation_rules
  organization_policy_reference

provenance
  creating_extension
  created_at
  source_revision
```

The capsule stores public identifiers and opaque references, never access
tokens, refresh tokens, private keys, passphrases, or recovered secret values.

### Capsule behavior

- A new pane, pane-local tab, or workspace tab gets a new PTY and a new capsule.
- Cloning copies the capsule's non-secret intent, then performs independent live
  discovery and credential resolution.
- An existing session is never silently rebound because another pane switches
  an account, subscription, project, cluster, or namespace.
- Explicit rebinding creates a visible transition, invalidates relevant cached
  context, and records non-secret audit metadata.
- Context displayed beside a historical prompt remains the snapshot used for
  that command, even after the live capsule changes.
- Production risk is a session property and receives stronger confirmation
  policy; color alone is never the sole warning.
- An unavailable provider must be shown as unavailable or stale, not silently
  omitted in a way that suggests the session is local or safe.

## Provider-neutral core APIs

The exact API will require an ADR and versioned schemas. The following contracts
define the intended boundary.

### Session API

```text
session.current() -> SessionDescriptor
session.create(LaunchRequest) -> SessionId
session.clone(SessionId, CloneOverrides) -> SessionId
session.rebind(SessionId, CapsuleReference) -> Result
session.launch(LaunchRequest) -> SessionId
session.cancel(OperationId) -> Result
```

`LaunchRequest` contains a normalized executable path or approved executable
identity, an argv array, bounded environment overrides, a validated working
directory, an optional capsule reference, and a terminal-interactive flag.
There is no shell command string.

The capability broker must preserve the platform's exact argument semantics,
reject NUL and malformed values, bound argument and environment size, validate
the working directory, and show an approval prompt when policy requires it.

### Context contribution API

Extensions contribute typed, provider-neutral facts:

```text
ContextContribution
  extension_id
  session_id
  kind                 identity | location | cluster | namespace | remote | risk | tool
  label
  value
  icon_token
  semantic_role
  freshness            fresh | refreshing | stale | expired | unavailable | error
  observed_at
  expires_at
  details_action
  source_revision
```

The renderer decides layout, contrast, truncation, and accessibility. An
extension cannot submit arbitrary GPU commands, fonts, hit targets, escape
sequences, or raw styled text. This generic `StatusSegment`/context contribution
model replaces provider-specific renderer conditionals over time.

### Authentication API

```text
authentication.request_interactive(AuthRequest) -> AuthOperation
authentication.open_browser(ApprovedOrigin, CallbackPolicy) -> AuthOperation
authentication.status(AuthOperation) -> AuthStatus
```

Interactive CLI login should normally run in a visible PTY so the user sees
exactly which provider is requesting input. Browser launch is limited to
manifest-declared HTTPS origins and reviewed loopback callback behavior. The
extension receives completion state or an opaque credential reference, not a
copy of browser cookies or tokens.

### Secret-reference API

```text
secret_reference.create(Provider, ExternalId, Metadata) -> SecretReference
secret_reference.resolve_for_operation(SecretReference, OperationId) -> sealed handle
secret_reference.delete(SecretReference) -> Result
```

A sealed handle can be passed only to an approved process or broker operation.
It cannot be read by renderer code or converted into a general string. Where a
provider CLI or SSH agent can resolve credentials itself, Automexia should not
resolve them at all.

### Capability manifest example

```toml
id = "io.automexia.devops.ssh"
publisher = "io.automexia"
trust = "first-party-signed"

[[capabilities.filesystem_read]]
paths = ["${ssh_config}", "${ssh_known_hosts}"]
follow_symlinks = false

[[capabilities.process_launch]]
executables = ["ssh", "ssh-add", "ssh-keygen"]
interactive = true
shell_evaluation = false

[[capabilities.secret_reference]]
providers = ["os-keychain", "ssh-agent", "pkcs11", "fido2"]
raw_export = false

[capabilities.network]
direct = false

[capabilities.terminal]
read_scrollback = false
write_input = "user-confirmed-operation"
```

Provider extensions declare their own exact executables, configuration paths,
approved authentication origins, and endpoints. Wildcard executable paths and
unrestricted direct network access are not valid defaults.

## SSH strategy

### Baseline: system OpenSSH

The recommended first implementation launches the operating system's OpenSSH
client in a normal Automexia PTY. This preserves mature support for:

- `~/.ssh/config` and included configuration;
- `known_hosts` and strict host-key checking;
- `ssh-agent` and platform agents;
- FIDO2 and smart-card keys;
- PKCS#11 providers;
- SSH certificates;
- `ProxyJump`, `ProxyCommand`, multiplexing, and forwarding;
- organization hardening and familiar diagnostics.

Automexia must not reimplement parsing by flattening every OpenSSH option into
its own partial model. The extension may index common fields for search and
display, but OpenSSH remains the execution authority.

### `devops-ssh` extension

The first-party SSH extension should provide:

- import and indexing of OpenSSH hosts;
- saved connection aliases that refer to OpenSSH configuration;
- fuzzy search, tags, folders, favorites, and recently used hosts;
- visible username, hostname, port, jump path, and transport summary;
- quick connect into a new pane, pane-local tab, workspace tab, or window;
- jump-host chains and bastion templates;
- local, remote, and dynamic tunnel lifecycle UI;
- host-key change warnings with fingerprint comparison;
- agent and certificate-expiry status;
- connection diagnostics that reveal effective public configuration without
  exposing key material;
- optional cloud inventory import;
- AWS SSM, Azure Bastion, and GCP IAP connection adapters;
- later, optional SFTP as a separate UI surface and capability.

SFTP should not be a prerequisite for safe SSH delivery. It introduces file
write, overwrite, permission, symlink, and large-transfer concerns and should
ship only after the connection and capability model is proven.

### Host inventory record

An Automexia connection record should contain only non-secret metadata and
references:

```text
connection_id
display_name
tags
source                              openssh-config | cloud-inventory | manual
host_alias
hostname
port
username
transport                           ssh | aws-ssm | azure-bastion | gcp-iap
jump_connection_references
identity_reference                  ssh-agent | certificate | hardware | external vault
known_host_policy_reference
environment_capsule_template
last_used_at
favorite
```

Raw private-key bytes and passphrases are deliberately absent.

### Key and certificate custody

Preferred order:

1. short-lived SSH certificates issued by an organization CA;
2. FIDO2, PIV, smart card, TPM, Secure Enclave, or another non-exportable key;
3. an OS- or organization-managed SSH agent;
4. an encrypted private-key file referenced from OpenSSH configuration;
5. an OS keychain entry exposed only as an opaque reference when integration is
   unavoidable.

Automexia does not create a second general-purpose vault. If future usability
requires importing a key, import should move the key into an approved external
store or agent, verify success, and discard all Automexia copies. Key material
must never enter general config, SQLite, logs, crash reports, telemetry,
renderer snapshots, clipboard history, or AI prompts.

`keyring-core` plus only the required platform stores can provide a Rust
abstraction over OS credential stores, while `secrecy` reduces accidental
secret exposure and `zeroize` clears supported in-memory buffers. These are
defense-in-depth tools, not justification for long-term in-process custody or a
broad default backend set.

### Host-key verification

- Strict host-key checking is on by default.
- First use presents the full fingerprint and provenance; a vague "trust?"
  prompt is insufficient.
- A changed host key blocks automatic connection and explains the old and new
  fingerprints, affected alias, and known-host source.
- Extensions cannot silently delete or replace `known_hosts` entries.
- Organization host certificates and DNS SSHFP may be supported when the
  underlying OpenSSH policy supports them.
- User overrides are explicit, narrowly scoped, and auditable.

### Agent forwarding and tunnels

- Agent forwarding is off by default because a compromised remote host can use
  the forwarded agent while the connection exists.
- Enabling forwarding is per connection or per invocation, visibly indicated,
  and may be forbidden by enterprise policy.
- Tunnel listeners default to loopback, use explicit ports, and show active
  lifetime and owning session.
- Closing the owning session terminates its non-shared tunnels unless the user
  explicitly created a managed background tunnel.
- Background tunnels have bounded restart policy, clear health state, and an
  explicit stop control; they never survive silently.

### Rust-native SSH engine

A Rust-native SSH transport is not the first implementation. It may later ship
as a separate first-party transport service if Automexia needs structured SFTP,
embedded port-forward control, or platforms without an adequate OpenSSH client.
It must not run in the renderer or VT parser process. The system OpenSSH path
remains available as the compatibility and recovery path.

## Multi-cloud strategy

### General rule: provider-native authentication

Automexia orchestrates official tools and surfaces their state; it does not
normalize providers into a shared long-lived token format. Prefer:

- human federation and short-lived sessions for interactive users;
- workload identity for automation;
- provider-native CLI profiles and token caches;
- Kubernetes exec credential plugins for cluster authentication;
- organization policy enforced at the provider or cluster, not only in local UI.

### AWS extension

Recommended identity path:

- AWS IAM Identity Center or another federated identity source;
- short-lived STS role sessions;
- named AWS CLI profiles;
- `aws sso login --profile <profile>` in a visible session when required;
- EKS authentication through the supported exec credential flow;
- EKS access entries and IAM policy for server-side authorization;
- Systems Manager Session Manager for managed-instance access where possible.

Per-session intent should set the smallest required environment, normally
`AWS_PROFILE`, `AWS_REGION`, and `AWS_DEFAULT_REGION` when applicable. Static
access keys are not copied into the capsule. The extension displays account,
role, region, expiry, and source without displaying credentials.

### Azure extension

Recommended identity path:

- Microsoft Entra interactive authentication with MFA for human users;
- workload identity federation for automation;
- explicit tenant and subscription selection;
- AKS authentication through supported Microsoft Entra and `kubelogin` flows;
- Azure Bastion for administrative remote access when available.

Avoid relying on one mutable, global `az account set` state across many panes.
Use an identity-scoped `AZURE_CONFIG_DIR` when isolation requires it and pass
`--subscription` explicitly for sensitive operations. If Automexia provides a
strict wrapper, it must show the target subscription and reject ambiguity; it
must not silently rewrite arbitrary commands typed by the user.

### Google Cloud extension

Recommended identity path:

- Workforce Identity Federation for human workforce access;
- Workload Identity Federation for workloads;
- named `gcloud` configurations;
- `CLOUDSDK_ACTIVE_CONFIG_NAME` per session;
- the supported GKE authentication plugin;
- Identity-Aware Proxy TCP forwarding and OS Login for remote access.

Service-account keys should be avoided. Where an unavoidable legacy key exists,
Automexia references the provider-managed location and does not import the key.
The extension displays organization/project, principal, configuration, region
or zone, and token freshness without revealing token values.

### Kubernetes extension

Kubernetes context combines configuration, cluster, user, context, and
namespace. The extension should:

- parse kubeconfig locally and preserve multiple source files;
- create a per-session `KUBECONFIG` overlay instead of mutating the user's
  default current context globally;
- expose context, cluster, user reference, namespace, source, and freshness;
- use exec credential plugins exactly as specified by kubeconfig;
- require an allowlist or confirmation before executing an unfamiliar
  credential plugin;
- keep `ExecCredential` results in memory or the provider's supported cache;
- align Helm and related tools with the session's selected kubeconfig/context;
- represent cluster connectivity independently from local configuration state.

A kubeconfig is active content, not harmless data: it may cause execution of a
credential plugin. Imported or downloaded kubeconfigs must be treated as
untrusted and reviewed before execution.

### OpenShift extension

OpenShift uses Kubernetes-compatible configuration but adds domain workflows.
The extension should build on the Kubernetes extension and provide:

- `oc` discovery and context;
- explicit cluster/API endpoint and project display;
- `oc login` or `oc login --web` in a visible interactive flow;
- OpenShift project switching scoped to the session;
- route, build, deployment, and operator actions only after explicit network
  capability is granted;
- the same exec-plugin, certificate, and kubeconfig protections as Kubernetes.

Browser-login loopback callbacks require review and must not expose a listener
beyond the intended local interface.

### Infrastructure extension

OpenTofu/Terraform workspace is only one dimension of environment. Workspaces
must not be treated as a security boundary or as a replacement for separate
credentials and access controls. The extension should display:

- tool and version;
- working directory;
- backend type and public identifier;
- workspace;
- provider set;
- plan/apply risk and production classification;
- whether state encryption and remote locking are expected.

Avoid setting `TF_WORKSPACE` implicitly for an unrelated directory. Prefer
explicit working-directory and workspace selection recorded in the capsule.

## Connection flow

The standard quick-connect flow is:

```text
User chooses a saved environment or remote target
  -> extension resolves public configuration and opaque references
  -> capability broker validates extension, executable, argv, env, cwd, policy
  -> missing/expired identity starts visible provider-native authentication
  -> core creates a new PTY and immutable Environment Capsule
  -> system OpenSSH or provider CLI runs inside that PTY
  -> asynchronous discovery publishes a session-scoped context snapshot
  -> renderer displays fresh/stale/error state without blocking input
  -> audit records non-secret operation metadata
```

There is no stage where the renderer receives a private key or access token.

### Recommended remote transport order

Use the most identity-aware option supported by the target:

1. AWS Systems Manager Session Manager for eligible AWS instances;
2. Azure Bastion for protected Azure virtual machines;
3. GCP IAP TCP forwarding combined with OS Login;
4. OpenSSH with short-lived certificates and a bastion/jump host;
5. direct OpenSSH only where network policy explicitly allows it.

This order reduces public inbound ports and long-lived key distribution. Raw
SSH remains essential for generic systems and as a transparent fallback.

## Broker-process architecture

Network- and credential-adjacent first-party adapters should run outside the
renderer/VT process in a small provider broker or extension host when the
protected isolation gate requires it. In-process trusted adapters still use the
same application-owned ExternalToolRunner and receive no direct process
authority. The broker:

- communicates over a user-scoped named pipe on Windows or Unix-domain socket;
- never opens a general TCP control port;
- authenticates the local client and binds requests to a session and extension;
- accepts versioned typed messages, not arbitrary commands;
- validates exact executable, argv, environment, directory, endpoint, and size;
- applies deadlines, cancellation, concurrency limits, and output caps;
- redacts diagnostics before returning them;
- returns structured public metadata or connects an approved byte stream to a
  PTY, rather than returning raw credentials;
- drops privilege where supported and runs with the user's normal authority;
- terminates or revokes operations when the owning session or extension ends.

The broker is not a privileged daemon and does not bypass provider policy. It
exists to keep complex adapters and secret-adjacent operations away from the
rendering and terminal parser trust boundary.

## Security model

### Threats in scope

- malicious or compromised extensions;
- hostile PTY output from local or remote programs;
- malicious kubeconfig exec plugins;
- command/argument injection;
- leaked SSH keys, cloud tokens, passphrases, or environment variables;
- cross-pane identity confusion;
- use of a production session mistaken for development;
- changed SSH host keys and man-in-the-middle attacks;
- compromised jump hosts and forwarded agents;
- unrestricted browser callbacks or local IPC;
- excessive discovery processes, API requests, output, memory, or retries;
- secret capture by logs, crash reports, telemetry, clipboard, or AI agents;
- dependency and extension supply-chain compromise.

### Mandatory controls

1. Preserve the closed local bounded-control-string gate and require hosted
   fuzz/sanitizer plus native evidence before advertising remote sessions. A
   remote process remains an untrusted PTY producer.
2. Use exact argv arrays and approved executables. Never construct
   `sh -c`, `cmd /c`, or PowerShell command strings from extension data.
3. Deny network and process access by default. Grants are extension-specific,
   resource-specific, visible, revocable, and checked per operation.
4. Keep credentials in official provider caches, OS stores, agents, hardware,
   or short-lived broker handles. Do not place secrets in Automexia config.
5. Partition all caches and results by session, extension, provider identity,
   and source revision.
6. Treat config files as untrusted input: cap size and nesting, reject unsafe
   paths, and gate executable credential plugins.
7. Require strict SSH host-key verification and make changes blocking.
8. Keep agent forwarding disabled by default.
9. Protect local IPC with user-only permissions, peer identity checks,
   unguessable session binding, schema validation, and message-size caps.
10. Log only metadata: extension, action type, public target identifier,
    timestamp, result class, policy decision, and duration. Do not log commands
    typed by the user, terminal contents, environment, secrets, or tokens.
11. Sign first-party extensions and verify publisher, package hash, manifest,
    compatibility, and revocation status before loading.
12. Apply memory, CPU, process, file, network, retry, and output quotas.

### Frozen D0/D3 local baseline

The active schema-2 D0/D3 contract now makes controls 2, 3, 7, 8, 10,
and 11 machine-checkable without granting runtime authority; schema 1 remains
immutable historical evidence. It preserves interactive-shell ownership of
manual SSH, forbids download/install or substitution during startup and launch,
and binds `automexia.devops-ssh` to an exact publisher, trusted-loader SHA-256
identity source/size, workspace version, contract version, and reviewed/signed
verification. Unverified or mismatched principals fail closed. Capability
decisions bind exact operation/session/capsule/resource scope and expiry.

Nine boundary rows fix accepted/returned data, limits, cancellation, logging,
and failure behavior. Windows, macOS, and Linux use fixed system roots, WSL
remains disabled, and no resolver searches PATH or cwd. The hermetic native
protocol fixes loopback-only setup, isolated disposable authentication state,
bounded probes/timeouts, DNS/connect/auth cancellation, cleanup invariants,
evidence fields, redaction surfaces, and WSL's deny-until-native-gate behavior.
Strict host trust is preserved; forwarding and remote commands default off.
Production process, PTY, network, provider, authentication, key-custody, and
renderer authority remain false until ADR 0012 is accepted and F4/F5 pass.

### Local and server-side policy

Local policy can prevent dangerous launches, require confirmation, restrict
extensions, and label production. Cedar is a strong embedded Rust candidate
for deterministic authorization decisions. OPA is appropriate where an
organization already manages Rego policy and a separate policy service.

Local policy is defense in depth. The authoritative enforcement of cloud,
cluster, host, and infrastructure permissions remains IAM, Entra, Google Cloud
IAM, Kubernetes/OpenShift RBAC, SSH CA policy, network controls, and remote
service policy.

### AI extension isolation

An AI extension must not inherit the user's PTY environment, agent socket,
cloud CLI cache, SSH connection, capsule, or terminal history merely because it
runs in the same application. Each AI tool call requires:

- an explicit tool definition;
- a target session or environment selected by the user;
- the minimum capability for that operation;
- structured arguments and a preview for destructive operations;
- production-aware confirmation and organization policy;
- bounded output with secret redaction;
- a result audit record that excludes prompt and credential contents.

Approval to read context is not approval to run commands. Approval to run
`kubectl get` is not approval to run `kubectl delete`, and approval in staging
is not approval in production.

## Performance model

### Non-negotiable invariants

- Rendering, input delivery, PTY parsing, and resize never wait for extension,
  provider, filesystem, credential, or network I/O.
- The renderer consumes immutable, bounded snapshots through non-blocking reads.
- Every result is routed to the exact session that requested it.
- Slow, hung, rate-limited, or missing provider tools degrade context quality,
  not terminal responsiveness.
- Refresh failure preserves the last known-good snapshot and marks it stale or
  errored; it does not replace truth with an empty default.

### Discovery strategy

Use the least expensive trustworthy source in this order:

1. immutable launch and capsule metadata;
2. parsed local configuration;
3. official local CLI cache or status command;
4. lazy provider API query after explicit network permission;
5. user-requested deep inventory refresh.

Use filesystem watchers only for reliable files and retain periodic
reconciliation because providers and CLIs do not offer a universal event
stream. Apply:

- stale-while-revalidate snapshots;
- request coalescing/singleflight for identical lookups;
- shared parsed-file caches with session-specific views;
- bounded worker queues and provider-specific concurrency limits;
- exponential backoff with jitter for transient failures;
- negative caching for missing tools and unavailable profiles;
- cancellation when a session closes or its capsule revision changes;
- lazy loading of inventories and details;
- capped output, parse size, entry count, and recursion;
- separate cold-start, warm-start, and refresh measurements.

### Suggested budgets

The values below are initial engineering targets to validate through the
project's benchmark process, not current guarantees:

| Operation | Target behavior |
|---|---|
| Render/input path | Zero provider I/O and zero waits on extension workers. |
| Capsule seed | Available for first frame from immutable launch data. |
| Cached status lookup | Sub-millisecond typical local lookup. |
| Local config refresh | Background, bounded, and cancellable. |
| Individual optional CLI probe | One-second ceiling unless user initiated. |
| Provider API inventory | Lazy, visibly refreshing, independently cancellable. |
| Broker response payload | Schema- and size-bounded; large streams go to PTY or paged storage. |
| Memory | Bounded per extension/session with eviction and observable limits. |

Measure first editable prompt, first context, credential refresh, quick-connect
latency, tunnel startup, cold/warm config parsing, worker saturation, idle CPU,
memory per 10/50/100 sessions, and rendering/input latency while provider tools
are deliberately slow.

## User experience

The complete product contract for the window-level inventory, first-run scan,
authentication lifecycle, Connection Review, capability approval/revocation,
provider journeys, responsive/accessibility behavior, and renderer-neutral
goldens is [Connection Hub](CONNECTION-HUB.md). This section describes the
surrounding contribution points; it must not be implemented as separate
provider-specific dialogs.

### Command palette

Generic contribution points let extensions add commands such as:

- `SSH: Connect to Host...`
- `SSH: Open Tunnel...`
- `Environment: Open in New Pane...`
- `Kubernetes: Select Context for New Session...`
- `OpenShift: Log In...`
- `AWS: Log In to Profile...`
- `Azure: Open Subscription Session...`
- `Google Cloud: Open Configuration Session...`
- `Environment: Explain Active Context`
- `Environment: Refresh Context`

Sensitive actions show the target and effect before launch. A quick-connect
result should indicate transport, user, host, environment, and production risk,
not just a friendly alias.

Command completion and reusable DevOps commands use the separate
[Command Productivity](COMMAND-PRODUCTIVITY.md) contract. The palette consumes
the same typed Quick Action model; it does not turn display labels into shell
text. Actions insert for review by default, built-in packs enable no short alias
by default, and an exact launch crosses only the reviewed D3 broker. Native shell
editors remain completion owners. Provider-aware candidates appear only after
D6, from bounded cached public capsule data with freshness; opening a palette or
typing a key never calls a provider, authenticates, reads a credential cache, or
executes a plugin.

### Per-pane context

The existing per-command context design remains the right UI. Provider
extensions contribute segments, while core controls visual ordering,
accessibility, width, contrast, and stale/error treatment. A typical pane may
show:

```text
PRODUCTION | AWS 123456789012/Admin | eu-west-1 | EKS payments-prod | ns/api
```

Another pane in the same window may independently show:

```text
STAGING | Azure Contoso/Payments-Stage | westeurope | AKS stage-01 | ns/api
```

The context details action explains provenance, freshness, expiry, configuration
source, and which extension supplied each value. It never reveals secrets.

### Failure behavior

- Expired identity: keep the terminal usable, mark identity expired, and offer
  a visible provider-native login.
- Missing CLI: explain which official tool is required; never download or run it
  without a separate explicit installation workflow.
- Conflicting profiles: show the conflict and require selection.
- Slow provider: keep cached facts and show refreshing/stale state.
- Invalid kubeconfig: show the source and parse problem; do not execute plugins.
- Changed SSH host key: block connection and show fingerprints.
- Extension crash: terminate its operations, preserve the PTY/core, and show a
  bounded diagnostic.
- Offline system: local terminal and cached metadata remain available; network
  actions fail quickly and truthfully.

## Termix evaluation

Termix is a useful open-source reference, but it is not the right foundation for
Automexia core.

At the time of this research, Termix is an Apache-2.0 React/Node/Electron
application built around `ssh2` and xterm.js. It includes SSH, RDP, VNC,
Telnet, tunnels, SFTP, saved hosts and credentials, jump hosts, agent
forwarding, Vault/Bitwarden integration, RBAC, shared sessions, and an API. Its
latest stable release observed during the research was 2.6.1 on 2026-08-06.

### Good uses of Termix

- study its host inventory, search, grouping, quick-connect, tunnel, and SFTP UX;
- study import/export and external-vault integration expectations;
- test an optional companion integration through a stable API if users already
  operate a Termix server;
- compare workflows and feature coverage;
- reuse a clearly separable Apache-2.0 algorithm or data-model idea only after
  license, security, dependency, and architecture review.

### Why not embed it

- It would add an Electron/Node/React/xterm.js application stack beside a native
  Rust/WGPU terminal and duplicate Automexia's renderer and session model.
- Its `ssh2` execution path would bypass the deliberate system-OpenSSH strategy.
- It has a broad server/API/credential surface that is different from a small
  terminal extension boundary.
- Its API reference was still immature during review, and its release cadence
  would become a second application lifecycle inside Automexia.
- A patched high-severity stored-XSS advisory demonstrated that web/Electron
  rendering bugs in a connection manager can become local file disclosure.
  The project fixed the issue, but the incident illustrates the additional
  trust boundary an embedded web application would create.

### Recommendation

Do not fork or embed Termix as Automexia's SSH engine. Use it as a product and
UX reference. If demand exists, build an optional `termix-bridge` extension
that imports non-secret host metadata or calls a versioned Termix API. That
bridge must be optional, use explicit network permission, and never make Termix
credentials part of Automexia core.

## Reusable libraries and projects

| Project | Recommended use | Boundary/caution |
|---|---|---|
| System OpenSSH | Primary SSH execution engine | Launch with exact argv in a PTY; preserve native config and policy. |
| `keyring-core` plus exact stores | Optional OS secret-store abstraction | Prefer opaque references and agents; enable no unused platform/backend surface. |
| `secrecy` | Reduce accidental secret formatting/logging in Rust | Does not solve storage, access control, or compromise by itself. |
| `zeroize` | Clear supported in-memory secret buffers | Copies and OS/provider caches still require careful design. |
| `kube-rs` | Later structured Kubernetes inventory and watches | Use only in a provider extension/broker; CLI/config-first is simpler initially. |
| Kubie | Design reference for per-shell Kubernetes isolation | Useful model; do not make its future availability a core dependency. |
| Official AWS SDK for Rust | Later AWS inventory/API operations | CLI-first provides faster authentication compatibility and smaller initial scope. |
| Azure SDK for Rust | Later Azure inventory/API operations | It was pre-1.0 during research; isolate it behind an adapter. |
| Official Google Cloud Rust libraries | Later GCP inventory/API operations | Keep provider-specific dependencies out of core. |
| Cedar | Embedded local authorization engine | Strong fit for typed, deterministic local policy. |
| OPA | Enterprise policy integration | Prefer when the organization already operates Rego/OPA. |
| Teleport | External short-lived SSH/Kubernetes access platform | Integrate with its CLI/certificates; do not reproduce the control plane. |
| OpenBao | External SSH certificate authority and secret platform | Reference or integrate; Automexia remains a client, not a vault. |
| OpenTofu | IaC execution/context | Workspaces are not credential or authorization isolation. |
| Steampipe | Optional multi-cloud query/inventory inspiration | Heavy query engine; use as external integration, not terminal core. |
| Crossplane | External multi-cloud control-plane integration | It reconciles infrastructure; Automexia only launches/observes workflows. |
| Termix | UX/reference or optional bridge | Do not embed its Electron/Node application or make it core. |

The principle is to reuse mature identity, protocol, and control-plane systems
at their supported interfaces. A dependency belongs in core only if it is
provider-neutral and necessary for terminal or capability integrity.

## Delivery plan

The optimal early-release sequence is to keep v0.4 focused on core stability,
perform non-activated contract/threat-model work in parallel, and make v0.5.0
the first recommended DevOps-ready release with production system-OpenSSH
support. The v0.5.0 SSH work does not wait for v0.6's public extension sandbox.
The step-by-step execution authority is the
[early DevOps and SSH delivery track](STABILIZATION-ROADMAP.md#early-devops-and-ssh-delivery-track);
[ADR 0012](adr/0012-first-party-ssh-and-session-launch-boundary.md) must be
accepted before the new launch capability is enabled.

### Phase 0: v0.4 security and stability

- Preserve the completed bounded OSC/APC/DCS/XTGETTCAP caps, cancellation,
  discard, diagnostics, recovery tests, and nightly fuzz target.
- Preserve the existing local-only, reviewed extension boundary.
- Do not advertise managed remote sessions before hosted hostile-output and
  native security assurance pass.
- Keep provider-specific status behavior stable while APIs are designed.

Source criterion: passed locally. The step-by-step implementation and evidence
classification is maintained in the
[Phase 0 execution ledger](STABILIZATION-ROADMAP.md#phase-0-execution-ledger):
all four source obligations are preserved, while controlled AppVerifier/WPR,
Linux/macOS native GPU, hosted hostile-output, assistive-technology, benchmark,
and elapsed-baseline results remain explicit gates. Release criterion: every
remaining stabilization/hosted/native gate passes and hostile remote PTY output
remains bounded and recoverable.

### Phase 1: v0.5.0 provider-neutral internal APIs

- Extract the release-critical private extension API/runtime, DevOps, and
  UI-model crates; do not delay SSH for unrelated engine-directory movement.
- Define versioned `Session`, `LaunchRequest`, `EnvironmentCapsule`,
  `ContextContribution`, `StatusSegment`, and `Freshness` schemas.
- Replace provider-specific renderer branching with generic segment rendering.
- Move current local discovery behind internal first-party context providers
  without adding new network authority.
- Add session/cache isolation, stale-state, cancellation, saturation,
  accessibility, and performance tests.

Exit criterion: the renderer has no provider-specific dependency and existing
local context behavior is equivalent or better.

Phase 1 implementation status (2026-08-14): source-complete and locally
verified.

- The four release-critical crates are private workspace members. The API and
  UI-model crates have no frontend, renderer, GPU, PTY, window, or provider SDK
  dependency; the runtime receives exact-route wake behavior through an injected
  one-shot trait.
- Version 1 contracts reject unsupported versions, unknown fields, NUL data,
  invalid public controls, oversized text, oversized segment/provider/argument/
  environment/reference collections, and session/capsule mismatches during both
  construction and deserialization. Launch diagnostics redact arguments and
  secret-reference identifiers; environment values never enter the serialized
  launch contract.
- The current local provider moved intact behind `LocalContextProvider` and a
  golden pins every generic segment field. Its manifest adds no network,
  clipboard, or process authority.
- The renderer consumes generic `ContextContribution`/`StatusSegment` values;
  shared UI policy owns priority, responsive hiding/restoration, grapheme-safe
  labels, semantic colors, contrast, accessibility text, hit testing, and typed
  details routing.
- Per-session capsules, full cache keys, registration-before-dispatch ordering,
  bounded non-blocking submission, stale-operation cancellation, rebind cache
  invalidation, last-truth preservation, coalescing, and exact-route wakes have
  deterministic and Loom regressions. Clone tests prove new session/capsule
  identity and independent PTY ownership.
- A validated rebind plan distinguishes metadata-only changes from changes that
  require a fresh session. The user-facing capability broker and managed relaunch
  action remain deliberately inactive until Phase 2; Phase 1 grants no new
  process authority.

### Phase 2: v0.5.0 production first-party SSH

- Accept the proposed replacement ADR required by ADR 0003.
- Implement the exact-argv `session.launch` broker and capability UI.
- Ship `devops-ssh` initially as an OpenSSH config index and launcher.
- Add strict host-key UX, agent/certificate status, jump hosts, and tunnels.
- Keep raw key import and direct extension networking out of the first release.

Phase 2 implementation status (2026-08-17): **Partially done**. The disabled D4
inventory and the complete local F2/D5.0 capability-free baseline now provide
strict connection/profile/recipe/review/receipt/plan schemas, exhaustive auth
and result reducers, deterministic dry-run fingerprints, pure responsive Hub/
review/planner projections, all-provider/state/layout/accessibility fixtures,
and fuzz/mutation/benchmark ownership. They expose no filesystem, process,
network, provider, credential, PTY, listener, window, renderer, or GPU authority.
ADR 0012 acceptance, capability UI, D5.1 product integration/persistence, and
D5.2 OpenSSH launch/lifecycle/native evidence are not implemented.
Exit criterion for the first recommended DevOps-ready release: security review,
protected-path approvals, injection tests, cross-platform native SSH tests,
cancellation, cleanup, secret redaction, and resource limits pass while the
generic terminal remains complete with `devops-ssh` disabled.

The parallel CP0-CP3 command-productivity work may deliver native completion
health, typed persistent Quick Actions, optional aliases, and static DevOps packs
without waiting for provider APIs. SSH actions may reference only the bounded D4
public inventory and remain insert-only until D3 activation. CP4 provider-aware
actions wait for Phase 3 capsule isolation. The detailed ordering and gates are
in the
[command productivity delivery track](STABILIZATION-ROADMAP.md#command-productivity-delivery-track).

### Phase 3: v0.5.1 provider-native authentication and capsules

- Ship AWS, Azure, GCP, Kubernetes, and OpenShift extensions separately.
- Add per-session configuration isolation and explicit authentication flows.
- Implement Environment Capsule templates and safe clone/rebind behavior.
- Add exec-plugin allowlisting for Kubernetes.
- Add cloud-native remote transports: SSM, Bastion, and IAP/OS Login.

Exit criterion: concurrent production/staging/development sessions cannot
observe or mutate each other's identity, config, caches, or results.

### Phase 4: v0.5.1 follow-on lazy inventory and infrastructure workflows

- Add optional provider API access behind explicit endpoint/network grants.
- Add `kube-rs` or official SDK adapters only where structured APIs materially
  improve inventory or watch behavior.
- Add infrastructure context, plan review, risk classification, and policy.
- Add optional Steampipe, Teleport, OpenBao, or enterprise broker integrations.

Exit criterion: API unavailability or saturation has no measurable impact on
terminal input/render latency and all inventories expose freshness.

### Phase 5: v0.6 third-party ecosystem and AI tools

- Complete signed distribution, revocation, compatibility, sandboxing, quotas,
  and permission UX before a public SDK.
- Keep process/network/secret access denied by default.
- Introduce AI tools only through scoped, structured capabilities and
  environment-specific approval.
- Add organization policy and redacted audit export.

Exit criterion: a malicious or crashed third-party/AI extension cannot obtain
ambient terminal, credential, process, filesystem, or network authority.

## Testing and acceptance matrix

| Area | Required evidence |
|---|---|
| Session isolation | Parallel AWS/Azure/GCP/Kubernetes/OpenShift sessions retain exact capsule, environment, routes, caches, and results. |
| Argument safety | Property tests and platform-native tests prove exact argv with hostile aliases, paths, usernames, and Unicode. |
| Secret safety | Tests prove secrets are absent from logs, snapshots, crash bundles, telemetry, config, IPC diagnostics, and AI tool inputs. |
| SSH security | First-use, known host, changed key, certificate, agent, hardware key, jump host, and forwarding policies work on every supported OS. |
| Kubernetes security | Malicious/unknown exec plugins are blocked or confirmed; config sizes and paths are bounded. |
| Provider authentication | Interactive, expired, cancelled, offline, MFA, multiple-identity, and token-refresh paths are tested without token disclosure. |
| Capability policy | Deny, allow-once, persisted grant, revocation, publisher change, manifest change, and enterprise override are deterministic. |
| Command productivity | Native shell completion remains functional with Automexia integration enabled or disabled; typed actions, aliases, scope/precedence, quoting, insertion-without-Enter, secret-negative behavior, and uninstall residue pass on supported shells/OSes. |
| Performance | Slow/hung CLIs and APIs do not affect input, rendering, PTY parsing, resize, or unrelated sessions. |
| Resource lifecycle | Sessions, broker operations, tunnels, processes, handles, sockets, tasks, and caches terminate cleanly. |
| Output hardening | Remote hostile control-string and graphics corpora obey memory/time caps and deterministic recovery. |
| Accessibility | Context freshness, risk, approvals, host-key changes, errors, and tunnel state are conveyed without color-only meaning. |
| Supply chain | Signed package, manifest, hash, publisher, SBOM, advisories, revocation, and compatibility are verified. |

## Architecture rules for review

A proposed SSH, cloud, Kubernetes, OpenShift, infrastructure, or AI feature is
acceptable only if all of the following remain true:

1. Automexia without extensions is still a complete generic terminal.
2. Core contains no provider-specific policy or SDK dependency.
3. Each PTY owns its own environment identity and extension state.
4. The renderer never blocks on extension, process, credential, or network I/O.
5. Secret values do not cross into core UI or persistent general storage.
6. Process launches use exact executable and argv contracts.
7. Network and executable access is denied by default and narrowly granted.
8. Provider-native identity and server-side authorization remain authoritative.
9. Remote output is treated as hostile.
10. AI agents receive no ambient user authority.
11. Failures are truthful, cancellable, bounded, and isolated.
12. Implementation follows a reviewed ADR when it expands current capabilities.

If a feature violates one of these rules, it should be redesigned as an
extension, broker operation, external-system integration, or explicit future
capability rather than added directly to the terminal core.

## Primary references

### Automexia

- [Architecture](ARCHITECTURE.md)
- [Liquid Hacker UX](LIQUID-HACKER-UX.md)
- [Configuration](CONFIGURATION.md)
- [Roadmap](ROADMAP.md)
- [Stabilization roadmap](STABILIZATION-ROADMAP.md)
- [Command Productivity](COMMAND-PRODUCTIVITY.md)
- [Security debt](SECURITY-DEBT.md)
- [ADR 0003: extension capability and threading boundary](adr/0003-extension-capability-and-threading.md)
- [ADR 0006: prompt-owned context](adr/0006-prompt-context-and-workspace-actions.md)
- [ADR 0007: pane-local independent sessions](adr/0007-pane-local-session-tabs.md)
- [ADR 0012: proposed first-party SSH and scoped session launch](adr/0012-first-party-ssh-and-session-launch-boundary.md)
- [ADR 0015: shell-native completion and typed Quick Actions](adr/0015-shell-native-completion-and-typed-quick-actions.md)

### SSH and external access

- [Termix repository](https://github.com/termix-ssh/termix)
- [Termix releases](https://github.com/Termix-SSH/Termix/releases)
- [Termix security model](https://docs.termix.site/features/authentication/security/)
- [Termix API keys](https://docs.termix.site/features/api/api-keys/)
- [Termix security advisory GHSA-m3cv-5hgp-hv35](https://github.com/Termix-SSH/Termix/security/advisories/GHSA-m3cv-5hgp-hv35)
- [AWS Systems Manager Session Manager](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager.html)
- [Azure administrative access and Bastion](https://learn.microsoft.com/en-us/azure/networking/design-guide/developer-admin-access)
- [Google Cloud IAP TCP forwarding](https://docs.cloud.google.com/iap/docs/using-tcp-forwarding)
- [Google Cloud OS Login](https://docs.cloud.google.com/compute/docs/oslogin)
- [Teleport authentication architecture](https://goteleport.com/docs/reference/architecture/authentication/)
- [OpenBao signed SSH certificates](https://openbao.org/docs/next/secrets/ssh/signed-ssh-certificates/)

### Kubernetes and OpenShift

- [Kubernetes: configure access to multiple clusters](https://kubernetes.io/docs/tasks/access-application-cluster/configure-access-multiple-clusters/)
- [Kubernetes authentication and exec credential plugins](https://kubernetes.io/docs/reference/access-authn-authz/authentication/)
- [Kubeconfig v1 API](https://kubernetes.io/docs/reference/config-api/kubeconfig.v1/)
- [ExecCredential v1 API](https://kubernetes.io/docs/reference/config-api/client-authentication.v1/)
- [Kubernetes v1.35 credential-plugin allowlist](https://kubernetes.io/blog/2026/01/09/kubernetes-v1-35-kuberc-credential-plugin-allowlist/)
- [OpenShift CLI documentation](https://docs.redhat.com/en/documentation/openshift_container_platform/4.20/html/cli_tools/openshift-cli-oc)
- [`kube-rs`](https://github.com/kube-rs/kube)
- [Kubie](https://docs.rs/crate/kubie/latest)

### AWS

- [AWS IAM security best practices](https://docs.aws.amazon.com/IAM/latest/UserGuide/best-practices.html)
- [AWS CLI IAM Identity Center configuration](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)
- [AWS CLI configuration and profiles](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html)
- [AWS CLI environment variables](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html)
- [Amazon EKS kubeconfig](https://docs.aws.amazon.com/eks/latest/userguide/create-kubeconfig.html)
- [Amazon EKS access entries](https://docs.aws.amazon.com/eks/latest/userguide/access-entries.html)
- [AWS process credential provider](https://docs.aws.amazon.com/sdkref/latest/guide/feature-process-credentials.html)
- [AWS SDK for Rust](https://docs.aws.amazon.com/sdk-for-rust/)

### Azure

- [Authenticate with Azure CLI](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli)
- [Azure CLI interactive authentication](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli-interactively)
- [Manage Azure subscriptions with Azure CLI](https://learn.microsoft.com/en-us/cli/azure/manage-azure-subscriptions-azure-cli)
- [Azure CLI configuration and `AZURE_CONFIG_DIR`](https://learn.microsoft.com/en-au/cli/azure/azure-cli-configuration)
- [AKS Microsoft Entra authentication](https://learn.microsoft.com/en-gb/azure/aks/entra-id-control-plane-authentication)
- [Workload identity federation considerations](https://learn.microsoft.com/en-us/entra/workload-id/workload-identity-federation-considerations)
- [Azure SDK for Rust](https://github.com/Azure/azure-sdk-for-rust)

### Google Cloud

- [`gcloud` named configurations](https://docs.cloud.google.com/sdk/gcloud/reference/config/configurations)
- [Workforce Identity Federation](https://docs.cloud.google.com/iam/docs/workforce-identity-federation)
- [Workload Identity Federation best practices](https://docs.cloud.google.com/iam/docs/best-practices-for-using-workload-identity-federation)
- [Service account best practices](https://docs.cloud.google.com/iam/docs/best-practices-service-accounts)
- [GKE cluster authentication](https://docs.cloud.google.com/kubernetes-engine/docs/how-to/cluster-access-for-kubectl)
- [Google Cloud Rust libraries](https://docs.cloud.google.com/rust/docs/reference)

### Policy, secrets, infrastructure, and inventory

- [`keyring-rs`](https://github.com/open-source-cooperative/keyring-rs)
- [`secrecy`](https://docs.rs/secrecy/latest/secrecy/)
- [`zeroize`](https://docs.rs/zeroize/latest/zeroize/)
- [Cedar policy](https://github.com/cedar-policy/cedar)
- [Open Policy Agent](https://www.openpolicyagent.org/docs)
- [OpenTofu providers](https://opentofu.org/docs/v1.11/language/providers/)
- [OpenTofu workspaces](https://opentofu.org/docs/language/state/workspaces/)
- [OpenTofu state encryption](https://opentofu.org/docs/v1.10/language/state/encryption/)
- [OpenTofu CLI environment variables](https://opentofu.org/docs/cli/config/environment-variables/)
- [Steampipe](https://github.com/turbot/steampipe)
- [Crossplane providers](https://docs.crossplane.io/latest/packages/providers/)
