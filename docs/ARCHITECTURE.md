# Architecture

## Layers

Automexia v0.4 separates product policy from inherited terminal engines. The
provider-neutral Phase 1 extraction is now implemented without moving inherited
engine directories or creating a second PTY/process owner.

```text
apps/automexia-terminal
  product lifecycle, CLI, windows, PTY/session owner, renderer adapter
                 |
                 | compatibility facades and typed adapters
                 v
automexia-extension-api / automexia-extension-runtime
  bounded versioned contracts, cache/queue/cancellation lifecycle
automexia-devops / automexia-ui-model
  local provider adapter, generic status/layout/accessibility policy
                 |
                 v
rio-backend / rio-vt / teletypewriter / rio-window
  config parsing, VT/grid, PTY, platform event/window contracts
                 |
                 v
sugarloaf / rio-graphics / rio-fonts
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
  IDs, manifests, capabilities, launch requests, capsules, contributions

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

The window-level discovery/review surface is the planned
[Connection Hub](CONNECTION-HUB.md). It projects provider-neutral, bounded
models through `automexia-ui-model`; provider extensions cannot draw their own
approval UI, place work on render/input/VT threads, or turn a displayed label
into executable text. The Hub does not resize a PTY and does not own credentials.

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

Current source status is deliberately narrower. While ADR 0012 is proposed,
the exact resolver/argv/cwd/environment/lease/audit candidate is included only
under `#[cfg(test)]`; production builds contain no broker module or successful
managed-launch path. The review harness converts an authorized request back
into the existing `SessionLaunchDescriptor` seam but never spawns or attaches a
process. Exact limits, platform identity rules, verification commands, and
remaining activation gates are documented in the
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

CP2.0 provides a renderer-/PTY-independent typed Quick Action model, bounded
in-memory TOML parser, and deterministic validator in the exact three-file
`automexia-devops::actions` allowlist. CP2.1 adds an exact five-file,
application-owned persistence boundary under `automexia::quick_actions`: bounded
no-follow private storage, atomic primary/one-previous recovery, nonblocking
cross-process CAS, immutable fingerprinted last-known-good snapshots, exact
parent watch filtering, bounded coalescing, periodic reconciliation, and CRUD.
It is not wired to startup, rendering, shell profiles, providers, or execution.
Later shell aliases, functions, abbreviations, and completion adapters remain
removable generated artifacts.
Actions insert for review without Enter by default. Raw shell snippets are
insert-only; exact execution uses only the D3 typed launch broker. Native user
definitions win unless the user chooses a visible reversible override.
Provider-aware actions consume bounded cached public capsule context only after
D6, with session/generation keys, freshness, cancellation, and stale-result
rejection.

The concrete CP2/CP3 ownership map, canonical schema, atomic persistence
transaction, per-shell projection boundary, and resource budgets are specified
in [DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md).

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
`session.launch` capability required by `devops-ssh`. A grant is
scoped to publisher/extension/version, executable ID, operation kind, session
and capsule, allowed public environment deltas, and interactive mode. It is
checked at operation time, revocable, and audited without secrets. It is not a
general `Command`, shell, scripting, or subprocess API.

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
