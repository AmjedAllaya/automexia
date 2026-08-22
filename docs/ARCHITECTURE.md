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

### M3 direct OpenSSH review boundary

automexia-devops::connections::direct_openssh extends the F2 owner without a new
crate edge or authority. It now owns two immutable stages: a pending preparation
with no executable identity and an identity-bound review requiring a canonical
current ssh executable, identity observation, and host-trust state. Both accept
only the exact M3 direct alias/literal grammar, canonical F2 planning, exact
session.launch, and an all-false authority ceiling.

The desktop composition root maps either one current direct D4 record or one
transient user-entered literal host into a stable opaque public profile.
Inventory generation binds profile/capsule revision; metadata revision joins
the source revision; a domain-separated hash creates the public model ID without
copying the raw record ID or literal host. Unsupported ProxyJump and invalid
records fail closed. The runtime clones one bounded inventory record under its
existing lock, then performs pure composition without I/O, worker creation, or
renderer-path discovery. Literal input is capped at 512 bytes, accepts only one
ASCII host/alias argument, is never persisted or added to history, and is
conservatively classified as production risk.

automexia-ui-model owns the nine-section pending and identity-bound projections
plus the literal editor's textbox/instructions/status/Review/Cancel reading
order. While that nested editor owns the modal, the renderer omits the redundant
top-level close action so its hit area cannot compete with the field; visible
Cancel and Escape retain deterministic dismissal. The application controller
discards or rebuilds inventory preparation
after selection, route, runtime-state, generation, catalog, or metadata changes;
literal review is isolated from those unrelated refreshes and editor state is
cleared on cancel, close, or successful preparation. The native renderer
consumes only presentation state, keeps the modal background inert, and groups
review into Connection, Safety, and Launch cards. Exact inventory aliases,
opaque references, executable digests, and fingerprints do not enter that view.
Because production M2 activation remains gated, the three approval actions are
available but currently end at a protected-review diagnostic before executable
or filesystem resolution. No process, PTY, network, credential, listener,
host-trust mutation, persistence, or secret authority is granted.

The dormant post-activation M3 lifecycle keeps ContextManager as the only PTY,
process, and route owner. Actual child exit, cancel, revoke, route close, and
shutdown become fixed redacted results. A bounded connection worker persists only
provider-neutral receipts and opaque reconnect identity; reconnect always returns
to current D4-source validation and fresh review/approval. Connection Library and
receipt storage share a connection-owned private-filesystem adapter, not the Quick
Actions runtime, for stable Windows handle identity, native link/reparse
rejection, no-follow reads, private
permissions, identity snapshots, atomic replacement, and directory sync.

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

Current D0/D3 source status remains fail-closed. Active schema 3 freezes the exact
17-option-plus-destination M3 grammar, fresh full-review/executable binding,
actual child-outcome mapping, bounded private receipt/recovery contract, and
stale-source reconnect rule while retaining schema 1 and schema 2 as immutable
hash-checked history. The manual-shell baseline, package policy, nine trust
boundaries, platform resolution, and hermetic native protocol remain unchanged.
ADR 0012 is accepted by the project owner.

The broker and one Router-owned `ExternalToolRunner` compile in production, but
`MANAGED_SESSION_LAUNCH_ENABLED` is false and the linked package candidate is
`Unverified`. Authorization therefore denies before executable/filesystem
resolution. Behind that denial, the runner accepts only an opaque fresh-review
binding, re-hashes current native executable identity, enforces exact ordered
argv, and owns 50 active operations plus 256 redacted audits/receipts/reconnect
candidates. `ContextManager` alone creates the PTY and publishes the route.
Application child-exit reconciliation supplies the real outcome; close never
assumes success. A nonblocking Router-attached connection worker persists at
most 256 provider-neutral receipts/2 MiB with private atomic primary/previous
recovery. Reconnect uses opaque current-inventory/source identity and always
returns to fresh review and approval.

No production child can start until ADR 0003 protected approvals, real loader
attestation/revocation, controller consumption of a current attested executable
observation, and native forced-cleanup/OpenSSH/resource/accessibility gates pass.
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
The planned CP5 surface adds no second line editor. Shell integration is the
only adapter allowed to observe editor-owned bounded state through a versioned,
opt-in, session-capability-authenticated local pipe/socket. A renderer-/PTY-/
network-/process-independent `automexia-completion` model may own immutable
candidates, deterministic ranking, resource limits, and cancellation only after
its dependency-boundary ADR is accepted. `automexia-ui-model` owns the pane-local
listbox projection; the desktop frontend owns transport and rendering adapters.
Engine, VT, PTY, renderer, extension, and DevOps context paths may not infer,
produce, execute, or persist editable command text. The editor revalidates the
exact generation and replacement span and performs shell-native escaped
insertion without Enter. Failure destroys private transient state and returns to
CP1 native completion.

See [Command Productivity](COMMAND-PRODUCTIVITY.md), the accepted
[compatibility baseline](COMMAND-PRODUCTIVITY-COMPATIBILITY.md), the
[threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md), and
[ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md).

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
- PowerShell, Bash, and Zsh integrations assign every prompt a monotonic OSC
  133 `aid`. Stock CMD publishes `A/B` semantic boundaries without inventing an
  unstable identity because its prompt language has no pre/post-command hook. The VT
  grid stores that identity on the semantic prompt row, marks metadata-only
  writes dirty, and preserves it through scrollback and reflow. Renderer caches
  use the identity rather than resize-dependent absolute row numbers.
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

The v0.4 frontend owns a flat list of typed runtime bindings. It scans that
list for each key event, applies user entries by removing overlapping defaults,
and maintains separate hard-coded macOS, Windows, and Linux/BSD default tables.
Live reload rebuilds the list for existing windows. Command-palette labels are
currently duplicated platform constants rather than registry-derived data.

This is sufficient for the tested default subset but is not the final strict
compatibility architecture. The
[full Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) places a
future `automexia-keybindings` crate between configuration and the frontend.
That private crate must remain renderer-, PTY-, and GPU-independent and own
typed action IDs, triggers, predicates, origins, compilation, direct lookup,
sequence tries, collision analysis, and reverse action lookup. Adoption starts
with a behavior-preserving adapter; profiles, fallthrough, sequences, tables,
and new actions follow only after equivalence tests pass.

`cargo ready` includes the architecture gate. For focused diagnosis,
`cargo xtask verify architecture` checks the Cargo graph and critical source
invariants. Tests cover publication-before-wake ordering, exact-route wake-up,
bounded queue pressure, busy/disconnected workers, session isolation, prompt
lifecycle, resize/reflow, and semantic precedence.

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
  completion refresh and later CP4/D5/D6 own provider-aware context.

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
