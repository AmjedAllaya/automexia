
**Repository-alignment preface (2026-08-23).**

This hardening plan is a candidate input for the next execution phase. Interpret
its checklist as a delta against audited committed baseline
`20ff7928ea2d1eac5d26f13c62d0cda9b86bc078`, not as proof that listed source work is absent.

Current audit indicates substantial provider/Ghostty source already exists. The priority is activation/native/security/accessibility/signing/packaging/resource evidence.

Do not use this plan to:
- force a greenfield process topology;
- force public Wasmtime/WIT;
- force new crate splits;
- force candidate dependency upgrades.

Use it to convert existing source-complete work into releasable behavior.

See [`ACTIVATION_HARDENING_DELTA.md`](../repository_integration/ACTIVATION_HARDENING_DELTA.md) for the pinned-baseline priority summary.

---

# Automexia Terminal — Current Version Hardening, Fixes, and Improvement Plan

**Document type:** Current-version architecture and activation-readiness plan
**Primary implementation language:** Rust
**Product interaction model:** Terminal-centric, keyboard-first, with lightweight renderer-native graphical overlays and optional pointer interaction
**Scope:** Managed SSH, provider-neutral authentication, AWS/Azure/GCP/Kubernetes/OpenShift/Teleport providers, Provider-aware Quick Actions, DevOps extension architecture, process isolation, credentials, overlays, testing, release evidence, and activation strategy

---

## 1. Purpose

Automexia has reached a point where a large amount of functionality is source-complete or locally validated, while several important execution paths remain intentionally nonactivated or insufficiently proven in controlled native environments.

The next priority should **not** be maximizing the number of new source-complete features.

The next priority should be turning:

```text
source-complete
+ contract-tested
+ nonactivated
```

into:

```text
architecturally isolated
+ native tested
+ real-provider tested
+ controlled-preview activated
+ independently auditable
+ release-ready
```

This document defines the recommended changes, fixes, refactors, validation work, architecture constraints, and activation sequence required for the current version.

---

## 2. Product Interaction Model

Automexia should **not** be described as GUI-free.

The correct product definition is:

> **Automexia is terminal-centric and keyboard-first, not GUI-free.**
>
> Lightweight renderer-native overlays are first-class product surfaces for discovery, selection, status, review, configuration, extensions, connections, and Quick Actions.
>
> Every essential workflow must be fully operable from the keyboard.
>
> Pointer interaction may exist as an optional convenience, but no important action may require a mouse.

This means the following are valid and should remain:

- Connection Hub
- Extension list
- Quick Actions popup
- Command palette
- Provider/resource selector
- Permission review
- Confirmation popup
- Status popup
- Settings picker
- Search popup
- Session switcher
- Help overlay
- Notifications
- Progress views
- Small interactive contextual surfaces

This does **not** mean Automexia should become a Termius-style application with:

- permanent navigation sidebars,
- dashboard-centric workflows,
- card-based connection management as the primary interface,
- mouse-required buttons,
- persistent management screens,
- large settings windows,
- click-only features.

The terminal remains the primary surface.

The normal interaction model should remain:

```text
Shell / terminal
      ↓
keyboard shortcut
      ↓
temporary overlay
      ↓
search / select / review
      ↓
action
      ↓
overlay disappears
      ↓
back to terminal
```

---

## 3. Overall Current Assessment

| Area | Assessment |
|---|---|
| Rust architecture | Strong direction |
| Managed SSH source | Mature |
| SSH activation | Needs hardening and native evidence |
| SSH trust/routing | Strong design |
| SSH tunnels | Good source architecture; native evidence still needed |
| Workspace automation | Good foundation |
| Provider-neutral authentication | Strong |
| AWS/Azure/GCP/Kubernetes/Teleport | Source-mature, integration-unproven |
| Provider-aware Quick Actions | Strong defensive design |
| OpenBao | Correctly deferred |
| Extension architecture | D7/CP6 nonactivating source boundary accepted in ADR 0029 and implemented locally; activation and public distribution remain unaccepted |
| Overlay architecture | Keep and formalize |
| Cross-platform native evidence | Insufficient for stable activation |
| Status/documentation governance | Needs cleanup |
| Release provenance/signing | Not ready |
| Fuzzing | Targets exist; actual campaigns required |
| Stable release readiness | Not yet |

---

## 4. Immediate Strategic Change: Create an Activation Hardening Milestone

Do not make the next major milestone primarily about new end-user features.

Create a dedicated milestone such as:

```text
AH1 — CURRENT VERSION ACTIVATION HARDENING
```

### Goal

Turn the existing managed-access architecture into a system that is:

- execution-authority complete,
- provider-isolated,
- native-tested,
- credential-safe,
- extension-safe,
- cross-platform validated,
- release-evidence backed,
- incrementally activatable.

### AH1 should include

1. Managed OpenSSH authority isolation.
2. Process Broker hardening.
3. Executable identity binding.
4. Provider environment isolation.
5. Structured command authority.
6. Shell-safe insertion rendering.
7. Extension boundary formalization.
8. Overlay framework formalization.
9. Native SSH validation.
10. Real provider validation.
11. Actual fuzz campaigns.
12. Durable evidence.
13. Status governance.
14. Per-capability activation.
15. Frozen integration revision and security approval.

---

## 5. P0 — Managed OpenSSH Configuration Authority

This is one of the most important items to verify before activation.

Automexia currently emphasizes:

- exact OpenSSH arguments,
- exact review,
- executable identity binding,
- exact capabilities,
- no free-form `-o`,
- no free-form `ProxyCommand`,
- no remote command text,
- explicit route and generation authority.

Those guarantees are incomplete if managed `ssh` may silently inherit behavior from ambient OpenSSH configuration.

By default, OpenSSH may read:

```text
~/.ssh/config
/etc/ssh/ssh_config
```

These files may alter execution through directives such as:

```text
ProxyCommand
ProxyJump
Match exec
LocalCommand
RemoteCommand
LocalForward
RemoteForward
DynamicForward
IdentityAgent
IdentityFile
CertificateFile
KnownHostsCommand
ControlMaster
ControlPath
```

### Required design

Define separate execution modes.

#### 5.1 Strict Managed SSH

Default managed Automexia mode:

```text
Automexia typed SSH plan
        ↓
validated route
        ↓
validated identity
        ↓
validated trust
        ↓
controlled config authority
        ↓
OpenSSH
```

Prefer either:

```text
ssh -F none ...
```

or:

```text
ssh -F <Automexia-generated-controlled-config> ...
```

The key property must be:

```text
reviewed managed plan
=
executed managed plan
```

Ambient SSH configuration must not silently expand execution authority.

#### 5.2 Reviewed SSH Config Import

Users should still be able to benefit from `~/.ssh/config`.

Do not simply trust the entire file.

Use:

```text
~/.ssh/config
      ↓
parser
      ↓
directive classification
      ↓
typed supported subset
      ↓
review
      ↓
managed representation
      ↓
controlled execution
```

Safe/importable fields may include:

```text
HostName
User
Port
ProxyJump
IdentityFile reference
ServerAliveInterval
```

Potentially dangerous or execution-changing fields require stronger review or remain unsupported in managed mode:

```text
ProxyCommand
LocalCommand
RemoteCommand
Match exec
arbitrary shell-backed directives
```

#### 5.3 Manual SSH Remains Untouched

Normal shell-owned:

```bash
ssh host
```

must continue to use the user’s normal SSH environment.

This creates a clean separation:

```text
Manual shell SSH
→ user-owned configuration and behavior

Managed Automexia SSH
→ Automexia-reviewed execution authority
```

#### Required ADR

Create or update an ADR defining:

```text
Managed SSH Configuration Authority
```

and explicitly state what ambient configuration is allowed, imported, blocked, or ignored.

---

## 6. P0 — Harden ExternalToolRunner into a Process Broker

The current application-owned runner is the right ownership model.

“Process Broker” below names the logical hardening role of the existing
application-owned `ExternalToolRunner`; it does not require a new crate, daemon,
or competing process owner.

However, it must not become a general arbitrary execution service.

Avoid an extension-facing API like:

```rust
run(path, args, env)
```

That effectively grants arbitrary process execution.

The architecture should be:

```text
Extension / Provider
        ↓
Typed Provider Plan
        ↓
Policy Validation
        ↓
Executable Registry
        ↓
Process Broker
        ↓
OS Process
```

### Recommended Rust model

```rust
pub struct ApprovedLaunchPlan {
    pub executable: ExecutableIdentity,
    pub argv: BoundedArgs,
    pub environment: SanitizedEnvironment,
    pub capabilities: CapabilitySet,

    pub resource_id: ResourceId,
    pub session_id: SessionId,
    pub capsule_id: CapsuleId,
    pub generation: GenerationId,

    pub risk: EnvironmentRisk,
}
```

Consider a controlled executable registry:

```rust
pub enum ManagedExecutable {
    OpenSsh,
    AwsCli,
    AzureCli,
    Gcloud,
    Kubectl,
    Oc,
    Tsh,
}
```

Providers request a known managed tool.

They do not request arbitrary executable paths.

---

## 7. P0 — Make Executable Identity First-Class

Executable review must bind to the actual file used at spawn time.

Do not treat:

```text
same filename
```

as equivalent to:

```text
same executable identity
```

### Recommended Rust structure

```rust
pub struct ExecutableIdentity {
    pub canonical_path: PathBuf,
    pub file_identity: FileIdentity,
    pub sha256: Digest,
    pub size: u64,
    pub version: Option<VersionEvidence>,
    pub package: Option<PackageIdentity>,
    pub signer: Option<SignerIdentity>,
    pub observed_at: SystemTime,
}
```

The required invariant is:

```text
reviewed executable identity
=
spawned executable identity
```

#### Windows executable identity

Validate:

- canonical path,
- file identity,
- digest where required,
- package/source identity where available,
- signer where applicable,
- replacement between review and process creation,
- post-spawn executable identity where feasible.

#### Linux

Validate:

- canonical path,
- inode/device or equivalent identity,
- package provenance where required,
- replacement race protection.

#### macOS

Also consider:

- code-signing identity,
- bundle/package identity,
- quarantine/signing state where appropriate.

---

## 8. P0 — Enforce One Process-Execution Authority in CI

Create a repository-level rule:

> Only the application-owned ExternalToolRunner authority may directly launch
> external provider or managed tool processes.

Provider crates should not independently call:

```rust
std::process::Command
```

or equivalent OS launch APIs.

Recommended dependency direction:

```text
devops-aws
     ↓
ProcessBroker

devops-azure
     ↓
ProcessBroker

devops-gcp
     ↓
ProcessBroker

devops-kubernetes
     ↓
ProcessBroker

devops-teleport
     ↓
ProcessBroker
```

Add architecture tests or repository scans that fail if unauthorized crates import or invoke process-launch functionality.

---

## 9. P0 — Provider Process Environment Isolation

Provider isolation must include environment variables, not only command-line profile selection.

Do not use:

```text
inherit parent environment
+
override a few variables
```

Use:

```text
parent environment
        ↓
explicit filtering
        ↓
approved OS baseline
        +
provider-approved variables
        +
capsule-controlled variables
        ↓
child process
```

### Recommended Rust representation

```rust
pub struct SanitizedEnvironment {
    pub baseline: BTreeMap<OsString, OsString>,
    pub provider: BTreeMap<OsString, OsString>,
}
```

Prefer:

```rust
OsString
PathBuf
```

for OS-facing values rather than assuming UTF-8.

Each provider must have a reviewed environment policy.

Every provider-affecting variable must be one of:

```text
1. explicitly removed
2. explicitly supplied
3. explicitly reviewed and bound
```

Never accidentally inherited.

---

## 10. P0 — AWS Isolation

AWS profiles alone are not sufficient authority.

Ambient environment credentials may override profile-based behavior.

A dangerous case is:

```text
Automexia capsule:
profile = staging

Inherited process environment:
AWS_ACCESS_KEY_ID = production credential
AWS_SECRET_ACCESS_KEY = ...
AWS_SESSION_TOKEN = ...
```

The UI may say “staging” while execution occurs under another identity.

### Required AWS environment policy

Review/remove/bind variables such as:

```text
AWS_ACCESS_KEY_ID
AWS_SECRET_ACCESS_KEY
AWS_SESSION_TOKEN
AWS_SECURITY_TOKEN
AWS_PROFILE
AWS_DEFAULT_PROFILE
AWS_REGION
AWS_DEFAULT_REGION
AWS_CONFIG_FILE
AWS_SHARED_CREDENTIALS_FILE
AWS_ROLE_ARN
AWS_WEB_IDENTITY_TOKEN_FILE
AWS_ENDPOINT_URL
AWS_ENDPOINT_URL_*
AWS_CA_BUNDLE
AWS_PAGER
AWS_CLI_AUTO_PROMPT
```

The exact list should come from a maintained provider policy rather than being frozen forever.

### Bind actual identity

For sensitive AWS actions:

```text
Capsule expected identity
        ↓
fresh STS GetCallerIdentity
        ↓
account/ARN comparison
        ↓
match?
        ↓
continue
```

Bind:

- account,
- principal/ARN,
- role where appropriate,
- region where relevant,
- profile/config source,
- generation.

If the identity differs, invalidate the plan and require a fresh review.

---

## 11. P0 — Azure Isolation

Do not rely on mutable global Azure CLI active subscription state.

Avoid managed workflows based on:

```text
az account set ...
```

followed by another command.

Prefer:

- explicit tenant,
- explicit subscription,
- explicit resource,
- isolated config state where necessary.

Managed Automexia authority must come from the capsule, not from whatever Azure CLI globally happens to have selected.

Where useful, use an isolated:

```text
AZURE_CONFIG_DIR
```

for Automexia-managed contexts or controlled test fixtures.

---

## 12. P0 — Google Cloud Isolation

Do not mutate the user's global active `gcloud` configuration for managed execution.

Prefer:

```text
gcloud ... --configuration <name>
```

and explicit account/project parameters where relevant.

Sanitize or bind:

```text
CLOUDSDK_CONFIG
CLOUDSDK_ACTIVE_CONFIG_NAME
CLOUDSDK_CORE_PROJECT
CLOUDSDK_CORE_ACCOUNT
other provider-affecting CLOUDSDK_* values
```

The capsule must remain the authority.

---

## 13. P0 — Kubernetes/OpenShift Isolation

For managed operations, use an exact reviewed kubeconfig source.

Prefer:

```text
--kubeconfig <Automexia-reviewed-file>
```

rather than relying on:

```text
KUBECONFIG
default kubeconfig discovery
merged ambient kubeconfigs
current-context
```

Bind where relevant:

```text
cluster
context
namespace
user/auth source
generation
```

Also review newer ambient configuration surfaces such as:

```text
kuberc/preferences
```

instead of assuming kubeconfig is the only configuration input.

---

## 14. P0 — Kubernetes Exec Plugin Authority

Kubeconfig exec plugins are executable authority.

Do not treat them as passive configuration.

The model should be:

```text
kubeconfig exec declaration
        ↓
ExecutableIdentity
        ↓
arguments
        ↓
environment
        ↓
capabilities
        ↓
review
        ↓
ProcessBroker
```

A kubeconfig must never silently grant arbitrary process execution.

Current default-deny/review behavior should remain.

---

## 15. P0 — Teleport Ownership Boundary

The current Teleport philosophy should remain:

Teleport owns:

- MFA,
- browser/device flow,
- credential format,
- cache format,
- certificate/session internals.

Automexia owns:

- reviewed target context,
- command plan,
- resource identity,
- route/session isolation,
- expiry awareness,
- launch authority,
- stale-context rejection.

Avoid copying Teleport secrets into Automexia simply to gain deeper control.

Prefer orchestration over credential duplication.

---

## 16. P0 — Windows `.cmd` and Script Execution

Windows provider tooling requires an explicit design because some CLIs may be exposed through command wrappers.

Do not solve this by adding:

```rust
run_shell(command: String)
```

to the Process Broker.

That would bypass the exact-argv security model.

Use:

```rust
pub enum ExecutableKind {
    NativeBinary,
    TrustedBatchWrapper,
    TrustedInterpreterScript,
}
```

A trusted wrapper path should bind:

```text
known wrapper
known interpreter
known package identity
strict argument encoding
```

For example:

```text
Azure CLI
    ↓
verified az.cmd
    ↓
verified cmd.exe
    ↓
strict generated invocation
```

This should be provider-specific and narrowly reviewed.

Fuzz Windows command encoding with:

```text
&
|
"
^
%
!
()
spaces
Unicode
CR
LF
```

and other interpreter edge cases.

---

## 17. P0 — Structured Commands Must Be Authoritative

Do not let a shell command string become the authoritative representation of an action.

Use:

```rust
pub struct StructuredCommand {
    pub executable: CommandExecutable,
    pub args: Vec<CommandArgument>,
}
```

Then separate execution from display/insertion.

### Execution path

```text
StructuredCommand
      ↓
ProcessBroker
      ↓
argv execution
```

### Insertion/copy path

```text
StructuredCommand
      ↓
ShellRenderer
      ↓
editable textual command
```

Never parse the displayed command back into authority.

The structured form is authoritative.

---

## 18. P0 — Shell-Specific Command Rendering

`Insert without Enter` is an excellent permanent design decision.

However, inserted command text still needs shell-specific quoting.

Implement:

```text
ShellRenderer
├── Bash
├── Zsh
├── Fish
├── PowerShell
├── Cmd
└── Nushell
```

Do not assume POSIX quoting works everywhere.

Example:

```text
Structured plan:
Executable: ssh
Arg 1: -p
Arg 2: 22
Arg 3: alice@example-host

        ↓
Bash renderer

ssh -p 22 'alice@example-host'
```

The renderer exists only for user-visible/editable text.

Execution still uses argv directly.

---

## 19. P0 — Control-Character and Terminal Injection Hardening

Provider metadata, remote names, hostnames, project names, resource labels, and comments may eventually contain attacker-controlled text.

Treat them as data.

Review or reject dangerous control characters where relevant:

```text
NUL
CR
LF
ESC
C0 controls
C1 controls
unexpected bidi controls
```

Never allow resource metadata to become:

- terminal escape control,
- command separator,
- hidden multi-line insertion,
- UI spoofing.

Apply this protection to:

- Quick Actions,
- Connection Hub,
- notifications,
- logs,
- receipts,
- copy operations,
- shell insertion,
- provider status labels.

---

## 20. P0 — Risk and Production Classification

Production confirmation is good, but the risk value must not be inferred from names such as:

```text
name contains "prod"
```

Use explicit metadata.

```rust
pub enum EnvironmentRisk {
    Development,
    Test,
    Staging,
    Production,
    Critical,
}
```

Bind risk into:

```text
Resource
Capsule
AccessPlan
Authorization
Quick Action
Receipt
```

If risk changes between review and execution, invalidate the approval.

---

## 21. Improve Production Confirmation UX

Avoid confirmation fatigue.

Use risk-proportional review.

### Low-authority actions

Examples:

```text
inspect
copy
insert without Enter
view status
```

Use prominent risk display, but not necessarily repeated blocking confirmation.

### Real execution

Examples:

```text
connect
execute provider command
assume privileged role
open tunnel
```

Use stronger confirmation.

### Extreme/high-blast-radius actions

Examples:

```text
production broadcast
non-loopback tunnel
remote forwarding
critical role assumption
multi-target action
```

Use the strongest review.

The user should not become trained to press Enter repeatedly through warnings.

---

## 22. P0 — Credential Capsules Must Never Contain Actual Credentials

Provider capsules should contain:

```text
provider
identity
account/project/subscription
role
region
resource
freshness
expiry
auth state
risk
provenance
generation
```

They should not contain:

```text
password
private key
access token
refresh token
session token
MFA secret
vault master key
```

Actual secret authority belongs behind:

```text
CredentialHandle
```

or the provider's own external security system.

---

## 23. Improve Data Classification Terminology

Avoid calling all non-secret capsule metadata “public.”

Use at least:

```text
Public
Non-secret private metadata
Sensitive
Secret
```

Examples of non-secret private metadata:

```text
AWS account ID
Azure subscription
tenant ID/name
GCP project ID
Kubernetes cluster
Teleport proxy
internal hostname
production environment name
```

These may be safe for local display but should still normally be:

- excluded from telemetry by default,
- excluded from AI by default,
- excluded from exports unless requested,
- redacted from public diagnostics where appropriate.

---

## 24. Formal Authentication State Machine

Nineteen internal authentication states may be justified.

But transitions must be centralized and validated.

Do not allow every provider to assign arbitrary states.

Prefer:

```rust
AuthStateMachine::transition(event)
```

with exhaustive transition tests.

Possible conceptual transitions:

```text
Missing
  ↓
Authenticating
  ↓
BrowserPending / DevicePending / MfaPending
  ↓
Ready
  ↓
Refreshing
  ↓
Ready
```

and:

```text
Ready
  ↓
Expired
```

or:

```text
Any active state
  ↓
Revoked
```

Illegal transitions must fail closed.

---

## 25. Simplify User-Facing Authentication Labels

Internally:

```text
19 states
```

may be appropriate.

The Connection Hub should expose a simpler user-facing vocabulary such as:

```text
READY
AUTH REQUIRED
WAITING FOR MFA
WAITING FOR BROWSER
EXPIRED
OFFLINE
BLOCKED
ERROR
```

Detailed state information can be shown on request.

This preserves internal correctness without overwhelming the user.

---

## 26. P0/P1 — Formalize the Overlay Framework

Do not remove Automexia's graphical popups.

Formalize them as a core product subsystem.

Create a renderer-native overlay layer:

```text
automexia-overlay
```

with semantic primitives such as:

```text
Picker
List
Table
Tree
Input
SecretInput
Confirmation
Details
Progress
Notification
Status
Help
```

Example Rust model:

```rust
pub enum OverlayModel {
    Picker(PickerModel),
    Table(TableModel),
    Confirmation(ConfirmationModel),
    Details(DetailsModel),
    Progress(ProgressModel),
}
```

The feature supplies semantic content.

The core supplies:

- rendering,
- layout,
- keyboard behavior,
- pointer behavior,
- themes,
- accessibility,
- animation,
- focus,
- resizing.

---

## 27. Keyboard Must Be Mandatory; Pointer Optional

The policy should be:

```text
keyboard path = mandatory
pointer path  = optional convenience
```

Valid pointer behaviors include:

- click to select,
- double-click to connect,
- wheel scrolling,
- hover help,
- context highlighting.

But every important pointer action must have a keyboard equivalent.

Do not allow:

```text
mouse-only Connect
mouse-only menu
right-click-only critical action
```

---

## 28. All Interaction Paths Must Converge on Commands

UI events must not own business logic.

Use:

```text
Keyboard ─────┐
              │
Pointer ──────┼──→ CommandId → Policy → Action
              │
Palette ──────┤
              │
Extension ────┘
```

Example:

```rust
CommandId::ConnectSelectedResource
```

should be the authority.

Not:

```text
button callback
```

This makes keyboard-first interaction structural.

---

## 29. Extensions Must Not Draw Arbitrary Pixels

Extensions should request semantic UI:

```text
show_picker
show_table
show_input
show_confirmation
show_details
show_progress
show_notification
```

They should not receive:

```text
raw GPU device
arbitrary renderer access
native window handles
arbitrary new windows
pixel drawing
```

Architecture:

```text
Extension
   ↓
Declarative Overlay Model
   ↓
Core Overlay System
   ↓
Renderer
```

This gives every extension consistent:

- keyboard interaction,
- pointer support,
- accessibility,
- themes,
- focus behavior,
- testing,
- responsive layout.

---

## 30. Accessibility Tree Must Be Independent from Rendering

GPU visual tests are useful, but screen readers need semantics.

Each overlay should generate an accessibility tree with:

```text
role
name
description
value
state
selected
focused
disabled
relationships
```

Then platform bridges expose it through:

```text
Windows UI Automation
macOS accessibility APIs
Linux AT-SPI
```

The rule for visible state should be:

```text
Color = supplemental
Icon  = supplemental
Text  = authoritative
```

Examples:

```text
● Ready
! Authentication required
▲ Production — High Risk
```

Do not rely on color alone.

---

## 31. P0/P1 — Formalize the Extension Boundary Now

The marketplace can wait.

The extension architecture cannot.

Automexia's product model is:

```text
Terminal Core
      ↓
Extension Platform
      ↓
DevOps
Database
AI
Git
Security
...
```

Therefore first-party providers should already depend on extension-facing abstractions rather than internal terminal implementation details.

Create or stabilize crates such as:

```text
automexia-extension-api
automexia-command-api
automexia-resource-api
automexia-access-api
automexia-capabilities
automexia-overlay-api
```

Current first-party modules such as:

```text
devops-aws
devops-azure
devops-gcp
devops-kubernetes
devops-teleport
```

should communicate through:

```text
ResourceProvider
ActionProvider
AccessPlanProvider
AuthenticationProvider
CommandProvider
```

rather than directly touching unrelated terminal internals.

---

## 32. Native First-Party Rust Extensions Are Acceptable Initially

Do not force immediate WASM conversion.

A practical transition is:

```text
Today:

First-party Rust extension
      ↓
Extension API
      ↓
Trusted Core
```

Later:

```text
Third-party WASM extension
      ↓
WIT
      ↓
Wasmtime
      ↓
same conceptual API
      ↓
Trusted Core
```

The important work now is to make the boundary real.

---

## 33. Extension Process Authority Must Be Narrow

Extensions should not receive raw Process Broker access.

They should request:

```text
execute validated AWS action
execute reviewed SSH plan
execute validated Teleport plan
```

not:

```text
run arbitrary program
```

Likewise:

```text
credentials.request()
```

not:

```text
read vault internals
```

And:

```text
ui.show_picker()
```

not:

```text
draw arbitrary GPU frame
```

---

## 34. Narrow Network Capabilities

Avoid eventually giving ordinary provider extensions:

```text
network = true
```

If `aws`, `gcloud`, `tsh`, or `kubectl` needs network access, the managed child process may receive it.

The extension itself may not need raw networking.

Prefer capability types like:

```text
browser.open
callback.listen_loopback
provider-process.network
http.connect(domain-set)
```

rather than unrestricted network access.

---

## 35. Keep Extensions Out of the Terminal Hot Path

The hot path must remain:

```text
Keyboard
 ↓
Input Manager
 ↓
PTY
```

and:

```text
PTY
 ↓
VT Engine
 ↓
Terminal State
 ↓
Renderer
 ↓
GPU
```

Never:

```text
PTY
 ↓
DevOps extension
 ↓
future local automation extension
 ↓
Renderer
```

No extension may be able to stall terminal typing/rendering.

---

## 36. M6 Broadcast Hardening

Broadcast is security-sensitive.

Raw indefinite keystroke mirroring can leak secrets:

```text
broadcast armed
      ↓
one target asks for sudo password
      ↓
user types secret
      ↓
secret sent to all targets
```

Prefer:

```text
reviewed line broadcast
```

Example:

```text
Broadcast armed: 8 targets

> systemctl status app

Review
Enter → send
```

Automatically disarm after execution unless explicitly kept armed.

At minimum, exclude from broadcast:

```text
secret inputs
credential dialogs
MFA inputs
password/passphrase prompts
Automexia secure-input overlays
```

Invalidate/disarm broadcast when:

```text
target set changes
workspace generation changes
session reconnects
risk changes
resource identity changes
```

---

## 37. Tunnel Authority Must Stay Typed

Keep typed tunnel models:

```rust
enum Tunnel {
    Local(LocalTunnel),
    Remote(RemoteTunnel),
    Dynamic(DynamicTunnel),
}
```

Do not introduce:

```text
ssh_extra_args = "..."
```

Loopback should remain the default.

Require stronger review for:

```text
0.0.0.0
::
remote forwarding
non-loopback listeners
```

Tunnel ownership must remain tied to:

```text
session
lease
route
generation
```

and must clean up deterministically.

---

## 38. OpenBao Must Remain Blocked Until ADR Acceptance

Do not implement OpenBao before ADR 0024 or equivalent approval.

The ADR should answer:

```text
Who authenticates to OpenBao?
Who owns the OpenBao token?
Where is it stored?
Can extensions materialize it?
Who renews leases?
Who revokes leases?
How are SSH certificates issued?
Where are certificates written?
Who deletes them?
What survives restart?
What is audited?
What happens offline?
How is disable/uninstall cleanup handled?
```

Only after these decisions should a provider implementation exist.

---

## 39. Password Manager Integration Philosophy

For SSH, prefer agent protocols.

Example:

```text
Automexia
   ↓
OpenSSH
   ↓
SSH_AUTH_SOCK
   ↓
1Password / Bitwarden / KeePassXC
```

Prefer this over:

```text
password manager
   ↓
private key bytes
   ↓
Automexia process memory
```

Direct private-key materialization should be exceptional and separately reviewed.

---

## 40. Bind SSH Agent Authority

If managed SSH uses:

```text
SSH_AUTH_SOCK
```

the agent endpoint itself is part of execution authority.

Bind:

```text
agent endpoint
agent identity/type where known
observed public identities
generation
```

into the plan.

Do not review one agent and execute against another because the environment changed.

---

## 41. Agent Forwarding Must Remain Disabled by Default

Keep the current default.

If forwarding is introduced later, require:

```text
separate capability
separate review
approved destination
safe OpenSSH/vendor patch evidence
```

Do not silently enable:

```text
ForwardAgent yes
```

---

## 42. Convert OpenSSH Security-Version Analysis into Executable Policy

Do not leave OpenSSH security requirements only in documentation.

Create a typed security-evidence model.

```rust
pub struct SshSecurityEvidence {
    pub version: OpenSshVersion,
    pub vendor: Option<Vendor>,
    pub pq_kex: PqCapability,
    pub forwarding_security: ForwardingSecurity,
    pub provenance: PackageProvenance,
}
```

The policy should not be:

```text
version >= X
```

only.

Allow:

```text
approved upstream version
OR
supported vendor build with verified required backports
```

This is necessary because Linux distributions and platform vendors may backport security fixes without matching the upstream version number exactly.

---

## 43. Do Not Fork OpenSSH Crypto Policy Without Need

Automexia should not maintain a large static hardcoded list of:

```text
KexAlgorithms
Ciphers
MACs
```

unless the product explicitly decides to enforce an independent cryptographic policy.

Prefer:

```text
validate capability
+
preserve secure OpenSSH defaults
+
preserve upstream warnings
```

Do not automatically suppress non-PQ warnings.

Do not silently downgrade algorithms for convenience.

---

## 44. Controlled Activation Tiers

A simple:

```text
disabled
vs
production
```

model is too binary.

Add controlled activation tiers.

```rust
pub enum ActivationTier {
    Disabled,
    ControlledFixture,
    InternalPreview,
    UserPreview,
    Stable,
}
```

### Disabled

Source exists, execution unavailable.

### ControlledFixture

Execution enabled only against explicit controlled test fixtures.

### InternalPreview

Real provider/tool execution available to approved internal users.

### UserPreview

Opt-in preview, provider-specific kill switches, still not stable.

### Stable activation tier

Release-qualified, signed, cross-platform validated.

This lets Automexia exercise the real execution path much earlier without exposing it broadly.

---

## 45. Activate Capabilities Independently

Avoid one global switch.

Use separate capability activation:

```text
managed_ssh
aws_execution
azure_execution
gcp_execution
kubernetes_execution
openshift_execution
teleport_execution
provider_quick_action_execution
```

Recommended controlled activation order:

```text
Managed SSH
    ↓
AWS
    ↓
Azure
    ↓
GCP
    ↓
Kubernetes/OpenShift
    ↓
Teleport
```

Do not activate everything simultaneously.

---

## 46. OS-Native Child Ownership and Cleanup

### Windows child ownership

Prefer:

```text
Job Object
+
ConPTY ownership
+
bounded graceful termination
+
forced cleanup
```

where appropriate.

### Unix

Use:

```text
PTY
+
session/process-group ownership
+
SIGTERM
+
deadline
+
SIGKILL
```

The supervisor must know which children are:

```text
owned
external
intentionally detached
```

A browser launched for OAuth/MFA is not equivalent to a managed SSH child tree.

Cancellation must not accidentally kill unrelated user applications.

---

## 47. Structured Async Ownership in Rust

Avoid important orphan tasks such as:

```rust
tokio::spawn(async move {
    // forgotten lifecycle
});
```

for session/provider lifecycle operations.

Use structured ownership:

- supervisors,
- `JoinSet`,
- cancellation tokens,
- task groups,
- explicit shutdown.

Required lifecycle:

```text
session closes
      ↓
cancel owned tasks
      ↓
await task termination
      ↓
close process/PTY resources
      ↓
release leases/routes
```

---

## 48. Bounded Channels Everywhere

Keep and expand current bounded-queue practice.

Use bounded channels for:

```text
provider updates
extension events
resource discovery
audit events
receipt publication
overlay model updates
session lifecycle events
```

Avoid unbounded memory growth under:

- malicious remote output,
- broken provider tools,
- event storms,
- slow extensions.

---

## 49. Tokio Should Not Own the Terminal Render Hot Path

Tokio is appropriate for:

```text
IPC
provider orchestration
filesystem discovery
credential refresh
extension lifecycle
network-adjacent coordination
```

Keep:

```text
keyboard input
VT parsing
grid updates
damage calculation
render preparation
GPU submission
```

on tightly controlled synchronous/dedicated-thread paths.

Do not turn rendering into thousands of async tasks.

---

## 50. Rust Unsafe-Code Policy

Use:

```rust
#![forbid(unsafe_code)]
```

where practical in higher-level crates.

Candidates:

```text
resources
access
credentials
provider-auth
commands
quick-actions
workspace
extension-api
provider adapters
```

Allow `unsafe` only in reviewed low-level crates such as:

```text
pty-platform
conpty
renderer backend
Ghostty FFI adapter
OS integration
```

Every unsafe block should document its invariant.

---

## 51. Strongly Typed Authority IDs

Avoid using generic `String` or integer values for authority-bearing identifiers.

Use newtypes:

```rust
SessionId
RouteId
PaneId
ResourceId
CapsuleId
GenerationId
CredentialHandle
ProviderId
ExecutableId
ApprovalId
OperationLeaseId
WorkspaceId
```

This strengthens:

- stale-generation rejection,
- cross-session isolation,
- cross-provider isolation,
- route ownership,
- receipt binding.

---

## 52. Secret Types Must Redact by Construction

Use wrappers such as:

```rust
Secret<T>
Sensitive<T>
CredentialHandle
```

Do not provide unsafe default formatting.

Default:

```text
Debug → [REDACTED]
Display → [REDACTED]
```

Secret serialization must be explicit and rare.

Prevent accidental disclosure through:

```text
logs
panic messages
crash reports
telemetry
debug formatting
receipts
tests
```

---

## 53. Quick Action Scale Testing

Current small cached benchmarks are encouraging but insufficient for future scale.

Test:

```text
100 resources
1,000 resources
10,000 resources
50,000 resources
```

with mixed providers:

```text
SSH
AWS
Azure
GCP
Kubernetes
OpenShift
Teleport
future database resources
future Git resources
future extensions
```

Measure:

```text
p50
p95
p99
allocation count
memory
overlay open latency
filter latency
first-frame latency
```

The user experiences the whole interaction:

```text
shortcut
  ↓
overlay visible
  ↓
search
  ↓
highlight update
```

not only the internal search function.

---

## 54. Define Explicit UX Performance Budgets

Create budgets such as:

```text
warm overlay open:
< 50 ms p95

cached query/filter:
within one frame

keypress-to-highlight:
< 16 ms p95 target

provider process launches:
0 on keystroke path

network calls:
0 on keystroke path
```

Exact numbers may change after measurement.

The important part is having a regression ratchet.

---

## 55. Actually Run Fuzz Campaigns

“Fuzz target compiled” means only:

```text
FUZZ BUILD: PASS
```

It does not mean:

```text
FUZZ EXECUTION: PASS
```

Run real campaigns.

High-value fuzz targets:

```text
VT parser
OSC/DCS/APC sequences
Unicode
SSH host input
OpenSSH version parser
AWS CLI output/JSON
Azure CLI output/JSON
GCP output/JSON
Teleport status output
kubeconfig YAML
OpenShift config
Quick Action metadata
shell renderers
Windows batch quoting
capsule decoding
provider state parsing
```

Persist:

- crash inputs,
- regression inputs,
- interesting corpus additions.

Track CPU-hours or execution duration for release evidence.

---

## 56. Mutation Testing Terminology

Keep the existing mutation scenarios.

But document precisely:

```text
6 security-policy mutation scenarios passed
```

rather than implying comprehensive repository-wide mutation testing.

Prioritize future mutations around:

```text
authorization
stale generation rejection
risk revalidation
credential handling
process execution
provider isolation
```

These areas provide much higher security value than mutating every line of code.

---

## 57. Native Testing Is Now Higher Value Than More Provider Features

The current architecture has enough source-level capability that native integration testing should become the priority.

Run:

```text
Windows ConPTY + real OpenSSH
Linux PTY + real OpenSSH
macOS PTY + real OpenSSH
```

Then exercise real controlled:

```text
AWS CLI
Azure CLI
gcloud
kubectl
oc
tsh
```

with controlled resources.

The next bugs worth finding are increasingly:

```text
process lifecycle
CLI behavior
environment precedence
PTY behavior
browser/MFA lifecycle
expiry races
filesystem/path behavior
OS-specific execution differences
```

not missing type definitions.

---

## 58. Deterministic Provider Test Environments

Create controlled dedicated provider environments.

Examples:

```text
test AWS organization/account
test Azure tenant/subscription
test GCP project
ephemeral Kubernetes cluster
ephemeral OpenSSH server
controlled Teleport environment
```

Test:

```text
login
expiry
refresh
offline
denied permissions
wrong account
wrong tenant
changed configuration
stale capsule
cancellation
process crash
network loss
revocation
disable
uninstall
cross-session isolation
```

---

## 59. Real Browser/MFA Test Campaigns

Mocks are not enough for final validation.

Create controlled campaigns for:

```text
browser opens
browser blocked
browser closed
timeout
MFA success
MFA failure
device flow
token expiry during action
network disconnect
user cancellation
provider denial
```

Keep authentication ownership with official provider tooling whenever possible.

Automexia orchestrates and observes state.

It should not reimplement provider authentication unnecessarily.

---

## 60. Durable QA Evidence

Do not rely only on:

```text
target/qa/...
```

because `target/` is disposable.

Create a small durable commit-bound evidence record.

Example:

```text
evidence/
└── <commit-sha>/
    └── verification.json
```

Include:

```text
commit
timestamp
Rust toolchain
OS
architecture
test counts
Clippy
doc tests
architecture checks
fuzz build
fuzz execution
mutation campaigns
native fixture status
provider fixture status
activation state
artifact digest
```

Large logs can remain CI artifacts.

The small evidence manifest should survive.

---

## 61. Fix Documentation Status Drift

Do not manually synchronize roadmap status across many Markdown files.

Create a machine-readable source of truth:

```text
project-status.toml
```

or:

```text
project-status.json
```

Example:

```toml
[m13]
source = "complete"
unit_tests = "complete"
contract_tests = "complete"
fake_provider = "complete"
windows_native = "partial"
linux_native = "pending"
macos_native = "pending"
live_provider = "pending"
security_review = "pending"
activation = "disabled"
release = "blocked"
```

Generate:

```text
roadmap
feature matrix
phase audit
release status
documentation summaries
```

from this source.

---

## 62. Replace One-Dimensional “Done” Labels

Avoid phrases such as:

```text
fully done locally
partially done overall
source complete
nonactivated
```

without dimensions.

Use a status matrix.

Example:

| Dimension | M13 |
|---|---|
| Source | Complete |
| Unit tests | Complete |
| Contract tests | Complete |
| Fake-provider tests | Complete |
| Windows native | Partial |
| Linux native | Pending |
| macOS native | Pending |
| Real provider | Pending |
| Security review | Pending |
| Activation | Disabled |
| Stable release | Blocked |

This is much harder to misinterpret.

---

## 63. Historical Audits Need Commit Stamps

Every generated audit/report should include:

```text
Status snapshot
Commit: <SHA>
Generated: <timestamp>
Supersedes: <optional prior audit>
Superseded by: <optional later audit>
```

Then a historical document saying:

```text
M8 not implemented
```

can remain valid as:

```text
true at commit X
```

rather than conflicting with newer status.

---

## 64. Remove Absolute Local Documentation Paths

Do not commit links such as:

```text
D:/workstation/projects/...
```

Use repository-relative links:

```text
docs/SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md
```

Absolute developer paths break on:

```text
GitHub
Linux
macOS
CI
other Windows systems
documentation generators
```

If absolute paths exist only in transient local reports, this is less serious.

---

## 65. Branch and Exact-Head Approval Strategy

A long-running branch containing many phases conflicts with exact-head approval semantics.

If:

```text
approval = SHA A
```

and a new commit creates:

```text
SHA B
```

then the exact-head approval for SHA A should no longer authorize SHA B.

Use:

```text
feature branch
      ↓
review
      ↓
merge
      ↓
integration branch
      ↓
freeze
      ↓
security approvals
      ↓
release candidate
```

Do not seek final protected activation approvals while still adding unrelated feature commits to the same exact-head branch.

---

## 66. DCO Is Not Cryptographic Provenance

Keep DCO.

But document it separately from:

```text
signed Git commit/tag
binary signature
installer signature
SBOM
SLSA provenance
Sigstore attestation
package attestation
```

For stable release, add appropriate cryptographic artifact provenance.

---

## 67. Rust Supply-Chain Hardening

Before stable release, require:

```text
Cargo.lock committed
rust-toolchain.toml pinned
advisory checking
dependency license/source policy
restricted Git dependencies
exact revisions for approved exceptions
SBOM generation
release dependency review
```

Eventually add stronger dependency governance such as:

```text
cargo-vet or equivalent
```

Pay special attention to crates handling:

```text
PTY
ConPTY
process creation
TLS
crypto
WASM
credential storage
parsers
updater
```

---

## 68. Release Rings

Use release rings:

```text
Controlled
Internal
Preview
Beta
Stable
```

### Controlled

- developers only,
- synthetic resources,
- test accounts.

### Internal

- real managed OpenSSH,
- real provider test accounts,
- known users.

### Preview

- opt-in,
- provider-by-provider,
- kill switch,
- strong diagnostics,
- still not stable.

### Beta

- broader validation,
- signed packages,
- multi-OS coverage.

### Stable release ring

Requires:

- three-OS validation,
- accessibility evidence,
- signed packaging,
- resource soak evidence,
- security approval,
- release provenance.

---

## 69. Long-Duration Evidence Should Match Release Tier

Do not necessarily require 30 days of evidence before any controlled activation.

Use staged evidence.

Example:

```text
Controlled:
hours/days

Internal:
several days

Preview/Beta:
longer repeated campaigns

Stable:
full long-duration baseline
```

This avoids waiting until the end to exercise real code paths.

---

## 70. Existing Decisions That Should Remain

The following current decisions are strong and should not be weakened.

| Decision | Recommendation |
|---|---|
| Insert without Enter | Keep permanently |
| Broker-required action fails closed | Keep |
| Changed SSH host key blocked | Keep |
| No free-form SSH `-o` in strict managed mode | Keep |
| Agent forwarding disabled by default | Keep |
| Exact generation checking | Keep |
| Stale-result rejection | Keep |
| Final authorization revalidation | Keep |
| Redacted diagnostics | Keep |
| Bounded snapshots | Keep |
| No provider work on keystroke path | Keep |
| Teleport owns MFA/cache | Keep |
| OpenBao requires separate ADR | Keep |
| No secret parsing from SSH prompts | Keep |
| Official provider CLIs own auth initially | Keep |
| Production risk displayed redundantly | Keep |
| Manual shell SSH remains available | Keep |
| Route/session/capsule isolation | Keep |
| Bounded provider contexts | Keep |
| Cancellation/revocation generation safety | Keep |

---

## 71. Target Architecture After Hardening

```text
                         AUTOMEXIA
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
         ▼                  ▼                  ▼
 Terminal Runtime      Product Overlays    Command System
         │                  │                  │
 PTY / ConPTY        keyboard + pointer      CommandId
 VT / renderer       accessibility tree        │
         │                  │                  │
         └──────────────────┼──────────────────┘
                            │
                            ▼
                       Trusted Core
                            │
            ┌───────────────┼───────────────┐
            │               │               │
            ▼               ▼               ▼
       Resource API     AccessPlan       Extension API
            │               │               │
            │               │          first-party Rust
            │               │          future WASM/WIT
            │               │               │
            └───────────────┼───────────────┘
                            │
                    Capability Broker
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
      Process Broker   Credential Broker   UI Broker
             │              │
             │              ├─ SSH agents
             │              ├─ OS keychain
             │              ├─ 1Password
             │              ├─ Bitwarden
             │              ├─ OpenBao later
             │              └─ ...
             │
      Sanitized execution
             │
     ┌───────┼────────┬───────┬─────────┬─────────┐
     ▼       ▼        ▼       ▼         ▼         ▼
    ssh     aws       az    gcloud    kubectl     tsh
```

The overlays remain a first-class product subsystem.

They simply do not become the authority layer.

---

## 72. Recommended Execution Order

The following sequence is recommended for the current version.

### Step 1 — Freeze New Feature Expansion Temporarily

Create the Activation Hardening milestone.

Do not add OpenBao, advanced local automation execution, database execution, or many new provider types until the current authority boundaries are proven.

---

### Step 2 — Fix Managed SSH Configuration Authority

Implement or verify:

```text
-F none
```

or a generated reviewed managed config.

Separate:

```text
manual SSH
managed strict SSH
reviewed imported SSH config
```

---

### Step 3 — Make Process Broker the Only External Execution Authority

Enforce in CI.

No provider crate may independently launch managed external tools.

---

### Step 4 — Implement Provider-Specific Sanitized Environments

Order:

```text
AWS
Azure
GCP
Kubernetes/OpenShift
Teleport
```

Remove dependency on mutable ambient provider state.

---

### Step 5 — Bind Actual Provider Identity Before Sensitive Execution

Examples:

```text
AWS STS identity
Azure tenant/subscription
GCP account/project/config
Kubernetes context/cluster
Teleport proxy/cluster/user/expiry
```

Revalidate immediately before execution.

---

### Step 6 — Introduce StructuredCommand and ShellRenderer

Every Quick Action must have:

```text
structured authority
+
shell-specific textual representation
```

Never execute copied/displayed strings.

---

### Step 7 — Formalize the Overlay Framework

Keep:

- Connection Hub,
- Extension List,
- Quick Actions,
- popups,
- menus,
- confirmations.

Make:

```text
keyboard mandatory
pointer optional
accessibility semantic
commands authoritative
```

---

### Step 8 — Freeze the Internal Extension API

Move current first-party DevOps providers behind:

```text
Resource
Command
AccessPlan
Authentication
Capability
Overlay
```

interfaces.

Do this before creating many additional extensions.

---

### Step 9 — Harden M6 Broadcast

Prefer reviewed line broadcast.

Block secret input propagation.

Auto-disarm on identity/target/generation/risk changes.

---

### Step 10 — Implement Controlled Activation Tiers

Exercise the real broker and provider code in controlled fixtures before user activation.

---

### Step 11 — Run Native Managed SSH

Order:

```text
Windows
Linux
macOS
```

Test:

```text
PTY lifecycle
host-key prompts
password/passphrase behavior
agent behavior
cancellation
disconnect
exit status
child cleanup
resource bounds
```

---

### Step 12 — Controlled Provider Activation One Provider at a Time

Recommended order:

```text
AWS
Azure
GCP
Kubernetes/OpenShift
Teleport
```

Do not activate all simultaneously.

---

### Step 13 — Run Real Fuzz Campaigns and Resource Soaks

Do not stop at fuzz compilation.

---

### Step 14 — Replace Manual Status Synchronization

Generate roadmap and status docs from one machine-readable source.

---

### Step 15 — Freeze the Integration SHA

After the code and evidence stabilize:

```text
freeze exact head
      ↓
security review
      ↓
protected approval
      ↓
package/sign
      ↓
release candidate
```

---

### Step 16 — Keep OpenBao Blocked

Do not implement until ADR 0024 is accepted and credential/certificate custody is fully decided.

---

## 73. Priority Matrix

### P0 — Must Fix or Verify Before Managed Activation

- OpenSSH ambient config authority.
- One Process Broker authority.
- Executable identity check-to-spawn.
- Provider environment isolation.
- Provider identity/context revalidation.
- Structured command authority.
- Shell-safe insertion.
- Kubernetes exec-plugin authority.
- Windows wrapper execution safety.
- Risk binding.
- Credential capsule secret exclusion.
- Native SSH lifecycle tests.

### P1 — Must Complete Before Broad Preview

- Overlay framework.
- Accessibility semantic model.
- Extension boundary.
- Broadcast hardening.
- controlled activation tiers.
- per-provider activation.
- durable evidence.
- actual fuzzing.
- real provider test accounts.
- real browser/MFA campaigns.
- Linux/macOS native provider evidence.

### P2 — Must Complete Before Stable Release

- signed packages/installers,
- SBOM/provenance,
- long-duration resource evidence,
- multi-runner performance baselines,
- complete screen-reader validation,
- dependency governance,
- final extension sandbox strategy,
- stable WIT/extension ABI preparation,
- release documentation generation,
- exact-head protected approvals.

---

## 74. Recommended Additional ADRs

Create or revise ADRs for:

```text
Managed SSH configuration authority
Process Broker as sole execution authority
Executable identity and check-to-spawn
Provider environment isolation
Structured command authority
Overlay framework and keyboard-first interaction
Extension API boundary
Credential capsule data classification
Activation tiers
Provider execution isolation
Broadcast security
OpenSSH version/vendor security policy
OpenBao token and certificate custody
```

These decisions are important enough not to live only in implementation code.

---

## 75. Recommended Rust Crate Boundaries

A hardened Rust workspace could move toward:

```text
crates/
├── core-types/
├── command-api/
├── resource-api/
├── access-api/
├── extension-api/
├── overlay-api/
├── capabilities/
│
├── process-broker/
├── executable-identity/
├── credential-broker/
├── provider-auth/
├── risk-policy/
│
├── pty/
├── conpty/
├── sessions/
├── terminal-engine/
├── renderer/
├── overlays/
├── accessibility/
│
├── shell-render/
│   ├── bash/
│   ├── zsh/
│   ├── fish/
│   ├── powershell/
│   ├── cmd/
│   └── nushell/
│
└── providers/
    ├── aws/
    ├── azure/
    ├── gcp/
    ├── kubernetes/
    ├── openshift/
    └── teleport/
```

The exact names are flexible.

The dependency boundaries are more important than the names.

---

## 76. Security Invariants to Add to CI

Create machine-enforced checks for invariants such as:

```text
No provider crate directly launches processes.
No extension crate imports renderer internals.
No extension crate receives raw credential storage access.
No managed SSH path executes without approved executable identity.
No provider action executes from stale generation.
No Quick Action executes from display text.
No production execution bypasses risk binding.
No broker-required action falls back to shell execution.
No provider process runs on keystroke filtering paths.
No secret type implements unsafe Debug/Display.
No activation state changes without approved release policy.
```

Architecture rules should fail CI rather than depend only on developer memory.

---

## 77. Definition of Activation-Ready

A provider should not be described as activation-ready until all of the following are true.

```text
Source complete
Unit/contract tests complete
Security-policy tests complete
Executable identity verified
Environment policy complete
Provider context bound
Native execution tested
Cancellation tested
Revocation tested
Offline behavior tested
Process cleanup proven
Cross-session isolation proven
Credential leakage checks passed
Accessibility reviewed
Controlled fixture execution passed
Evidence tied to exact commit
Activation policy approved
```

---

## 78. Definition of Stable-Release-Ready

Stable release requires more than activation readiness.

Add:

```text
Windows native validation
Linux native validation
macOS native validation
screen-reader evidence
signed package
SBOM
build provenance
dependency review
long-duration resource evidence
performance regression baseline
fuzz campaign evidence
security approval
rollback/revocation plan
```

---

## 79. Final Recommendation

Automexia should now optimize less for:

```text
more source-complete features
```

and more for:

```text
authority correctness
provider isolation
native execution
extension boundaries
controlled activation
cross-platform evidence
durable auditability
```

The current product direction should remain:

```text
fast Rust terminal
+
keyboard-first interaction
+
small renderer-native graphical overlays
+
optional pointer convenience
+
typed capability-driven extensions
+
secure provider orchestration
+
delegated credential custody
```

The Connection Hub, extension popup, Quick Actions, menus, status surfaces, confirmations, icons, colors, responsive layouts, pointer support, and accessibility work should remain.

The rule is not:

```text
no GUI
```

The rule is:

```text
terminal-first
keyboard-complete
overlay-oriented
command-authoritative
pointer-optional
capability-restricted
```

That is the product model that best matches the architecture and vision.

---

## 80. Condensed Final Checklist

Before enabling managed SSH/provider execution broadly:

- [ ] Isolate managed OpenSSH from unreviewed ambient config.
- [ ] Make Process Broker the only process-launch authority.
- [ ] Bind reviewed executable identity to the spawned executable.
- [ ] Sanitize provider child environments.
- [ ] Remove dependence on mutable global provider context.
- [ ] Revalidate real provider identity before sensitive execution.
- [ ] Use structured commands as authority.
- [ ] Add shell-specific insertion rendering.
- [ ] Harden terminal/control-character handling.
- [ ] Bind explicit environment risk.
- [ ] Keep capsules free of secrets.
- [ ] Centralize authentication state transitions.
- [ ] Formalize the overlay framework.
- [ ] Require keyboard-complete UX.
- [ ] Keep pointer support optional.
- [ ] Route all UI actions through commands.
- [ ] Prevent extensions from arbitrary rendering.
- [ ] Formalize first-party extension APIs.
- [ ] Narrow extension network/process capabilities.
- [ ] Harden broadcast against secret leakage.
- [ ] Keep tunnels typed.
- [ ] Keep OpenBao blocked until ADR acceptance.
- [ ] Prefer SSH agents for password-manager integration.
- [ ] Bind SSH agent identity.
- [ ] Keep agent forwarding disabled by default.
- [ ] Turn OpenSSH security policy into executable release checks.
- [ ] Add controlled activation tiers.
- [ ] Activate providers independently.
- [ ] Prove OS-native child cleanup.
- [ ] Enforce structured async ownership.
- [ ] Keep channels bounded.
- [ ] Keep Tokio out of rendering hot paths.
- [ ] Enforce a strict Rust unsafe-code policy.
- [ ] Strongly type authority IDs.
- [ ] Redact secrets by construction.
- [ ] Scale-test Quick Actions.
- [ ] Define UX performance budgets.
- [ ] Actually run fuzz campaigns.
- [ ] Expand native Windows/Linux/macOS testing.
- [ ] Build controlled provider fixtures.
- [ ] Run real MFA/browser campaigns.
- [ ] Make QA evidence durable.
- [ ] Generate status docs from one source of truth.
- [ ] Add commit stamps to historical audits.
- [ ] Remove committed workstation-local paths.
- [ ] Freeze integration head before final protected approval.
- [ ] Separate DCO from cryptographic provenance.
- [ ] Harden Rust dependency governance.
- [ ] Use staged release rings.
- [ ] Complete stable-release evidence before GA.

---

**End of document**


## 81. Verification and Benchmarking Playbook

This section defines **how every implemented Automexia capability must be proven functional**, not merely compiled.

The verification philosophy is:

```text
Source exists
   ≠
Feature works

Unit tests pass
   ≠
Native integration works

Native integration works once
   ≠
Release-ready

Release-ready
   =
correctness
+ security
+ native behavior
+ performance
+ resource bounds
+ accessibility
+ failure handling
+ repeatable evidence
```

Every feature should therefore have a verification record covering:

```text
Functional correctness
Negative/failure behavior
Security invariants
Cross-session isolation
Cancellation
Cleanup
Persistence/restart behavior
Accessibility
Performance
Memory/resource bounds
Cross-platform behavior
Real integration behavior
```

---

## 82. Required Test Layers

Automexia should use a layered test strategy.

```text
Layer 1   Compile/static checks
Layer 2   Unit tests
Layer 3   Property/model tests
Layer 4   Contract tests
Layer 5   Component integration tests
Layer 6   Hostile-input/fuzz tests
Layer 7   Native OS tests
Layer 8   Real provider/tool tests
Layer 9   Manual UX/accessibility validation
Layer 10  Performance benchmarks
Layer 11  Stress/soak/leak campaigns
Layer 12  Release-candidate validation
```

A feature should not be promoted directly from unit tests to Stable.

---

## 83. Layer 1 — Static and Compile-Time Validation

Run on every pull request.

Minimum checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --doc --locked
cargo nextest run --workspace --locked --profile ci
```

Also enforce:

```text
dependency policy
architecture dependency rules
forbidden process-launch imports
unsafe-code policy
secret scans
local-path scans
activation-state invariants
documentation link checks
schema validation
license/provenance policy
```

### Acceptance

A PR cannot merge if:

- formatting differs,
- warnings exist,
- architecture boundaries fail,
- provider crates directly launch managed processes,
- source introduces a forbidden secret/log pattern,
- generated status documentation is stale,
- activation defaults change unexpectedly.

---

## 84. Layer 2 — Unit Tests

Every pure or mostly pure Rust function should have focused tests.

Examples:

```text
SSH destination validation
host/user/port parsing
risk classification
capsule transitions
generation invalidation
argument construction
environment filtering
shell quoting
resource normalization
provider output parsing
receipt redaction
command-plan hashing
overlay navigation
focus movement
```

Tests must cover:

```text
valid minimum
valid maximum
empty
boundary length
one above maximum
Unicode
malformed Unicode boundary cases where applicable
control characters
duplicate values
stale IDs
wrong provider/session
wrong generation
```

### Naming

Prefer behavioral names:

```rust
#[test]
fn changed_host_key_is_blocked() {}

#[test]
fn stale_capsule_generation_cannot_execute() {}

#[test]
fn bash_renderer_quotes_newline_as_data_or_rejects_it() {}
```

Avoid generic names such as:

```rust
test_1()
works()
```

---

## 85. Layer 3 — Property and Model Tests

Use property testing for data-heavy invariants.

High-value targets:

```text
bounded strings
resource IDs
provider contexts
shell renderers
SSH argument plans
kubeconfig merging
capsule state transitions
generation ordering
overlay selection models
route ownership
```

Properties should include:

```text
parse(render(x)) preserves supported structured data
invalid characters never appear unescaped in shell output
stale generation is never accepted
resource normalization is deterministic
merging order is deterministic
redaction never exposes source secret
serialization round trips preserve identity
```

For state machines, generate random event sequences and assert that illegal authority transitions never occur.

---

## 86. Layer 4 — Contract Tests

Each security-sensitive feature should expose a stable machine-readable contract fixture.

Examples:

```text
SSH launch contract
provider-auth contract
Quick Actions CP4 contract
extension capability contract
overlay keyboard contract
risk policy contract
```

Contract tests should prove:

```text
allowed decisions remain allowed
denied authority remains denied
required fields cannot disappear
schema versions are explicit
backward-incompatible changes require review
```

For security contracts, include explicit negative fixtures.

Example:

```json
{
  "attempt": "provider_action_from_stale_generation",
  "expected": "deny"
}
```

A contract change should be reviewed as an architecture/security change, not merely a snapshot update.

---

## 87. Layer 5 — Component Integration Tests

Test real components together without requiring public cloud infrastructure.

Examples:

```text
ProcessBroker + fake executable
CredentialBroker + fake credential provider
ResourceIndex + fake DevOps provider
Overlay + command registry
Session supervisor + test PTY child
ProviderAuth + controlled fake browser callback
```

The fake executable should record:

```text
argv
environment
cwd
stdin
signals
exit behavior
```

so tests can assert the exact launch.

Never rely only on mocking the ProcessBroker method itself.

The process boundary should be tested.

---

## 88. Layer 6 — Fuzz and Hostile-Input Testing

Compilation of a fuzz target is not a fuzz campaign.

Track separately:

```text
FUZZ BUILD
FUZZ EXECUTION
FUZZ DURATION
FUZZ CORPUS SIZE
CRASHES
REGRESSION CORPUS
```

### Priority fuzz targets

#### Terminal

```text
VT parser
OSC
DCS
APC
CSI
Unicode/graphemes
graphics protocol
clipboard control sequences
hyperlinks
shell integration sequences
```

#### SSH

```text
host input
user input
port input
ProxyJump model
host-key metadata
ssh -V parser
ssh -Q parser
ssh-agent listing parser
```

#### Providers

```text
AWS JSON/text
Azure JSON/text
gcloud JSON/text
Teleport status/version output
kubeconfig YAML
OpenShift config
provider expiry timestamps
browser callback parameters
```

#### Product

```text
Quick Action metadata
shell renderers
Windows cmd wrapper quoting
resource search input
capsule serialization
IPC framing
extension manifests
```

### Campaign policy

For every release candidate:

```text
short PR fuzz smoke
+
nightly longer campaign
+
weekly extended campaign
+
release-candidate campaign
```

Any discovered crash becomes a permanent regression input.

---

## 89. Layer 7 — Native OS Testing

The actual native process and terminal paths must run on:

```text
Windows
Linux
macOS
```

Do not infer Linux/macOS correctness from Windows unit tests.

### Windows native matrix

Test:

```text
ConPTY creation
resize
UTF-8
Ctrl+C
Ctrl+Break where relevant
child exit
job-object cleanup
forced process-tree cleanup
cmd wrapper execution
PowerShell insertion rendering
high-DPI overlay scaling
Windows UI Automation
```

### Linux native matrix

Test:

```text
PTY allocation
process groups
SIGTERM/SIGKILL
Wayland
X11 where supported
Bash/Zsh/Fish
Vulkan/software fallback where applicable
AT-SPI
```

### macOS native matrix

Test:

```text
PTY allocation
process-group cleanup
Zsh
Metal renderer
Retina scaling
fullscreen/window transitions
VoiceOver
code-signed packaged build
```

---

## 90. Layer 8 — Real Provider and Tool Testing

Source-complete provider adapters are not sufficient.

Run controlled real tool campaigns for:

```text
OpenSSH
AWS CLI
Azure CLI
gcloud
kubectl
oc
tsh
```

Test with dedicated non-production resources.

Every provider campaign must include:

```text
happy path
expired authentication
wrong identity
offline
network loss
timeout
user cancellation
process crash
stale configuration
revocation
disable/uninstall
relaunch/restart
cross-session isolation
```

---

## 91. Layer 9 — Manual UX Testing

Automation does not fully prove human interaction quality.

Each overlay should have a manual script.

Example generic script:

```text
1. Open overlay only by keyboard.
2. Navigate every item using keyboard only.
3. Trigger every primary action without mouse.
4. Close with Escape.
5. Reopen and verify focus restoration.
6. Resize from tiny window to large/4K/8K.
7. Repeat with 125%, 150%, 200% scaling where supported.
8. Repeat using pointer as optional convenience.
9. Verify pointer does not expose actions missing from keyboard.
10. Verify disabled actions explain why.
11. Verify production/high-risk state is visible without relying on color.
12. Verify long provider/resource names truncate safely.
13. Verify Unicode and RTL/bidi cases do not spoof action text.
14. Verify screen-reader announcement.
```

The result should be recorded against the exact commit.

---

## 92. Layer 10 — Benchmarking Principles

A benchmark must answer a product question.

Bad benchmark:

```text
function X takes 10 µs
```

without context.

Good benchmark:

```text
Filtering 10,000 cached resources after each keystroke
has p95 < one frame and performs zero provider processes/network calls.
```

Benchmark three categories:

```text
Microbenchmarks
Component benchmarks
End-to-end user-visible benchmarks
```

---

## 93. Benchmark Reproducibility

For every benchmark record:

```text
commit SHA
Rust toolchain
OS/build
CPU
RAM
GPU for rendering tests
power mode
debug/release profile
warm/cold state
dataset size
sample count
benchmark command
median
p95 where meaningful
variance/confidence
memory measurements
```

Do not compare:

```text
different machines
different power modes
debug vs release
cold vs warm
```

as if they were identical baselines.

Longitudinal performance gates should use stable controlled runners.

---

## 94. Microbenchmark Policy

Use microbenchmarks for pure hot functions such as:

```text
resource filtering
fuzzy scoring
capsule validation
risk revalidation
argument construction
shell quoting
provider parsing
overlay layout computation
VT parsing
damage computation
```

Measure:

```text
small
typical
large
hostile/worst-case
```

datasets.

Avoid benchmarks that the compiler can optimize away.

Use realistic inputs and consume results through the benchmark harness.

---

## 95. End-to-End Benchmark Policy

Measure user-visible flows.

Examples:

```text
application process start → terminal interactive
global shortcut → overlay first visible frame
keypress → search result highlight
Enter → reviewed plan constructed
approval → child process started
child output → terminal visible
provider refresh → cached status visible
extension command → extension activated
```

These matter more than isolated functions.

---

## 96. Performance Regression Gates

Create stable baselines for important metrics.

Example policy:

```text
latency regression > 5%
→ investigate / block according to metric criticality

memory regression > 10%
→ investigate

new unbounded allocation
→ block

provider process on keystroke path
→ block immediately
```

Use statistical comparison rather than a single measured run.

---

## 97. Memory and Resource Benchmarking

Track:

```text
RSS
private bytes / working set where appropriate
handles
file descriptors
threads
child processes
PTYs
GPU memory
open sockets
temporary files
leases/routes
```

Measure:

```text
idle
1 session
10 sessions
50 sessions
1 provider refresh
repeated connect/disconnect
repeated overlay open/close
```

The key question is not only peak usage.

It is:

```text
does usage return to baseline after cleanup?
```

---

## 98. Soak and Leak Campaigns

Run long-lived tests.

Examples:

```text
8-hour terminal idle
8-hour active output
24-hour provider refresh
1,000 connect/disconnect cycles
10,000 overlay open/close cycles
continuous resize
continuous session creation/destruction
repeated authentication expiry/refresh
```

Track slope over time.

A small steady leak that unit tests cannot detect is still a release blocker.

---

## 99. Failure Injection

Every managed feature must be tested while dependencies fail.

Inject:

```text
child hangs
child exits immediately
child writes malformed output
child writes huge output
network disappears
DNS stalls
provider token expires
browser never returns
credential provider locks
SSH agent disappears
SSH agent changes
filesystem becomes read-only
disk becomes full
IPC peer crashes
extension crashes
renderer restarts
```

The expected behavior should be explicit.

No failure should silently expand authority.

---

## 100. Managed SSH — Automated Functional Test Matrix

### Strict configuration authority

Automated test:

1. Create a temporary fake home.
2. Write a hostile `.ssh/config`.
3. Add a `ProxyCommand` or `LocalCommand` that writes a marker file.
4. Launch strict managed SSH.
5. Assert marker file does not exist.
6. Assert launched arguments/config use only the managed authority.

Expected:

```text
ambient hostile SSH config has zero effect
```

### Executable replacement

1. Review executable A.
2. Replace path with executable B.
3. Attempt spawn.
4. Assert denial.

### Agent replacement

1. Review agent socket A.
2. Change `SSH_AUTH_SOCK` to B.
3. Execute.
4. Assert stale-plan denial.

### Host key

Test:

```text
first use → review
known key → allowed
changed key → blocked
known_hosts corruption → safe failure
```

### Cancellation

Cancel during:

```text
DNS
connect
authentication
active shell
```

Verify owned descendants terminate and routes/leases disappear.

---

## 101. Managed SSH — Manual Native Test Script

For each supported OS:

```text
1. Connect to controlled OpenSSH server.
2. Authenticate using password.
3. Authenticate using encrypted private key.
4. Authenticate using SSH agent.
5. Authenticate using certificate.
6. Trigger first-use host key.
7. Reconnect with known key.
8. Change server key and verify hard block.
9. Disconnect server abruptly.
10. Cancel during connection.
11. Kill client child externally.
12. Resize terminal repeatedly.
13. Run full-screen TUI remotely.
14. Produce high-volume output.
15. Run Unicode/emoji/combining-character output.
16. Exit normally and verify exact exit status.
17. Verify no child/PTY/route remains.
18. Verify logs contain no password/passphrase/private material.
```

Repeat with 1, 10, and 50 parallel sessions.

---

## 102. Managed SSH — Benchmarks

Measure:

```text
plan validation
executable identity verification
review → child spawn
PTY attach
first remote output
steady-state output throughput
CPU during idle SSH
memory per session
cleanup latency
```

Datasets:

```text
1 session
10 sessions
50 sessions
```

Also benchmark large `known_hosts` files if Automexia parses or presents them.

---

## 103. SSH Routes / ProxyJump Testing

Automated cases:

```text
single jump
multiple jumps
duplicate jump
loop in route
invalid port
IPv6 jump
long hostname
changed jump-host key
stale route generation
```

Security requirement:

```text
free-form ProxyCommand remains unavailable in strict managed mode
```

Manual test:

```text
client → bastion → target
```

then kill:

```text
target
bastion
client network
```

separately and verify correct cleanup/recovery.

---

## 104. Tunnel Testing

For Local, Remote, and Dynamic tunnels test:

```text
loopback listener
requested port occupied
listener becomes occupied between review/start
IPv4
IPv6
remote target offline
session closes
lease expires
cancellation
process crash
```

Manual validation:

```text
1. Start tunnel.
2. Confirm exact listening endpoint.
3. Send test traffic.
4. Confirm traffic reaches only expected target.
5. Close session.
6. Confirm listener no longer exists.
```

For non-loopback/remote forwarding verify the stronger confirmation is unavoidable.

Benchmark:

```text
tunnel setup latency
throughput
CPU overhead
cleanup latency
10/50 concurrent tunnels
```

---

## 105. Provider-Neutral Authentication Testing

Test every one of the internal authentication states.

For each state prove:

```text
allowed next events
denied next events
cached status text
recovery action
redaction
expiry behavior
generation behavior
```

Generate random transition sequences and verify illegal transitions never grant `Ready`.

Test:

```text
refresh finishes after revocation
browser completes after cancellation
old generation returns after rebind
provider disabled while authenticating
shutdown during callback
```

Every late completion must be discarded.

Benchmark:

```text
bind/read 1 capsule
64 capsules
1,000 capsules if plausible
state transition
cache lookup
generation invalidation
```

---

## 106. AWS Testing

### Automated controlled tests

Test conflicting ambient credentials:

```text
capsule = staging
environment credentials = another account
```

Expected:

```text
ambient credentials removed or explicitly rejected
```

Test:

```text
valid SSO session
expired SSO session
wrong account
wrong role
wrong region
browser cancelled
network offline
STS denied
SSM target unavailable
EKS context stale
```

Before sensitive execution, assert fresh identity matches expected account/principal.

### Manual AWS test

```text
1. Select test account.
2. Authenticate using real IAM Identity Center.
3. Cancel browser login once.
4. Retry successfully.
5. Let session expire.
6. Attempt action.
7. Confirm recovery flow.
8. Switch identity outside Automexia.
9. Attempt stale action.
10. Confirm rejection/review.
```

### Benchmarks

```text
cached account search
capsule construction
STS identity check
plan generation
SSM launch preparation
EKS plan preparation
```

Do not count provider network latency as Automexia CPU performance; report separately.

---

## 107. Azure Testing

Test:

```text
wrong ambient active subscription
wrong tenant
expired authentication
browser cancellation
device-code flow where supported
Bastion target unavailable
AKS context mismatch
AZURE_CONFIG_DIR conflict
```

Managed plans must remain tied to explicit tenant/subscription.

Manual:

```text
authenticate
switch active subscription outside Automexia
execute previously prepared action
verify stale/mismatch rejection
```

Benchmark:

```text
subscription cache search
context validation
plan construction
Azure CLI spawn preparation
```

---

## 108. Google Cloud Testing

Test conflicts in:

```text
active gcloud configuration
project
account
CLOUDSDK_CONFIG
CLOUDSDK_* overrides
```

Test:

```text
expired auth
browser cancellation
IAP denial
OS Login denial
GKE credentials/context mismatch
offline behavior
```

Managed action should never mutate global active configuration merely to execute.

Benchmark:

```text
configuration cache search
capsule validation
IAP/OS Login plan construction
GKE plan construction
```

---

## 109. Kubernetes/OpenShift Testing

### Hostile kubeconfig corpus

Include:

```text
huge files
duplicate contexts
duplicate clusters
duplicate users
malformed YAML
anchors/aliases if parser supports them
unknown fields
very long names
control characters
relative paths
symlinks
exec plugins
conflicting files
```

Verify deterministic merge and explicit source grants.

### Exec plugins

Test:

```text
unknown executable
replaced executable
unexpected arguments
unexpected environment
stale review
timeout
malformed credential output
```

### Manual Kubernetes and OpenShift test

```text
1. Connect to controlled cluster.
2. Switch global kubectl current-context outside Automexia.
3. Run managed action.
4. Verify Automexia still uses exact reviewed kubeconfig/context.
5. Expire credentials.
6. Verify recovery.
7. Test kubectl exec.
8. Test OpenShift rsh/exec as applicable.
```

Benchmark:

```text
kubeconfig parse
merge 1/10/100 files
context search 100/1k/10k
plan construction
```

---

## 110. Teleport Testing

Test:

```text
valid login
expired login
wrong proxy
wrong cluster
environment override attempt
agent override attempt
browser cancellation
MFA failure
offline
tsh process crash
stale certificate expiry
logout/revocation
```

Manual controlled flow:

```text
1. tsh login through Automexia.
2. Complete real MFA.
3. Connect to controlled node.
4. Let certificate/session expire.
5. Attempt stale Quick Action.
6. Verify review/re-authentication.
7. Logout outside Automexia.
8. Verify cached state is not treated as executable authority.
```

Benchmark:

```text
cached status parse
expiry validation
plan construction
search over node cache
```

---

## 111. OpenBao Testing Requirements Before Implementation Activation

Once ADR 0024 is accepted and implementation exists, tests must cover:

```text
login
token helper ownership
locked/missing helper
lease renewal
lease expiry
revocation
server unavailable
server restarts
certificate issuance
certificate cleanup
stale certificate
cross-session certificate isolation
disable/uninstall cleanup
```

Secret tests must prove:

```text
token never enters logs
token never enters normal SQLite metadata
token never enters Quick Action cache
extension receives handle rather than raw token unless explicitly authorized
```

Benchmark:

```text
cached status
lease tracking
certificate plan construction
cleanup
```

Network issuance latency should be reported separately from local CPU overhead.

---

## 112. Provider-Aware Quick Actions Testing

This feature needs unusually strong automated coverage because it sits on the keystroke path.

### Keystroke invariant

Instrument process launches.

Test:

```text
type 10,000 search keystrokes
```

Assert:

```text
provider process launches = 0
network calls = 0
credential materializations = 0
```

### Generation

Test:

```text
query A starts
query B supersedes A
A completes late
```

Expected:

```text
A discarded
```

### Final authorization

Prepare action, then mutate:

```text
resource generation
provider identity
risk
capsule
route
session
```

Attempt action.

Expected:

```text
revalidation fails
```

### Insertion

For every supported shell test:

```text
spaces
quotes
Unicode
shell metacharacters
newlines
control chars
hostile provider names
```

Assert:

```text
no automatic Enter
no hidden newline
no extra command
```

### Benchmark scale

Run:

```text
16
100
1,000
10,000
50,000
```

cached actions.

Measure:

```text
snapshot creation
filter
ranking
first visible result
allocation
memory
p95/p99
```

---

## 113. Overlay Framework Testing

Every overlay must have automated tests for:

```text
keyboard-only completion
Escape close
focus restoration
selection movement
PageUp/PageDown
Home/End where applicable
search typing
IME handling
paste handling
disabled item
empty state
error state
loading state
resize
small viewport
large viewport
DPI scaling
```

Pointer tests are additional:

```text
click selection
double-click action
wheel
hover if supported
```

The pointer path must invoke the same `CommandId` as keyboard.

---

## 114. Overlay Accessibility Testing

Automated semantic-tree tests should assert:

```text
correct roles
names
states
selection
focus
disabled state
risk text
provider text
```

Manual controlled validation should include:

```text
Windows: Narrator and NVDA where supported by release policy
macOS: VoiceOver
Linux: Orca
```

For each overlay:

```text
open
announce title
navigate
announce selection
announce status/risk
execute
restore focus
```

No critical meaning may depend solely on color/icon.

---

## 115. Overlay Rendering Benchmarks

Measure:

```text
open → first frame
keyboard event → selection frame
search event → filtered frame
resize → stable frame
large list scrolling
```

Datasets:

```text
10 items
100
1,000
10,000
```

Track:

```text
CPU
allocations
frame time
GPU time if available
memory
```

Benchmark at:

```text
100%
150%
200% scale
4K
high-density display
```

where practical.

---

## 116. Extension Architecture Testing

First-party Rust extensions must be tested as if they were external consumers of the extension API.

Architecture tests should fail if extension crates reach into forbidden internals.

Test capability denial:

```text
extension requests process authority it lacks
extension requests credential raw material
extension requests forbidden network access
extension requests raw renderer
```

Expected:

```text
deny
```

Test crash behavior:

```text
extension panics/crashes
```

Expected:

```text
terminal remains alive
session remains alive
extension can be disabled/restarted
```

Future WASM tests must additionally cover:

```text
memory limit
fuel/CPU limit
infinite loop
trap
malformed component
invalid WIT version
```

---

## 117. Extension Activation Benchmarking

Measure:

```text
terminal startup with extension installed but inactive
extension lazy activation
first command
second command
memory after activation
memory after disable/unload where supported
```

Hard requirement:

```text
installed-but-unused extensions must not materially damage startup.
```

---

## 118. Credential Broker Testing

Test:

```text
locked provider
unlocked provider
missing provider
expired handle
revoked handle
wrong session
wrong provider
wrong resource
stale generation
shutdown
```

Redaction tests should inject canary secrets and scan:

```text
logs
SQLite
receipts
crash output
clipboard
telemetry fixtures
Quick Action snapshots
```

Expected:

```text
0 unauthorized canary occurrences
```

---

## 119. OS Secure Store Testing

For local secure storage adapters test:

```text
write
read
update
delete
locked state
user denial
restart
wrong user
corrupted reference
```

Do not test only API return values.

Verify no plaintext secret appears in Automexia metadata files.

---

## 120. Broadcast Testing

Automated security test:

```text
arm broadcast
simulate secure/password input state in one session
send secret-like keystrokes
```

Expected:

```text
secret is not broadcast
```

Test automatic disarm on:

```text
target change
session reconnect
workspace generation
risk change
resource replacement
```

Manual:

```text
broadcast a harmless command to 2, 10, 50 controlled sessions
verify exact target list and isolated outcomes
```

Benchmark:

```text
dispatch latency
CPU
memory
50-target bounded behavior
```

---

## 121. Session Supervisor Testing

Test:

```text
normal exit
non-zero exit
signal termination
PTY failure
child hang
shutdown
UI restart
session-host restart policy
```

If persistent sessions are supported, test:

```text
UI process dies
session continues
UI relaunches
reattach
terminal state correct
```

Verify no false-success receipts.

---

## 122. IPC Testing

Test:

```text
valid request
unknown message version
truncated frame
oversized frame
duplicate request ID
cancelled request
peer disconnect
slow peer
out-of-order completion
```

Benchmark:

```text
round-trip latency
throughput
large terminal-state messages
control-message latency during output flood
```

High-frequency terminal traffic must not starve control messages.

---

## 123. VT / Terminal Engine Testing

Use:

```text
known VT fixtures
differential tests where possible
fuzzing
real TUI applications
```

Manual applications:

```text
vim/neovim
tmux
htop/btop
less
fzf
shell prompts
Unicode test tools
```

Test:

```text
alternate screen
mouse reporting
bracketed paste
focus reporting
true color
hyperlinks
cursor styles
wide characters
combining characters
emoji
reflow
scroll regions
```

Benchmark:

```text
bytes parsed/sec
large scrollback
reflow
damage generation
high-output commands
```

---

## 124. Renderer Testing

Automated:

```text
golden screenshots for deterministic scenes
damage-region tests
glyph-cache tests
resize tests
font fallback tests
```

Manual:

```text
Intel/AMD/NVIDIA where required
RDP
software fallback
HiDPI
multi-monitor
window resize storms
fullscreen
```

Benchmarks:

```text
frame time
GPU time
CPU render preparation
glyph cache hit rate
VRAM
scroll performance
```

Do not gate rendering only on screenshots; semantic terminal state should have independent tests.

---

## 125. Input Testing

Test:

```text
normal keys
modifiers
AltGr
IME
dead keys
composition
paste
bracketed paste
function keys
key repeat
global shortcuts
application key protocols
```

Product overlay shortcuts must not accidentally consume modified keys intended for terminal applications unless explicitly bound.

Benchmark:

```text
input event → PTY write
input event → overlay selection update
```

---

## 126. Resource Index Testing

Test:

```text
duplicate resources
provider removal
provider disable
resource rename
stale generation
large metadata
Unicode names
risk updates
```

Benchmark datasets:

```text
100
1,000
10,000
50,000
100,000 if architecture targets enterprise scale
```

Measure:

```text
insert/update
remove
full snapshot
search
filter
memory
```

---

## 127. Workspace / Recipe Testing

Automated test each execution stage:

```text
Resolve
Preflight
Authenticate
Connect
Initialize
Verify
Ready
Cleanup
```

Inject failure at each stage.

Verify:

```text
deadline
cancellation
retry bound
cleanup
receipt
generation
```

Test:

```text
--no-hooks
migration
CAS conflict
import/export
clone/rebind
restore
```

Never automatically restore credential authority or destructive operations without fresh review.

---

## 128. Persistence and Migration Testing

Every schema migration should be tested from:

```text
old supported version
current version
corrupt database
partial migration
interrupted write
```

Test crash between:

```text
write temporary
fsync
rename
```

where persistence requires atomicity.

Maintain fixture databases for old versions.

---

## 129. Documentation and Status Testing

Documentation itself should be machine-checked.

Test:

```text
relative links valid
no D:/ developer-local paths
status generated from current source
commit stamp present
ADR index matches files
feature matrix matches project-status
activation defaults match docs
```

Fail CI when generated status docs are stale.

---

## 130. Packaging and Signing Testing

Before Stable:

```text
install fresh
upgrade
downgrade policy
repair
uninstall
reinstall
signature verification
revoked package
corrupt package
tampered executable
```

Verify uninstall does not delete user-owned:

```text
SSH config
provider credentials
manual cloud configuration
```

unless explicitly selected.

---

## 131. Security Regression Suite

Create a named suite specifically for authority failures.

Include:

```text
stale generation accepted
wrong session accepted
wrong provider accepted
executable replaced
agent replaced
risk downgraded
ambient AWS credentials
ambient Azure subscription
ambient gcloud config
ambient kubeconfig
unreviewed exec plugin
shell insertion newline
Windows cmd injection
changed SSH key
credential logged
extension process escape
provider process on keystroke
```

Every past security bug should become a permanent regression test.

---

## 132. Manual Release-Candidate Checklist

Before promoting a build:

```text
[ ] clean install on Windows
[ ] clean install on Linux
[ ] clean install on macOS
[ ] terminal opens
[ ] shell works
[ ] overlays keyboard-complete
[ ] pointer optional paths work
[ ] screen reader smoke
[ ] managed SSH controlled fixture
[ ] SSH host-key change blocked
[ ] provider auth cancellation
[ ] provider expiry recovery
[ ] Quick Action insert-without-Enter
[ ] production review
[ ] extension disable/re-enable
[ ] crash/restart recovery
[ ] uninstall preserves user-owned data
```

Record operator, commit, artifact digest, OS, and result.

---

## 133. CI Pipeline Structure

Recommended pipeline:

### PR Fast Lane

```text
format
clippy
unit
contract
architecture
docs
short property tests
fuzz build
microbenchmark smoke/no-regression sanity
```

### PR Native Required Jobs

Platform-specific targeted:

```text
Windows process/ConPTY
Linux PTY
macOS PTY
```

### Nightly

```text
full nextest
long property tests
fuzz campaigns
native provider fixtures where available
performance benchmarks
resource/leak tests
```

### Weekly

```text
long fuzz
long soak
50-session stress
renderer stress
provider expiry/revocation campaigns
```

### Release Candidate

```text
all required native platforms
real controlled provider campaigns
screen-reader validation
packaging/signing
SBOM/provenance
performance regression
long resource evidence
manual checklist
```

---

## 134. Benchmark Baseline Storage

Keep benchmark results tied to:

```text
benchmark name
commit
runner identity
dataset
toolchain
```

Do not put only a human-readable sentence in Markdown.

Store machine-readable benchmark artifacts such as:

```text
benchmarks/
  baselines/
    <runner>/
      <benchmark>.json
```

or CI artifact storage with retained baseline metadata.

The roadmap may summarize results, but the machine-readable result is authoritative.

---

## 135. Benchmark Review Rules

A benchmark regression is not automatically a bug, but it must be explained.

For each meaningful regression record:

```text
metric
old
new
percentage
variance
reason
accepted?
reviewer
```

Do not “fix” benchmark gates by blindly updating baselines.

---

## 136. Feature Verification Record Template

Every major feature should have a small verification record.

Example:

```yaml
feature: provider-aware-quick-actions
commit: a9f173d...
source: pass
unit: pass
contract: pass
property: pass
fuzz_build: pass
fuzz_execution: pass
windows_native: pass
linux_native: pending
macos_native: pending
manual_keyboard: pass
manual_pointer: pass
accessibility: partial
benchmark:
  dataset: 10000
  p50_us: ...
  p95_us: ...
activation: disabled
release_ready: false
```

This is far clearer than “fully done locally.”

---

## 137. Definition of Functionally Complete

A feature may be called **Functionally Complete** only when:

```text
happy path tested
negative paths tested
failure recovery tested
cancellation tested
cleanup tested
cross-session isolation tested
real component/native path tested where applicable
```

Unit tests alone are not enough.

---

## 138. Definition of Performance-Qualified

A feature is **Performance-Qualified** only when:

```text
representative dataset benchmarked
worst-case/large dataset benchmarked
memory bounded
no hot-path external work
baseline stored
regression threshold defined
```

---

## 139. Definition of Release-Qualified

A feature is **Release-Qualified** only when:

```text
Functionally Complete
+
Security Qualified
+
Performance Qualified
+
Native Qualified
+
Accessibility Qualified where user-visible
+
Evidence attached to exact release commit
```

---

## 140. Recommended Immediate Verification Order

For the current implementation, prioritize testing in this order:

```text
1. Managed OpenSSH ambient-config isolation
2. ProcessBroker exact executable identity
3. Provider sanitized environments
4. Windows wrapper safety
5. Quick Action structured command + shell quoting
6. AWS real controlled account
7. Azure real controlled subscription
8. GCP real controlled project
9. Kubernetes exact kubeconfig + exec plugin
10. Teleport real MFA/proxy
11. Linux native managed SSH
12. macOS native managed SSH
13. overlay keyboard/accessibility campaigns
14. 1/10/50-session resource tests
15. actual fuzz campaigns
16. long soak campaigns
```

This order attacks the highest-risk unknowns first.

---

## 141. Final Testing Principle

The core release rule should be:

> **Do not promote a feature because the code looks complete. Promote it only when the exact authority-bearing path has been exercised under normal, hostile, failure, cancellation, restart, and resource-pressure conditions, with repeatable evidence tied to the exact commit.**

For Automexia specifically:

```text
Test the typed plan.
Test the broker.
Test the real process.
Test the real PTY.
Test the real provider.
Test the real failure.
Test the cleanup.
Test the human interaction.
Measure the latency.
Measure the memory.
Keep the evidence.
```

That is how the project can move from a sophisticated implementation to a robust terminal platform.

## 142. August 2026 Ecosystem Re-Audit Addendum

The current-version hardening milestone also includes the following ecosystem updates:

### P0 — apply to the current repository

- Preserve the current extension/process ownership unless a real project ADR changes it; remove generic process/network authority where it exists and replace it with the narrowest typed current-owner capability.
- Pin the **current adopted** release Rust toolchain in reproducible builds; treat a Rust-edition/toolchain upgrade as a separate migration.
- Use release-build dependency acquisition separation plus locked/offline/network-restricted compilation where practical.
- Sanitize/bind OpenSSH askpass/agent environment.
- Bind GCP login/credential config provenance where those configuration paths are used.
- Use Kubernetes credential-plugin allow/deny controls in addition to Automexia executable-identity checks where the installed kubectl/kuberc feature set supports them.
- Treat OpenBao token-helper environment as constructed/sanitized if a real project ADR later authorizes OpenBao.
- Remove stale video assumptions from active project docs; video remains product-discovery research.

### Conditional dependency hardening — only if adopted later

- If Wasmtime becomes the real public extension runtime, add an explicit security-patch/LTS SLA and process/sandbox defense-in-depth.
- If SQLite becomes a project dependency and WAL is used concurrently, pin a version containing the relevant WAL correctness fixes and qualify backup/locking behavior.
- If Tokio is adopted for a concrete owner, pin/support it according to the selected maintenance policy rather than adding it globally.

### P1

- Use AccessKit-first accessibility plumbing while retaining native screen-reader qualification.
- Keep Automexia WIT API independent from WASI profile/version.
- Add documentation coherence CI and technology-baseline manifest checks.
- Refine KeePassXC/Bitwarden/1Password agent capabilities in CredentialBroker.
- Replace generic roadmap references to large AI/LLM work with narrowly scoped local automation only if separately justified.

## Ghostty Work Reclassification / Merge Hardening

Before merging the reported Ghostty compatibility commit or successors:

### P0 architecture checks

- `automexia` remains the default profile.
- Ghostty migration code is optional and does not become a core runtime dependency.
- Normal startup requires neither Ghostty nor Zig.
- Core/session/renderer/security crates do not depend on Ghostty-specific config/profile/action types.
- Compatibility actions resolve to Automexia typed commands and normal policy/owners.
- No unsupported Ghostty action falls through to shell execution.
- Config migration remains preview-first, bounded, non-shell-evaluating, non-networked, and atomic.
- Version-pinned profiles are immutable.

### G6 containment

Any existing parked-PTY/undo-close/topology-history code is treated as Persistent Session Supervisor source, not as Ghostty compatibility authority.

Project ADR 0028 is accepted for bounded complete top-level tab parking. For
that implemented scope:
- preserve its exact memory-only limits and cleanup owner;
- do not change close-tab default semantics;
- do not claim release qualification;
- require a separate decision for split, pane-local-tab, or native-window scope.

### Required evidence before release

```text
Ghostty fixture provenance
Windows native migration/profile
Linux native migration/profile
macOS native migration/profile
inspector keyboard/accessibility
real fuzz campaign receipts
binding lookup benchmark budgets
generated-reference drift verification
```

Persistent-session history additionally requires:

```text
1/10/50 parked session resource runs
TTL/count/cleanup
process/network visibility
credential expiry
context drift
crash/restart
native topology lifecycle
```

### Governance

An implementation agent may create a **Proposed** ADR. It may not mark a new security/lifecycle authority Accepted unless project governance explicitly authorized that exact decision.

The audited commit
`20ff7928ea2d1eac5d26f13c62d0cda9b86bc078` is committed source evidence. It
is not, by itself, native-runtime or stable-release certification.
