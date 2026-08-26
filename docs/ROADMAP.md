# Roadmap

For an evidence-based phase-by-phase comparison of this roadmap with the
current source, tests, benchmarks, platform coverage, security controls, and
release gates, see the [phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md).

For the ordered SSH, connectivity, multi-cloud, Quick Actions, and autocomplete
implementation checklist, use the
[connectivity and command-productivity focus roadmap](CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md).
This roadmap continues to own release sequencing and canonical phase status;
the focus roadmap owns the next executable checklist and dependencies.
The [detailed SSH/connectivity/multi-environment/multi-cloud plan](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md)
owns the M0-M13 implementation and evidence checklist used for future phases.

## Product direction

Automexia's purpose is broader than software development or remote operations:
it is a flexible terminal for making complex command-driven workflows faster,
simpler, and easier to control. The current milestones establish the dependable
terminal, workspace, productivity, security, and extension foundations needed
for that direction. Later, independently reviewed work may support deeper data,
automation, media, and video workflows without turning those ideas into current
feature claims. The [Product vision](PRODUCT-VISION.md) owns this purpose and
audience; the roadmap owns sequencing, and the [feature catalog](FEATURES.md)
owns availability.

The dependency order is: first stable terminal, shared extension/DevOps
foundations, a small domain-neutral workflow contract, a minimal Automation
Studio release, then a dedicated video-editing extension. Non-production AS0,
LO0 and video research may proceed earlier, but they do not add v0.4 release
scope. The video product reuses generic workspace and task services rather than
Studio-specific editor or LLM internals.

The optional LLM Orchestration extension may proceed after the neutral workflow
contract is proven. It remains independently releasable from Studio and video,
does not block either, and never becomes a dependency of the terminal core or a
domain extension. Automexia remains complete without a model, provider account,
network connection or paid API.

Semantic diagnostic navigation is a separate post-v0.4 core workflow track. Its
DN0 design work may proceed without runtime authority, but DN1 product work
begins only after the first stable terminal release. It is independent of SSH,
Automation Studio, and video delivery and does not become their release blocker
without an explicit future release decision.

Situation-aware production operations is a separate optional `PO0-PO8` DevOps/
SRE track. It builds on CP5 insertion, D2/D6 environment evidence, CP4 typed
actions, and DN navigation, but does not merge or activate those owners. PO1-PO4
follow the applicable read-only provider and CP5 gates; PO5 adds only bounded
incident/log/time/handoff ownership; PO6 waits for D3/provider activation for
managed actions and diagnostic sessions; PO8 keeps cross-environment comparison
read-only. The feature stays deterministic and human-controlled,
with no LLM dependency in the terminal core or DevOps/SRE extension. The
[canonical specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md) owns the
detailed sequence and safeguards.


<!-- roadmap-status-register:start -->

## Current feature status

The status appears before every feature/phase and uses exactly **Fully done**,
**Partially done**, or **Not done**. It describes implementation at the phase's
defined source/local boundary; release evidence remains a separate gate and is
summarized in the linked phase audit. This register is machine-checked against
the audit's executive matrix so the two cannot drift.

| Status | Feature / phase | Current scope |
|---|---|---|
| **Fully done** | v0.4/S0 | Core identity, hostile-input bounds, atomic reload, versioned hosted-CI/repository protection, exact annotated tag/default-branch/fork/DCO provenance, authenticated stable-tag audit enforcement, and local mutations are source-complete. The fork tag is published. Controlled workflows still need reviewed integration to `main`; plan-limited rulesets, reviewer capacity, executed exact-head hosted runs, historical linearity/DCO resolution, reporting, and secret-scanning entitlements remain external. |
| **Partially done** | v0.4/S1 | Source implementation is fully done: the opt-in deterministic visual fixture, bounded Windows WGPU/CPU native/resource harness, separate AppVerifier Basics/low-resource phases, exact 24-suite evidence policy, mutation-tested validator, controlled workflow, and fail-closed release dependency are implemented. Actual Linux/macOS/named-GPU/elevated/accessibility runs and approved visual matrices remain external. |
| **Fully done** | S2 | Source and automation are fully implemented: bounded no-follow evidence, repeated-sample/confidence quality, clean exact-commit/operator binding, independently reviewed 30-day baseline activation, exact waivers, 90-day retention, and a fail-closed 5% latency/10% memory release ratchet. Release evidence remains collecting until 30 controlled consecutive days are reviewed and activated. |
| **Partially done** | D0 | ADR 0012 is accepted; schema 6 freezes M3-M5 direct/routed/tunnel argv, trust/status, tunnel lifecycle, receipt/reconnect, and native-manifest rules while retaining immutable schemas 1/2/3/4/5. Protected approvals and real native execution remain. |
| **Fully done** | D1 | Four private provider-neutral crates and bounded contracts satisfy the source boundary. |
| **Fully done** | D2 | Generic status, immutable history, capsules, isolation, cancellation, and freshness are implemented. |
| **Partially done** | D3 | The hard-disabled broker/runner performs bounded current-executable review, binds exact managed argv and executable identity, publishes only through ContextManager, reconciles actual child outcomes, terminates owned process groups/Job Objects, joins PTY workers, and queues bounded redacted receipts. Exact-head approvals/server enforcement, attestation/activation, and real native OpenSSH descendant/resource/accessibility proof remain. |
| **Fully done** | D4 | The bounded OpenSSH inventory/persistence package is complete but deliberately disabled. |
| **Partially done** | D5.0-D5.2 | D5.1, nonactivated F5.1-F5.3, and F5.4's local assurance path are complete: typed routes/tunnels/trust/lifecycle plus exact native host/commit/OpenSSH/artifact binding and a protected manual workflow pass. Protected activation/attestation, actual status/SSH execution, and controlled real native descendant/listener/resource/accessibility manifests remain. |
| **Partially done** | M6/F6 | Accepted ADR 0023, schema-2 review/edit/migration/recovery, public preview-first CLI, immutable worker publication, Connection Hub workspace catalog/restore review, typed recipes/no-hooks, narrow remote initialization, and armed broadcast are complete locally and nonexecuting. D3/M5 activation, managed execution, and controlled native/resource/accessibility/release evidence remain. |
| **Fully done** | D6.0/M7 | Provider-neutral authentication is source-complete locally: strict bounded records, immutable session capsules, 19-state lifecycle, exact allow-once capability review, isolation/redaction, cached-only UI semantics, passive-status migration, fuzz/mutation, and bounded lifecycle/benchmark evidence. Real provider CLIs and native provider evidence remain outside this framework. |
| **Partially done** | D6.1/M8 | AWS source, strict cached capsule publication, six-provider Hub catalog/review, and M11 private EKS ingestion are fully done locally without execution. D3 activation/attestation and controlled real AWS/native/resource/accessibility/release evidence remain. |
| **Partially done** | D6.2/M9 | Azure source, cached Hub review, and M11 private AKS ingestion lifecycle are fully done locally without execution. D3 activation/attestation and controlled real Azure/native/resource/accessibility/release evidence remain. |
| **Partially done** | D6.3/M10 | Google Cloud source, cached Hub review, and M11 private GKE ingestion lifecycle are fully done locally without execution. D3 activation/attestation and controlled real Google/native/resource/accessibility/release evidence remain. |
| **Partially done** | D6.4/M11 | Kubernetes/OpenShift source, the app-owned bounded private transient allocation/validation/revalidation/revoke/cleanup lifecycle, and cached Hub review are fully done locally without execution. D3 activation, real clients/clusters/plugins, Unix native no-follow, controlled cleanup/resources/accessibility/release evidence remain. |
| **Partially done** | D6.5/M12 | Teleport source and cached Hub review are fully done locally without execution. OpenBao is **Not done** and remains blocked on proposed ADR 0024 acceptance; D3/native Teleport activation evidence remains external. |
| **Partially done** | D7 | **Fully done at the accepted source boundary; partially done overall.** ADR 0029 and the exact contract digest are accepted. The private pure model, strict signed local-bundle verifier, protected atomic disabled store, custom-WIT/no-default-WASI Wasmtime conformance host, exact grants/lifecycle, non-executing signed action-pack mapping, selected-input consent/response model, renderer-neutral review states, app adapter, fuzz/property/mutation checks, and benchmarks are implemented locally. Component activation, public downloads/SDK, distribution transport, provider calls, native released UX, signed three-platform packages, malicious-corpus drills, 1,000 cycles, and 30-day evidence remain release-gated. |
| **Partially done** | LO0 | The canonical optional LLM Orchestration specification, proposed ADR 0033, build/wrap/adopt boundary, neutral-workflow dependency direction, provisional limits, and future evidence ladder exist. No workflow-model crate, model/provider adapter, action registry, plan executor, product surface, MCP mapping, model download, or runtime authority exists; ADR acceptance and a versioned machine contract remain. |
| **Not done** | LO1-LO5 | No domain-neutral action/workflow implementation, orchestrator extension, model request, context-consent UI, reviewed one-run execution, multi-extension workflow, provider delivery, conversation store, or native/release evidence exists. |
| **Partially done** | AS0 | The proposed Automation Studio architecture, ADR 0030, technology decision, ownership boundary, AS0-AS6 sequence, and future evidence ledger exist. ADR acceptance, a numeric machine contract, exact dependency audit, and native Windows/macOS/Linux X11/Wayland editor-host proof remain. |
| **Not done** | AS1-AS6 | No document service, workspace-trust implementation, editor/webview surface, LSP/DAP broker, language server, Studio or DevOps/SRE integration package, file-write capability, typed script-run product path, remote/mobile client, or advanced runtime exists. |
| **Partially done** | DN0 | The canonical Semantic Diagnostic Navigator specification, evidence ledger, proposed ADR 0032, ownership/resource/privacy/test contract, and DN0-DN6 sequence exist. ADR acceptance, frozen machine-checked limits, failing tests, and production-owner approval remain. |
| **Not done** | DN1-DN6 | No failed-command action, generic scan, diagnostic detector, anchor cache, highlight, setting, specialized/user pattern, extension contribution, or native release evidence exists. |
| **Partially done** | PO0 | The canonical specification, exact proposed PO0 contract/digest with 17 record payloads and 37 phase/effect/authority-bound actions, semantic checker/mutations/source nonactivation scan, UX/build blueprint, 2026 primary-source baseline, proposed ADR 0034, provider/rule/lifecycle/settings/traceability details and PO0-PO8 sequence exist. Runtime schemas and parsers, protected ADR/exact-digest and owner/dependency/capability acceptance, reviewed prototypes, first failing real-path tests, independent threat review and packaged nonactivation evidence remain. |
| **Not done** | PO1-PO8 | No production passport/lock; change/ownership/drift, resource/scheduling, cohort/environment, network/SLO, live-log/time/handoff or dependency view; situation-aware candidate; rollout prioritization; impact preflight; GitOps/JIT/policy composition; Incident Mode; managed operation/diagnostic session; journal; runbook pack; provider adapter; local ranker; execution path; or native/release evidence exists. |
| **Fully done** | CP0 | Architecture, threat model, ceilings, fixtures, mutation tests, and nonactivation policy are complete. |
| **Fully done** | CP1 | Native shell completion, diagnostics, explicit bounded refresh, precedence, and lifecycle are complete locally. |
| **Fully done** | CP2.0 | The bounded typed Quick Action model and hostile corpus are complete at their pure boundary. |
| **Fully done** | CP2.1 | Private atomic persistence, compare-and-swap, recovery, watches, and benchmarks are complete as an internal library. |
| **Fully done** | CP2.2 | Local layered search, review, administration, import/export, recovery, and insert/copy UI are implemented; hosted evidence remains. |
| **Fully done** | CP3.0 | The pure five-shell compiler recomputes source identity; requires complete collision/completion/tool evidence; verifies owner, structured, body, and rollback identities; retains degraded tool UX detail; and has serializer/native/tamper tests, all-shell fuzzing, a 256-binding benchmark, and mutation ratchets with activation disabled. |
| **Fully done** | CP3.1 | Explicit opt-in persistent aliases, crash-safe atomic generations, active/rollback topology and permission verification, exact compiler-bound native startup/reload, detailed reusable-CAS dry runs, stable diagnostics, rollback, exact uninstall, and WSL lifecycle gates are implemented at the source/local boundary. |
| **Fully done** | CP3.2 | Eleven reviewed static DevOps packs provide 33 disabled-by-default typed actions, an exact-payload digest, truthful health/version/completion evaluation, correct update/overlay/deprecation semantics, exact review previews, stale-revision preflight, and fail-closed alias eligibility. |
| **Fully done** | CP3.3 | Explicit selected PowerShell/Bash/Zsh/Fish/CMD/Git alias import and exact just/Task/mise workspace bridges are dry-run/CAS managed, bounded, insert-only, path-free digest/revision trusted, revocable, removal-safe, runtime-authorized, fuzzed, benchmarked, and mutation-gated. |
| **Partially done** | CP4 | Fully done locally at the product-integrated nonactivating boundary: seven provider-aware projections, retained cached-product-to-selected-route handoff, immutable route snapshots, idempotence/revocation, stale-generation rejection, final copy/insert revalidation, accessible context/risk states, production confirmation, fuzz/benchmark/policy evidence. Approved provider refresh/capsule production, exact execution, OpenBao, and real native/provider/accessibility/release evidence remain. |
| **Fully done** | CP5.0 | Native API/version research, a bounded non-runtime editor-state prototype, 32/128/512 matcher evidence, dependency review, privacy delta, and a retain-CP1/defer-P2 decision are machine-gated. |
| **Partially done** | CP5.1-CP5.6 | ADR 0025 is accepted. CP5.1 authenticated protocol/endpoints/route exchange, CP5.2 sources, CP5.3 ranking, and CP5.4 UI/publication are fully done at source/local boundaries. CP5.5 has an inert helper target and four bidirectional native adapters with local Windows PowerShell and WSL Bash/Zsh/Fish evidence. Reviewed launcher, signing/attestation, WSL relay, live composition, activation, and successful three-OS/accessibility/package/resource/30-day evidence remain. CP1 remains fallback. |
| **Partially done** | CP6 | **Fully done at the accepted no-provider source boundary; partially done overall.** Verified capability-free action packs map into existing typed disabled insert-only actions. Explicit selected text receives bounded normalization/redaction, exact data-flow review, digest/route/generation/expiry-bound single-use consent, strict typed bounded output, independent risk, and cancel-first accessible review models. Provider/tool/workflow/MCP calls, ambient data, Enter, execution, public UI/SDK/downloads, and release activation remain disabled; native privacy/accessibility/provider evidence is external. |
| **Partially done** | G0 | Pinned Ghostty 1.3.1 Linux/BSD source/binary/checksum provenance, generated fixtures/references, deterministic Windows adaptation, accepted ADR 0026, classic golden, property tests, and offline verification are implemented. Native macOS fixture plus native Linux/macOS release evidence remain. |
| **Fully done** | G1 | The pure private typed/compiled registry, schemas, origins, scopes, policies, direct/reverse indexes, sequence trie/table reservation, classic adapter, registry-derived palette, and latency evidence are implemented. |
| **Fully done** | G2 | Explicit default/moving/pinned profiles, typed bind/unbind layers, strict diagnostics, immutable last-known-good publication, transactional hotkeys/palette, and dry-run/confirmed atomic migration are implemented. |
| **Fully done** | G3 | Structured outcomes, performable/unconsumed fallthrough, exact pending-byte sequences, bounded tables/catch-all/chains, all-surface snapshots, per-route state, and shell passthrough are implemented and tested. |
| **Fully done** | G4 | Clear semantics, extended selection, branded pane-footer search, bottom-centered visible-pane search, typed independent splits, logical resize, transactional zoom/equalize, and private bounded screen export with cleanup are implemented and tested. |
| **Partially done** | G5 | CLI inspection, safe migration, xtask generation/verification/tests, generated docs, property/fuzz targets, allocation-free indexes, and Windows Criterion evidence are implemented. Native three-platform keyboard/visual/AT/resource evidence and an activated 30-day baseline remain. |
| **Partially done** | G6 | Accepted ADRs and a redacted inspector are implemented; complete closed top-level window tabs have bounded parked-PTY undo/redo. Individual split, pane-local-tab, and native-window closure history plus native lifecycle evidence remain. |

G6 also lacks user-visible parked-session count, list, and clear controls; the
implemented newest-tab restore shortcut is not a complete management surface.

<!-- roadmap-status-register:end -->

## v0.4 — standalone stability

Complete product rebranding, configuration coexistence/migration, contributor
automation, mandatory multi-platform CI, coverage/security policy, and signed
desktop artifact production. Only the newest v0.4 patch is supported.

The v0.4 source-level identity, hostile-control-string, atomic-reload, exact
stable-source provenance, and authenticated repository-audit S0 gates are
closed locally and protected by contributor checks. Stable release still
requires reviewed workflow integration plus the declared native Linux/macOS,
hosted security, visual,
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
| v0.5.0 D5 + CP2/CP3 | Generated operation registry; action search/review/insert; collision-safe optional aliases; read-only host/group/tag/recent/favorite inventory; Connection Review; quick connect; destination selection; routes/jumps; typed tunnels; identity references; host-key explanation; safe workspace intent/restore; native completion | Structured SFTP, provider API inventory, shared sessions, proprietary identity, model-assisted workflow execution |
| v0.5.1 D6 + CP4 | Immutable per-pane cloud/cluster/infrastructure context; official AWS/Azure/GCP/Kubernetes/OpenShift/Teleport/OpenBao flows; static imports; explicit provider refresh; capsule-aware actions; reviewed multi-target operations | Ambient provider processes, global context mutation, background authentication, secret custody |
| Post-v0.5.1 PO1-PO8 | Production passport/lock; bounded explain/change/compare/network/SLO/dependency views; situation-aware native command ranking/refusal; impact/GitOps/JIT/policy preflight; Incident hypotheses, time/log navigation, handoff and journal; one-action observe/stabilize/verify/recover; managed port-forward/probe/debug sessions; declarative organization packs; read-only cross-environment comparison | No per-key provider work, raw observability storage, credential custody, blind restart, implicit Enter, autonomous remediation, merged cross-environment authority, core/domain LLM dependency, or managed action/session before D3/provider gates |
| v0.6+ D7 + CP5/CP6 | Independently gated file transfer, bounded session memory/bookmarks, team inventory/policy, read-only-first collaboration, additional transports, signed ecosystem packs, optional editor bridge, and selected-input model explanation/suggestion | Workflow planning/execution and any capability that has not passed its own file/network/peer/privacy/sandbox/native release gate |
| Post-foundation LO0-LO5 | Optional separately installed orchestrator proposes typed plans over registered domain actions; core policy, review, one-run grants and existing brokers remain authoritative | No terminal/domain dependency on models, no required paid API, no direct model tool handle, no unattended high-risk mode, and no release before its separate gates |

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

ADR 0035 records the completed feature-ownership audit. Provider-neutral
connectivity/workspace/authentication contracts now live in `automexia-connectivity`;
Quick Action and editor-suggestion contracts live in
`automexia-command-productivity`; optional local context remains in
`automexia-devops`; provider-specific adapters remain independently disabled;
and generic command-result feedback is application-renderer behavior. This
placement correction changes no D, CP, S, G, or release-evidence status.

| Release/phase | Terminal-core work | First-party extension work | Adopted/wrapped authority | Explicit hold |
|---|---|---|---|---|
| v0.5.0 D3-D5 and CP2-CP3 | Preserve the one nonactivated `ExternalToolRunner`; complete protected activation/native proof; generate CLI/help/completion/schema artifacts from typed registries; preserve exact launch, session, capsule, risk, redaction, and resource policy | Safe OpenSSH inventory; exact SSH/jump/tunnel requests; Connection Review; typed actions, aliases, and first-party packs | System OpenSSH; shell-native editors/completion; planned `clap_complete`, `clap_mangen`, `schemars`, and AccessKit. CP5.0 retained the in-tree matcher after measuring Nucleo. | Native SSH stack, provider SDK bundle, secret vault, structured SFTP, untrusted extensions |
| Protected credential slice | Opaque identity references, public auth state, protected input, approval/revocation, and canary/redaction rules | Version-aware Teleport/OpenBao/agent integration returning public state only | Agents, FIDO, external vaults, `tsh`, OpenBao/Smallstep; exact `keyring-core` stores plus `secrecy`/`zeroize` only after a custody ADR | Private-key formats, CA, password manager, credential sync, recovery claims |
| v0.5.1 D6 and CP4 | Immutable per-pane Capsules, explicit refresh, last-known-good state, provider-neutral inventory, provenance/freshness/risk, and cross-pane isolation | Separately enabled AWS, Azure, GCP, Kubernetes, OpenShift, infrastructure, and enterprise-policy adapters | Official provider CLIs/config first; OPA only for an existing organization policy service | Direct provider SDK until CLI/config cannot meet a measured pagination/watch/cancellation/performance need |
| v0.6+ D7 and CP5-CP6 protected features | Storage/redaction contracts, file-operation states, WIT capabilities, quotas, signed-bundle policy, selected-input model consent/risk boundary | Transfer, Mosh, serial, logs/search, team Git, collaboration, local policy, sandboxed ecosystem, and CP6 provider adapter as separate slices | System `sftp`/`scp`, Mosh, Git, SOPS/age, Upterm, optional `rusqlite`, `openssh-sftp-client`, `serialport`, Cedar, Wasmtime/WASI | Tool/workflow execution, Telnet, custom relay, embedded inference, SQLCipher, and direct SDKs require independent justification |
| LO0-LO5 optional orchestration | Pure workflow schema; app-owned registry, plan policy/review, one-run grants, execution composition and receipts after acceptance | One separately installed LLM Orchestration extension; domain extensions publish deterministic typed actions without model dependencies | User-operated local/self-hosted inference first; explicit remote adapters later | Direct model tools, arbitrary MCP, embedded inference, silent provider fallback, conversation storage and unattended high-risk work remain separately gated |

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
- the production-compiled capability broker, application runner, approval UX,
  guarded PTY seam, and route publication belong to Phase 2/D3 and remain
  nonactivated until every protected and native gate passes.

#### Phase 2 preparation status (2026-08-22)

ADR 0012 is accepted by the project owner. Mutation-checked schema 6 preserves
immutable schemas 1-5 and freezes manual-shell/missing-client behavior, package
identity, exact M3-M5 route/tunnel/trust/lifecycle rules, grants/audits, nine
trust boundaries, four-platform resolution, all-false runtime authority, and 23
native scenarios including current OpenSSH PQ/warning/agent-binding and tunnel
collision evidence.

The production-compiled broker remains hard disabled and the linked package is
unverified. One Router-owned runner now enforces exact typed scope, 50 active
operations, 256 redacted audits, bounded public environment, a trusted cwd,
replay-safe leases, actual child outcome, cancellation, route close, redacted
notifications, bounded durable receipts/reconnect candidates, and application
shutdown. It
opens and re-compares the executable guard consumed by ContextManager's exact
PTY seam; the route is published only after insertion. The review exposes
allow-once/session/deny pointer, keyboard, focus, accessibility, and fixed
recovery behavior without rendering private launch data.

D0 and D3 are therefore **partial, not shipped**. ADR 0003's two independent
exact-head approvals/server enforcement, real loader attestation/revocation,
native graceful/forced cleanup, Windows/macOS/Linux/WSL OpenSSH fixtures,
D4-to-D5 activation, and controlled process/PTY/renderer/accessibility/resource
evidence remain required. The exact current contract is documented in
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

This completes D4 only. D3 production activation and later D5 connection
features remain blocked by ADR 0003 protected approvals/server enforcement,
real first-party package attestation, and controlled native lifecycle,
accessibility, and performance evidence. The exact grant UX and guarded
review-to-route seam now exist locally without production authority.

#### D5.0 Connection Hub model status (2026-08-17)

Status: **Partially done**. All source-local, non-executing D5.0/F2 work is
fully implemented; the protected ADR decision is not done, so the phase cannot
close or activate.

| Feature | Status | Evidence / remaining work |
|---|---|---|
| Provider-neutral definition/observation/intent/review/receipt/profile/recipe/step/tunnel/plan schemas | **Fully done locally** | Strict schema 1 models, sealed validated wrappers, and fixed ceilings live in `automexia-connectivity::connections`. |
| Hostile input, duplicates, cycles, policy, retry, and redaction validation | **Fully done locally** | Integration/property/record/state tests plus mutation and architecture ratchets fail closed. |
| Deterministic dry-run resolution and approval invalidation | **Fully done locally** | Target, identity, route, executable, tunnel, recipe, capability, and source changes are covered. |
| Authentication and result state machines | **Fully done locally** | Every public state and illegal/terminal transition is table-tested; auth results are bound to the active operation generation. |
| Hub, Connection Review, and recipe-planner projection contract | **Fully done locally** | Wide/medium/narrow/text-scale, stale-selection focus recovery, live progress, route-aware modal focus, value-redacted labels, reading order, all-state accessibility, and structured goldens pass without a renderer. |
| Synthetic/deep assurance | **Fully done locally** | Ten-provider/all-auth fixtures, 64-step benchmark, fuzz target, 30 required regressions, and bypass/panic mutation ratchets are owned. |
| Process/network/provider/credential/PTY/listener authority | **Fully disabled** | F2 has no filesystem, process, socket, provider, credential, PTY, window, or GPU owner. |
| Protected activation evidence | **Partially done; external gates open** | ADR 0012 is owner-accepted; ADR 0003 exact-head approvals/server enforcement, attestation, and native proof remain before D5.2 production execution. Capability-free D5.1 stays governed by accepted ADR 0022. |

This status does not activate connection authority. D5.1 now owns one app-scoped
joined runtime, exact reviewed native file selection, virtualized product modal,
public favorite/tag CAS review, read-only recent/library projections, and
visibly disabled execution. Windows source/test/security/performance/release-
build evidence passes locally. Native macOS/Linux picker/permission and
controlled screen-reader runs remain external; D5.2 separately owns reviewed
OpenSSH execution and lifecycle.

#### D5.1 read-only Connection Hub status (2026-08-21)

Status: **Fully done locally; release evidence partially done.** All requested
read-only inventory behavior and non-authority boundaries are implemented.
Catalog-only controls appear only when useful, hidden controls reject input and
leave accessibility order, and restrained semantic colors are redundant with
code-native icons and text. The product first-run frame now presents two
bounded choices because the non-activated D5.2 literal-host workflow is also
available. A prompt-ready 1600x950 Windows frame at 125% scale passed the prior
D5.1 manual visual review; the new literal dialog has renderer-neutral and
geometry evidence but still needs controlled native pixel/accessibility review.
The local 10,000-record release benchmark measured 7.1790–7.7931 ms against the
below-16-ms target; the release executable is 22,670,336 bytes, a 650,752-byte
(2.96%) increase from the same-host pre-M1 baseline. The remaining evidence is
native macOS/Linux picker and permission/recovery coverage plus controlled
Narrator/NVDA, VoiceOver, and Orca verification. Those external gates do not
permit or block ordinary terminal/manual OpenSSH use and do not imply D5.2.

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

#### CP0-CP1 command-productivity status (2026-08-17)

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
launch authority. CP2.0/CP2.1 foundations and CP2.2 user-facing search,
review, bounded administration/import/export, and insertion are implemented
locally. Hosted native shell insertion, controlled screen-reader evidence, and
the 30-day performance/resource baseline remain release gates; exact launch
remains disabled behind D3.

#### CP3.1 persistent alias status (2026-08-17)

- **Fully done** — One private immutable generation publishes exact PowerShell,
  Bash, Zsh, Fish, and CMD artifacts with source-CAS/pointer-last recovery and
  one authenticated rollback generation.
- **Fully done** — Read-only doctor and startup verify active plus retained
  generations, exact directory topology, permissions/ACLs, SHA-256, source and
  shell identity, and the exact `automexia-devops/0.4.0` compiler manifest.
- **Fully done** — Dry runs expose directly reusable revision/generation CAS
  inputs, source/artifact identity, bindings, collisions and exact owner
  fingerprints, completion state, and tool health; repeated local executable and
  completion observations are cached per unique identity.
- **Fully done** — Twenty-one owned security/lifecycle regressions, three CLI detail
  tests, eight contract mutations, strict Clippy, native Windows checks, and the
  full local WSL Bash/Zsh/Fish lifecycle pass. The final local 256-alias release
  benchmark measured 24.228-26.100 ms compile, 35.476-37.391 ms durable publish,
  and 8.580-9.228 ms doctor; nightly and release workflows own the WSL lifecycle gate.
- **Partially done** — Hosted macOS/native matrix publication and the controlled
  named-hardware 30-day startup/resource baseline remain release evidence; they
  are not missing CP3.1 implementation.
- **Fully done** — CP3.2 ships eleven immutable first-party manifests and 33
  disabled-by-default typed actions with no default aliases. A reviewed digest
  freezes the exact payload; pure caller-supplied health distinguishes registry
  readiness from provider readiness; version-only updates remain unchanged;
  exact argv/effect/risk/documentation previews and stale-revision preflight
  protect enablement; fuzzing, benchmarks, eight mutations, and synchronized
  documentation gate the phase.
- **Fully done** — CP3.3 consumes only explicitly supplied native inventories
  for PowerShell, Bash, Zsh, Fish, CMD/DOSKEY, and Git; imports only explicitly
  selected simple fixed-token actions through dry-run/CAS conflict and rename
  review; and leaves native sources untouched.
- **Fully done** — exact just, Task, and mise task bridges persist only named
  insert-only workspace actions. Private path-free digest/revision receipts,
  explicit trust/revocation/removal, bounded background ancestor/cache refresh,
  and review/insertion authorization checks fail closed on source change.
  Automexia never lists tasks, parses recipes, runs providers, reads credentials,
  accesses the network, executes tasks, or projects workspace aliases.

#### CP5 Shell Completion and Suggestions order

When CP2-CP4 dependencies permit, CP5 executes in this fixed order:

1. **CP5.0 research/baseline — fully done:** native UX/API evidence, a pure
   insertion prototype, matcher comparison, and build/wrap/adopt decision retain
   CP1 and defer P2 without adding a runtime dependency or surface.
2. **Fully done at source boundary / partially done for release — CP5.1 bridge:**
   strict schema-1 request/submission/replacement framing, route capability and
   generation checks, restrictive Windows named-pipe and Unix-socket adapters,
   joined latest-only work, replay rejection, cleanup, property/fuzz, and native
   Windows fixtures exist. Native Linux/macOS runtime fixtures and WSL relay proof
   remain release gates; no endpoint is activated.
3. **Fully done at source boundary / partially done for release — CP5.2 sources:**
   the exact six bounded local sources, independent history/frequency opt-ins,
   cached-public LKG/freshness, privacy exclusions, and deterministic source
   policy are implemented. Activated native-shell/provider fixtures remain gated.
4. **Fully done at source boundary / partially done for release — CP5.3 ranking:**
   deterministic Unicode/grapheme-aware ranking, stable ties, one-latest queues,
   byte/count/cache limits, cancellation, insertion revalidation, fuzzing, and
   32/128/512 plus near-limit frame benchmarks are implemented. Named-hardware
   latency/allocation and sustained resource evidence remain.
5. **Fully done at source boundary / partially done for release — CP5.4 UI:** a
   pane-owned renderer-neutral listbox/controller/renderer plus bounded source
   publication, exact candidate reconstruction, authenticated reply, route/
   deadline/kill controls, and tiny-to-8K 100–300% tests exist. Live composition,
   native GPU/IME/pointer/keyboard, and assistive-technology evidence remain.
6. **Fully done at inert source bridge / partially done for composition — CP5.5:**
   the helper target, bounded transport/session/endpoint runner, strict response
   envelope, and PowerShell/Bash/Zsh/Fish response/replacement adapters exist with
   local native evidence. Reviewed launcher/restricted inheritance, signed
   artifact, WSL relay, interactive PowerShell insertion, profile/uninstall proof,
   live composition, and activation remain; PowerShell 5.1/CMD retain native CP1.
7. **Partially done — CP5.6 release gate:** bounded broker/mailbox kill/reset/
   disable/uninstall, LKG, exact route limits, mutation policy, local native shell
   tests, and OS-specific hosted steps exist while activation stays false.
   Successful three-OS endpoint/shell/accessibility/package/rollback, named-
   hardware resource campaigns, 1,000 real endpoint cycles, and 30-day evidence
   remain external prerequisites.

The exact contract, provisional budgets, UI behavior, source precedence,
rejected dependencies, and acceptance criteria live in
[Command Productivity](COMMAND-PRODUCTIVITY.md).

### Post-v0.4 semantic diagnostic navigation (DN0-DN6)

**DN0 is partially done at the documentation boundary. DN1-DN6 are not done.**
The canonical
[Semantic Diagnostic Navigator](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md) and proposed
[ADR 0032](adr/0032-bounded-semantic-diagnostic-navigation.md) define the target.
They add no current action, shortcut, detector, setting, runtime dependency,
cache, telemetry, persistent index, or extension permission.

The product sequence is:

1. finish and publish the stable v0.4 terminal without this feature;
2. accept or supersede ADR 0032 and close DN0's measurable limits/threat/test
   contract;
3. deliver exact failed-command traversal independently of heuristic detection;
4. prove a bounded on-demand scanner and generic Error/Fatal sections;
5. finish accessible product interaction and native/resource assurance;
6. add specialized formats and user patterns only through separate evidence;
7. consider extension contributions only after their privacy/capability
   boundary, and ADR 0029 where third-party delivery applies.

DN1-DN4 may be scheduled alongside v0.5 work after v0.4, but their release
assignment remains open. They do not wait for managed SSH, multi-cloud,
Automation Studio, or a public marketplace, and none of those tracks wait for
the navigator. DN6 is different: ambient terminal-history access is not part of
the current extension model and remains blocked on a separately accepted
decision.

Current reusable evidence is deliberately narrow:

- semantic previous/next prompt actions and bounded terminal search exist;
- supported shell integrations attach OSC 133 command results to prompt rows;
- stock CMD and unsupported shells remain neutral when status is unknown;
- prompt identity survives supported scrollback/reflow cases;
- renderer-neutral UI, route scheduling, and bounded cache patterns exist.

This does not mean diagnostic navigation is partially implemented. Current
result metadata does not provide a durable complete command-output region; the
command-result assurance page also records a long-output/scrollback surface gap.
There is no generic stable logical-line identity, diagnostic scanner, detector,
section locator, navigation state, highlight, or extension contract.

The planned owner split is fixed unless ADR 0032 is superseded:

- `rio-vt`/`Crosswords` owns grid truth, prompt results, bounded normalized line
  batches, content-free positions, viewport movement, and reflow/eviction
  signals, but no format/domain detector meaning;
- one route `Context` owns one navigator, cancellation generations, at most one
  continuation, optional bounded content-free anchors, cleanup, and publication;
- an app-owned pure module owns generic candidate filtering, detectors, bounded
  section reconstruction, and cache policy;
- the renderer owns only content-free highlight geometry, paint, precedence,
  and accessibility projection;
- extensions receive no history by default and never receive grid, PTY,
  renderer, process, window, credential, or mutable session handles.

#### DN0 - decision, threats, limits, and tests

**Partially done at the documentation boundary.** Complete only after explicit
ADR acceptance, reviewed action naming, exact module/dependency ownership,
frozen machine-checked ceilings and mutation owners, a hostile/false-positive
corpus, and failing tests for every first-slice behavior/lifecycle contract.
No dependency or runtime activation is authorized by the current proposal.

#### DN1 - exact failed-command navigation

**Not done.** Add separate previous/next failed-command actions and palette
entries using trusted supported-shell result metadata. Navigate to the exact
prompt/input anchor, never a fabricated output region. Cover non-zero, success,
unknown status, oldest/newest/no-wrap/forward-to-live, manual movement,
new output, resize, eviction, route/session isolation, close, accessibility, and
disabled behavior. Assign no default shortcut before collision evidence.

#### DN2 - bounded on-demand scan infrastructure

**Not done.** Add bounded normalized logical-line batches, route-local scan
generations, one coalesced low-priority continuation, stale-result rejection,
optional 256-entry content-free cache, exact viewport placement, and complete
cleanup. Scan outside paint/input/PTY/resize/startup paths, release the terminal
lock before classification, and use deterministic row/byte ceilings plus a
measured yield target. Use generation invalidation or a proven small-cache
reflow remap; do not add a global logical-line ID first.

#### DN3 - generic built-in error sections

**Not done.** Add bounded structured severity, conservative anchored Error/Fatal
text, and bounded section reconstruction. Keep failed command, class, severity,
provenance, and confidence separate. Warnings remain outside the default action.
Malformed, oversized, Unicode, wrapped, overwritten, mixed-format, quoted,
negated, path/command, "0 errors", and output-storm cases require deterministic
negative tests, property checks, fuzz ownership, and performance ceilings.

#### DN4 - product interaction and assurance

**Not done.** Add transient renderer-neutral non-color highlighting, selection/
search precedence, immediate reduced-motion-safe scrolling, content-free
accessible announcements, tiny-to-8K responsive behavior, experimental
enable/disable, last-known-good configuration, and complete native/resource
evidence. A scan cannot pass merely because its focused test or one Windows host
passes; input/frame/lock/allocation/cleanup ratchets and Windows, macOS, Linux
X11/Wayland, WGPU/CPU where applicable, and assistive-technology evidence remain
separate gates.

#### DN5 - specialized formats and user patterns

**Not done.** Add Python tracebacks, Rust panics, compiler formats, and other
formats as independent measured slices rather than one bundle. User patterns
come only after built-in precision is stable, compile during atomic
configuration validation, and remain bounded to the reviewed count/source/
automaton/execution policy. Every detector has versioned identity, positive and
negative fixtures, limits, provenance, disable, cache invalidation, fuzz,
benchmark, native UX, rollback, reference, and changelog evidence.

#### DN6 - extension contributions

**Not done and not authorized.** Prefer declarative rules that core executes
without exposing content. Any parser that observes bounded terminal text gains a
privacy-sensitive read capability with explicit consent/scope, isolation,
quotas, revocation, cleanup, and malicious-detector evidence. It cannot persist
or forward text and cannot inherit filesystem, process, network, provider,
credential, PTY, renderer, or AI authority. Third-party delivery also waits for
ADR 0029.

#### Cross-phase release gates

The disabled path must perform no scan, create no worker, write no storage, and
change no terminal behavior. The grid plus trusted shell metadata remains
canonical; transient batches are discarded, anchors/metrics are content-free,
alternate screens are unavailable, and eviction/truncation/unsupported shells
remain truthful. Every implemented slice updates guide, exact action/config
reference, architecture/ADR, testing evidence, feature assurance, roadmap/audit,
and changelog together. The exact provisional ceilings and complete test ladder
are owned by the canonical specification.

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

### Post-v0.5.1 situation-aware production operations (PO0-PO8)

**PO0 is partially done only at a documentation/research-planning boundary.
PO1-PO8 are not done.** The
[canonical specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md),
[exact proposed PO0 contracts](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md),
[future testing contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md), and
[proposed ADR 0034](adr/0034-situation-aware-production-operations.md) define
the target without adding a provider capability, watcher, completion source,
setting, journal, model, execution path, or release claim.

PO0 now includes a strict non-activating JSON planning contract, canonical
digest, semantic checker, mutation suite, current-source nonactivation scan,
all 17 exact neutral record payloads, 37 action profiles with phase, effect and
evidence authority, provider/version/freshness profiles, rule and lifecycle
tables, planned settings and requirement traceability. The planning mirror is
not a runtime schema. This does **not** advance PO0 beyond partial: runtime
schemas/parsers, ADR/owner acceptance, dependency approval, reviewed prototypes,
first failing real-path tests, independent threat review and packaged
nonactivation evidence remain open.

The outcome is a production-aware terminal workflow that shows the exact
environment, prioritizes relevant native commands from current evidence,
answers recurring change, state, comparison, network and user-impact questions,
explains uncertainty, previews impact, preserves GitOps and just-in-time access
controls, and helps the operator investigate, hand over, verify and recover. It includes the requested
`kubectl rollout` experience: Automexia may prioritize `status`, `history`,
`undo`, `restart`, or diagnosis for affected workloads, but only after resolving
the real controller owner, cause, authority, policy, GitOps state, blast radius,
and evidence freshness. A restart is never preferred merely because Pods are
unhealthy.

The delivery order is:

1. **PO0 — decision and contract:** accept or supersede ADR 0034 and approve or
   revise the current proposed machine-contract digest; freeze exact owners,
   capabilities, limits, refusal/ranking semantics and dependencies; complete
   prototypes, first failing real-path tests and packaged nonactivation proof.
2. **PO1 — environment passport and lock:** add read-only pane-local provider,
   account, region, cluster, namespace, identity, expiry, GitOps, incident, and
   freshness context with exact context-change diff and production fail-closed.
3. **PO2 — bounded investigation evidence:** normalize public provider
   observations and deliver independently gated change/ownership/drift,
   resource/scheduling explanation, healthy cohort/revision comparison,
   read-only network-path diagnosis, optional SLO summaries, dependencies,
   health, rollout, events and permissions. Keep active probes, raw logs, time
   series, credentials, and persistent evidence caches out.
4. **PO3 — situation-aware completion:** layer deterministic hard gates,
   evidence/refusal reasons, lexicographic ranking, and exact insert-without-
   Enter behavior over CP5 while CP1 remains immediately available.
5. **PO4 — production preflight:** compose exact argv/targets, impact,
   permissions, risk, GitOps, JIT elevation, policy, change windows, approval,
   verification, and recovery without executing an action.
6. **PO5 — Incident Mode and journal:** pin incident context, hypotheses and
   coverage-qualified negative evidence; provide a bounded timeline, optional
   memory-only log fan-in, trusted/approximate time navigation and session
   journal; support reviewed redacted handoff/export with exact privacy,
   retention, recovery, and uninstall rules.
7. **PO6 — reviewed operation and diagnostic-session lifecycle:** only after D3
   and the applicable provider capability are independently activated, compose
   one explicit execute-observe-stabilize-verify-recover operation. Add
   Kubernetes port forward, controlled probe, or safe debug slices only behind
   their own one-use grant and native/provider cleanup evidence. Never chain a
   second mutation automatically.
8. **PO7 — organization packs and guided workflows:** accept signed, versioned,
   declarative runbook/policy packs and one-step-at-a-time cross-tool workflows.
   Packs add no scripts, credentials, capabilities, or execution grants.
9. **PO8 — adapter and release maturation:** add separately proven provider
   slices, read-only multi-region/cluster comparison with reviewed service
   equivalence and independent authority, and optionally evaluate a small local
   tie-breaker that can only reorder already-valid candidates. Deterministic
   policy remains authoritative.

#### PO phase-by-phase experience and build contract

The detailed surface layouts, copy, keyboard model, responsive behavior, user
journeys, implementation ownership, and UX definition of done are frozen by the
[Production Operations UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md).
The roadmap summary below is mandatory for planning and status review; it does
not activate any feature.

**PO0 — decision, prototype, and enforceable contract**

- **User experience:** validate nonactive prototypes of the environment
  passport, situation list, candidate detail, preflight, operation monitor, and
  Incident workspace. Cover the common read-only journey, reviewed insertion,
  managed-operation journey, narrow panes, keyboard-only use, stale/offline
  state, refusal, cancellation, failure, and recovery. Wording must distinguish
  observed fact, correlation, inference, unknown, and policy decision.
- **Implementation work:** accept or supersede ADR 0034 and accept or revise the
  current strict proposed machine contract; freeze versioned typed
  schemas, limits, risk/refusal/ranking rules, ownership-path and operation/
  session-kind semantics, surface
  ownership, accessibility names, focus transitions, protocol frames, privacy
  fields, resource budgets, feature flags, operation/session kinds, rollback,
  and status transitions.
  Maintain the present semantic checker, mutation suite, traceability and source
  nonactivation scan; create the exact scenario corpus and first failing
  real-path tests, benchmark plan, full dependency review, prototype evidence
  and packaged nonactivation test.
- **Exit:** owners approve the decision, threat model, prototype flows, wording,
  machine contract, and evidence plan. PO projections and actions remain absent
  from all six reused surfaces in the released runtime, and no capability,
  watcher, provider request, setting, or execution route is activated.

**PO1 — environment passport, lock, and context change**

- **User experience:** extend the existing context line above the command; do
  not add a second dashboard. In one glance it shows environment class, provider
  account/project, region, cluster/namespace, active identity/role, expiry,
  GitOps owner, incident, and freshness. Compact/minimal layouts preserve
  `PROD`, target, identity, and freshness first. A changed context shows the
  exact before/after fields with `Adopt`, `Keep locked`, and safe dismiss/close
  behavior; it never steals editor focus.
- **Implementation work:** add one immutable route-scoped passport snapshot,
  provenance/freshness per field, environment classification, production
  unknown/fail-closed state, lock generation, field-level context diff, and pure
  responsive projection. Reuse D2 route/session/revision cancellation and the
  existing UI/accessibility model; keep provider resolution off startup, typing,
  PTY, resize, and render paths. Clear snapshots and locks on route close,
  logout, revoke, disable, uninstall, and shutdown.
- **Exit:** exact pane/tab/window/clone isolation, context-change rejection,
  stale/offline recovery, focus stability, screen-reader semantics, controlled
  pixels, native shells, latency, memory/handle cleanup, and removal are proven.

**PO2 — bounded evidence, recent changes, and dependency graph**

- **User experience:** evidence appears only inside the requested situation or
  detail surface. One `Investigate` action exposes relevant `Explain state`,
  `What changed?`, `Compare with healthy`, `Diagnose connection`, and
  `Show user impact` choices. Rows show source, age, scope, coverage and
  supporting/contradicting/unknown state. Changes say “correlated with” unless
  an independent authority proves cause. Cohorts, network vantage, low-traffic
  SLO limits and unavailable sources are visible; raw logs and dashboards are
  not copied into PO2.
- **Implementation work:** define provider-neutral knowledge/evidence quality,
  observation, change, field ownership/drift, typed findings, cohort/revision,
  network-layer, permission, rollout, health and approved SLO-summary records.
  Resolve ownership by stable UID; normalize semantic fields rather than YAML;
  build bounded dependency/impact graphs; consume provider-native GitOps diff/
  ignore/self-heal behavior; preserve explicit unknowns and correlation-not-
  causation. Each adapter owns scope, deadline, quota, polling/watch, relist,
  throttle, cancellation, freshness, redaction and cleanup. No active probe,
  provider mutation, raw stream or default evidence disk cache exists.
- **Exit:** explainer/scheduling, change/ownership/drift, cohort/revision,
  network-path and SLO boundary tables pass with independent provider oracles;
  property/fuzz/model tests cover clocks, false-causal language, normalization,
  watch/relist/offline/stale/revoke, graph/cohort/fan-out limits, secret canaries,
  low traffic, unavailable vantage, resources, disable/uninstall and cleanup.

**PO3 — situation-aware completion and deterministic prioritization**

- **User experience:** typing `kubectl rollout` and explicitly requesting
  completion opens the existing CP5 pane-local list, not a new popup. A maximum
  of five current-situation rows show command, exact target, one reason,
  freshness, and `Read-only`, `Review required`, or `Refused`; normal shell
  completion remains reachable. Arrows move, detail is explicit, Escape closes,
  and Tab/Right Arrow inserts only the authenticated span without Enter.
- **Implementation work:** construct candidates only from an immutable passport
  plus evidence generation. Apply stale, target-UID, authorization, policy,
  GitOps, scope, contradiction, and supported-action hard gates before a stable
  lexicographic rank. Prefer diagnosis and reversible actions when evidence is
  equal; keep unknown business priority unknown. Project through the current
  CP5 controller/renderer/editor bridge, cancel obsolete generations, preserve
  stable selection, and keep CP1 immediately available when PO is disabled,
  offline, slow, or failed.
- **Exit:** the Kubernetes cause/owner corpus and cross-tool cases match an
  independent reference ranker; exact editor bytes/no Enter, native shells,
  stale cancellation, ordinary-keystroke zero-I/O, p95/p99 explicit-completion
  latency, accessible listbox behavior, responsive pixels, and fallback pass.

**PO4 — impact, authority, policy, GitOps, JIT, and recovery preflight**

- **User experience:** choosing a mutation opens one review in the fixed order
  environment → target → observed reason → expected impact → authority →
  GitOps/policy/change window → verification → recovery. Cancel is always
  available. Before PO6, the positive action is only `Insert reviewed command`;
  the shell owns later Enter. JIT access, ticket, or approval is a separate
  explicit step and cannot be silently requested.
- **Implementation work:** compose typed executable/argv/targets/UIDs, target
  count and blast radius, credential expiry, live permission result, admission
  limits, GitOps ownership/drift/sync window, organization policy, change
  window, approval/ticket, verification probes, timeout, and recovery options.
  Distinguish real provider dry-run/plan support from estimates. Bind all fields
  to route, context, evidence, policy, and candidate revisions; expire the review
  on any relevant change. Do not execute, elevate, approve, or mutate global CLI
  state in this phase.
- **Exit:** exhaustive boundary tables and independent provider/policy/GitOps
  comparisons prove fail-closed production behavior, truthful dry-run wording,
  exact target/argv display, cancellation, expiry, accessibility, native review,
  and forbidden-side-effect absence.

**PO5 — Incident workspace, live evidence navigation, and content-minimized journal**

- **User experience:** one deliberate action enters a pane/workspace showing
  objective, locked environment, current facts, hypotheses/contradictions/
  missing evidence, a short timeline, tried actions and one next safe action.
  Trusted entries can jump to their source; approximate terminal anchors say so.
  Optional log fan-in is visibly source-separated, pausable and memory-only with
  gaps/drops shown. Journaling is `Session only` by default. Handoff previews
  included/redacted facts, hypotheses, outcomes, unresolved questions and links.
- **Implementation work:** add incident/hypothesis/session state, coverage-
  qualified negative evidence, bounded deduplicated timeline, trusted/
  approximate anchors, evidence/policy/receipt references and content-minimized
  journal. Add a separate bounded per-source live-log controller with
  backpressure, source-local sequence, clock/gap state and Error Navigation
  references. Store no raw terminal output, log stream, time series, provider
  body, secret or arbitrary environment value. Optional persistence keeps its
  protected atomic migration/retention/recovery/export/delete lifecycle.
- **Exit:** session-only zero-write, log ordering/gap/backpressure and time-anchor
  oracles; hypothesis/negative-evidence, crash/restore/handoff/export/retention,
  privacy canaries, 24-hour resources, multiple-operator conflict, native focus/
  accessibility and complete stream/storage removal pass.

**PO6 — one managed operation or diagnostic session**

- **User experience:** only after separate activation, preflight may offer
  `Execute reviewed action`. A nonmodal monitor shows exact target and one state:
  preparing, running, observing, stabilizing, succeeded, failed, cancelled,
  uncertain, or recovery available. The same review/monitor pattern owns a
  Kubernetes port forward, controlled probe or safe debug session; each shows
  listener/vantage/image/profile, authority, lifetime and cleanup. Success is
  never inferred from process exit alone and no second mutation auto-starts.
- **Implementation work:** route one typed action through the activated D3 or
  provider broker after final context/UID/evidence/permission/policy revalidation.
  Own exact argv/request/environment, deadlines, process descendants, provider
  operation IDs, cancellation, before-state, progress observations,
  stabilization/regression/verification predicates, uncertain/partial results,
  content-minimized receipts, and a separate reviewed recovery proposal. Each
  port-forward/probe/debug slice binds exact target, local endpoint or vantage,
  immutable image/profile, traffic/time limits, authorization/admission,
  descendants/listeners/temporary resources and cleanup. Never evaluate display
  text or silently fall back to shell insertion.
- **Exit:** real disposable provider/cluster workflows prove exact side effects,
  no forbidden second action, process/resource cleanup, timeout/cancel/context-
  change/policy-revoke behavior, observation/stabilization/regression,
  verification and recovery, plus per-slice listener/probe/debug privilege,
  orphan and cleanup matrices, native monitor UX, packaging, rollback, and
  controlled release. D3 and each provider capability must already be
  independently active.

**PO7 — signed organization packs and one-step guided workflows**

- **User experience:** installation shows publisher, signature, version,
  supported tools, rules, required data, conflicts, and zero execution grants.
  A guided workflow presents one reviewed step, its evidence and expected
  result, then stops. The operator explicitly requests the next step; disabling
  or removing a pack restores deterministic built-in behavior immediately.
- **Implementation work:** define strict declarative schemas for service
  criticality, ownership, dependency hints, allowed/refused actions, verification,
  and runbook steps. Reuse the accepted D7 signature/provenance/limits/store
  boundary only after its capability decision. Reject scripts, shell strings,
  callbacks, credentials, provider authority, hidden capabilities, cycles,
  unsupported steps, policy conflicts, downgrade, and compromised updates.
- **Exit:** malicious-pack, canonical-digest, revoke/update/rollback, policy-
  conflict, one-step boundary, cross-tool result, provenance, disable/uninstall,
  accessible review, and native package tests pass without granting capability.

**PO8 — adapter maturation, cross-environment comparison, optional local
tie-breaker, and release proof**

- **User experience:** AWS, Azure, Google Cloud, Kubernetes/OpenShift, GitOps,
  identity, and observability slices use the same passport, row, detail,
  preflight, monitor, and incident language. Unsupported or disconnected sources
  say what is missing and preserve safe shell completion. An optional local
  tie-breaker has an understandable setting and explanation; turning it off
  immediately restores deterministic order. `Compare environments` is read-only
  and first shows the reviewed service-equivalence mapping, selected regions or
  clusters, independent authorization and freshness, traffic/window coverage,
  and fan-out limit. It never creates a multi-environment mutation.
- **Implementation work:** ship adapters one at a time behind separate feature,
  capability, quota, version, auth, redaction, resource, rollback, and release
  gates. If adopted, a small offline model may only reorder candidates that
  passed deterministic gates; it cannot create/refuse/execute actions, read raw
  logs or secrets, use a network, or override policy. Enforce memory/CPU/model-
  size limits, signed provenance, deterministic fallback, and instant disable/
  uninstall. Add cross-environment comparison only after explicit equivalence
  mapping and independent per-environment access; never merge authority. No LLM
  becomes a core or domain-extension dependency.
- **Exit:** each named adapter has controlled native account/environment proof;
  the optional ranker has differential safety, adversarial, latency, memory, and
  fallback evidence; and Windows, Linux, and macOS package, accessibility,
  visual, soak, rollback, removal, and protected release gates pass on the exact
  artifact being claimed.

Provider/network work stays off typing, PTY, resize, renderer, and startup hot
paths. One active request per pane, bounded namespace-scoped collection,
generation cancellation, explicit freshness, no disk evidence cache by default,
and content-minimized receipts keep the feature lightweight. Managed mutation
through Automexia remains impossible until current context, permission, policy,
impact, confirmation, execution, monitoring, recovery, native, resource,
accessibility,
and release gates all pass.

## v0.6 — sandboxed extension platform and selected-input model suggestions

**D7/CP6 is partially done overall and fully done locally at the accepted source
boundary.** ADR 0029 and the exact digest-frozen contract were accepted on
2026-08-25. The repository now owns private pure policy models, strict local
signed-bundle verification, protected atomic disabled storage, custom WIT and a
no-default-WASI Wasmtime conformance host, exact grants and lifecycle, signed
non-executing action-pack mapping, selected-input consent/response models,
renderer-neutral review states, a nonactivating product adapter, property/fuzz/
mutation coverage, and same-host benchmarks.

Release authority remains false. There is no active guest component, public
registry/download/SDK, distribution transport, startup or typing network,
provider request, model tool/workflow/MCP call, automatic execution, process,
credential, or PTY authority. The public execution API fails with
`ActivationDenied`; the terminal adapter also hard-denies downloads, provider
calls, and grants. Private first-party features and CP1-CP3 remain the fallback.

The package path is explicit-local-file-only. Verification binds exact content,
publisher/key, signature, provenance, SPDX/license evidence, compatibility,
time, trust and current revocation before bytes reach no-follow staging. The
store publishes only `installed-disabled`, retains two validated generations,
recovers last-known-good state, and removes only Automexia-owned package data.
The Component Model feature inventories exact imports, links no default WASI,
and applies fuel, epoch deadlines, memory/table/instance/transfer/output/log
limits with cancellation and joined workers.

The initial CP6 slice receives only explicitly selected bounded text after
normalization, redaction preview, and per-request provider/locality/model/
destination/purpose/retention/size/risk review. Consent is exact, expiring,
route/generation bound and single-use. Responses are strict typed bounded
explanations or suggestions with independently assigned risk. Ambient terminal,
history, clipboard, file, environment, agent, credential, provider, capsule,
connection, other-pane, log, telemetry and support-bundle data are unavailable.
Copy/insert never implies Enter.

The source boundary cannot become a public release until protected exact-head
approvals, named trust/revocation/update owners, dependency/supply-chain drills,
native Windows/Linux/macOS signed package and sandbox evidence, actual product
visual/accessibility/IME/focus evidence, privacy/legal approval per provider,
controlled resource baselines, 1,000 lifecycle cycles, the 30-day soak, and
verified kill/disable/uninstall/rollback/fallback evidence pass. Public download
also requires a separate network/distribution ADR and maintained update client.

### Optional LLM Orchestration extension (LO0-LO5)

**LO0 is partially done only at a documentation/research-planning boundary.
LO1-LO5 are not done.** The canonical
[LLM Orchestration specification](LLM-ORCHESTRATION-EXTENSION.md),
[testing contract](LLM-ORCHESTRATION-TESTING.md), and
[proposed ADR 0033](adr/0033-optional-llm-orchestration-extension.md) define the
target without adding a dependency, model request, workflow runtime, UI, or
authority.

The delivery order is:

1. **LO0 — decision and contract:** accept the boundary, freeze a strict machine
   contract and exact owners/limits/threats; documentation alone activates
   nothing.
2. **LO1 — neutral workflow model:** add pure versioned action descriptors,
   plans, digests, one-run grants and structured results below all domain
   extensions, with no I/O or execution authority.
3. **LO2 — plan only:** add a disabled-by-default first-party extension, explicit
   context consent and one local/self-hosted provider path that returns draft
   plans but invokes no action.
4. **LO3 — reviewed one-run execution:** let the application registry validate,
   review, grant and invoke sequential low-risk steps through existing brokers,
   interrupting again for production and other sensitive work.
5. **LO4 — bounded multi-extension workflows:** add dependencies, safe
   concurrency, structured results and at most three replans with deterministic
   partial-failure, cancellation, recovery, resource and leak proof.
6. **LO5 — provider/ecosystem expansion:** add individually reviewed remote
   adapters, optional storage, organization policy or typed MCP mappings only
   after their own privacy, cost, security, native and lifecycle evidence.

The orchestrator is optional and independently installable. Models propose
candidate typed plans; they receive no executor, PTY, shell, filesystem,
credential, provider, DevOps, Studio, video, or MCP handle. The application owns
the current action registry, independent risk/policy checks, exact plan review,
one-run grant, final revalidation, execution, cancellation and receipts. Local or
self-hosted inference is the first direction, no paid API is required, and no
provider fallback is silent.

LO1 may begin only after explicit LO0 acceptance. Later LO work is independent of
Automation Studio and video release ordering and must not block either. Domain
extensions remain deterministic and fully useful when the orchestrator is
absent, disabled, offline, crashed or uninstalled. Third-party delivery also
waits for the accepted D7 package/sandbox boundary.

### Automation Studio and DevOps/SRE scripting (AS0-AS6)

**AS0 is partially done only at a proposal/research-planning boundary. AS1-AS6
are not done.** The canonical
[Automation Studio architecture](AUTOMATION-STUDIO-ARCHITECTURE.md), future
[testing contract](AUTOMATION-STUDIO-TESTING.md), and
[proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md)
define the target without adding a dependency, runtime, product UI, or authority.


The release position is intentionally narrower than the AS0-AS6 phase list:

- AS0 research and native feasibility may continue before the first stable v0.4
  release, but cannot add production dependencies or become a release blocker;
- AS1-AS2 product implementation follows the stable terminal and reuses the
  independently proven extension, workspace, process, and DevOps boundaries;
- the first releasable minimal Studio must pass AS2 before any dedicated video-
  editing extension is released;
- AS3-AS6, LO and video research may then proceed independently through their
  own gates; video does not wait for unrelated Studio debugging, remote,
  collaboration, or orchestration work.

The target keeps three packages independently owned:

- optional Automation Studio for embedded files, diff and diagnostics beside
  terminal panes;
- optional DevOps/SRE for terminal-native infrastructure workflows, target/risk
  context, templates, plans and typed run intents;
- optional language/tool add-ons for exact grammars, language servers,
  formatters, linters, validators and CLIs.

A metadata-only DevOps/SRE Pack may install a compatible set for convenience,
but it grants no authority. The terminal remains useful when every package is
disabled or uninstalled.

The ordered delivery is:

1. **AS0 — decision and feasibility:** accept the durable boundary only after a
   CodeMirror/Monaco comparison, Wry/native-host proof on Windows, macOS, Linux
   X11 and Linux Wayland, exact dependency review, limits fixture and terminal-
   only baseline.
2. **AS1 — neutral contracts:** implement core-owned documents, atomic save and
   recovery, workspace trust, grants, typed IPC, lifecycle and surface
   abstraction without a product editor or process/network authority.
3. **AS2 — minimal Studio:** add local open/edit/search/diff/save/recovery with
   native keyboard, IME, accessibility, resource, crash and package evidence;
   no LSP or script execution.
4. **AS3 — language intelligence:** add a bounded LSP 3.18 broker and small
   first-party language packs through supervised installed servers.
5. **AS4 — DevOps/SRE scripting:** bind a saved document revision, exact tool
   identity/argv, environment capsule, target, opaque secrets, risk, limits,
   plan/review generation, capability and expiry in a typed run intent. Never
   type into an existing PTY or press Enter.
6. **AS5 — domain packs:** add Terraform/OpenTofu, Kubernetes/OpenShift,
   Ansible, cloud and policy integrations only through individual version,
   provider, security, native, resource and lifecycle gates.
7. **AS6 — advanced/remote:** consider DAP, remote documents, mobile,
   collaboration and neutral workflow-registry interoperability only through
   separate ADRs and threat models. Studio itself has no model/provider logic.

First-party Studio work does not wait for a public marketplace after its own
acceptance, but third-party add-ons still require D7/ADR 0029. CP5 remains the
separate optional native shell-line suggestion bridge; it is not this file editor.

### Future specialized workflow domains

Specialized domains may be explored after the core and ecosystem boundaries are
proven, and the terminal must remain useful when every such extension is
disabled. Candidate domains include data processing, automation, and creator
workflows. Media and video support is research only: Automexia does not
currently ship editing, timeline, rendering, or media-project runtime features.

Any specialized domain must define its owner, dependencies and licenses,
process/network/file authority, CPU/GPU/memory/storage limits, cancellation and
recovery, accessibility and native-platform evidence, and complete
disable/uninstall/rollback behavior before implementation or product claims.

A dedicated video-editing release follows the minimal AS2 Studio release, but
it does not become a Studio add-on. Both may reuse generic core-owned workspace,
file, task, progress, cancellation, recovery, and extension-lifecycle services;
video must own its media project, preview, timeline, render, CPU/GPU, and storage
contracts without depending on Studio's webview, editor state, LSP, or DevOps
packages. Terminal-native media-tool use and non-production video research may
continue before that release order is reached.

See the [Product vision](PRODUCT-VISION.md) and the non-authoritative
[video automation RFD](../automexia_docs_repository_aligned/research_proposals/rfds/RFD_VIDEO_AUTOMATION.md).

## Compatibility track

Automexia keeps its classic `automexia` shortcuts as the implicit default. The
explicit `ghostty-1.3` profile and moving `ghostty` alias are implemented, but
an unqualified cross-platform compatibility release is still gated. The
[full Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) is the
source of truth: G1-G4 are fully done locally; G0/G5 remain partially done on
native release evidence; G6 is partially done at the accepted modal redacted
inspector, parked count/list/newest-restore/two-step-clear, and complete-top-
level-tab history scope. Individual topology history and controlled native
lifecycle proof remain gated. Complete compatibility is not a v0.4 release
criterion.

## Assurance delivery track

The [stabilization roadmap](STABILIZATION-ROADMAP.md) is authoritative for the
detailed implementation and exit gates. The version assignment is:

| Capability | v0.4 | v0.5.0 SSH | v0.5.1 multi-cloud | v0.6 ecosystem |
|---|---|---|---|---|
| Test orchestration and evidence | Pinned Nextest/JUnit, timeouts, leak reporting, and redacted QA bundle | Component profiles plus SSH/launch/broker evidence | Provider authentication, capsule, and remote-transport evidence | Public SDK/package/sandbox evidence |
| Visual verification | Deterministic renderer state, controlled frame capture, diffs, and human platform review | Generic status, quick-connect, host-key, tunnel, grant, and error goldens | Multi-provider identity, risk, login, expiry, and stale-state goldens | Third-party and selected-input model-consent UX contracts |
| Property/concurrency testing | Proptest resize/session invariants and initial bounded Loom models | Exact argv, operation/cancellation, queue/cache, tunnel, and teardown state machines | Capsule isolation, provider refresh, exec-plugin, and broker state machines | Capability/sandbox/package state machines |
| Performance | Execute Criterion, collect 30-day baselines, record startup/interaction/resource data | Enforce ratchets and add SSH index/connect/tunnel/saturation budgets | Add CLI/config/API refresh and 10/50/100-session budgets | Add public SDK/sandbox overhead budgets |
| Native assurance | AppVerifier/WPR and controlled Windows/Linux/macOS GPU/shell matrices | System OpenSSH, agents, certificates, host keys, jumps, tunnels, cancellation, and cleanup on each OS | Official provider CLIs, Kubernetes/OpenShift, SSM/Bastion/IAP, offline/expiry paths | Sandboxed third-party and CP6 provider isolation |
| Test-strength/security ratchets | Longer fuzz corpora and Automexia-owned coverage baseline | Fuzz config/index/IPC/diagnostics; mutation-test policy and argv validation | Fuzz provider/config/exec-plugin adapters; audit SDK/CLI supply chain | Public extension supply-chain, signature, revocation, and capability audits |
| Command productivity | CP0 baseline plus CP1 shell-native managed completion, diagnostics, explicit bounded refresh, and CMD fallback; no action-store claim | CP2-CP3.3: typed persistent actions, opt-in aliases, static DevOps packs, selected native imports, and trusted exact workspace task bridges | CP4: capsule/provider-aware cached actions with freshness and brokered exact launch | CP5/CP6: optional editor bridge, signed ecosystem packs and selected-input suggestions after separate gates |
| LLM orchestration | No model or workflow-runtime claim; LO0 documentation may proceed | No dependency on v0.5 SSH delivery | Neutral workflow design may reuse proven typed actions without gaining provider authority | LO1-LO5 only after separate acceptance, security/privacy/resource/native/lifecycle evidence; never a Studio/video blocker |
| Automation Studio | No file-editor claim | Preserve terminal-only fallback while AS0 evaluates dependencies and the native host | Reuse provider context and typed review without coupling documents to provider extensions | AS1-AS6 document/editor/LSP/DevOps execution phases only after their separate acceptance and native evidence |

No single host or test layer may claim complete assurance. Pull requests prove
deterministic contracts, nightly jobs explore expensive state and native
behavior, release jobs require controlled hardware and packaging evidence, and
maintainers record the remaining visual/accessibility decisions.
