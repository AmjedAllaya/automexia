# Build, wrap, and adopt architecture

## Status and purpose

This is the canonical technology-ownership specification for the planned
v0.5+ terminal-first remote-operations product. It records what belongs in the
terminal core, what belongs in independently enabled first-party extensions,
what must remain an external authority, and what is deferred to a separately
approved sandboxed ecosystem.

This page is a plan, not a shipped-feature claim. A library or tool named here
is not an active dependency until its owning D, CP, or S phase passes the
required architecture decision, threat model, license and supply-chain review,
benchmarks, platform evidence, documentation, and feature-ledger gate.

Dependency acquisition and product builds are separate stages. A controlled
acquisition job may fetch and verify an exact lockfile, tool, model, or managed
runtime; ordinary and release builds consume only those verified inputs and run
without network access where the platform permits. Reviews cover build scripts,
procedural macros, native code, downloaded assets, and transitive provenance,
not only runtime APIs. Every managed binary or model needs a digest, license,
source, update owner, rollback path, and emergency disable mechanism before it
enters a protected slice.

The durable decision is [ADR 0020](adr/0020-hybrid-build-wrap-adopt-boundary.md).
Release sequencing remains authoritative in the [roadmap](ROADMAP.md), and
evidence gates remain authoritative in the
[stabilization roadmap](STABILIZATION-ROADMAP.md).

## Architectural decision

Automexia uses a hybrid architecture:

- **Build** the product-defining terminal experience: typed operations,
  keyboard grammar, Connection Review, Environment Capsules, approval state,
  lifecycle orchestration, responsive overlays, Quick Actions, aliases,
  workspaces, redaction, resource policy, and cross-session isolation.
- **Adopt** focused libraries for non-differentiating algorithms or platform
  abstractions, but only behind Automexia-owned bounded contracts.
- **Wrap** mature security and interoperability authorities such as OpenSSH,
  official provider CLIs, agents, vaults, Git, Mosh, and Upterm through one
  hardened process boundary.
- **Defer or isolate** capabilities whose protocol, secret, file, network, or
  untrusted-code authority has not passed a protected milestone.

No single dependency is a safe remote-operations engine. Automexia's product
value is how it composes established authorities into a fast, reviewable,
keyboard-first workflow. It must not embed a second terminal UI framework,
shell editor, SSH implementation, password vault, cloud-login system, database
engine, policy language, WebAssembly runtime, model runtime, or collaboration
relay into the core merely to gain a feature quickly.

## Ownership taxonomy

| Boundary | Owns | Must not own |
|---|---|---|
| **Terminal core** | PTY/grid/session/window lifecycle; typed operation and action models; generic capability broker; central external-tool runner; exact-argv policy; Environment Capsule contract; pane/tab/workspace placement; renderer-neutral overlays and accessibility semantics; audit/redaction/resource ceilings | Provider business logic; secrets; SSH/cloud authentication; direct provider networking; remote policy; shell editing/history; extension-specific caches |
| **First-party extensions** | Safe domain parsing; provider-specific validation and normalization; exact launch requests; public inventory/context; provider workflows; transport/file/collaboration adapters; version-aware output parsers | Renderer, VT, PTY, or window handles; shell command strings; unrestricted environment; ambient credentials; another extension's grants/cache; independent process spawning |
| **External authorities** | SSH protocol/crypto/config execution; cloud login/MFA; private-key custody; organization vault/policy; provider authorization; Git transport; Mosh transport; collaboration relay; task-runner semantics | Automexia UI state or implicit access to terminal history, clipboard, other panes, or protected product state |
| **Sandboxed third-party extensions (later)** | Explicitly approved WIT capabilities inside Wasmtime/WASI limits | Default filesystem, network, process, clipboard, PTY, terminal history, environment, credential, or secret access |

First-party code may be linked in-process only while it remains trusted,
provider-bounded, renderer-independent, and free of direct process/network or
secret authority. A slice that needs those authorities sends a typed request to
the core broker or, when the D7 isolation gate requires it, runs behind the
authenticated extension-host boundary. Logical extension ownership does not
permit a second PTY or process-launch implementation.

## Dependency direction and repository placement

```text
apps/automexia-terminal
  composition, windows, renderer adapter, PTY/session ownership
       |
       +--> automexia-ui-model
       |      renderer-neutral chrome, overlays, accessibility semantics
       |
       +--> automexia-extension-api
       |      versioned IDs, operations, capabilities, capsules, schemas
       |
       +--> automexia-extension-runtime
       |      bounded queues, generations, cancellation, immutable caches
       |
       +--> core capability broker / ExternalToolRunner
       |      canonical executable + exact argv + bounded environment
       |                 ^
       |                 |
       +--> first-party extensions
       |      devops-ssh, provider, transfer, collaboration adapters
       |                 |
       |                 v
       |      system OpenSSH / official CLIs / Git / Mosh / Upterm
       |
       `--> inherited terminal engines
              rio-backend, rio-vt, teletypewriter, Sugarloaf, rio-window
```

Provider-specific types may depend on generic extension contracts. Generic
contracts must never depend on a provider extension. UI models consume bounded
public projections; they never call a provider, read a credential, perform
filesystem or network IO, or spawn a process. The renderer and resize/input
paths must not wait for any extension, database, fuzzy matcher, external tool,
or accessibility platform adapter.

## Complete decision matrix

| Area | Technology or authority | Decision | Placement |
|---|---|---|---|
| CLI and operation grammar | Existing `clap`; planned `clap_complete` and `clap_mangen` | Adopt | Core-generated interface and `xtask` artifacts |
| Fuzzy search | `nucleo` on a bounded cancellable worker | Adopt after benchmark | Core search service over immutable public records |
| Accessibility | AccessKit over renderer-neutral semantics | Adopt | Core platform adapter; semantics remain Automexia-owned |
| Configuration schemas | Serde/TOML plus `schemars` | Adopt | Generic API/model crates; generated artifact checks |
| SSH, jumps, and tunnels | Installed system OpenSSH | Wrap | `devops-ssh` adapter through core broker |
| SSH inventory | Non-executing safe-subset parser | Build | First-party SSH extension |
| Credentials | Agents and external vaults; OS keyring only when unavoidable | Hybrid | Opaque core references plus separately approved adapters |
| SFTP | System `sftp`/`scp` first; structured client later | Hybrid | First-party transfer extension |
| Mosh | Installed `mosh` binary | Wrap | Optional transport extension |
| Serial | `serialport` behind a blocking worker | Conditional | Optional transport extension after hardware evidence |
| Telnet | Installed client, insecure, disabled by default | Defer | Optional policy-gated legacy adapter only |
| Cloud authentication | Official `aws`, `az`, `gcloud`, `kubectl`, `oc`, `tsh` CLIs | Wrap | Independently enabled provider extensions |
| Cloud inventory | CLI JSON first; direct Rust SDK only with measured need | Hybrid | Separate feature-gated provider extensions |
| Quick Actions and aliases | Typed model with native shell projections | Build | Core productivity model plus first-party packs |
| Completion | PSReadLine/Readline/ZLE/Fish ownership; optional Carapace | Hybrid | Core static registry, shell adapters, optional external bridge |
| Workspaces | Existing panes, pane tabs, and layout | Build | Core |
| Situation-aware production operations | Existing CP5/D2/D6/CP4/D3/DN contracts; official provider/GitOps/JIT/authorization/observability/network authorities; Kubernetes API semantics; OpenTelemetry conventions where useful | Build pure typed investigation/policy/ranking/session composition; wrap external authorities and active operations; adopt provider libraries only after measured adapter need | Optional DevOps/SRE layer and separately enabled adapters; no provider, log-stream, listener, debug, execution or model logic in core |
| Domain-neutral workflows | Versioned typed action descriptors, plans, digests, one-run grants and structured results | Build only after LO0 acceptance | Private pure `automexia-workflow-model`; application remains registry/review/execution root |
| Semantic diagnostic navigation | Existing grid/prompt metadata plus bounded in-tree parsing/matching infrastructure | Build product orchestration; reuse existing primitives; no new dependency approved by DN0 | Core grid access/navigation primitive plus app-owned pure detectors and renderer-neutral highlight |
| Embedded file editing | CodeMirror 6; Monaco retained as the measured desktop comparison | Adopt conditionally after AS0 | Optional first-party Automation Studio behind a core document/surface boundary |
| Native editor surface | Wry over system webviews; platform-specific Linux adapter if required | Conditional after native proof | Core platform adapter; one host per top-level window initially |
| Language intelligence | LSP 3.18 with installed supported servers | Adopt protocol / wrap tools | Core language broker plus independently enabled first-party add-ons |
| Debugging | Debug Adapter Protocol and installed adapters | Defer | AS6 only after a separate authority/threat ADR |
| Session logs | Bounded chunks plus SQLite/FTS metadata | Hybrid | Core contracts; storage worker behind a protected milestone |
| Team inventory | Git-transported non-secret state and semantic review | Hybrid | First-party team extension through core broker |
| Local policy | Rust invariants first; Cedar later | Hybrid | Core invariants; optional policy adapter |
| Enterprise policy | Existing organization OPA | Wrap | Optional enterprise extension |
| Collaboration | Upterm before a custom relay | Wrap | Optional collaboration extension |
| Untrusted extensions | Wasmtime Component Model with custom WIT and no default WASI | Adopted for accepted disabled conformance source | D7 private host, never terminal-core authority; activation/public distribution remain release-gated |
| LLM orchestration | User-chosen local/self-hosted endpoint first; explicit remote adapters later | Build neutral workflow policy; wrap external inference | Optional first-party extension; model/provider has zero direct action authority |
| Testing and supply chain | Existing suite plus `cargo-vet` and scoped `cargo-mutants` | Extend | Contributor/CI tooling, never runtime |

## One external-tool boundary

The terminal core must provide one `ExternalToolRunner` service inside the D3
capability/session-launch broker; it is not a second process or PTY owner.
OpenSSH, provider CLIs, Mosh, Git, Upterm, SOPS/age, task-runner bridges, and
any future external adapter use this service rather than calling `Command`, a
shell, or a provider-specific launcher directly.

Every request and result must provide:

1. a stable, canonical executable identity and compatibility/version state;
2. an exact executable and argument vector, never a concatenated command;
3. a validated working directory and allowlisted environment overrides;
4. null or protected standard input until reviewed interaction is required;
5. per-stream byte and line ceilings with bounded diagnostic retention;
6. startup, idle, and total deadlines;
7. generation cancellation, descendant-process termination, and session-owned
   teardown;
8. redacted structured events that never contain secrets or an environment
   dump;
9. explicit extension, operation, session, capsule, and capability identity;
10. no invocation on render, resize, startup, or keystroke paths.

The core owns executable resolution, request validation, capability decisions,
process ownership, cancellation, and public diagnostics. An extension owns only
the adapter that maps a typed domain operation into the request and parses its
bounded, versioned, public result. A broker is not a privileged daemon and does
not bypass OS, provider, or organization policy.

## Feature and technology placement

### Command mode, Connection Hub, and navigation

**Core terminal builds:** the operation registry; Vim-style mode transitions
and collision-checked leader grammar; pane-local overlays; responsive layout;
stable selection IDs; recent/favorite/exact-alias/environment/provider ranking;
generation cancellation and stale-result rejection; Connection Review and
risk review; and the renderer-neutral accessibility tree.

**Core adopts:** `clap_complete` for static shell completions,
`clap_mangen` for generated manuals, `schemars` for schemas derived from the
same Serde models, `nucleo` for measured large-list search, and AccessKit for
native accessibility APIs. `clap_mangen` generation belongs in `xtask`, not a
build script, so ordinary builds stay non-mutating and inexpensive.

**Extensions contribute:** immutable, bounded host, action, context, workspace,
tunnel, identity, and provider records. They do not render or own focus.

**Rejected:** Ratatui, egui, Tauri, or another UI framework. Sugarloaf/WGPU,
Taffy, the application event loop, and the existing focus/accessibility model
remain the only native product UI system. AccessKit initially covers chrome and
structured overlays; terminal-grid accessibility remains an Automexia-owned
text/document projection.

### Semantic diagnostic navigation

**Core builds:** the route-local on-demand navigator; separate failed-command
and Error/Fatal actions; exact generations and cancellation; bounded transient
line batches; content-free anchors; section reconstruction policy; viewport
placement; renderer-neutral highlight/accessibility state; and all limits,
privacy, cleanup, and recovery behavior.

**Core reuses:** existing prompt/result metadata, grid/reflow/eviction behavior,
route scheduling, typed action and command-palette infrastructure, bounded
search/matching support, structured parsing already present in the workspace,
and the renderer-neutral UI model. DN0 approves no new dependency.

**Placement:** `rio-vt` owns only terminal truth, bounded normalized logical-line
access, content-free positions, viewport movement, and lifecycle signals. Pure
generic detectors begin in an app-owned module outside terminal locks. The
renderer only paints an immutable content-free projection.

**Extensions:** none in DN1-DN5. DN6 prefers declarative rules executed by core.
Any extension parser that observes output requires a separate privacy-sensitive
capability decision, explicit scope/revocation/quotas/cleanup, and ADR 0029 for
third-party delivery.

**Rejected:** continuous indexing, per-pane workers, a persistent log database,
color classification, format/domain parsers in the VT engine, a global logical-
line identity as a DN1 prerequisite, unrestricted regex, network/AI detection,
and treating every non-zero result as a confirmed error.

### SSH connections, routes, tunnels, and trust

**External authority:** installed system OpenSSH owns protocol and cryptography,
algorithm negotiation, agents, certificates, FIDO keys, config execution,
known hosts, jump/proxy behavior, multiplexing, and forwarding.

**SSH extension builds:** passive bounded parsing of safe inventory fields;
inheritance and provenance; route graphs and cycle detection; host-key
explanation; connection review; exact OpenSSH request mapping; reconnect state;
and tunnel health, expiry, stop, restart, ownership, and cleanup.

**Core provides:** the ExternalToolRunner, exact capability decision, normal
Automexia PTY/session creation, operation lifecycle, and public diagnostics.

Inventory must not evaluate `Match exec`, `ProxyCommand`, `LocalCommand`,
command substitution, `ssh -G`, or arbitrary executable includes. OpenSSH may
evaluate configured behavior only after explicit user intent and review.
`russh`, `ssh2`, and `libssh2` are not primary-path replacements because that
would make Automexia responsible for protocol, crypto, compatibility, agents,
certificates, forwarding, trust, and future OpenSSH behavior.

### Credentials, certificates, and vaults

**External authorities:** OpenSSH agents and hardware-backed FIDO keys are the
default. 1Password, KeePassXC, Bitwarden, `gpg-agent`, Teleport, Smallstep, and
OpenBao keep secret custody and issue credentials through their supported
agent or CLI contracts.

**Core builds:** opaque identity references; public health and expiry; the
`ready`, `locked`, `missing`, `expired`, `MFA`, `offline`, `denied`, and
`cancelled` states; protected input; assignment and doctor flows; approval and
revocation; review of public identity/authority; and redaction/canary policy.

**Credential extensions build:** version-aware public-state adapters for
organization-owned authorities. They must not return private keys, passwords,
tokens, recovery material, or unrestricted environment values.

`keyring-core` plus only the required platform stores, `secrecy`, and `zeroize`
may be adopted only when a custody ADR proves an unavoidable stored secret.
They are defense in depth, not protected memory or a general vault. Automexia
does not build a key format, certificate authority, password manager,
credential sync service, cryptographic primitive, or recovery claim for
externally owned credentials.

### Mosh, serial, and Telnet

- **Mosh extension:** wrap installed `mosh`; detect tool/version; explain UDP
  and firewall needs; show that SSH forwarding is unavailable; deliver
  Linux/macOS first and explicit WSL routing on Windows; prove cleanup and
  reconnect. The core remains transport-neutral.
- **Serial extension:** conditionally adopt `serialport` with one blocking
  worker per session, bounded buffers, cancellation, device removal handling,
  and real-device Windows/Linux/macOS evidence before support is claimed.
- **Telnet extension:** defer. If retained for legacy equipment, use an
  installed client, disable by default, forbid stored passwords, mark the
  entire session plaintext/insecure, and let production policy deny it unless
  an administrator explicitly permits it.

### Remote files and transfers

Start with reviewed exact requests for system `sftp` and `scp`. Add structured
upload, download, and list operations before any dual-location navigator. A
later isolated transfer extension may evaluate `openssh-sftp-client` only after
its mutation-cancellation semantics are modeled: cancelling a future does not
guarantee a remote mutation was cancelled.

The transfer extension and generic operation model must represent `Pending`,
`Committed`, `CancelRequested`, `OutcomeUnknown`, `Reconciled`, and `Failed`.
It must implement temporary destinations and atomic finalize, overwrite review,
symlink/no-follow policy, local/remote containment, free-space preflight,
file/byte/depth/concurrency/rate ceilings, partial cleanup, integrity checks,
hostile-filename handling, and reconciliation after ambiguous cancellation.

`rclone`/`rsync` are advanced explicit sync adapters, not the default SFTP
implementation. `rclone` requires a reviewed known-host configuration and must
not introduce a second ambient credential/configuration path.

### Cloud, Kubernetes, and multi-environment operations

**Provider extensions wrap first:** official AWS CLI and Session Manager
plugin, `az` and Azure Bastion, `gcloud` and IAP, `kubectl`, `oc`, `helm`,
Terraform/OpenTofu, and `ansible-inventory`. Authentication stays interactive,
visible, and provider-owned.

**Core builds:** immutable per-pane Environment Capsules; exact profile,
subscription, project, context, and namespace selection; public expiry/status;
explicit refresh; last-known-good snapshots; provenance/freshness/risk labels;
session isolation; and bounded provider-neutral inventory records.

M7 now implements that core boundary without a new dependency: strict public
context/capsule/observation/operation/receipt/audit schemas, 19 authentication
states, exact one-time capability review, external-browser/device/system-broker
metadata, three isolation strategies, generation-safe rebind/cancel/shutdown,
and memory-only public status. It deliberately adopts no OAuth library, cloud
SDK, embedded browser, credential store, or provider configuration writer.
Official CLIs remain the authentication/custody authorities, while D6.1-D6.5
extensions will contribute exact provider-specific operations later.

The isolation policy rejects Automexia-managed global selection such as
`az account set`, `gcloud config set`/configuration activation,
`kubectl`/`oc config use-context`, kubeconfig setters, and
`aws configure set`. A later adapter must use reviewed exact arguments,
bounded public scoped environment names, or an approved private transient
configuration reference. Browser metadata accepts HTTPS origins and exact HTTPS
or IP-literal loopback callbacks only; the official CLI, not Automexia core,
owns and closes any listener.

M11 adopts `serde-saphyr` 1.1.0 only inside the Kubernetes extension, with
deserialization enabled and serialization/includes disabled. Its MIT OR
Apache-2.0 license, pure typed parser, duplicate-key errors, merge-key policy,
and configurable resource budgets fit the 1 MiB untrusted kubeconfig boundary;
`cargo deny` passes. The archived `serde_yaml` is not adopted, generic JSON
values are avoided, and a handwritten YAML parser is rejected. The OpenShift
package reuses this public kubeconfig contract but remains independently
disabled and owns only exact `oc` plans.
M12 wraps the organization-installed Teleport `tsh`; it does not bundle
Teleport, implement its protocol, read `~/.tsh`, or adopt an SSH agent/browser/
MFA stack. The reviewed 18.10 client contract uses exact local version/status
commands and exact login/logout/SSH plans with agent addition, ambient Teleport
environment, surprise relogin, and automatic access requests disabled. The
adapter adopts pinned `time` 0.3.55 with only `std` and `parsing` for bounded RFC
3339 expiry decoding. A handwritten timestamp parser and accepting unreviewed
future `tsh` majors were rejected. The dependency is MIT OR Apache-2.0, pure
Rust in this feature set, off hot paths, and passed advisories/bans/licenses/
sources policy. Removing the Teleport package and `time` workspace dependency
is the rollback; official Teleport state remains untouched.
**Extensions build:** version-aware JSON/config parsers and normalization.
Kubeconfig credential `exec` plugins must never run during passive indexing;
launch uses an explicit deny/allow/allowlist decision.

Direct AWS, Azure, Google Cloud, or `kube-rs` dependencies are phase-two,
feature-gated extension choices. Each must prove that explicit-refresh CLI or
config adapters cannot satisfy required pagination, watch, cancellation, or
performance behavior. Disabling a provider must remove its network and
credential authority.

### Quick Actions, aliases, hooks, and automation

**Core productivity code builds:** typed actions and stable IDs; exact argv for
brokered local operations; shell-scoped insert-only text where exact argv is
impossible; validated typed placeholders; risk, target, capability, lifecycle,
retry/backoff/deadline/failure policy; dry run and exact preview; scope and
precedence; collision detection; pack provenance/version; reversible native
projection and uninstall; safe mode; and `--no-hooks` recovery.

**First-party packs contribute:** reviewed, versioned actions for OpenSSH, Git,
Docker, Kubernetes, Helm, Terraform/OpenTofu, AWS, Azure, and GCP. Packs have no
process authority; execution still passes through the core broker or inserts
into the shell for review.

Reuse current Serde/TOML, hashing, atomic replacement, notification, and bounded
worker infrastructure. Reject Handlebars/Jinja-style templates, implicit shell
interpolation, and command-string concatenation. `just`, Task, and `mise` may be
explicit trusted bridges but never the canonical action store or an automatic
directory-open evaluator.

### Completion and inline suggestions

PSReadLine, Bash Readline, ZLE, and Fish retain editing, history, cursor,
quoting, completion, and autosuggestion ownership. CMD retains its truthful
limited behavior. Carapace is an optional external compatibility bridge and
must not replace user configuration.

**Core builds:** static completion generation from the operation registry;
cached host/action/workspace/context candidates; explicit provider generation;
an authenticated session-scoped editor bridge before any renderer popup;
correct replacement spans and shell escaping; bounded candidates/memory;
generation cancellation; and source, freshness, type, and risk labels.

**Extensions contribute:** immutable cached provider candidates generated only
by an explicit refresh. Automexia never infers editable input from terminal
cells and never invokes cloud or DevOps providers per keystroke.

### Situation-aware production operations

**Build:** the product-defining pure types, route/passport/evidence/policy
binding, change/ownership, resource explanation, cohort comparison, network-path,
SLO-summary and resource/dependency contracts; deterministic hard gates;
explainable ranking/refusal; impact projection; Incident Mode, live-log reference,
operation and managed-session state; content-minimized receipts; limits; and
immutable UI projection. Reuse CP5 insertion and the one application composition
root.

**Wrap:** Kubernetes/cloud authorization, provider CLIs/APIs, GitOps
reconciliation and sync windows, JIT elevation, admission/policy, service
catalogs, incident tools and observability sources through separate bounded
adapters. These external systems remain authoritative for identity, permission,
declared state, audit and business policy. Wrap active probes, Kubernetes
port-forward/debug operations and any provider live-log source only through the
existing activated process/provider broker with separate capability and cleanup.

**Adopt conditionally:** a focused provider client such as `kube-rs` only when
bounded list/watch behavior materially improves a separately approved adapter
over official CLI/config sources; reuse OpenTelemetry semantic conventions for
normalization where they fit. Record exact dependency features, license,
advisories, provenance, binary/startup/resource cost, platform support,
authority, disable/uninstall and native/provider evidence before adoption.

**Reject:** an LLM agent, a second shell editor or runner, `kubectl` plugin as
the existing-command owner, provider queries per keystroke, raw log/metric
storage, an embedded observability platform, one opaque priority score,
automatic remediation and organization runbooks as arbitrary scripts. A small
local model may be evaluated only later as a removable tie-breaker over
already-valid candidates.

The detailed proposed accepted/rejected boundary is in
[Situation-Aware Production Operations](SITUATION-AWARE-PRODUCTION-OPERATIONS.md).
Exact format, rule, provider, policy, lifecycle, dependency and ceiling
decisions are in the non-activating
[PO0 contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md).
The exact product surfaces, interaction rules and implementation map are in the
[Production Operations UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md).
PO0 approves no dependency.

The build/wrap/adopt boundary applies only to Automexia-owned actions. Native and
reviewed insertion replace editor text while the shell owns any later Enter;
they cannot claim policy enforcement or monitored outcome. Only PO6 may compose
one activated managed broker action or diagnostic session with final
revalidation, observation, stabilization, verification, receipt, cleanup and
recovery for its exact reviewed scope. Policy can disable Automexia's candidate
or managed route but cannot stop an equivalent command typed into an unrestricted
shell, and the UI must never imply otherwise.

### Workspaces and restoration

The core owns declarative workspaces on the existing pane, pane-tab, and session
model; public connection intent; current-directory and Capsule restoration;
failure isolation; and confirmation before rerunning commands. It never resumes
an interrupted destructive command automatically.

`tmux` and Zellij remain external interoperability targets. A reviewed hook may
explicitly attach to them; neither is embedded as a second session/layout
authority. Zellij's restore-intent-without-automatic-execution behavior is the
preferred safety model.

### Logs, bookmarks, and searchable session memory

Use versioned TOML/JSON plus atomic replacement for small authoritative state,
`rusqlite`/SQLite FTS for high-cardinality public metadata and search only after
the protected storage milestone, bounded append-only chunks (optionally Zstd)
for event streams, and redacted asciicast v3 for interchange.

The core owns the event/redaction schema, secure-input exclusion, retention,
rotation, quotas, full-disk behavior, bookmarks/comments, search authorization,
delete/export, crash-safe index rebuild, and secret-canary tests. Credentials,
raw environment variables, clipboard contents, and unrestricted input must not
enter SQLite, recordings, diagnostics, or exports.

SQLCipher is deferred until a separate key-custody and recovery ADR. Data
minimization and OS storage protection come first.

### Team inventory and policy

An optional team extension wraps external Git for non-secret transport and
history. It owns canonical serialization, semantic diffs, schema migration,
conflict detection, propose/review/apply, signed-catalog verification,
provenance, and rollback. Avoid `git2` network operations initially so
libgit2/libssh2/TLS do not create another credential path. SOPS/age is limited
to explicit encrypted export or organization-managed files; ordinary shared
records contain opaque secret references.

Non-negotiable invariants remain Rust code: no secret logging, exact argv, no
silent host-key replacement, least privilege, and production confirmation.
Cedar is the planned embedded candidate for later customizable local RBAC/ABAC
after a protected policy milestone. OPA is an enterprise adapter for existing
organization Rego policy, not a second default engine. Remote IAM, RBAC, SSH CA,
network, and provider policy remain authoritative.

### Collaboration

An optional collaboration extension should wrap Upterm before Automexia builds
a relay. Defaults are read-only, `--no-sftp`, explicit authorized keys, an
explicitly chosen or self-hosted relay, visible owner/participants, expiring
invitations, manual input-control handoff, immediate revocation, and no hidden
background share. `tmate` is a UX reference, not a strategic dependency.

A native protocol is allowed only after peer identity, encryption, replay
protection, ordering, rate/queue limits, malicious-peer tests, independent
security review, and incident response are funded and accepted.

### Automation Studio, language tools, and DevOps/SRE integration

The proposed [Automation Studio architecture](AUTOMATION-STUDIO-ARCHITECTURE.md)
uses a separate optional first-party foundation extension for files, diff and
diagnostics presentation. The DevOps/SRE extension remains independently useful
from the terminal and contributes only bounded domain templates, public
target/risk context, plans, and typed run intents. A metadata-only Pack selects
compatible packages without gaining authority.

Core builds the reusable authority brokers: canonical documents and atomic
recovery, workspace trust, native editor-surface lifecycle, typed webview IPC,
bounded LSP mediation, existing exact-argv process/session ownership, opaque
credential references, and redacted receipts. Studio, DevOps/SRE, and language
add-ons never receive direct file, process, network, credential, provider,
window, renderer, accessibility-platform, or PTY handles.

CodeMirror 6 is the recommended first editor proof because its modular MIT
state/view model and mobile support fit an optional lightweight surface. Monaco
remains a serious desktop comparison, not the default, because its official
mobile support is absent and its service/worker surface is wider. Wry is a host
candidate, not an approved dependency: Windows, macOS, Linux X11, and Linux
Wayland/GTK must pass real focus, IME, scaling, accessibility, GPU, packaging,
crash, and cleanup evidence on Automexia's current window stack.

Language features adopt LSP rather than editor-specific providers. Add-ons
declare exact server/tool descriptors and supported versions; the core broker
owns spawn, JSON-RPC framing, workspace trust, limits, cancellation, stale
rejection, and mediation of edits/commands/configuration. DAP is deferred because
debug attachment, evaluation, consoles, ports, and child execution widen
authority.

Script/tool execution extends the one ExternalToolRunner with a typed saved-
revision intent. It binds executable identity/version, exact argv, cwd,
Environment Capsule, target, opaque secret references, operation/risk, limits,
review/plan generation, capability, and expiry. It never builds a shell command,
types into an existing PTY, or presses Enter.

The runner supervises but does not automatically sandbox an installed process.
Exact executable/argv/cwd/environment, deadlines, output ceilings, cancellation,
and descendant cleanup constrain Automexia's use of the process; they do not by
themselves remove the operating-system account's file or network authority. Any
stronger confinement claim requires separately reviewed native/container
sandbox evidence and an explicit fallback when that profile is unavailable.

### Extension sandbox

Trusted first-party modules continue to use the typed extension API and bounded runtime. Accepted D7 source validates future untrusted Component Model code through a feature-gated Wasmtime conformance host with custom WIT and no default WASI; public execution remains denied. Native shared libraries are not the public extension model.

Automexia still owns WIT interfaces, manifests, approval/revocation, CPU fuel,
epoch deadlines, memory/output/file/network/concurrency quotas, signed-bundle
verification, compatibility/migration, crash isolation, and guest-output
sanitization. A guest starts with no filesystem, network, process, clipboard,
PTY, environment, history, or secret authority. WebAssembly is defense in
depth, not permission.

### Model assistance and orchestration

The accepted D7/CP6 source boundary covers an initial selected-input explanation and suggestion slice only. Automexia owns explicit selection, redaction, exact data-flow disclosure, single-use consent, a bounded typed response, independent risk, and copy/insert without Enter. Provider/tool/workflow/MCP calls and execution remain hard-disabled. Accepted [ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md) and the [D7/CP6 audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md) are the exact nonactivating authority for this source slice.

The separate proposed LLM Orchestration extension may later turn explicit user
intent into a candidate typed workflow across installed domain extensions. A
pure private `automexia-workflow-model` would own action/plan/grant/result data;
the desktop composition root would own the action registry, risk/policy checks,
review, one-run grants, final revalidation, execution through existing brokers,
cancellation, and receipts. The model/provider receives no executor, PTY, shell,
filesystem, credential, provider, or mutable extension handle.

Inference remains outside the desktop binary initially. A user-operated local or
self-hosted endpoint is the first direction; separately configured remote
adapters remain inside the optional extension. No paid API is required, no
provider fallback is silent, and an allowlisted MCP mapping is deferred until a
later reviewed capability maps into the same typed registry. Schema-valid model
output is not proof of safety or authorization, and MCP transport authorization
is not an Automexia capability grant. The complete boundary is in
[Optional LLM Orchestration extension](LLM-ORCHESTRATION-EXTENSION.md) and
[proposed ADR 0033](adr/0033-optional-llm-orchestration-extension.md).

D7/CP6 acceptance and LO0 acceptance are separate. D7 now adds reduced-feature Wasmtime, Ed25519, ZIP, Unicode and WIT source dependencies for private local verification and conformance, but still adds no public SDK/download, distribution transport, provider adapter, active component, tool call or execution authority. LO0 remains proposal-only and inherits none of D7 acceptance.

## Dependency introduction order

| Gate | Core additions | Extension additions | Remains external or deferred |
|---|---|---|---|
| **Near-term reviewed slices** | `clap_complete`, `clap_mangen`, `schemars`; AccessKit platform adapters; `nucleo` only after matcher benchmark; ExternalToolRunner contract | Safe SSH/provider parsers and exact request adapters | OpenSSH and official provider CLIs remain installed authorities |
| **Protected credential slice** | Opaque reference/state models | Exact `keyring-core` stores plus `secrecy`/`zeroize` only if an ADR proves custody unavoidable | Agents, FIDO, Teleport, OpenBao, Smallstep, and external vaults remain primary |
| **Protected feature milestones** | `rusqlite` storage worker; Cedar local-policy adapter | `openssh-sftp-client`, `serialport`, direct provider SDKs only after feature-specific proof | Mosh, Git, SOPS/age, Upterm, tmux/Zellij, rclone/rsync remain external |
| **D7 ecosystem accepted source** | Pure WIT/capability/lifecycle/consent models and app denial adapter | Reduced-feature Wasmtime Component Model, Ed25519 verification, strict manual ZIP ingestion and WIT parsing; no default WASI | Public distribution/SDK, component/provider activation, model inference and collaboration relay remain separately gated |
| **LLM Orchestration LO0-LO5** | Pure workflow model, registry/policy/review/execution composition after acceptance | Optional orchestrator and reviewed local/remote provider adapters | Inference endpoints remain user/organization owned initially; MCP and embedded inference deferred |
| **Automation Studio AS0-AS4** | Document/trust/surface/LSP brokers and typed saved-revision run intent after acceptance | CodeMirror-based Studio, DevOps/SRE and starter language/tool add-ons | System webviews and installed supported language servers/tools; Wry remains conditional on native proof |
| **Deferred/rejected** | None | External Telnet adapter only if policy and demand justify it | Native primary SSH engine, password vault, embedded provider login, second shell-line editor or terminal UI framework |

No dependency enters a runtime crate from this table alone. Its change must pin
version/features, review license/source/advisories/provenance, record binary and
startup cost, assign an update owner, add `cargo xtask doctor` behavior, and
prove disable/uninstall/degraded-operation behavior.

## Verification contract

Every adopted crate or external adapter requires:

- deterministic fake-executable tests for exact argv, working directory,
  allowlisted environment, version states, stdin policy, and redaction;
- truncated, malformed, oversized, hostile, Unicode, and unsupported-version
  fixtures;
- offline, locked, expired, MFA, denied, cancelled, timeout, descendant-cleanup,
  and repeated-lifecycle outcomes where applicable;
- property/model tests for state transitions and cross-session isolation;
- fuzz targets for parsers, external JSON, inventory, routes, policy, file
  frames, log frames, and collaboration messages;
- benchmarks for cold/warm start, parse, search, cache refresh, cancellation,
  teardown, peak memory, and binary-size impact;
- one controlled real-tool native integration job on each claimed platform;
- accessibility semantics and responsive goldens for every visible surface;
- feature-disable tests proving ordinary terminal and native tool behavior is
  unchanged when the extension is absent.

Keep Proptest, Loom, Criterion, Insta, cargo-nextest, fuzzing, sanitizers,
Clippy, cargo-deny, dependency review, CodeQL, SBOMs, attestations, PTY tests,
and native platform matrices. Add `cargo-vet` only with named audit ownership
and a ratcheted exemptions policy. Add `cargo-mutants` first to pure security
decisions and state machines such as native-wins, production-deny,
host-key-change, redaction, capability denial, and no-silent-fallback. A retry
that passes remains a reported flaky failure, never silent health.

## Review checklist

A feature review must answer all of these before implementation:

1. Is this product-defining policy, a provider-specific adapter, or an external
   authority?
2. Can the result be expressed as a bounded, versioned, non-secret typed model?
3. Does any process invocation use the single ExternalToolRunner?
4. Does the design avoid renderer, resize, input, PTY, and startup blocking?
5. Are secret custody and authentication still owned by the external authority?
6. Does disabling the extension remove its process/network/file authority and
   leave the terminal functional?
7. Are cancellation, ambiguous outcomes, cleanup, full-disk/offline behavior,
   stale data, and recovery explicit?
8. Are accessibility, responsive layout, platform differences, benchmarks,
   threat tests, and documentation part of the same milestone?

## Primary references

- [`clap_complete`](https://docs.rs/clap_complete/latest/clap_complete/) and
  [`clap_mangen`](https://docs.rs/clap_mangen/latest/clap_mangen/)
- [`nucleo`](https://docs.rs/nucleo/latest/nucleo/)
- [AccessKit](https://accesskit.dev/)
- [`schemars`](https://docs.rs/schemars/latest/schemars/)
- [OpenSSH manuals](https://www.openssh.org/manual.html)
- [`openssh-sftp-client`](https://docs.rs/openssh-sftp-client/latest/openssh_sftp_client/)
- [`serialport`](https://docs.rs/serialport/latest/serialport/)
- [`rusqlite`](https://docs.rs/rusqlite/latest/rusqlite/)
- [Cedar](https://docs.cedarpolicy.com/) and
  [Open Policy Agent](https://www.openpolicyagent.org/docs)
- [Wasmtime security](https://docs.wasmtime.dev/security.html)
- [cargo-vet](https://mozilla.github.io/cargo-vet/) and
  [cargo-mutants](https://mutants.rs/)
- Credential and identity authorities:
  [keyring-rs guidance](https://github.com/open-source-cooperative/keyring-rs),
  [secrecy](https://docs.rs/secrecy/latest/secrecy/),
  [zeroize](https://docs.rs/zeroize/latest/zeroize/),
  [OpenBao SSH certificates](https://openbao.org/docs/next/secrets/ssh/signed-ssh-certificates/),
  and [Teleport tsh](https://goteleport.com/docs/connect-your-client/teleport-clients/tsh/)
- External transports and file tools:
  [Mosh](https://github.com/mobile-shell/mosh),
  [rclone SFTP](https://rclone.org/sftp/), and
  [Upterm](https://github.com/owenthereal/upterm)
- Provider authorities:
  [AWS IAM Identity Center](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html),
  [AWS Session Manager plugin](https://github.com/aws/session-manager-plugin),
  [Azure CLI authentication](https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli),
  [Azure Bastion CLI](https://learn.microsoft.com/en-us/cli/azure/network/bastion),
  [Google Cloud CLI configurations](https://cloud.google.com/sdk/docs/configurations),
  [GCP IAP SSH](https://docs.cloud.google.com/compute/docs/connect/ssh-using-iap),
  [Kubernetes kubeconfig security](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/),
  [`kubectl config use-context`](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_config/kubectl_config_use-context/),
  and [Kubernetes credential-plugin policy](https://kubernetes.io/docs/reference/kubectl/kuberc/)
- Native-app authentication:
  [OAuth 2.0 for native apps (RFC 8252)](https://www.rfc-editor.org/rfc/rfc8252)
- Optional provider SDK candidates:
  [AWS SDK for Rust](https://docs.aws.amazon.com/sdk-for-rust/),
  [Google Cloud Rust libraries](https://docs.cloud.google.com/rust/docs/reference),
  [Azure SDK for Rust](https://learn.microsoft.com/en-us/azure/developer/rust/sdk/overview),
  and [kube-rs](https://github.com/kube-rs/kube)
- Shell and workspace interoperability:
  [PSReadLine predictors](https://learn.microsoft.com/en-gb/powershell/scripting/learn/shell/using-predictors),
  [bash-completion](https://github.com/scop/bash-completion),
  [Carapace](https://carapace.sh/carapace.html),
  [tmux control mode](https://github.com/tmux/tmux/wiki/Control-Mode), and
  [Zellij session resurrection](https://zellij.dev/documentation/session-resurrection.html)
- Storage and team interchange:
  [SQLite FTS5](https://www.sqlite.org/fts5.html),
  [asciicast v3](https://docs.asciinema.org/manual/asciicast/v3/), and
  [SOPS](https://github.com/getsops/sops)
- Optional model and LLM authorities:
  [llama.cpp](https://github.com/ggml-org/llama.cpp) and
  [MCP authorization](https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization)
- Test orchestration:
  [cargo-nextest](https://www.nexte.st/) and
  [Rust Fuzz Book](https://rust-fuzz.github.io/book/)
