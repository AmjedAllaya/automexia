# Architecture

## Layers

Automexia v0.4 separates product policy from inherited terminal engines. The
provider-neutral Phase 1 extraction is now implemented without moving inherited
engine directories or creating a second PTY/process owner.

```text
apps/automexia-terminal
  product lifecycle, CLI, windows, PTY/session owner, renderer adapter
  |-- automexia-devops ------------> automexia-extension-api
  |     local providers                 bounded contracts/text policy
  |-- automexia-extension-runtime --> automexia-extension-api
  |     queue/cache/cancellation
  |-- automexia-ui-model ----------> automexia-extension-api
  |     layout/accessibility/color; contract consumer only
  `-- rio-backend / rio-vt / teletypewriter / rio-window
          config, VT/grid, PTY, platform window/event contracts
            `--> sugarloaf / rio-graphics / rio-fonts
                   GPU and font rendering engines
```

`rio_backend::config::product` is the single v0.4 compatibility adapter for
product identifiers and configuration paths. Other crates must not duplicate
Automexia IDs or path policy.

## v0.5 DevOps composition

v0.5 keeps the terminal core provider-neutral while delivering production SSH
through an optional first-party extension. The release-critical composition is:

```text
apps/automexia-terminal
  window/session/PTY owner
  renderer adapter
  application capability broker
  executable resolver and launch/cancel lifecycle
          | typed, versioned contracts only
          v
automexia-extension-api
  IDs, manifests, capabilities, launch requests, capsules, contributions,
  Unicode-safe bounded contract text normalization

automexia-extension-runtime
  bounded work queues, immutable caches, cancellation, operation lifecycle

automexia-devops
  provider-neutral context model plus current behavior-preserving adapters

automexia-ui-model
  provider-neutral status/accessibility/action models

first-party extensions
  devops-context
  devops-ssh
  later: kubernetes, openshift, aws, azure, gcp, infrastructure
          | exact approved process request; no PTY/renderer handle
          v
application capability broker
          | canonical executable + argv + trusted environment + cwd
          v
new Automexia PTY/session
          |
          v
system OpenSSH or official provider CLI
```

Phase 1 moved the pure contracts, bounded runtime primitives, current local
DevOps implementation, and renderer-independent UI policy into the four private
crates above. `apps/automexia-terminal/src/automexia/api.rs` and
`automexia/builtins/devops/mod.rs` are compatibility facades;
`automexia/runtime.rs`, renderer status paint, and `context/launch.rs` are
application adapters. Provider detection and policy may not move back into the
renderer or PTY path. The remaining capability broker and managed SSH work is
ordered in the [early DevOps/SSH delivery track](STABILIZATION-ROADMAP.md#early-devops-and-ssh-delivery-track).

### Core and extension ownership

Core owns:

- terminal engines, PTYs, sessions, routes, grids, input, scrollback, and paint;
- immutable launch descriptors and exact process/PTY/session teardown;
- generic extension types, lifecycle, capability decisions, quotas, and audit
  metadata;
- executable resolution, exact-argv validation, trusted child environment
  construction, working-directory validation, and cancellation;
- generic status-segment layout, semantics, accessibility, and details routing;
- hostile-output limits, redaction, and protected local IPC when a later
  extension host is required.

First-party extensions own provider and transport interpretation: safe
configuration parsing, version-aware public-output normalization, exact launch
request construction, inventory and context refresh, and domain-specific
workflows. They never receive renderer, VT, PTY, window, unrestricted process,
or ambient credential authority. Mature protocol, authentication, secret,
provider, and collaboration systems remain external authorities.

The complete build-versus-adopt matrix and dependency sequence are defined in
[Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md) and
[ADR 0020](adr/0020-hybrid-build-wrap-adopt-boundary.md). In particular, all
OpenSSH, provider CLI, Mosh, Git, Upterm, SOPS/age, and task-runner adapters use
one application-owned `ExternalToolRunner`; an extension may not create a
second process-launch path. The runner accepts only a canonical executable,
exact argv, validated cwd, and bounded allowlisted environment, and owns
deadlines, output caps, cancellation, descendant cleanup, session identity,
version state, and redacted diagnostics.

`ExternalToolRunner` is the logical authority, not a requirement for a new
crate, daemon, or second scheduler. It may remain in the application composition
root while ownership is singular, typed, tested, and kept off startup, input,
resize, PTY, and renderer hot paths.

### VT control-string trust boundary

Child-process output is untrusted. The VT parser caps retained OSC, APC, and
XTGETTCAP payloads, discards overflow through a terminator, treats CAN/SUB as
cancellation rather than successful dispatch, and never logs rejected payload
contents. Sixel decoding is streamed and dimension-bounded; synchronized update
storage is separately capped. Synchronized-update storage is allocated only
when the mode is used instead of reserving its 2 MiB ceiling per pane. After a
sequence completes, unusually large OSC, APC, and synchronized-update
allocations are released; normal OSC spills up to 64 KiB, APC chunks up to
8 KiB, and synchronized updates up to 64 KiB retain capacity for reuse. The
architecture gate requires deterministic boundary/recovery/allocation tests,
the construction benchmark, and the nightly mixed-control-string fuzz target.

### Image protocol and quick-look boundaries

Sixel, Kitty Graphics, and iTerm2 OSC 1337 remain VT/application protocols:
untrusted PTY bytes are parsed into bounded graphics state, snapshotted by the
owning route, and projected by the frontend renderer into Sugarloaf overlays.
Placements retain pane clipping, scroll/history, alternate-screen, clear, and
texture-eviction semantics. iTerm2 decoding validates base64/declared size,
4096x4096 dimensions, and a 96 MiB decoder allocation before constructing a
graphic.

Local filename quick look is a separate Automexia-owned overlay described by
[ADR 0014](adr/0014-explicit-bounded-image-quick-look.md). Pointer hit testing,
listing-glyph removal, quoted/Unicode candidate normalization, bounded file
opening/decoding, and the thumbnail cache live in the private, renderer-free
`automexia-image` crate. Only its pure candidate/token functions run on the
application thread; they perform no filesystem access. Stable plain hover
submits after 100 ms; click pins the card; arrows browse only while pinned;
Escape dismisses.
Mouse-reporting children keep ownership unless Shift is held, and a pane-local
latch consumes both halves of a preview click. A 16-owner latest-request queue
coalesces obsolete work per window; one worker performs local
metadata/read/header-gate/decode/downscale and publishes only a current
generation to that window's single-result mailbox through an exact-route wake.
There is no shared completion queue.
A 16-entry/32 MiB access-ordered cache validates path, length, and modification
version before returning a shared thumbnail. Sugarloaf receives the same pixel
allocation with a stable preview key/time, uploads at most 1280x960, and
positions it from the current pane rectangle. Dismissal removes the active
overlay/data entry; bounded CPU/GPU caches retain reusable content only until
normal eviction. It does not mutate terminal cells, PTY size,
selection, scrollback, or prompt metadata. The decoder fuzz target depends on
this pure crate rather than the GUI/frontend graph, keeping nightly sanitizer
builds small and free of window-system dependencies.

### Runtime configuration transaction

A file-watch event builds a candidate config, applies platform/theme validation,
prepares a replacement font library, validates all global hotkeys, and attempts
registration changes before committing live config/window state. Any fallible
preparation error retains the last-known-good application generation and does
not recreate PTYs. Hotkey replacement registers additions before removals and
uses compensating rollback with explicit diagnostics for an OS rollback failure.
Reload events execute serially on the application event loop.

Extensions own:

- domain configuration parsing and inventory;
- provider-specific commands and official authentication flows;
- connection/capsule templates and bounded public metadata;
- context contributions, freshness, errors, and typed user actions;
- exact requests to launch OpenSSH/provider CLIs;
- later, explicitly granted provider API adapters outside renderer/VT code.

Core never depends on OpenSSH parsing libraries, cloud provider SDKs,
Kubernetes/OpenShift clients, Termix, Electron/Node, or AI orchestration code.
Typing `ssh`, `kubectl`, `oc`, `aws`, `az`, or `gcloud` manually
remains ordinary shell/PTY behavior with every extension disabled.

### Managed SSH data flow

```text
user selects connection
  -> devops-ssh resolves one non-secret ConnectionRecord
  -> extension submits typed LaunchRequest
  -> capability broker validates publisher/grant/executable/argv/cwd/policy
  -> application creates independent Context + route + PTY + capsule
  -> application starts the canonical system OpenSSH executable
  -> OpenSSH resolves full user configuration/agent/keys/host-key policy
  -> remote bytes enter the normal untrusted PTY parser
  -> immutable session-scoped context snapshot wakes only that route
```

The extension never receives the inherited environment, agent protocol, private
key, passphrase, access token, PTY handle, process handle, renderer object, or
terminal history. OpenSSH is execution authority; the extension's static index
is discovery/UI metadata and cannot replace OpenSSH's complete configuration
semantics.

The window-level discovery/review surface is specified by the
[Connection Hub](CONNECTION-HUB.md). F2/D5.0 owns the pure provider-neutral Hub,
review, and planner projections in `automexia-ui-model`. D5.1 adapts those values
through one Router-owned `ConnectionHubRuntime`, a per-screen controller, and a
Sugarloaf modal. The runtime owns one joined worker, memory-only reviewed grants,
generation cancellation, D4 metadata CAS, Connection Library initialization,
and route-specific publish-before-wake; explicit shutdown cancels and joins it.
Provider extensions cannot draw approval UI, place work on render/input/VT
threads, or turn displayed labels into executable text. The Hub does not resize
a PTY, launch a process, access a network/provider, or own credentials.

The worker inbox has a fixed capacity of two and uses non-blocking submission.
When saturated it rejects the newest request with the stable redacted
`connection-worker-busy` diagnostic; obsolete queued generations are discarded
before work begins. Exact-file reviews also carry the initiating controller's
request token. Another screen can neither render, confirm, cancel, nor revoke
that memory-only grant. Catalog projections and their public summaries are
rebuilt only when the immutable catalog identity, committed query, filters, or
preferences change; unchanged renderer frames reuse the cached projection, and
IME preedit performs bounded query validation without traversing the catalog.

### F2/D5.0 non-executing connection-planning boundary

`automexia-devops::connections` is the provider-neutral owner for strict public
connection/profile/recipe documents, validation, authentication/result
reducers, canonical fingerprints, and deterministic dry-run plans.
`automexia-ui-model::connection_hub` consumes only those bounded public values
to produce renderer-neutral Hub, Connection Review, and recipe-planner view and
accessibility models.

The boundary is deliberately capability-free:

- it owns no files, persistence, processes, sockets, provider clients,
  credentials, agents, PTYs, listeners, windows, renderer objects, or GPU state;
- every resolved plan carries `execution_enabled = false` and explicit false
  process/network/provider/credential/PTY/listener authority entries;
- Enter/select opens review only; generic review/planner primary actions remain
  disabled, while direct-OpenSSH allow-once/session/deny decisions reach only
  the separately hard-disabled broker; the modal background stays inert and no
  projection requests a PTY resize;
- opaque identity/source/context references redact their debug representation,
  planner accessibility summaries expose action kinds rather than value
  contents, and serialized records accept public metadata/references only;
- validated profile/recipe wrappers have no public unchecked constructor,
  duplicate review identities and hostile plan overrides fail closed, and late
  authentication completions cannot cross the active operation generation; and
- fixed collection/byte/dependency/retry/time ceilings and fallible sequence
  conversion are checked before a plan can be projected.

D4 remains the independent static OpenSSH inventory/persistence owner. D5.1's
application adapter passes only confirmed exact grants into D4, initializes
private preference/profile state once, and renders immutable projections; D4
parsing and storage do not move into the UI model. D5.2 must pass the
application-owned capability/process/PTY/route gate before translating a
reviewed plan into an execution request. No lower crate may bypass those owners.

### M3-M4 reviewed OpenSSH route and trust boundary

`automexia-devops::connections::direct_openssh` extends the F2 owner without a
new crate edge or authority. It owns a pending preparation with no executable
identity and an identity-bound review requiring a canonical current `ssh`
executable, identity observation, route, and host-trust evidence. M3 direct and
M4 config-routed grammars share canonical F2 planning, exact `session.launch`,
an all-false authority ceiling, immutable argument vectors, and full-review
equality before any launch binding.

The desktop composition root maps either one current direct D4 record or one
transient user-entered literal host into a stable opaque public profile.
Inventory generation binds profile/capsule revision; metadata revision joins
the source revision; a domain-separated hash creates the public model ID without
copying raw record or literal values. D4 supplies only canonical first-value
comma-separated ProxyJump chains, capped at 8 hops and 2 KiB; dynamic,
ProxyCommand, ambiguous, executable, and excessive routes fail closed. The
runtime clones one bounded record under its existing lock, then performs pure
composition without I/O or new work. Literal host input is capped at 512 bytes;
optional user and decimal port are distinct validated fields. They are transient,
never persisted/history-written, and conservatively production-risk classified.

`automexia-ui-model` owns the pending and identity-bound projections plus the
literal editor's Host/User/Port/Review/Cancel reading order. While that nested
editor owns the modal, the renderer omits the redundant
top-level close action so its hit area cannot compete with the field; visible
Cancel and Escape retain deterministic dismissal. The application controller
discards or rebuilds inventory preparation
after selection, route, runtime-state, generation, catalog, or metadata changes;
literal review is isolated from those unrelated refreshes and editor state is
cleared on cancel, close, or successful preparation. The native renderer
consumes only presentation state, keeps the modal background inert, and groups
review into Connection, Safety, and Launch cards. Exact inventory aliases,
opaque references, executable digests, and fingerprints do not enter that view.
Unknown/first-use/known/changed evidence carries the complete public algorithm
and full 32-byte OpenSSH SHA-256 fingerprint. The Safety projection wraps it
without truncation; changed keys cannot bind, and no owner writes `known_hosts`.
Public agent/certificate/hardware status is an exact non-executing
`ssh-add -l -E sha256` request/parser capped at 2 seconds, 64 KiB, and 64 rows.
The `C` handoff copies the reviewed command without newline, Enter, or execution.
Because production M2 activation remains gated, approval actions end at the
protected diagnostic before executable/filesystem resolution. No process, PTY,
network, credential, listener, trust mutation, persistence, or secret authority
is granted.

The dormant post-activation M3 lifecycle keeps ContextManager as the only PTY,
process, and route owner. A separate bounded application worker observes the
exact broker executable off input, PTY, renderer, and startup hot paths. It
publishes a generation-bound result before its route wake; the controller
rejects stale preparation, source, executable, or freshness state, and launch
requires a second explicit approval of that executable-bound review. Actual
child exit, cancel, revoke, route close, and shutdown become fixed redacted
results. Windows keeps descendants in the guarded kill-on-close Job Object and
uses bounded termination; Unix retains the unreaped leader identity while
signalling the owned process group, so cleanup does not target a reused PID.
PTY reader workers publish completion and are joined with a bounded deadline.
A bounded connection worker persists only provider-neutral receipts and opaque
reconnect identity; reconnect always returns
to current D4-source validation and fresh review/approval. Connection Library and
receipt storage share a connection-owned private-filesystem adapter, not the Quick
Actions runtime, for stable Windows handle identity, native link/reparse
rejection, no-follow reads, private
permissions, identity snapshots, atomic replacement, and directory sync.

### M5 reviewed OpenSSH tunnel and native evidence boundary

`automexia-devops::connections::openssh_tunnels` extends the existing pure
connection owner. It validates and canonicalizes local, remote, and dynamic TCP
forwarding; caps each profile at the existing 32-tunnel limit; derives exact
transport, endpoint, risk, confirmation, lifetime, and OpenSSH-listener
descriptors; and keeps profile/plan/review fingerprints endpoint-sensitive. The
existing no-tunnel M3/M4 argument grammars remain byte-for-byte unchanged.

Tunnel-bearing requests use a distinct typed-direct grammar: `-F none`, fixed
defensive `-o` values, one explicit `GatewayPorts` value, exact `-L`, `-R`,
or `-D` pairs, and one literal destination. Configuration aliases and jump
routes fail closed because their effective configuration is not an owned,
reviewable input. Binds default to `127.0.0.1`; remote, non-loopback, or
production forwarding requires a fresh Allow-once decision, so a session grant
cannot silently expand listener authority. Automexia never opens a competing
socket: the approved system OpenSSH child would own every listener.

The bounded lifecycle is a pure session/generation/tunnel snapshot. Only
monotonic owner events may move planned -> starting -> ready, collision, failed,
cancelled, or closed. Stale scopes and terminal reversals are rejected; route
close terminalizes every nonterminal entry. The UI model projects exact reviewed
public endpoints, OpenSSH ownership, confirmation, and state into compact
icon/color/text rows and accessibility nodes. None of this grants process,
network, PTY, filesystem, credential, or listener authority while activation is
false.

Schema 5 preserves immutable schemas 1-4 and freezes the tunnel grammar,
lifecycle, strong-confirmation, and native-evidence rules. The bounded Python
validator accepts only exact ordered Windows/macOS/Linux manifests tied to the
current contract and source commit, rejects WSL and synthetic release evidence,
caps bytes/durations/resources, requires zero cleanup and redaction leaks, and
binds before/after manual-SSH plus disable/uninstall baselines. Its prerequisite
probe executes only fixed OpenSSH `-V` arrays with bounded output; it never
installs a server, changes a service, reads SSH configuration, or contacts a
network.

The controlled validator then binds a real manifest to the executing native OS
and normalized architecture, the exact clean requested commit, the fixed
OpenSSH client/server versions, and no-follow identity-stable SHA-256 reads of
the application binary, application package, and OpenSSH client. Files are
capped at 4 GiB; links, replacement/growth while hashing, zero-sentinel real
baselines, and path-bearing output fail closed. The manual-only F5 workflow has
read-only repository permission, a protected environment, credential-free
exact-commit checkout, and a restricted ephemeral runner group. Only its
path-free summary is uploaded. Controlled real OpenSSH, resource, and
accessibility runs remain external release evidence and do not enable the broker.

### M6 typed automation and declarative workspace boundary

M6 extends the existing provider-neutral connection model without creating a
runtime authority. `automexia-devops::connections::automation` owns pure
resolved-run review, narrow remote initialization, and lifecycle reducers;
`workspace` owns declarative layout, connection binding, restore, and armed
broadcast reducers. Both reject hostile/oversized input, bind immutable
revisions and generations, and expose `execution_enabled = false`. They contain
no filesystem, process, PTY, network, credential, provider, listener, renderer,
or clock primitive and therefore remain off startup, input, PTY, resize, and
render hot paths.

The application Connection Library remains the one persistence authority. Its
schema advances to 2 inside the established private `library.v1.json` and
`library.previous.v1.json` names so schema-1 recovery topology is not split
between competing files. A schema-1 document can only become an in-memory
migration preview; reviewed compare-and-swap persists schema 2. Editor and
import previews bind the base revision and full validated document fingerprint.
Recipe changes atomically update exact recipe references, advance dependent
profile and workspace revisions, update fingerprints, and clear every affected
approval. Stale writers, dangling/mismatched bindings, revision overflow, links,
malformed/oversized input, and exhausted bounded fresh-ID attempts fail closed.
Workspace transfer preserves only redacted topology and assigns fresh local IDs;
it never transfers live connection bindings.

A reviewed recipe run retains the fixed ten-stage order but may omit empty
stages. `NoHooks` removes recipe-origin steps and resequences only the trusted
planner resolve/connect steps. The review boundary independently revalidates
stage/action compatibility, risk, confirmation, deadline, and retry policy so a
forged resolved plan cannot downgrade privilege. Runtime state is monotonic and
per generation: every step has a bounded deadline, automatic retries require the
existing idempotent/noninteractive/nonmutating/cancellation-safe declaration,
and cancellation, shutdown, or connection replacement terminalizes obsolete
work.

Remote initialization does not expose a shell command string. It carries typed
working-directory, public-environment, `sudo`/`doas` user-switch, and
verification operations plus an explicit POSIX-sh or PowerShell dialect. A
future activated adapter must encode those types and preserve review; arbitrary
scripts/templates, terminal-cell readiness inference, hidden key injection, and
implicit Enter remain outside this capability.

A workspace stores windows, a bounded acyclic pane-split graph, and immutable
profile/recipe bindings. Restore emits a fresh reviewed generation with both
automatic reconnect and interrupted-action resume false. Broadcast stores its
exact command only in a transient redacted-debug review, caps it at 8 KiB and 50
targets, arms for at most 60 seconds, separately confirms production, and records
only per-target digest/outcome diagnostics. It never requests Enter. Structured
UI projections use redundant icon/color/text state, exact preview, alert/switch/
textbox semantics, and deterministic focus restoration without owning pixels or
execution.

The application composes, but does not duplicate, those authorities:
`connections::workspaces` binds current library fingerprints to restore, recipe,
and broadcast reviews; `workspaces_cli` owns bounded preview-first management;
the existing joined Hub worker publishes one immutable `Arc` library snapshot;
and the route-local controller/renderer owns catalog selection and restore
review. CLI file reads reject links and excess bytes, writes require both library
and entity revisions, external library replacement invalidates transient review,
and every product projection reports execution disabled and PTY input false.

[Accepted ADR 0023](adr/0023-typed-automation-and-declarative-workspaces.md)
records this durable boundary. Managed execution adapters remain gated by ADR
0012/D3/M5 protected activation, attestation, and native lifecycle evidence.
CP3.3 trusted workspace tasks stay a distinct insert-only local authority.

### Environment Capsule contract

Every managed session has a non-secret, immutable `EnvironmentCapsule`:

```text
identity/profile/role reference
account, subscription, project, tenant or organization
region and zone
kubeconfig source, context, cluster and namespace
infrastructure directory, backend and workspace
remote connection reference and transport
risk classification and local policy reference
creating extension, source revision, timestamps and freshness
```

Capsules contain opaque identity/secret references, never credential values.
A clone copies intent into a new capsule ID, starts a new PTY and performs fresh
resolution. An explicit rebind cancels old-revision work and creates/restarts a
session when environment changes cannot be safely applied in place. A profile,
subscription, project, context, or namespace switch in one session cannot
mutate another session or its historical prompt snapshots.

### Provider-neutral contribution contract

Extensions publish bounded `ContextContribution` values containing extension
and session IDs, a typed kind, bounded label/value, icon token, semantic role,
`fresh|refreshing|stale|expired|unavailable|error` freshness, observation and
expiry times, source revision, and a typed details action. Core projects these
into `StatusSegment` values and exclusively owns visual order, truncation,
contrast, accessibility, interaction geometry, and GPU drawing.

An extension cannot publish arbitrary styled terminal bytes, GPU commands,
fonts, escape sequences, hit targets, or unbounded text. Provider-specific
color mapping leaves the renderer during the v0.5 adapter migration; semantic
roles remain stable and core/theme policy chooses the final accessible color.

### M7 provider-neutral authentication capsule boundary

M7 implements the accepted [hybrid build/wrap/adopt decision](adr/0020-hybrid-build-wrap-adopt-boundary.md)
without adding a provider SDK, browser server, credential store, launcher, or
persistence owner. `automexia-devops::connections::provider_auth` is the
single pure owner of provider-context/capsule validation, public authentication
observation, exact operation/isolation/browser policy, visible-review digest,
recovery, receipt/audit redaction, and bounded session lifecycle.
`EnvironmentCapsuleTemplate` remains the existing connection-level owner and
now embeds validated public provider contexts; the extension runtime treats any
provider-context change as a rebind requiring a fresh session.

Ingress is strict and bounded: 16 MiB per public document, 64 capsules, 16
providers per capsule, 32 public scope fields, 16 browser origins, eight
capability requests, 128 exact arguments, 16 public environment names, five
minutes per requested operation, and seven days per freshness window. IDs,
public text, origins, IP-literal loopback callbacks, opaque references,
timestamps, context/provider agreement, and BLAKE3 review digests are validated.
Control and bidirectional format characters, duplicate/unknown fields, secret
flags, malformed bracketed IPv6 authorities, capabilities outside the exact
process and applicable network scope, and global provider-context mutation fail
closed. Publication validates a complete candidate snapshot and accepts only
the capsule-pinned configuration reference, provider, and risk classification;
a rejected result leaves the current operation available for cancellation.

`ProviderAuthCapsuleStore` owns public observations in memory only. Every
lookup and transition binds capsule ID, session, revision, provider, and
generation. Rebind cancels old operations before installing the fresh capsule;
late results and sibling reads are rejected. Refresh, authentication,
MFA/browser/device waits, ready, expiry, offline, denial, unsupported,
cancellation, stale/error, revocation, provider disable/uninstall, and shutdown
are explicit. Last-known-good public context survives degraded states but never
becomes proof that provider credentials remain valid.

An adapter may construct an official-CLI request only in D6.1-D6.5. M7 review
shows and binds the operation/session, ordered arguments, exact capability
requests, isolation, browser flow/origins/callback, and risk. Authorization
requires exact current `AllowOnce` decisions for the reviewed executable,
operation, session, capsule revision, process, and applicable network resource.
The approved value exposes only typed executable/argument access to the existing
D3 runner seam. M7 cannot spawn, connect, listen for callbacks, read/write a
provider file, acquire credentials, touch a token/certificate cache, own a PTY,
or render. The official CLI owns external browser/device/system-broker/MFA
behavior and closes any callback listener it creates.

Passive DevOps status now reads only bounded public local files and supplied
session metadata. It no longer starts WSL `sh -c` or provider CLIs. Hub and
palette projections consume cached records and expose text/icon/color recovery
states without performing refresh work on startup, input, PTY, resize, or
renderer paths. Provider-specific parsing, exact argv construction, product
controls, real-tool lifecycle, and native accessibility remain independent
D6.1-D6.5 owners.

The product composition remains authority-free. `apps/automexia-terminal` owns
one immutable `ProviderProductSnapshot` over exactly AWS, Azure, Google Cloud,
Kubernetes, OpenShift, and Teleport. It accepts only a completely validated M7
capsule, rejects unsupported providers before publication, caps the public scope
summary at 512 bytes, and exposes only cached identity, scope, provenance,
freshness, recovery, and risk to the renderer-neutral Hub model. A replacement
must change both capsule ID and session and increase capsule revision; reuse,
sibling publication, revoke, and shutdown fail closed or clear the catalog.

The controller and renderer add one Providers list/review route alongside
Connections and Workspaces. `C`/`W`/`P`, exact pointer targets, bounded selection,
responsive 320 px through 5K/high-scale layouts, and semantic dialog/status/row
nodes stay inside the modal. The review has no enabled action and explicitly
sets execution and PTY-input requests false. UI open, selection, pointer, key,
layout, paint, and accessibility projection therefore cannot trigger discovery,
login, network, process, filesystem, or provider work.

### M8 AWS adapter source boundary

`extensions/devops-aws` is the independently disabled D6.1 owner. It depends on
the provider-neutral M7 contracts and the dependency-free `configparser` parser;
it contains no SDK, filesystem, process, network, PTY, cache, background task,
or persistence owner. Supplied exact configuration bytes are capped at 1 MiB and
128 profiles. Only profile name, region, account, role, source-profile, and SSO
session references cross the boundary. Credential/cache/unknown values are
ignored, while duplicate profiles, hostile formatting, invalid UTF-8, and
oversized public fields fail closed.

IAM Identity Center PKCE/device login and regional STS observation are exact M7
operations. The configuration reference, capsule/session/revision, profile,
region, provider, browser origin, executable, ordered argv, process/network
capabilities, timeout, provenance, freshness, and risk are review-bound. STS
output has a separate strict 64 KiB public-only decoder. AWS CLI 2.22.0 is the
minimum recorded PKCE behavior and Session Manager plugin 1.1.17.0 is the
minimum supported plugin floor; actual executable attestation and release
compatibility remain the application/native gate.

SSM is an immutable nonactivated plan that names both official tool identities,
target, region/profile, production risk, interactive PTY, and process-tree
cleanup. It cannot execute around the D3 runner. EKS emits `update-kubeconfig
--dry-run` only; D3 activation must pass the returned code-capable kubeconfig
through M11 validation into an exact private transient source. The adapter never names,
merges, or changes the user's kubeconfig/current context. Reverting or disabling
the package removes only Automexia's AWS catalog authority and does not alter
AWS CLI configuration, credentials, or sessions.

### M9 Azure adapter source boundary

`extensions/devops-azure` is the independent, disabled D6.2 owner. It depends
only on M7, the extension API, and existing `serde_json`; it adds no Azure SDK,
filesystem, process, network, token-cache, PTY, browser, background task, or
persistence owner. An exact supplied JSON document is capped at 256 KiB, 128
accounts, 4,096 nodes, depth 32, and 4 KiB public fields. Only subscription,
tenant, cloud, state/default flag, public identity, and identity kind survive.
Duplicate subscriptions, malformed/hostile text, excessive complexity, and keys
that name tokens, client secrets, passwords, private keys, or certificates fail
closed. Caller-created public account values are revalidated at context ingress.

Login operations bind exact tenant, session/capsule/revision, official `az`
identity, ordered arguments, Microsoft login origin, system-broker, browser, or device
policy, timeout, risk, and exact process/network scopes. Status uses `az account
show --subscription ... --output json`; the adapter contains no `az account set`
or ambient `AZURE_CONFIG_DIR` mutation. Public identity kind stays visible so a
human Entra/MFA flow is not presented as password-bearing automation.

Bastion is an immutable AAD-only review plan bound to a target resource ID under
the capsule subscription. It records that Azure CLI may start a child SSH
process, requires an interactive PTY and whole-tree cancellation, and keeps
execution false behind D3. AKS splits exact arguments around an opaque private
output reference; M11 alone may resolve that reference to a newly allocated,
private, validated, lifecycle-owned transient file. No user kubeconfig or
current context is named. Disabling/reverting the package removes only the
Automexia catalog entry and does not alter Azure CLI state.

### M10 Google Cloud adapter source boundary

`extensions/devops-gcp` is the independent disabled D6.3 owner. It uses M7 and
the already-adopted dependency-free `configparser`, with no Google SDK,
filesystem, environment discovery, process, network, credential DB, PTY,
browser, background task, or persistence owner. One exact caller-supplied named
configuration is capped at 256 KiB, 64 sections, 512 entries, and 4 KiB public
fields. Only account/project/region/zone survive. Duplicate/malformed/hostile
input and credential/token/secret/password/private-key/login-config/token-file
keys fail closed; caller-created configurations are revalidated at ingress.

User browser/remote-bootstrap login and public project/IAM observation carry the
capsule configuration through exact `--configuration` arguments. Configuration,
account, project, region/zone, session/revision, executable/argv, endpoint,
browser policy, timeout, risk, and process/network scopes are review-bound. No
operation activates or changes global configuration, updates ADC, prints a
token, evaluates a shell, or uses ambient `CLOUDSDK_ACTIVE_CONFIG_NAME`.
Workforce `--login-config` and Workload `--cred-file` exist only as immutable
nonexecuting intents split around opaque private references.

IAP is bound to capsule project/zone/configuration, requires interactive PTY and
whole-tree cancellation, and leaves SSH-key and OS Login behavior with gcloud.
GKE carries an opaque M11-owned private reference and the fixed `KUBECONFIG`
environment name; it cannot name or merge a user file. All execution stays false
behind D3/M11. Disabling/reverting the package removes only its catalog entry and
does not alter gcloud configuration, credentials, SSH state, or kubeconfig.

### M11 Kubernetes and OpenShift source boundary

`automexia-devops-kubernetes` is the only M11 kubeconfig source/merge owner.
It is independently disabled and off startup, renderer, input, resize, PTY, and
passive Hub paths. Explicit user-selected files use an absolute grant and the
same stable regular-file identity/snapshot checks as other security-sensitive
reads: links/reparse points, non-regular files, files above 1 MiB, replacement,
and changes during or after review fail closed. Cloud-generated configuration
enters through a separate opaque private-transient reference and never names a
user file.

The adapter adopts `serde-saphyr` 1.1.0 with deserialization only and no include
feature. A typed schema plus duplicate-key errors, merge-key denial, one-document
limit, and explicit byte/event/alias/anchor/depth/node/scalar/comment budgets
precedes Automexia's 16-source and 256-item limits. Source order follows
`KUBECONFIG` precedence for the first current context; any duplicate cluster,
context, user, or source identifier fails as an ambiguous merge rather than
silently selecting a different identity. Only cluster origin/TLS policy,
context, user reference, namespace/project, provenance, revision, and
freshness survive. Token, password, username, certificate/key data, exec
environment values, paths, and full server paths never enter the public model.
External credential paths, auth-provider blocks, proxy routes, non-loopback
HTTP, controls/bidi, and secret-bearing exec flags fail closed.

Exec plugins are `DenyAll`. M11 can produce a nonactivated review bound to exact
executable identity and SHA-256, ordered argv, environment names, interactivity,
session, capsule revision, deadline, output ceiling, and process-tree
cancellation. It never stores an `ExecCredential` result and cannot run the
plugin. A bounded version signal surfaces native Kubernetes credential-plugin
policy only for the reviewed 1.35-1.36 client range; older clients remain under
Automexia `DenyAll` and future versions require compatibility review. A provider capsule pins the exact source-set revision, context, cluster,
user reference, namespace/project, server origin, TLS policy, provider relation,
freshness/expiry, provenance, and risk. Production rejects insecure TLS.

Immutable plans cover `kubectl auth whoami`, `kubectl config view --minify`, and
`kubectl exec` with private `KUBECONFIG`. `automexia-devops-openshift` is a
separate disabled package that depends on the shared public kubeconfig contract
but owns only `oc`: web login must target a newly allocated private transient
output, project inspection is read-only, and `rsh` pins context/project. Neither
package emits `use-context` or a mutating `oc project` action, and all execution
flags remain false behind D3. Reverting or disabling either registration does
not edit kubeconfig or CLI state.

The application `provider_transients` module is the sole private-file lifecycle
owner for cloud-generated and OpenShift output. Each manager allocates an exact
private connection subroot, creates no-follow private files, admits at most 16
active files of at most 1 MiB, validates generated kubeconfig through the M11
parser before publishing an opaque handle, and binds provider relation, capsule,
session, generation, source revision, expiry, and a content digest. Exact-path
resolution is application-private and revalidates ownership and content so
replacement or post-publication tamper fails closed. Expiry, explicit revoke,
provider disable, session revoke, shutdown, and drop remove owned files; startup
recovery examines at most 64 roots/files older than 24 hours. Public errors,
handles, and debug output disclose no path or file contents.

This local lifecycle creates no provider process or network authority. EKS, AKS,
GKE, and OpenShift operations can use it only after the protected D3 runner is
activated and supplies reviewed output. Real clients/plugins/clusters, Unix
native no-follow execution, forced process-tree cleanup, sustained resources,
controlled accessibility, packaging, signing, and release evidence remain
external activation gates.

### M12 Teleport organization-identity source boundary

`automexia-devops-teleport` is the independently disabled D6.5 Teleport owner.
It is a pure adapter over caller-supplied bytes and typed M7 capsules: no
filesystem, process, socket, browser, agent, certificate cache, PTY, worker,
persistence, renderer, startup, input, or resize authority exists in the crate.
The application registers its manifest separately with only `ProcessSpawn` and
`Network`, while every returned plan remains `execution_enabled == false`.

Status ingress is frozen to the reviewed Teleport 18.10 client schema from
`tsh status --client --format=json`. The decoder caps output at 256 KiB, JSON at
4,096 nodes and depth 16, total active-plus-profile records at 32, collection
items at 64, and public fields at 4 KiB. Benign unretained fields are discarded;
malformed structure, non-empty environment overrides, sensitive unknown key
names, userinfo/non-HTTPS proxy
URLs, controls/bidi, duplicate profiles, and expired selections fail closed.
Only normalized proxy, cluster, user, roles, logins, Kubernetes hints, RFC 3339
expiry, and official-CLI provenance survive; unretained traits and profile data
are discarded. `time` 0.3.55 parses the timestamp with only `std` and `parsing`.

Exact local plans use `tsh version --client` and `tsh status --client
--format=json`. Login, logout, and SSH bind proxy, cluster, user, capsule ID,
session, revision, configuration reference, destination, ordered argv,
capabilities, timeout/output ceilings, and whole-tree cancellation. All plans
force `--add-keys-to-agent=no` and clear Teleport/agent overrides. SSH also uses
`--relogin=false --request-mode=off`, preventing a reviewed connection from
silently reauthenticating or creating an access request. `tsh` remains the only
owner of its cache, certificates, browser, MFA, hardware keys, access policy,
and any interactive authentication.

Version support is intentionally narrow: Teleport 18.10 or later within major
18 is accepted; earlier 18.x and future majors fail closed until reviewed.
Revalidation rejects capsule, session, revision, proxy, cluster, user, and
expiry drift before a plan may reach the protected D3 seam. Logout is the only
revoke request and does not delete user-owned Teleport data. Disabling or
reverting the package removes its catalog entry only. Real `tsh`, proxy,
network, browser/MFA, cache/certificate/agent, PTY, cleanup/resource,
accessibility, packaging, signing, and release evidence remain D3/native gates.
OpenBao is not part of this authority and remains absent pending ADR 0024
acceptance.

### M13 provider-aware Quick Actions boundary

M13/CP4 is product-integrated at a nonactivating boundary. It extends the existing
Quick Action model rather than introducing a second provider-action registry.
`automexia-devops::actions::provider` owns capability-free candidate, binding,
snapshot, digest, availability, redacted audit, and final revalidation types.
Each accepted first-party provider owns its exact executable/argument grammar;
the SSH projection remains in the pure action core to preserve the acyclic
provider graph. The application `quick_actions::providers` module is the only
cross-provider composition owner and accepts only a previously validated public
`ProviderCapsule`.

The Connection Hub product snapshot owns one optional retained
`ProviderProductPublication` containing the already validated public capsule.
Opening the Action Center synchronizes it to Quick Actions only when the
selected route, session, and capsule revision match exactly. Identical
publications are idempotent; absence, revocation, and mismatch clear stale
route candidates. This handoff performs no provider refresh or external I/O.

Composition performs no discovery, refresh, authentication, filesystem,
network, process, credential, PTY, renderer, startup, or input work. It builds a
complete immutable snapshot and index before publication; an unsupported or
rejected provider prevents partial replacement. OpenBao and local-container
contexts fail closed because neither has an accepted CP4 adapter. Provider
errors leave the last published snapshot unchanged.

`QuickActionRuntime` stores at most one immutable provider snapshot for each of
32 routes. Publication rejects a non-monotonic generation for the same session
and capsule revision. Search captures the route/session/revision/generation key,
uses only cached indexes, gives the capsule layer deterministic precedence over
a same-ID persisted action, and checks the key both before search and before
publishing a result. Route cleanup, explicit clear, and runtime shutdown remove
snapshots. A snapshot is in-memory only and never enters the CP2/CP3 action
store, export, alias, history, log, or provider configuration.

The screen carries the structured binding through selection and review. It
revalidates the exact route, session, capsule revision, generation, snapshot and
binding digests, freshness, expiry, environment risk, and execution mode before
placeholder expansion and again immediately before copy or bracketed insertion.
Only `InsertWithoutEnter` can continue. Refreshing, stale, expired, offline,
unavailable, error, changed, or broker-required decisions become an actionable
unavailable review; no ambient provider environment is substituted. Production
adds textual and accessible risk plus the existing second confirmation even for
a read-only observation.

Ceilings are 256 actions per snapshot, 16 per provider, 32 public presentation
fields, 32 route snapshots, and 128 search results. The schema-1 CP4 contract,
a dedicated authority/interactive-path checker and mutation suite, provider and
application tests, hostile-capsule fuzzing, and cached construction/search
benchmarks freeze this boundary. Approved provider refresh/capsule production,
exact execution, OpenBao, provider-native accounts/CLIs/clusters, controlled accessibility,
resource, packaging, and multi-OS release proof remain external activation
gates.

### Capability and process-launch contract

The v0.4 capability enum is descriptive and local-read-only in practice. The
v0.5 broker replaces broad `ProcessSpawn` authority with a scoped request:

```text
CapabilityRequest
  extension_id + publisher + version
  operation_id + exact session/capsule scope
  capability = session.launch
  executable_id
  ordered argv
  allowlisted public environment deltas
  validated working-directory reference
  interactive PTY intent
  reason and risk
```

The application maps `executable_id` to a canonical absolute path, never
searches the current directory, revalidates file identity before spawn, rejects
NUL/oversized/option-confused values, and never evaluates a shell command
string. Core builds the normal child environment; the extension does not read
it. A process, route, PTY, operation, capsule, and owned tunnels are bound before
publication so cancellation cannot hit a reused PID or sibling session.

v0.5.0 grants this capability only to reviewed first-party operations and only
for the exact OpenSSH tools they use. The extension itself has no direct-network
capability: the approved OpenSSH child connects exactly as it would when typed
in a shell. Arbitrary process/network access and third-party use remain denied.

Current D0/D3/M5 source status remains fail-closed. Active schema 5 freezes the
M3 direct, M4 routed, and M5 configuration-free typed-tunnel grammars; fresh
full-review/executable/endpoint binding; actual child-outcome mapping; bounded
tunnel and receipt lifecycle; stale-source reconnect; and exact native-manifest
rules while retaining schemas 1-4 as immutable hash-checked history. The
manual-shell baseline, package policy, nine trust boundaries, platform
resolution, and hermetic native protocol remain unchanged. ADR 0012 is accepted
by the project owner.

The broker and one Router-owned `ExternalToolRunner` compile in production, but
`MANAGED_SESSION_LAUNCH_ENABLED` is false and the linked package candidate is
`Unverified`. Authorization therefore denies before executable/filesystem
resolution. Behind that denial, the runner accepts only an opaque fresh-review
binding. A capacity-one, latest-generation review worker observes the exact
broker executable, publishes before waking the owning route, and lets the
controller install only a still-current 30-second review. The runner re-hashes
the native executable identity, enforces exact ordered argv, and owns 50 active
operations plus 256 redacted audits/receipts/reconnect candidates.
`ContextManager` alone creates the PTY and publishes the route.
Application child-exit reconciliation supplies the real outcome; close never
assumes success. A nonblocking Router-attached connection worker persists at
most 256 provider-neutral receipts/2 MiB with private atomic primary/previous
recovery. Reconnect uses opaque current-inventory/source identity and always
returns to fresh review and approval.

No production child can start until ADR 0003 protected approvals, real loader
attestation/revocation, and native OpenSSH descendant-cleanup/resource/
accessibility gates pass. Current executable observation, stale-result
rejection, guarded spawn identity, process-group/Job Object termination, and
bounded PTY worker joining are implemented locally but do not substitute for
those exact native runs.
User-owned OpenSSH configuration remains a launch-time trust surface and may
spawn helpers, so native descendant-tree proof is mandatory. Exact limits and
remaining gates are documented in the
[session-launch broker contract](SESSION-LAUNCH-BROKER.md).

### OpenSSH inventory and persistence boundary

`devops-ssh` statically indexes a bounded subset of granted OpenSSH config
files for concrete aliases and public display hints. It never evaluates
`Match exec`, `ProxyCommand`, `LocalCommand`, command substitution, shell
expansion, or `ssh -G` during background work. Includes have canonical-path,
permission, symlink, cycle, count, depth, and byte limits. Parse/refresh failure
retains the last known-good index and marks it stale/error.

Automexia metadata is versioned and atomically stored below the extension's own
state directory with user-only permissions. A connection record may contain ID,
display name, tags, favorite/recent state, source, alias, public host/port/user
hints, jump references, transport, capsule template, and an opaque identity
reference. Private keys, passphrases, cloud tokens, agent messages, recovered
secrets, and full inherited environments are forbidden.

OpenSSH/OS facilities retain custody of `known_hosts`, agents, encrypted key
files, FIDO2/PIV/PKCS#11 devices, and short-lived certificates. Strict host-key
checking remains enabled; Automexia never silently accepts, deletes, or replaces
a host key. Agent forwarding remains an explicit, visible, per-connection grant
and is off by default.

### Later provider-host boundary

v0.5.1 uses official CLIs and local configuration first. If a later inventory
feature needs direct SDK/API access, the adapter runs outside renderer/VT code
in a bounded extension host over a user-scoped named pipe on Windows or
Unix-domain socket. The host authenticates the local peer, accepts versioned
typed messages, applies endpoint/size/time/concurrency limits, redacts results,
and exposes no general TCP control port. It returns public structured metadata
or an approved stream, never raw credentials.

AI extensions use the same capability system but receive no ambient session
environment, SSH agent, cloud cache, terminal history, capsule, connection, or
production authority. Every tool call is a structured, exact-session request;
read authority does not imply command authority.

### Command productivity boundary

Planned v0.5 command productivity preserves PSReadLine, Readline, ZLE, Fish, and
CMD ownership of the editable buffer, cursor, history, quoting, and completion.
The application may diagnose and idempotently provision managed adapters, but it
must not reconstruct a command from terminal-grid cells. Official CLI completion
generators run only through bounded, explicit refresh; provider, network,
authentication, plugin, and secret-store work is forbidden on startup,
keystroke, render, VT, and PTY paths.

CP2.0/CP2.2 provide a renderer-/PTY-independent typed Quick Action model,
bounded in-memory TOML parser, deterministic validator, layered index, search,
and shell serializer in the exact four-file `automexia-devops::actions`
allowlist. CP2.1/CP2.2 use an exact eight-file, application-owned boundary under
`automexia::quick_actions`: bounded
no-follow private storage, atomic primary/one-previous recovery, nonblocking
cross-process CAS, immutable fingerprinted last-known-good snapshots, exact
parent watch filtering, bounded coalescing, periodic reconciliation, and CRUD.
CP2.2 activates that store only through one joined application worker with
bounded per-route latest-query coalescing, a 32-route admission ceiling, fair
multi-pane publication, exact close cleanup, and a narrow screen adapter. The
capability-free index revalidates every layer and keeps shell-user/global-user
precedence distinct. `automexia-ui-model` owns the responsive
list/review projection; the grid-owning screen module contains no Quick Action
domain model. It is not wired to shell profiles, providers, network, secrets, or
execution; secret and exact-launch actions are rejected before placeholder
collection. Later shell aliases, functions, abbreviations, and completion
adapters remain removable generated artifacts. Actions insert for review
without Enter by default through the shell editor's bracketed-paste path. Raw
shell snippets are insert-only; exact execution uses only the future D3 typed
launch broker. Native user
definitions win unless the user chooses a visible reversible override.
Provider-aware actions consume bounded cached public capsule context only after
D6, with session/generation keys, freshness, cancellation, and stale-result
rejection.

The concrete CP2/CP3 ownership map, canonical schema, atomic persistence
transaction, per-shell projection boundary, and resource budgets are specified
in [DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md).


CP3.0 now implements the per-shell projection boundary inside
`automexia-devops::actions`: validated tokens plus complete bounded caller
collision/completion/tool inventories produce deterministic in-memory PowerShell,
Bash, Zsh, Fish, or CMD artifacts and explicit decisions. The boundary recomputes
canonical source identity, verifies same-owner fingerprints, retains degraded
tool health for presentation, and verifies structured/body/rollback identities.
The crate performs no profile discovery, filesystem/process/environment/network/
secret work, publication, or activation.
CP3.1 now owns the application-side managed-file lifecycle: the app publishes
private immutable five-shell generations under a cross-process lock, commits the
activation pointer only after source compare-and-swap, recovers journaled
transitions to all-old or all-new, and exposes read-only health. The verifier
checks active and rollback generations, exact directory topology, permissions/
ACLs, SHA-256, and exact compiler/source/shell identity without repair. Existing
shell integration accepts the ordered versioned compiler manifest and applies
native-wins, last-known-good startup/reload without giving the pure compiler
filesystem or process authority. Local observations cache each executable and
completion identity once across all five projections.
The accepted but preview-disabled CP5 source boundary adds no second line
editor. Shell integration is the only adapter allowed to observe editor-owned
bounded state through a versioned, opt-in, session-capability-authenticated local
pipe/socket. Capability-free `automexia-devops::suggestions` owns
immutable requests/candidates, deterministic ranking, resource limits, and
cancellation. `automexia-ui-model::suggestions` owns the pane-local listbox
projection; the desktop frontend owns the joined broker, restrictive transport,
screen controller, and renderer adapters. No new runtime dependency is approved.
Engine, VT, PTY, renderer, extension, and DevOps context paths may not infer,
produce, execute, or persist editable command text. Any future activated native
adapter must revalidate the exact generation and replacement span and perform
shell-native escaped insertion without Enter. Failure destroys private transient
state and returns to CP1 native completion. The current request scaffolds do not
implement the signed helper or response/replacement path and are never sourced
by normal shell integration.

See [Command Productivity](COMMAND-PRODUCTIVITY.md), the accepted
[compatibility baseline](COMMAND-PRODUCTIVITY-COMPATIBILITY.md), the
[threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md), and
[ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md). Source
implementation follows accepted
[ADR 0025](adr/0025-authenticated-native-editor-suggestion-bridge.md); preview
and stable activation remain forbidden until its exact release gates pass.

## Dependency rules

- Automexia-owned crates form an acyclic graph. The extension API is the lowest
  shared contract layer; provider/domain crates never depend on the UI model,
  renderer, PTY, GPU, window, or application crates. The desktop frontend is the
  composition root. See [ADR 0019](adr/0019-acyclic-owned-crate-dependencies.md).
- Shared behavior belongs to the lowest cohesive owner. Contract normalization
  is implemented once in `automexia-extension-api`; responsive projection,
  accessibility, icon optics, and color policy remain in `automexia-ui-model`.
  A new utility crate is added only when it represents a durable independent
  boundary rather than a place for unrelated helpers.
- Engine crates never depend on the desktop frontend.
- VT parsing and PTY paths contain no extension or product-state logic.
- Extension API/model code is renderer-, GPU-, and PTY-independent.
- Core application/engine crates contain no SSH-, cloud-, Kubernetes-,
  OpenShift-, infrastructure-, or AI-provider business logic or SDK dependency.
- The application is the sole owner of process-to-PTY/session attachment.
  Extensions submit typed capability requests and never receive PTY, process,
  renderer, or mutable terminal-engine handles.
- Extension I/O runs on a bounded worker; the render thread uses non-blocking
  submission and cached immutable snapshots.
- Extension cache keys include extension, exact session/route, capsule/source
  revision, and request kind. Obsolete results are discarded before publication.
- GPU drawing stays in the frontend renderer adapter.
- Session IDs key worker results, completion state, and cached context; one
  window or pane cannot observe another session's state.
- Shell adapters own no renderer, VT, PTY, provider SDK, credential, or direct
  process/network dependency. Action parsing/indexing/generation runs off input
  and render paths; exact launch crosses only the reviewed capability broker.
- Credentials and inherited environments never enter renderer snapshots,
  extension context contributions, general configuration, diagnostics, or
  terminal-history metadata. Only public identity and opaque references cross
  the extension API.
- Session ownership is explicit: an OS window owns window-level
  `ContextGrid` tabs; each grid owns split-pane `ContextGridItem` nodes; each
  pane owns an ordered local tab stack with exactly one active `Context`.
  Every local tab has an independent PTY and route. Background route events
  are delivered to the matching context, all tabs track their pane's current
  dimensions, and only the active context is painted. Deliberate teardown
  records exact route tombstones so delayed PTY-exit events cannot close a
  sibling pane, tab, grid, or window.
  OS-window teardown is centralized: custom chrome, the `WindowClose` action,
  native close requests, and final-PTY exit remove exactly the addressed
  window route, cancel every timer owned by its top-level tabs/splits/local
  tabs, and clear settings/quake window references. Explicit `Quit` remains
  the only process-wide UI action. An intermediate close never exits the event
  loop or opens a quit confirmation; confirmation is reserved for an
  unconfirmed last-window close.

- Selection and search styling take precedence over semantic decoration.
- PowerShell, Bash, Zsh, and Fish integrations assign every prompt a monotonic
  OSC 133 `aid`. Stock CMD publishes `A/B` and closes the preceding command with
  a bare `D` before the next prompt; it does not invent an identity, status, or
  duration that its prompt language cannot expose. The VT grid stores an
  available identity on the semantic prompt row, marks metadata-only writes
  dirty, and preserves it through scrollback and reflow. Renderer caches use
  identities rather than resize-dependent absolute row numbers.
- OSC semantic rows are the prompt-lifecycle authority. The
  `automexia_prompt_active` user variable is retained only for first-paint and
  compatibility fallback behavior.
- Prompt row ownership is exclusive: shell integration emits the blank context
  spacer and complete path once as terminal-owned rows, while
  PSReadLine/Readline/ZLE or CMD's built-in editor owns only the lambda, editable command, and cursor
  row. `OSC 133;A` begins the active block, `B` enters input, and `C`, `D`, or
  the inactive user variable completes it. Repeating an active `aid` atomically
  clears the previous block before accepting its replacement. While input is
  active, the VT owns a compact copy of only those prompt rows. After each PTY
  batch it repairs a delayed screen or line erase from that copy, reflowing through
  the normal grid path; rows which cannot fit stay in scrollback until the
  viewport grows. Completed command history is never copied or replaced.
- Destination-row ownership follows terminal bytes rather than historical row
  position. Before printable output, partial erases, insertion, or deletion reuse
  a row outside the active prompt, the VT clears stale prompt/result metadata.
  Repaint inside an active identified prompt instead retags the destination with
  the current generation. This prevents formatter, full-screen, and alternate-
  shell output from making an overwritten historical result selectable.
- Adjacent PTY resize messages coalesce to the newest effective dimensions.
  Input and shutdown are barriers, duplicate effective sizes are skipped, and
  a transient PTY resize failure is logged without terminating the session.
  Every effective grid resize forces one complete renderer snapshot.
- The renderer owns a responsive window-header reservation. Content begins
  after the header and trailing gap: 56 logical pixels at comfortable sizes,
  50 in compact mode, and 44 in minimal mode. Local tabs never expand that
  window-wide reservation. Instead, each `ContextGridItem` with multiple local
  tabs independently reserves a 36 logical-pixel rail at the top of its own
  pane. Sibling panes keep their complete content height, and panes below 96
  logical pixels hide the rail while retaining every tab and restoring it when
  space returns. One pane-local geometry contract drives rail drawing and
  hit-testing, PTY rows, grid/image clipping, scrollbars, cursor trails, and IME
  placement. Live resize/DPI changes recompute it for every pane. Every visible
  eligible pane exposes its own direct select/close/add targets; the selected
  pane receives the stronger focus outline. Search, split, and focus commands remain available through
  keyboard bindings and the command palette;
  it never duplicates session facts that already belong to prompts. Every shell
  prompt reserves a semantic,
  blank `Prompt` row, a complete-path `PromptContinuation` row, and a short
  editable `PromptContinuation` row. The renderer paints operational context
  as compact, non-interactive semantic tags on the blank row without adding
  characters to PTY output. Tag geometry derives from the pane's logical row
  height, while the shared UI model resolves role tint and 4.5:1 text contrast.
  The shell line editor owns only the lambda/command row, while the full path
  is durable grid history. Stable `aid` identity reconnects all three rows after scrollback and
  reflow, so typing, command output, and resize cannot erase, duplicate, or
  attach them to the wrong command.
- Each pane also owns a renderer-only operational footer (ADR 0008). Layout
  subtracts its 32 logical pixels before resizing the PTY; text, images,
  scrollbars, mouse hit-testing, and prompt overlays therefore share one
  terminal-grid boundary. The footer reads only the frame snapshot and pane
  topology, never locks external workers or writes status bytes into terminal
  history. Its minimal status line reports UTF-8, shell-appropriate LF/CRLF,
  effective grid size, and local time; topology, history, and selection labels
  appear only when relevant and space permits. The existing focused-window
  title tick requests the redraw that keeps the clock current. It has no
  embedded action controls or hidden action hit targets; a click only focuses
  the exact pane. Footer geometry absorbs only the outer horizontal terminal
  margins: one pane spans the window edges, while multiple pane segments tile
  their shared split seams exactly. The vertical top-chrome/root offset remains
  part of the screen coordinate, keeping every footer at its pane bottom, and
  the active segment extends the focus accent.
  Panes below 112 logical pixels hide the footer and recover the space
  automatically when they grow.
- Terminal search has one `Screen`-owned query, direction, scope, and match
  state. `Ctrl+F` binds that state to the selected route and replaces only that
  pane's footer. If that footer is too narrow for every control, presentation
  falls back to the same bottom-centered renderer surface without changing the
  pane owner; visible-workspace search always uses that surface above footer
  chrome. Its bounded route order starts with the selected
  route, then visits the active local tab of each visible split in deterministic
  visual order. It never activates hidden pane-local or window-level tabs.
  `ContextManager` seeds its selected route from the authoritative initial
  context rather than a sentinel, so the first pane participates in footer
  ownership immediately. `Renderer` owns only theme-aware geometry, painting,
  cached hit regions, and hover state. The full painted surface consumes pointer
  input before splitter, terminal, or window-chrome routing; a pane focus change
  closes a pane-local
  search instead of transferring it. Search input is capped at 4 KiB UTF-8,
  reuses the existing bounded scrollback matcher, performs no filesystem,
  network, process, persistence, clipboard, or PTY write, and publishes matches
  only for the currently selected result route.
- A top-level tab has a window-local layout root even though each context
  dimension becomes pane-local after layout. New tabs inherit the active
  grid's current window viewport before their first drawable generation; they
  never reuse the reduced PTY/footer extent as a new root. Their tab label is
  available from semantic shell metadata or immutable launch intent before the
  PTY emits OSC titles, preventing startup profile commands from becoming
  transient visible identities.
- OSC 133 `C`/`D` records exit code and elapsed time on the stable prompt row.
  This metadata is copied, recycled, merged and split with the row and marks
  metadata-only snapshots dirty.

## Interactive performance invariants

- On Windows, `CSI ?9001h` switches keyboard delivery to ConPTY's Win32 input
  record protocol. The window backend retains the native virtual key, scan
  code, modifier/toggle state, and enhanced-key bit; the screen forwards that
  record only after Automexia-owned bindings have had an opportunity to handle
  it. Up Arrow remains shell-owned. The restored Automexia defaults use
  `Ctrl+R`/`Ctrl+D` for independent session clones and reserve
  `Ctrl+Alt+R`/`Ctrl+Alt+D` as explicit history-search/EOF passthroughs.
- Windows PTY ring-buffer producers and consumers check, mutate, wait, and
  notify under the same predicate mutex. This prevents the first input or
  output after an idle transition from losing its wakeup.
- Ordinary keyboard input is written to the PTY without scheduling a
  speculative frame. The terminal-damage event produced by parsed output is
  the redraw authority; frontend-only shortcuts still request an immediate
  redraw when they mutate local UI state.
- Incremental terminal snapshots return before copying style or row data when
  neither grid rows nor semantic metadata changed.
- Renderer row buffers, live DevOps segments, and stable OSC metadata retain
  their allocations across frames. They are rebuilt only when their source
  revision changes or a wider panel requires more capacity.
- The Windows vsync worker parks while there is no redraw or high-rate input.
  A redraw request or the transition into a sustained high-rate input burst
  wakes it, so isolated keys and idle terminals do not call DWM or scan the
  window registry speculatively.
- PTY parsing, DevOps discovery, and extension work stay off the render thread;
  the renderer consumes bounded cached snapshots without blocking on them.
- OpenSSH/config discovery, capability decisions that need filesystem metadata,
  provider CLI probes, authentication state, tunnel health, and later provider
  API work also stay off render, input, VT parsing, and PTY-resize paths. A slow
  or unavailable provider changes only the owning segment's freshness/error.
- Identical file/provider lookups are coalesced, caches use stale-while-
  revalidate, filesystem watchers are advisory and reconciled periodically,
  obsolete capsule generations are cancelled, and provider-specific concurrency
  limits prevent one extension from exhausting the shared worker budget.
- Managed session creation may paint immutable launch/capsule identity on its
  first frame, but live discovery is still queued immediately. Seed data is
  never allowed to claim authenticated/connectivity state it has not observed.
- A completed DevOps discovery publishes its session-scoped snapshot before it
  directly wakes the originating window and route. Initial PowerShell, CMD, or WSL
  context therefore appears without keyboard/mouse input; the short route timer
  remains only a queue-pressure and worker-failure fallback.
- New tabs and splits may seed their first frame from a snapshot no more than
  five seconds old only when path, title, distro, version, shell, and integration
  identity all match. Live discovery is still queued immediately, so reuse
  removes duplicate WSL/CLI startup latency without weakening pane isolation.
- Session clones cross a narrower boundary than ordinary process duplication.
  Every context stores an immutable descriptor containing its normalized
  executable, argv, configured environment overrides, profile identity, and
  starting directory. Clone invocation overlays the live OSC 7 directory and
  explicit distro/user/shell-path metadata, creates a new PTY/performer/route,
  and never copies jobs, process memory, terminal cells, input state, or
  extension caches. WSL identity is never inferred from a title. A strictly
  equivalent metadata seed may paint the new pane's chrome immediately, but
  the new PTY replaces it and queues live discovery on its first frame.
- The WSL probe reads Docker and Kubernetes configuration directly and invokes
  only CLIs whose live state cannot be obtained safely from bounded files.
  PowerShell installs prompt/command-lifecycle hooks, icon format data, and
  editor colors synchronously before the first editable prompt. It does not
  schedule a PowerShell event callback that could contend with PSReadLine's
  first command or history repaint.

## Keyboard compatibility boundary

The private, renderer-independent `automexia-keybindings` crate is the single
pure owner of stable action schemas, typed triggers/predicates/scopes/origins,
profile compilation, collision diagnostics, direct and reverse indexes,
sequence tries, table stacks, and action chains. It has no filesystem, process,
PTY, window, renderer, clipboard, environment, credential, or network
authority. The desktop frontend remains the sole owner of platform events and
all effects.

Configuration compiles `automexia`, the moving `ghostty` alias, or pinned
`ghostty-1.3` plus user bind/unbind layers off the input path. Only a complete
immutable registry is published. Strict errors and failed reloads retain the
last known-good registry, palette labels, and OS hotkeys. Focused lookup uses
allocation-free indexed physical, named, then logical precedence; sequence and
table state is isolated per route and flushes exact pending bytes on invalid
continuation, cancellation, or replacement.

The command palette, CLI inspection, collision reporting, generated references,
and frontend dispatch read that same registry. OS-global bindings are accepted
only by the existing typed global-hotkey owner. All-surface actions use a stable
route snapshot and coalesce damage; unavailable and adapted actions remain
explicit rather than being aliased to unrelated behavior.

Ghostty 1.3.1 Linux/BSD fixtures are checked in with source/binary/checksum
provenance; Windows is a deterministic, labeled adaptation. macOS profile
selection fails closed until a native fixture is reviewed. Normal builds,
startup, and tests remain offline and never execute Ghostty. See
[ADR 0026](adr/0026-versioned-ghostty-keybinding-profiles.md), the
[compatibility guide](GHOSTTY-KEYBOARD-COMPATIBILITY.md), and the
[release roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md).

The G0-G6 identifiers stay stable for roadmap and test traceability. Their
intent is classified separately as Automexia-owned terminal capability (TC),
explicit Ghostty migration (GM), or persistent-session lifecycle (PS). Ghostty
is a
differential reference for selected behavior, not an authority over Automexia's
architecture or a promise of exhaustive parity.

## Build artifact lifecycle

The contributor workflow treats build storage as a bounded resource. Fast
application builds remain incremental in the persistent Cargo target.
The latency-critical `rio-vt` parser/reflow crate uses optimization level 2 in
the development profile so everyday runs do not turn shell history repaints
into debug-only stalls; other workspace crates retain the fastest-to-compile
development optimization level.
Exhaustive all-target checks, warning-denied Clippy, and workspace tests run in
one direct-child verification target with `CARGO_INCREMENTAL=0`; normal process
exit removes that directory regardless of gate outcome. Windows launches copy
the verified debug executable to a unique runtime generation, preventing a
running image from locking the canonical Cargo output. Path containment and
reparse-point checks guard every workflow-owned recursive cleanup.

The two launch workflows also own shell provisioning as a fail-fast phase
immediately before process creation. A repository-source fingerprint and
installed-file/profile-marker checks make unchanged launches a no-op. Windows
prepares PowerShell, CMD, and WSL; Unix prepares Bash, Zsh, and user-local
terminfo. Verification-only commands never mutate a contributor profile. The
ordering and platform command specifications are unit-tested, while isolated
installer tests cover repeat runs and repair. See
[ADR 0009](adr/0009-launch-time-shell-provisioning.md).

CI caches downloaded dependencies but not compiled target trees. The rationale,
safety invariants, failure behavior, and tradeoffs are recorded in
[ADR 0005](adr/0005-storage-bounded-build-workflow.md).

Stable release trust begins only after platform packaging. Unsigned build
intermediates are isolated workflow artifacts and cannot enter the flat public
asset directory. Windows signs executables before MSI/ZIP assembly and verifies
the exact publisher, timestamp, trust chain, and code-signing purpose before a
controlled bounded Defender scan. macOS signs nested code inside-out with
hardened runtime, notarizes and staples the DMG, and passes Gatekeeper. Linux
retains native package validation. The final eleven-package allowlist is then
stream-hashed, SBOM-scanned, attested, and checksum-verified as the exact bytes
users receive. See [Release trust](RELEASE-TRUST.md) and
[ADR 0016](adr/0016-final-artifact-release-trust.md).

## Repository enforcement boundary

The repository is an external security authority, not an implication of local
source state. `.github/repository-protection.json` owns the exact remote target:
merge behavior, least-authority Actions, security features, hosted workflows,
required check names, reviewer capacity, and branch/tag rulesets. A bounded
local validator prevents workflow or contract drift; an authenticated audit
classifies each GitHub control as passing, failing, or externally unavailable.
[ADR 0031](adr/0031-versioned-hosted-ci-and-repository-protection.md) owns the
decision.

Remote mutation is explicit, administrator-only, repository-confirmed, and
idempotent. It may apply only the reversible controls in the versioned
contract. It cannot change visibility, billing, plan, collaborators, secrets,
credentials, release assets, or history. Rulesets have no bypass actor;
protected-path approvals bind to the exact pull-request head; workflow tokens
are read-only and cannot approve reviews; third-party Actions require the exact
allowlist plus full commit-SHA pins.

Hosted evidence belongs to the current protected revision. It passes only when
every required workflow is active and every declared default-branch evidence
workflow's latest run executed successfully on the exact current commit within
seven days. The release-only workflow is active but does not impersonate
default-branch evidence. Missing, stale, skipped-only, different-commit, and
zero-step billing-rejected runs fail or stay
explicitly external. Private-plan ruleset limits, reviewer capacity, reporting,
security entitlements, billing, and native runners remain external authorities;
source code must neither bypass them nor report them as complete.

## Capabilities

First-party extensions declare explicit local-read capabilities. v0.4 supports
only built-in, repository-reviewed extensions. Arbitrary commands, network
access, downloaded extensions, Wasm sandboxing, and a public SDK are outside the
v0.4 boundary. New capabilities require security review, CODEOWNERS approval,
two protected-path approvals, and an ADR.

v0.5.0 adds only the replacement-ADR-approved first-party
`session.launch` capability required by `devops-ssh`. Before authorization,
the principal is bound to an exact package digest, publisher/extension/version,
contract version, and repository-reviewed or first-party-signed proof. A grant
is scoped to executable resource, operation, session, capsule, decision time,
expiry, and public environment policy. It is checked at operation time,
revocable, replay-resistant, and audited without secrets. It is not a general
`Command`, shell, scripting, or subprocess API.

The first SSH release intentionally grants no direct extension network and no
raw secret access. The system OpenSSH child owns network and credential-agent
interaction under the user's existing OS/OpenSSH policy. v0.5.1 official CLI
adapters reuse the same exact-argv path. Direct SDK network, browser callback,
sealed secret-handle, third-party process, and AI tool capabilities require
their own reviewed schemas, quotas, threat models, and ADR changes.

## Accepted D7/CP6 sandboxed ecosystem source boundary

D7/CP6 is fully implemented locally at the accepted nonactivating source
boundary and partially implemented overall. Accepted
[ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md), the immutable
[schema-1 contract](../tests/fixtures/ecosystem/d7-cp6-ecosystem-contract-v1.json),
the separate acceptance receipt, and the
[implementation audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md) freeze ownership,
threats, ceilings, lifecycle, rollback, source dependencies and false release
authorities.

`automexia-ecosystem` is the pure renderer/PTY-independent owner for strict JSON,
manifest/path/compatibility, capability diff and exact grants, package and
verification records, bounded fair lifecycle, disabled distribution policy,
selected-input consent/response, and renderer-neutral review state. It performs
no filesystem, process, network, credential, provider, clipboard, PTY, renderer,
or model I/O.

`automexia-ecosystem-runtime` owns explicit local regular-file ingestion, bounded
manual ZIP parsing, Ed25519/provenance/SBOM/license/trust/time/revocation
verification, protected atomic disabled storage, recovery, exact uninstall,
signed non-executing action-pack mapping, and the optional Component Model
conformance host. The app adapter owns explicit composition and denies activation,
downloads, provider calls and grants. None of these owners is called from typing,
PTY, resize, renderer or startup hot paths.

The accepted logical world is `automexia:ecosystem/suggestion@1`; checked-in WIT
uses source world `extension` because interfaces and worlds share the package
item namespace while `suggestion` is also a required imported interface. The
acceptance receipt records that syntax mapping. Wasmtime is feature-gated, links
no default WASI, inventories exact imports, disables memory64, and applies fuel,
epoch deadlines, memory/table/instance/host-transfer ceilings, cancellation and
joined workers. Public execution always returns `ActivationDenied`; the private
permit is constructible only inside conformance tests.

A package is verified before bytes reach private no-follow staging. Content,
publisher/key, signature, provenance, SPDX/license evidence, compatibility,
time, trusted root and current revocation state bind into a bounded content-free
receipt. The store publishes only installed-disabled generations using fsync and
atomic replace, verifies Windows current-user-only protected DACLs, retains two
generations, reconstructs only valid last-known-good state, and removes only the
exact Automexia-owned subtree. Unix/macOS permission behavior remains a native
release gate.

Every capability defaults denied and binds publisher, extension, version, exact
digest, capability, scope, profile, expiry and generation. Revocation is checked
at invocation; selected input is one-shot. Guest/model output is bounded untrusted
typed data that re-enters ordinary host policy. It never becomes PTY input,
Enter, process execution, file/network/clipboard/credential/agent/provider/
capsule/connection access, or grant authority.

CP6 accepts only explicitly selected bounded text after normalization, redaction
preview and exact provider/locality/model/destination/purpose/retention/size/risk
review. Consent is digest/route/generation/expiry bound and single-use. Provider,
tool, workflow and MCP calls remain hard-disabled, and response risk is assigned
independently. Renderer-neutral package/lifecycle/model states are cancel-first,
responsive, reduced-motion aware and contain textual accessible meaning; no
native released surface is claimed.

Public download/SDK, component/provider activation and native release remain
blocked on protected exact-head approvals, named trust/revocation/update owners,
separate network distribution design, malicious package/component and
compromised-key drills, signed Windows/Linux/macOS package/sandbox evidence,
actual visual/accessibility/IME/focus evidence, provider privacy/legal review,
resource baselines, 1,000 lifecycle cycles, the 30-day soak, and verified
kill/disable/uninstall/rollback/fallback. Private first-party extensions and
CP1-CP3 remain authoritative fallback.

This D7/CP6 boundary is deliberately narrower than the proposed first-party LLM
Orchestration extension below. Neither decision activates or weakens the other.

## Proposed Automation Studio and DevOps/SRE composition boundary

Automation Studio is an AS0 proposal, not a current file editor or runtime.
[Proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md)
keeps it as an optional first-party foundation extension embedded beside terminal
panes, while DevOps/SRE remains a separate domain extension that also works in
terminal-only mode. A DevOps/SRE Pack is metadata-only convenience; it never
becomes a capability principal or merges package lifecycle.

The application composition root would own one canonical document service,
workspace trust, native editor-surface host, language-tool broker, existing
ExternalToolRunner/session broker, opaque credential references, and redacted
receipts. Studio owns disposable editor views and presentation. DevOps/SRE owns
domain templates, public target/risk context, typed plans, and result
interpretation. Neither receives direct filesystem, process, network,
credential, provider, renderer, window, accessibility-platform, or PTY handles.

CodeMirror 6 and Wry are conditional AS0 candidates, not dependencies approved
by this page. The native proof must cover the actual Automexia window stack on
Windows, macOS, Linux X11, and Linux Wayland/GTK, including focus, IME, scaling,
accessibility, GPU, packaging, crash, and cleanup. Language features would use a
bounded core LSP 3.18 broker; DAP remains a separately gated later capability.

Future script/tool execution accepts a saved canonical document revision and a
typed intent that binds exact executable/argv, working directory, Environment
Capsule, target, opaque secret references, risk, limits, review/plan generation,
capability, and expiry. Final revalidation is mandatory. There is no command
string, shell evaluation, implicit Enter, or input to an existing PTY.

The complete ownership, trust, lifecycle, platform, lightweight-profile, and
AS0-AS6 sequence lives in
[Automation Studio architecture](AUTOMATION-STUDIO-ARCHITECTURE.md); its future
evidence ladder is [Automation Studio testing](AUTOMATION-STUDIO-TESTING.md).

AS0 feasibility may proceed without production integration before the first
stable v0.4 release. AS1-AS2 follow that release, and the evidenced minimal
Studio precedes a dedicated video-editing extension. This does not make video a
Studio add-on: both may reuse generic core-owned workspace, file, task,
progress, cancellation, recovery, and lifecycle services, while video owns its
media project, preview, timeline, render, CPU/GPU, and storage boundaries and
does not depend on Studio editor/webview/LSP internals.


## Accessibility boundary

The v0.4 keyboard/focus/contrast/scaling contract, custom-surface inventory,
manual assistive-technology matrix, and truthful limitations are in
[`ACCESSIBILITY.md`](ACCESSIBILITY.md). The v0.5 platform semantic-tree design
is isolated behind the renderer-independent model in
[ADR 0013](adr/0013-renderer-independent-accessibility-model.md); PTY, provider,
extension, and GPU code do not call platform accessibility APIs directly.

## Persistence

Automexia owns `config.toml`, `themes/`, `extensions/`, and `logs/` under its
platform configuration root. Planned typed user actions live below
`actions/actions.toml`; per-shell completion and alias files below
`generated/` are disposable, digest-marked artifacts rebuilt from that source.
Normal terminal launch does not own or rewrite a shell profile. It validates a
package-adjacent integration tree and exposes it only to the child session.
Persistent profile integration is a distinct, explicit application command and
owns only exact marked blocks and bounded copied resources. The decision and
antivirus rationale are in
[ADR 0017](adr/0017-session-only-shell-integration.md).
The one-release migration reads a narrow Rio
allowlist and never modifies the source. See `docs/MIGRATION.md`.

Extension state is versioned below its own directory, atomically replaced,
bounded, and protected with user-only platform permissions. General config,
extension state, logs, renderer snapshots, diagnostics, QA bundles, and
telemetry may contain only public identifiers, policy decisions, freshness,
result class, and opaque references. SSH keys/passphrases, cloud/Kubernetes
credentials, provider tokens, agent messages, browser cookies, inherited
environment values, and recovered secrets are forbidden.

OpenSSH and provider-managed configuration/caches remain externally owned.
Automexia reads only granted sources and never edits `known_hosts`, SSH config,
provider credentials, kubeconfig, or CLI token caches as a hidden side effect.
An explicit export/import/rebind operation must define atomicity, permissions,
rollback, and redaction before it can write an external file.

## v0.5 boundary

The release-critical Phase 1 split into private `automexia-extension-api`,
`automexia-extension-runtime`, `automexia-devops`, and `automexia-ui-model`
crates is implemented. `automexia-app` remains deferred until ownership is
clear, and inherited engines remain in their attributed directories. The
production capability broker and `devops-ssh` implementation are later phases;
this extraction does not grant new process, network, clipboard, PTY, or renderer
authority. Engine grouping beneath `engine/` may happen only after these
behavior-equivalence adapters remain stable through the release gates.

The exact implementation order and acceptance evidence are in the
[early DevOps and SSH delivery track](STABILIZATION-ROADMAP.md#early-devops-and-ssh-delivery-track).
The parallel autocomplete/action work is ordered in the
[command productivity delivery track](STABILIZATION-ROADMAP.md#command-productivity-delivery-track).
The full research, provider mappings, Termix decision, library evaluation, and
long-term extension model are in
[SSH, DevOps, and multi-cloud extension architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md).

## CP3.2 static DevOps pack boundary

CP3.2 is fully implemented at a capability-free/application-owned split:

- `automexia-devops/src/actions/packs.rs` owns immutable schema-1 provider
  manifests, typed action construction, validation, pure health evaluation,
  manifest-aware alias eligibility, digests, deprecations, and update/overlay
  planning. Initialization asserts the reviewed full-registry digest, and update
  comparison normalizes only the manifest provenance version so functional
  metadata still produces an update. The module has no filesystem, process,
  environment, network, credential,
  shell-profile, UI, PTY, or execution authority.
- `validation.rs` delegates built-in alias review back to the immutable registry,
  so provenance drift and context/authentication/destructive/privileged effects
  fail closed even if a caller adds generic acknowledgement.
- `packs_cli.rs` owns explicit application configuration discovery and the
  existing private Quick Action transaction service. List/show/doctor never
  write or start a provider. Enable is dry-run first and exposes exact typed argv,
  effect/risk, documentation, alias eligibility, registry digest, and CAS
  revision. Apply rejects an already-stale revision before opening a writable
  store, still rechecks CAS against races, refuses overwrite, and leaves alias
  projection absent.
- Pack health consumes bounded observations supplied by a caller. Runtime tool
  discovery/execution is deliberately absent; CP1 continues to own explicit
  completion refresh; CP4 now owns cached provider-action context while D5/D6 own refresh and activation.

The exact serialized 11-pack/33-action payload, inventory, and source boundary
are frozen by the CP3.2 machine contract, reviewed digest assertion, mutation
checker, integration/CLI tests, Criterion targets,
and nightly libFuzzer target. CP3.3 imports and task bridges are outside this
boundary.

## CP3.3 native import and trusted workspace boundary

CP3.3 is fully implemented with the same capability-free/application-owned
split and adds no process, network, credential, recipe, provider, or task
execution authority:

- `automexia-devops/src/actions/imports.rs` owns the six bounded native inventory
  parsers, rejection codes, imported-action construction, exact task-bridge
  construction, canonical workspace source digest, trust receipt, and trusted
  layer validation. It has no filesystem, environment, process, network, UI,
  PTY, shell-profile, or execution dependency.
- `native_import.rs` performs bounded no-follow reads of an explicitly supplied
  inventory, requires unique selected names, previews conflicts and explicit ID
  renames, and applies one existing-store revision CAS transaction. It neither
  discovers native aliases nor modifies their source.
- `workspace.rs` owns `.automexia/actions.toml`, exact just/Task/mise bridge
  put/remove, stable path-derived workspace identity, locks, staged atomic
  persistence, and the private `workspace-trust.toml`. Trust receipts omit paths
  and bind identity, canonical digest, and exact revision. Read-only lookup
  validates existing private topology and cannot create or mutate state.
- `worker.rs` resolves a candidate through at most 64 ancestors off the renderer,
  input, and PTY paths; builds a validated workspace layer only with an exact
  receipt; admits at most 32 cached workspace indexes; and reconciles after 250
  ms. Per-route authorization binds requested path and workspace identity and
  expires after 30 seconds. A changed/malformed/linked/revoked source removes
  both the layer and authorization.
- `action_surface.rs` supplies the pane's current directory to the worker and
  rechecks authorization before review and again before insert/copy. Failure
  clears confirmation/expansion and exposes a textual refresh-and-review state.
  No CP3.3 action synthesizes Enter, exact launch, or alias projection.

The schema-1 CP3.3 contract freezes ten reviewed source files, security and
lifecycle invariants, fourteen named tests, nightly fuzz registration, parser/
trust benchmarks, synchronized documentation, and aggregate CI/xtask wiring.
[ADR 0021](adr/0021-trusted-workspace-task-bridges.md) records the durable trust
and no-discovery decision. Unresolvable WSL guest-only paths fail closed because
the desktop process cannot authenticate their host filesystem source.
