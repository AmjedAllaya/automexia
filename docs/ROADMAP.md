# Roadmap

For an evidence-based phase-by-phase comparison of this roadmap with the
current source, tests, benchmarks, platform coverage, security controls, and
release gates, see the [phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md).

## v0.4 — standalone stability

Complete product rebranding, configuration coexistence/migration, contributor
automation, mandatory multi-platform CI, coverage/security policy, and signed
desktop artifact production. Only the newest v0.4 patch is supported.

The v0.4 source-level identity, hostile-control-string, and atomic-reload S0
gates are closed locally and protected by contributor checks. Stable release
still requires the declared native Linux/macOS, hosted security, visual,
performance, packaging, signing, asset-rights, and repository-policy evidence;
local Windows success does not replace those gates.

The detailed [stabilization roadmap](STABILIZATION-ROADMAP.md) records completed
regressions, partially proven areas, the exact S0 implementation/test gates,
native Linux/macOS and visual review, performance baselining, hosted security,
and external release requirements. Windows-only results never satisfy a
cross-platform gate.

The v0.4 assurance milestone also closes the gap between logical correctness
and what a user actually sees. Automexia now has pinned Nextest/JUnit profiles
with timeout, leak, resource-group, and flaky-failure policy; shrinking layout
properties with persisted regressions; finite channel readiness models;
structured renderer-state snapshots; Windows process-resource ceilings; a
topmost client-region Windows final-frame smoke; and a bounded redacted
`cargo xtask qa --full --bundle` evidence archive. Controlled cross-platform
expected/actual/diff frame matrices, executed baseline history, AppVerifier/WPR
results, and cross-platform native evidence remain release gates. The existing
`cargo ready` contributor gate remains deterministic and `cargo automexia`
remains the fast launch path; native GPU, screen-reader, profiler, and
Application Verifier work belongs in explicit deep-test profiles rather than
ordinary application startup.

v0.4 now preserves the inherited Sixel, Kitty Graphics, and iTerm2 renderer
path while adding an explicit bounded local quick look for printed/selected
raster paths. The source contract, security limits, keyboard/pointer UX, WSL
mapping, strict header/file-version gates, latest-request coalescing,
32 MiB thumbnail reuse, zero-copy renderer handoff, isolated benchmark, and
focused tests are complete locally. Controlled Windows visual
review plus Linux/macOS protocol/quick-look evidence remain part of the v0.4
cross-platform release gate; remote fetching, SVG/PDF rendering, and image
editing are not implied by this milestone. See
[image previews](IMAGE-PREVIEWS.md) and
[ADR 0014](adr/0014-explicit-bounded-image-quick-look.md).

v0.4 also establishes an accessibility baseline: every custom chrome action
must remain keyboard-operable, focus-visible, contrast-checked, and usable at
200% scaling, and supported platforms receive recorded manual screen-reader
smoke evidence. A complete cross-platform accessibility tree is staged with
the v0.5 UI-model boundary below rather than being claimed from color and
keyboard tests alone.

### Release sequencing decision

v0.4 remains the frozen terminal-core stabilization milestone. Manual
`ssh host` already works through the user's shell and PTY, but v0.4 will not
grow saved-host, credential, tunnel, or cloud-connection behavior while its
release-assurance gates are open. The bounded-control-string source gate passed
locally on 2026-08-14; managed remote-session exposure still waits for hosted
fuzz/sanitizer and cross-platform native evidence plus its v0.5 capability and
security gates. Design, threat modeling, pure data contracts, and non-activated
tests may proceed in parallel.

The first recommended DevOps-ready product release is v0.5.0, not v0.6. It
ships the generic terminal core together with a separately enabled, first-party
`devops-ssh` extension based on the system OpenSSH client. This is the earliest
safe sequence because it reuses the terminal's mature PTY/session model and
OpenSSH's configuration, host-key, agent, certificate, hardware-key, jump-host,
and forwarding support without waiting for a public third-party SDK or adding a
second SSH protocol implementation to core.

The complete architecture, security rationale, provider strategy, dependency
evaluation, and research sources are consolidated in
[SSH, DevOps, and multi-cloud extension architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md).
The exact Connection Hub layout, discovery tiers, authentication state machine,
platform setup journeys, connection review, capability UX, provider flows,
external identity adapters, responsive/accessibility behavior, goldens, and
acceptance evidence are specified in
[Connection Hub](CONNECTION-HUB.md).
The [stabilization roadmap](STABILIZATION-ROADMAP.md#early-devops-and-ssh-delivery-track)
is authoritative for the implementation order and exit gates.

## Terminal-first remote operations strategy

Automexia will cover the useful connection, inventory, identity, automation,
workspace, transfer, session-memory, collaboration, multi-cloud, and governance
workflows associated with products such as Termius without copying a
screen-heavy GUI. The canonical product contract is
[Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md), and the
durable interaction decision is [ADR 0018](adr/0018-terminal-first-remote-operations.md).

The strategy uses three views of one typed operation registry:

1. the canonical `automexia <domain> <verb>` CLI for scripting, documentation,
   and recovery;
2. a configurable leader-key command mode for Vim-speed interaction;
3. bounded keyboard-driven overlays for fuzzy selection, browsing, comparison,
   preview, and explicit security review.

`ax` is only a proposed optional shorthand. CP3 may project it after explicit
consent and collision checks; the project must not silently shadow an existing
executable, shell alias, function, or abbreviation. Native shell editors keep
ownership of normal input, history, completion, cursor, quoting, and control
keys. Terminal cells and remote output are never trusted command intent.

This is a product-experience projection over existing phases, not a separate
authority or an implementation claim:

| Release/owners | Terminal-first outcome | Explicit exclusions until later |
|---|---|---|
| v0.5.0 D5 + CP2/CP3 | Generated operation registry; action search/review/insert; collision-safe optional aliases; read-only host/group/tag/recent/favorite inventory; Connection Review; quick connect; destination selection; routes/jumps; typed tunnels; identity references; host-key explanation; safe workspace intent/restore; native completion | Structured SFTP, provider API inventory, shared sessions, proprietary identity, AI execution |
| v0.5.1 D6 + CP4 | Immutable per-pane cloud/cluster/infrastructure context; official AWS/Azure/GCP/Kubernetes/OpenShift/Teleport/OpenBao flows; static imports; explicit provider refresh; capsule-aware actions; reviewed multi-target operations | Ambient provider processes, global context mutation, background authentication, secret custody |
| v0.6+ D7 + CP5/CP6 | Independently gated file transfer, bounded session memory/bookmarks, team inventory/policy, read-only-first collaboration, additional transports, signed ecosystem packs, optional editor bridge, and isolated AI explanation/suggestion | Any capability that has not passed its own file/network/peer/privacy/sandbox/native release gate |

The detailed mapping names every planned command, leader sequence, picker
behavior, risk review, data boundary, security/performance invariant, test
layer, and acceptance criterion. Roadmap examples remain non-shipped until the
feature catalog, public CLI/configuration/keyboard references, feature assurance
ledger, native evidence, and release gate are updated together.

## Build, wrap, and adopt sequence

The cross-feature technology and ownership policy is
[Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md);
[ADR 0020](adr/0020-hybrid-build-wrap-adopt-boundary.md) records why. The
terminal core owns product policy and the single capability/process boundary;
first-party extensions own provider-specific parsing and workflows; installed
tools and organization services retain protocol, authentication, credential,
and remote-authorization authority. A named dependency is planned, not shipped,
until its protected slice passes review and evidence.

| Release/phase | Terminal-core work | First-party extension work | Adopted/wrapped authority | Explicit hold |
|---|---|---|---|---|
| v0.5.0 D3-D5 and CP2-CP3 | Finish one `ExternalToolRunner`; generate CLI/help/completion/schema artifacts from typed registries; add bounded search and application-chrome accessibility adapters only after review; preserve exact launch, session, capsule, risk, redaction, and resource policy | Safe OpenSSH inventory; exact SSH/jump/tunnel requests; Connection Review; typed actions, aliases, and first-party packs | System OpenSSH; shell-native editors/completion; planned `clap_complete`, `clap_mangen`, `schemars`, measured `nucleo`, and AccessKit | Native SSH stack, provider SDK bundle, secret vault, structured SFTP, untrusted extensions |
| Protected credential slice | Opaque identity references, public auth state, protected input, approval/revocation, and canary/redaction rules | Version-aware Teleport/OpenBao/agent integration returning public state only | Agents, FIDO, external vaults, `tsh`, OpenBao/Smallstep; exact `keyring-core` stores plus `secrecy`/`zeroize` only after a custody ADR | Private-key formats, CA, password manager, credential sync, recovery claims |
| v0.5.1 D6 and CP4 | Immutable per-pane Capsules, explicit refresh, last-known-good state, provider-neutral inventory, provenance/freshness/risk, and cross-pane isolation | Separately enabled AWS, Azure, GCP, Kubernetes, OpenShift, infrastructure, and enterprise-policy adapters | Official provider CLIs/config first; OPA only for an existing organization policy service | Direct provider SDK until CLI/config cannot meet a measured pagination/watch/cancellation/performance need |
| v0.6+ D7 and CP5-CP6 protected features | Storage/redaction contracts, file-operation states, WIT capabilities, quotas, signed-bundle policy, AI risk/approval boundary | Transfer, Mosh, serial, logs/search, team Git, collaboration, local policy, sandboxed ecosystem, and AI adapters as separate slices | System `sftp`/`scp`, Mosh, Git, SOPS/age, Upterm, optional `rusqlite`, `openssh-sftp-client`, `serialport`, Cedar, Wasmtime/WASI | Telnet disabled by default; custom relay, embedded inference, SQLCipher, and direct SDKs require independent justification |

The core never embeds another terminal UI framework, shell editor, SSH engine,
cloud-login implementation, password vault, policy language, database engine,
WebAssembly runtime, model runtime, or collaboration relay as product logic.
Adopted libraries remain replaceable behind typed bounded contracts. Wrapped
tools all use the same exact-argv runner, and disabling an extension must remove
its process/network/file authority without degrading ordinary terminal use.

Before a dependency or external adapter enters a milestone:

1. pin version and minimal features; review license, source, advisories,
   provenance, binary/startup cost, platform support, owner, and update policy;
2. add fake-executable exact argv/environment/version/output/deadline/cleanup
   tests plus malformed, hostile, Unicode, offline, cancellation, and
   cross-session cases;
3. add the applicable property/model/fuzz/mutation tests, cold/warm and cleanup
   benchmarks, controlled native real-tool evidence, feature-disable behavior,
   accessibility semantics, responsive goldens, and documentation;
4. keep provider/network work off render, resize, startup, and keystroke paths;
5. record the activation and remaining external evidence in the phase audit and
   feature assurance ledger.

## v0.5 — DevOps foundation and production SSH

v0.5 combines the smallest necessary internal modularization with the earliest
production SSH extension. Work is ordered by dependency rather than by desired
UI: secure output handling, provider-neutral data contracts, generic launch
capability, and session isolation precede host management and quick connect.
Unrelated engine-directory movement must not delay the SSH release.

### v0.5.0 foundation

1. Extract private `automexia-extension-api`, `automexia-extension-runtime`,
   `automexia-devops`, and `automexia-ui-model` crates from the existing
   application and renderer seams. Remove deprecated Rio environment fallbacks
   only when the product version advances to v0.5; v0.4.x compatibility remains
   intentional until that version transition. Extract `automexia-app` only
   where ownership is already clear. Group inherited engines beneath another
   directory only after the release-critical split is stable.
2. Introduce versioned, renderer/PTY/GPU-independent contracts for
   `ExtensionId`, `SessionId`, `LaunchRequest`, `EnvironmentCapsule`,
   `ContextContribution`, `StatusSegment`, `Freshness`,
   `CapabilityRequest`, `CapabilityDecision`, `OperationId`, and typed
   public diagnostics.
3. Extend the immutable session launch descriptor rather than creating another
   process path. It must carry an approved executable identity, exact argv,
   bounded environment overrides, validated working directory, optional
   capsule reference, and interactive-PTY intent. It must never carry a shell
   command string or secret value.
4. Convert the provider-specific renderer path into a generic status-segment
   consumer. Existing local Git, Docker, Kubernetes, cloud, Terraform, OS, and
   user discovery first moves behind an adapter with behavior-equivalence tests;
   it must not be rewritten and extracted simultaneously.
5. Partition extension queues, immutable snapshots, operations, cancellations,
   and caches by extension plus exact session/route. Preserve publish-before-
   wake ordering, bounded submission, last-known-good snapshots, and truthful
   `fresh`, `refreshing`, `stale`, `expired`, `unavailable`, and
   `error` states.
6. Accept the replacement ADR required by ADR 0003 before enabling any new
   process capability. Capability grants must be exact, visible, revocable,
   audited without secrets, and denied by default outside reviewed first-party
   extensions.

#### Foundation implementation status (2026-08-14)

The provider-neutral Phase 1 subset is complete at the source boundary:

- all four private crates are extracted with an enforced dependency allowlist;
- versioned bounded schemas, redacted launch/capsule adapters, generic status
  projection, immutable prompt snapshots, session/capsule isolation, full cache
  identity, cancellation, non-blocking saturation, and last-truth failure
  behavior have focused regressions;
- renderer and PTY paths contain no provider implementation dependency;
- local Git/Docker/Kubernetes/cloud/Terraform/environment/OS/user behavior is
  preserved behind a first-party local provider with no new authority.

The following foundation work is intentionally not claimed by Phase 1:

- deprecated Rio environment fallbacks remain until the v0.5 version transition;
- `automexia-app` extraction and inherited engine directory grouping remain
  deferred because neither is required for the release-critical boundary;
- the capability decision UI, executable resolver, exact-argv broker, managed
  environment-changing rebind/relaunch action, and `devops-ssh` activation begin
  with Phase 2/D3 after the replacement ADR is accepted.

#### Phase 2 preparation status (2026-08-15)

The non-activated D3 review boundary is now implemented and tested. It adds the
typed `session.launch` capability identity, expiring exact-scope decisions,
active session/capsule registration, fixed-location OpenSSH resolver with
fail-closed configured overrides, native executable identity/revalidation,
exact one-argument SSH grammar, bounded core-owned environment/cwd policy,
monotonic replay-resistant operation leases, bounded lifecycle state,
revocation, and redacted audit records. The complete module is test-only and
the production graph still has no managed process capability.

D3 is therefore **partial, not shipped**. ADR 0012 is still proposed and the
real first-party package digest/signature/compatibility/revocation proof,
capability UI, atomic native check-to-spawn, real process/PTY/route attachment,
platform-native spawn/cancel/teardown evidence, D4-to-D5 activation integration,
and controlled process/PTY/renderer performance/leak evidence remain
required before activation. The exact current contract is documented in
[Exact-argument session-launch broker](SESSION-LAUNCH-BROKER.md).

#### D4 inventory status (2026-08-15)

The nonactivated OpenSSH inventory and metadata package is implemented. It
provides exact canonical grants, the reviewed resource ceilings, static
concrete-alias parsing, lexical bounded includes, symlink/ownership/permission
checks, redacted source-plus-line errors, strict public-only schema 1 metadata,
durable atomic user-only storage, exact-file watches, generation coalescing,
periodic reconciliation, and last-known-good recovery. Unit/property tests,
native Windows DACL tests, Unix permission tests, a 10,000-alias benchmark,
nightly fuzzing, architecture ratchets, and the feature-assurance ledger own
the boundary. See [OpenSSH inventory](SSH-INVENTORY.md).

This completes D4 only. D3 production activation and D5 connection UX remain
blocked by ADR 0012 acceptance, protected approval, first-party package
identity, visible exact grants, atomic native launch, and controlled native
lifecycle/performance evidence.

### v0.5.x command productivity track

Command completion and persistent DevOps shortcuts are now an explicit parallel
delivery track rather than an implied shell-integration side effect. The
authoritative design, resource ceilings, shell matrix, phases CP0-CP6, tests,
and acceptance criteria are in
[Command Productivity](COMMAND-PRODUCTIVITY.md); the ownership and trust decision
is [ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md). The
implementation-ready CP2/CP3 alias lifecycle, shell projection, pack catalog,
UX, and assurance plan is
[DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md).

1. Preserve PSReadLine, Readline, ZLE, Fish, and CMD ownership of command input,
   history, cursor, quoting, and completion. Never infer the editable command
   from terminal-grid cells.
2. Add idempotent, removable adapters and `doctor` diagnostics for PowerShell,
   Bash, Zsh, Fish, CMD, and WSL. Prefer official Git/Docker/Kubernetes/Helm/
   Terraform/OpenTofu/AWS/Azure/GCP/OpenSSH completion contracts and preserve
   native user definitions.
3. Store reusable commands as bounded, versioned, typed Quick Actions with
   global, shell, capsule, trusted-workspace, and session scopes. Persist the
   source atomically; treat generated shell aliases/functions as disposable.
4. Deliver reviewed DevOps packs with descriptive actions and **no short aliases
   enabled by default**. Optional aliases require collision validation, matching
   completion, reversible generation, and native definitions take precedence.
5. Insert expanded commands for review without Enter by default. Raw snippets
   remain shell-scoped and insert-only; exact execution is available only after
   D3 activation through typed executable/argv/cwd capability requests.
6. Keep startup and keystroke paths offline and secret-free. Provider-aware
   actions wait for D6 Environment Capsules and consume bounded cached public
   context with freshness and cancellation.
7. Deliver CP5 Shell Completion and Suggestions only after a separate bridge
   ADR and threat gate: baseline native editors, define a local authenticated
   generation-scoped bridge, broker local-only sources, benchmark deterministic
   matching, render a pane-owned accessible popup, activate shell by shell, and
   prove rollback/resource/security behavior before opt-in release.

CP0-CP3 may proceed alongside D5 when their own gates pass. CP4 depends on
D5/D6. CP5 is not a v0.5.0 blocker: CP1 native completion stays the default and
complete fallback, and the Automexia surface may ship only when a versioned
shell-editor bridge measurably improves a supported workflow. The bridge must
never scrape terminal cells, start providers per keystroke, read history files,
send command data elsewhere, or override user completion frameworks. This plan
does not claim that Automexia-rendered suggestions, Quick Actions, or generated
aliases are shipped in v0.4.

#### CP0-CP1 command-productivity status (2026-08-16)

CP0 is complete as a non-runtime architecture gate. ADR 0015 is accepted; the
native shell/provider compatibility baseline, conflict/precedence matrix,
versioned schema-1 fixtures, 16-threat model, exact ceilings, architecture
ratchets, mutation tests, CI ownership, and documentation are in place. The
baseline still rejects activation outside a 12-file CP1 allowlist. CP1 now
ships native-owned PowerShell/Bash/Zsh/Fish completion adapters, CMD fallback,
read-only health, and explicit bounded Docker/Kubernetes/OpenShift/Helm cache
refresh. Native definitions win; PowerShell requires explicit override consent.
No provider runs on startup or keystrokes. CP1 adds no network, secret,
clipboard, terminal-output, generated-alias, custom-completion-UI, or exact-
launch authority. CP2.0 model and CP2.1 persistence foundation are complete;
CP2.2 user-facing search/review/insertion is next.

#### CP5 Shell Completion and Suggestions order

When CP2-CP4 dependencies permit, CP5 executes in this fixed order:

1. **CP5.0 research/baseline:** measure native UX and evaluate PSReadLine,
   Bash/Readline, Zsh compsys, Fish, Reedline as a design reference, Nucleo as a
   benchmark candidate, and Carapace only as an explicit external adapter.
2. **CP5.1 bridge:** approve an opt-in local named-pipe/Unix-socket protocol with
   strict endpoint permissions, peer/session capability, buffer/cursor/span/
   generation state, bounded framing, privacy exclusions, and native fallback.
3. **CP5.2 sources:** expose shell-native completion, opt-in shell-owned history,
   current-directory/executables, accepted-candidate frequency, cached public
   provider data, and typed actions without network, authentication, secrets,
   remote-output inference, recursive walking, or per-keypress processes.
4. **CP5.3 ranking:** use deterministic explainable ranking, bounded caches and
   queues, latest-generation cancellation, exact shell escaping, and adopt a
   matcher dependency only after measured license/security/size benefit.
5. **CP5.4 UI:** render a responsive pane-owned accessible popup that avoids the
   cursor, IME, footer, tabs, sibling panes, and modals; includes type, source,
   freshness and risk; degrades to native UI at impossible sizes.
6. **CP5.5 activation:** enable only proven shell/version pairs, preserve all
   user bindings/completers/predictors, and keep truthful CMD/remote fallbacks.
7. **CP5.6 release gate:** pass native OS/shell, protocol, security, fuzz,
   accessibility, performance, resource-leak, resize, multi-pane, rollback, and
   uninstall evidence plus a 30-day preview baseline.

The exact contract, provisional budgets, UI behavior, source precedence,
rejected dependencies, and acceptance criteria live in
[Command Productivity](COMMAND-PRODUCTIVITY.md).

### v0.5.0 first-party SSH extension

The D5 product surface is the renderer-neutral Connection Hub rather than a
collection of unrelated host dialogs. Its implementation is sliced into a
contract/golden baseline, read-only discovery, and reviewed OpenSSH activation
as defined in [Connection Hub](CONNECTION-HUB.md#delivery-phases-and-exit-gates).

1. Ship `devops-ssh` as an optional, signed or compiled-in first-party
   extension. It may be included in the DevOps Pack but must be independently
   disabled without changing PTY, shell, or terminal behavior.
2. Use the system OpenSSH executable as the only initial SSH engine. Locate and
   validate the executable through platform policy; launch it in a new normal
   Automexia PTY using exact argv. Do not use `sh -c`, `cmd /c`, PowerShell
   evaluation, an embedded Node/Electron application, or a Rust SSH protocol
   engine for v0.5.0.
3. Index a bounded, non-executable subset of OpenSSH configuration for aliases,
   hostnames, ports, users, tags, favorites, recent connections, identity-file
   references, and jump-host display. Preserve OpenSSH as execution authority.
   Background indexing must never evaluate `Match exec`, `ProxyCommand`,
   `LocalCommand`, or another configuration command.
4. Provide quick connect into a new pane, pane-local tab, workspace tab, or
   window; config-defined and explicit jump hosts; visible connection intent;
   connection cancellation; and local, remote, and dynamic tunnels whose
   listeners default to loopback and whose lifetime is owned visibly.
5. Delegate private-key custody to `ssh-agent`, OS keychains/agents, hardware
   tokens, encrypted OpenSSH files, or short-lived SSH certificates. Persist
   only opaque references and public metadata. Never write key bytes,
   passphrases, tokens, agent messages, or recovered secrets to Automexia
   config, logs, telemetry, crash bundles, snapshots, clipboard history, or AI
   prompts.
6. Leave strict host-key checking enabled and let OpenSSH own its normal
   interactive verification. Add Automexia explanations for first-use and
   changed-key states only when they preserve the full fingerprint and cannot
   silently edit `known_hosts`. Agent forwarding remains off by default.
7. Keep extension direct-network capability disabled. The approved OpenSSH
   child process owns its connection exactly as when the user types `ssh`.
   Direct extension networking, structured SFTP, background sync, and an
   embedded SSH engine are later, separately reviewed capabilities.

### v0.5.0 release gate

v0.5.0 is the preferred first broad DevOps release when all of the following
are true:

- every v0.4 security, parser, native, packaging, and hosted release gate still
  passes;
- the generic core runs completely with the DevOps Pack disabled;
- OpenSSH is absent, supported, outdated, misconfigured, cancelled, offline,
  and hostile-output cases fail truthfully without affecting another session;
- exact-argv property/native tests cover spaces, Unicode, leading dashes,
  hostile aliases, long values, and Windows/Unix argument semantics;
- parallel connections own independent PTYs, routes, capsules, caches, host-key
  prompts, tunnels, process trees, and teardown;
- slow indexing and connection setup do not block input, parsing, rendering,
  resizing, or unrelated panes;
- no secret appears in persistent state or redacted QA evidence;
- passive Hub discovery performs no process/network/authentication work; every
  `Unknown`, `Ready`, `Locked`, `Missing`, `Expired`, `MFA required`,
  `Cancelled`, `Offline`, `Denied`, `Unsupported`, and error path has truthful
  tested recovery behavior;
- first-run agent/setup guidance, Connection Review, exact capability
  approval/revocation, safe one-click reconnect, and externally owned
  credential recovery warnings pass on Windows, macOS, and Linux;
- keyboard, screen-reader, contrast, production-risk, and error semantics are
  tested on supported platforms;
- any command-productivity capability included in the release passes its CP exit
  gate: shell-disabled behavior remains native, generated files are removable,
  native aliases win, insertion never sends Enter, and startup/keystroke paths
  perform no provider, network, authentication, or secret work.

The v0.5 keybinding crate still begins as a behavior-preserving typed registry
before profiles or sequences. GPU drawing stays in the frontend adapter;
extension, policy, DevOps model, keybinding, and UI-model crates remain
renderer-, PTY-, and GPU-independent.

Move the v0.4 property and concurrency seams into those private crates so
Loom can exhaustively model bounded queue/snapshot interleavings and Miri can
exercise pure state without window-system or GPU FFI. Add an AccessKit-backed
platform adapter over a renderer-independent accessibility model for tabs,
panes, palette items, search, selection, status, and terminal text semantics.
Require roles, names, states, actions, focus order, keyboard-only navigation,
high-contrast/reduced-motion behavior, and Narrator/NVDA, VoiceOver, and
AT-SPI/Orca smoke evidence before claiming cross-platform accessibility.

After the v0.4 performance baseline is trustworthy, make latency/memory
ratchets enforceable, run scoped mutation testing on Automexia-owned pure
modules, and evaluate `cargo vet` imports and first-party audits. Supply-chain
auditing must have a named maintainer and review policy before it becomes a
required check; it complements rather than replaces `cargo deny`, dependency
review, CodeQL, SBOMs, and release attestations.

## v0.5.1 — multi-cloud and orchestrator extensions

After v0.5.0 proves the generic launch/capability boundary, deliver separately
enabled `devops-context`, `devops-kubernetes`, `devops-openshift`,
`devops-aws`, `devops-azure`, `devops-gcp`, and
`devops-infrastructure` extensions. A DevOps Pack meta-package enables the
reviewed set but does not merge their authority.

Each new PTY receives an immutable, non-secret Environment Capsule that pins
identity/profile, account/subscription/project, region/zone, kubeconfig,
cluster/context/namespace, infrastructure directory/backend/workspace, remote
transport, provenance, freshness, risk, and policy references. Clones copy
intent and perform fresh credential/context resolution; a provider switch in
one pane never mutates another pane.

Implementation order is:

1. local configuration parsing and per-session environment isolation;
2. provider-native interactive authentication through visible official CLIs;
3. Kubernetes/OpenShift exec-plugin allowlisting and explicit kubeconfig
   source/context/namespace selection;
4. AWS IAM Identity Center/STS, Microsoft Entra MFA/workload identity, and
   Google Workforce/Workload Identity integration without copying tokens;
5. AWS SSM Session Manager, Azure Bastion, and GCP IAP plus OS Login as
   preferred cloud-native remote transports;
6. lazy, explicitly permitted provider API inventory only after CLI/config
   flows, caching, cancellation, redaction, and resource budgets are proven.

The user journeys and independent D6.1-D6.5 release slices for AWS, Azure,
Google Cloud, Kubernetes/OpenShift, Teleport, and OpenBao are governed by
[Connection Hub](CONNECTION-HUB.md#provider-setup-journeys). A provider slice
does not inherit another provider's grant, cache, token, process, or release
claim.

Provider SDKs remain isolated behind extension-host adapters. CLI-first
delivery is intentional: it preserves official authentication, limits binary
and dependency growth, and avoids binding the terminal core to unstable or
provider-specific SDKs. OpenTofu/Terraform workspaces are displayed as context,
never treated as credential or authorization boundaries.

## v0.6 — sandboxed extension platform and AI tools

Evaluate third-party extension distribution, a public SDK, package signatures
and revocation, capability UX, quotas, and a Wasm or equivalent sandbox only
after the first-party SSH and provider contracts are proven. First-party
DevOps delivery does not wait for this public ecosystem.

Extension-platform work inherits the established QA profiles. Capability,
sandbox, migration, and distribution changes require property/fuzz corpora,
resource ceilings, mutation-tested policy code, accessibility semantics, and
redacted QA evidence before a public SDK or third-party download path ships.

AI agent orchestration remains a separate extension family. An AI extension
does not inherit a PTY environment, SSH agent, cloud cache, terminal history,
capsule, or connection. Each tool call names the exact session/environment,
structured operation, risk, and requested capability; read permission never
implies command permission, staging permission never implies production
permission, and destructive operations require policy plus explicit review.

## Compatibility track

Automexia ships its classic shortcut table. Ghostty compatibility is not an
implicit default or currently selectable profile. The separate
[full Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) records
every implemented, partial, planned, deferred, and excluded capability. Its
ordered work covers fixture provenance, the typed registry, atomic reload,
versioned profiles, dispatch semantics, missing actions, generated tooling,
and later high-lifecycle features. Complete compatibility is deliberately not
a v0.4 release criterion.

## Assurance delivery track

The [stabilization roadmap](STABILIZATION-ROADMAP.md) is authoritative for the
detailed implementation and exit gates. The version assignment is:

| Capability | v0.4 | v0.5.0 SSH | v0.5.1 multi-cloud | v0.6 ecosystem |
|---|---|---|---|---|
| Test orchestration and evidence | Pinned Nextest/JUnit, timeouts, leak reporting, and redacted QA bundle | Component profiles plus SSH/launch/broker evidence | Provider authentication, capsule, and remote-transport evidence | Public SDK/package/sandbox evidence |
| Visual verification | Deterministic renderer state, controlled frame capture, diffs, and human platform review | Generic status, quick-connect, host-key, tunnel, grant, and error goldens | Multi-provider identity, risk, login, expiry, and stale-state goldens | Third-party and AI capability UX contracts |
| Property/concurrency testing | Proptest resize/session invariants and initial bounded Loom models | Exact argv, operation/cancellation, queue/cache, tunnel, and teardown state machines | Capsule isolation, provider refresh, exec-plugin, and broker state machines | Capability/sandbox/package state machines |
| Performance | Execute Criterion, collect 30-day baselines, record startup/interaction/resource data | Enforce ratchets and add SSH index/connect/tunnel/saturation budgets | Add CLI/config/API refresh and 10/50/100-session budgets | Add public SDK/sandbox overhead budgets |
| Native assurance | AppVerifier/WPR and controlled Windows/Linux/macOS GPU/shell matrices | System OpenSSH, agents, certificates, host keys, jumps, tunnels, cancellation, and cleanup on each OS | Official provider CLIs, Kubernetes/OpenShift, SSM/Bastion/IAP, offline/expiry paths | Sandboxed third-party and AI extension isolation |
| Test-strength/security ratchets | Longer fuzz corpora and Automexia-owned coverage baseline | Fuzz config/index/IPC/diagnostics; mutation-test policy and argv validation | Fuzz provider/config/exec-plugin adapters; audit SDK/CLI supply chain | Public extension supply-chain, signature, revocation, and capability audits |
| Command productivity | CP0 baseline plus CP1 shell-native managed completion, diagnostics, explicit bounded refresh, and CMD fallback; no action-store claim | CP2-CP3: typed persistent actions, opt-in aliases, static DevOps packs | CP4: capsule/provider-aware cached actions with freshness and brokered exact launch | CP5/CP6: optional editor bridge and signed ecosystem packs after separate gates |

No single host or test layer may claim complete assurance. Pull requests prove
deterministic contracts, nightly jobs explore expensive state and native
behavior, release jobs require controlled hardware and packaging evidence, and
maintainers record the remaining visual/accessibility decisions.
