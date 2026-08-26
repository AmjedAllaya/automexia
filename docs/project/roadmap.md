# Roadmap

This page is a condensed project overview retained from the documentation
reorganization. The [canonical roadmap](../ROADMAP.md) and
[phase implementation audit](../PHASE-IMPLEMENTATION-AUDIT.md) are the
machine-enforced sources for current sequencing and phase status. This summary
must not override them. Detailed test commands and evidence live in
[Testing](../TESTING.md).

A phase can be source-complete while release evidence is still partial. User-facing guides remain authoritative for what users can rely on today.

## Product direction

Automexia is being built for anyone who uses commands to turn ideas into
results, not only for software development or remote operations. The current
phases provide the terminal, workspace, productivity, security, and extension
foundations. Later data, automation, media, video and optional LLM orchestration
work must be proposed and proven as separate slices; none of it is claimed as a
current feature. Automexia remains complete without a model, account, network
connection or paid API. See the [Product vision](../PRODUCT-VISION.md).

## Executive phase matrix

| Track | Phase | Implementation | Release evidence | Conclusion |
|---|---|---|---|---|
| Core | v0.4/S0 | **Fully implemented locally** | **Partial / external** | Identity, hostile-input bounds, atomic reload, current terminal gates, exact stable tag/main/fork/DCO provenance, and authenticated repository-audit enforcement are complete. Hosted workflow integration, historical linearity/DCO resolution, final brand/governance/signing inputs, plan/reviewer/security prerequisites, and controlled native evidence remain. |
| Assurance | v0.4/S1 | **Fully implemented in source** | **Partial / external** | Exact deterministic visual hooks, Windows WGPU/CPU native/resource automation, separate bounded verifier phases, a strict 24-suite policy/validator, controlled workflow, and fail-closed stable-tag dependency are complete. Controlled Linux/macOS/named-GPU/elevated/accessibility evidence and approved visual matrices remain. |
| Performance | S2 | **Fully implemented in source; collecting** | **External baseline pending** | Strict bounded comparable evidence, repeated-sample quality, native memory composition, exact source/operator binding, independent activation review, 90-day retention, and fail-closed release enforcement exist; delivery still requires 30 reviewed consecutive controlled-runner days. |
| DevOps | D0 | **Partial** | **Partial** | ADR 0012 is accepted; schema 6 ratchets immutable schemas 1/2/3/4/5 with exact M3-M5 routes/tunnels, trust/status, tunnel lifecycle, receipt/reconnect, and native-manifest rules. Protected approvals and real native execution remain. |
| DevOps | D1 | **Fully implemented** | **Partial** | Four private provider-neutral crates and bounded contracts satisfy their source boundary. |
| DevOps | D2 | **Fully implemented** | **Partial** | Generic status, immutable history, capsule/cache/session isolation, cancellation, and truthful freshness exist. |
| DevOps | D3 | **Partial; nonactivated** | **Blocked** | The hard-disabled broker/runner owns bounded current-executable review, exact managed argv/identity binding, actual outcomes, process-group/Job Object termination, bounded PTY-worker joining, and redacted receipts through ContextManager; attestation, protected activation, and real native descendant/resource/accessibility proof remain. |
| DevOps | D4 | **Fully implemented; read-only product adapter active** | **Partial** | Bounded OpenSSH inventory/persistence is connected only to D5.1 reviewed browsing; it retains no process/network/launch authority. |
| SSH UX | D5.0-D5.2 | **Partial overall; D5.1 and nonactivated F5.1-F5.3 source complete locally** | **Partial/blocked** | Typed direct/config-jump/tunnel argv, loopback defaults, strong tunnel review, full trust evidence, guarded lifecycle, receipts, reconnect, and compact tunnel states pass locally. Protected activation/attestation, actual status/SSH execution, and controlled real native descendant/listener cleanup and accessibility proof remain. |
| Multi-environment | M6/F6 | **Partial overall; review/edit source and product surfaces fully implemented locally** | **Blocked on D3/M5 execution evidence** | Accepted ADR 0023; schema-2 persistence/migration/recovery; public preview-first CLI; immutable worker publication; Connection Hub catalog/restore review; typed recipes/no-hooks; narrow remote initialization; armed broadcast; and bounded tests pass. Managed execution and controlled native/resource/accessibility/release evidence remain. |
| Multi-cloud | D6.0/M7 | **Fully implemented locally** | **Partial/external** | Strict provider-neutral schemas, immutable session capsules, 19-state lifecycle, exact allow-once review, isolation/redaction, passive-status migration, cached-only semantics, fuzz/mutation, bounded lifecycle, and a 64-capsule benchmark pass. Real provider CLI/native evidence remains external. |
| Multi-cloud | D6.1-D6.5 | **Partial overall; M8-M11 and M12 Teleport source-complete/nonactivated** | **Partial/blocked** | AWS, Azure, Google Cloud, Kubernetes/OpenShift, and Teleport bounded exact adapters pass locally. D3 product activation and controlled provider/native evidence remain; OpenBao is absent pending ADR 0024 acceptance. |
| Ecosystem | D7 | **Fully implemented locally at accepted nonactivating source boundary; partial overall** | **Blocked on protected release/native evidence** | Accepted ADR 0029 and exact digest; private strict policy/runtime crates, signed local verification/store, optional no-WASI Wasmtime conformance, capabilities/lifecycle, disabled action packs, selected-input consent/product adapter, 17 mutations, properties, fuzz harness, benchmarks and Windows evidence exist. Activation/download/public SDK/provider calls remain false; trust governance and native release evidence remain. |
| LLM orchestration | LO0 | **Partially implemented at documentation-only proposal boundary** | **Blocked on ADR acceptance and machine contract** | Specification, proposed ADR 0033, neutral-workflow direction, provisional limits and future evidence ladder exist; no workflow crate, model/provider adapter, registry, executor, UI, MCP mapping or runtime exists. |
| LLM orchestration | LO1-LO5 | **Not implemented** | **Blocked on LO0 and later protected/native gates** | No neutral workflow implementation, orchestrator extension, model request, reviewed one-run execution, multi-extension workflow, provider delivery or release evidence exists. |
| Automation Studio | AS0 | **Partially implemented at proposal-only boundary** | **Blocked on acceptance/native feasibility** | Architecture, proposed ADR 0030 and the future test ledger exist. Exact dependencies, limits and native Windows/macOS/Linux X11/Wayland editor-host proof remain. |
| Automation Studio | AS1-AS6 | **Not implemented** | **Blocked on AS0** | No document service, editor/webview, LSP/DAP broker, Studio/DevOps integration, file-write or typed script-run product path exists. |
| Diagnostic navigation | DN0 | **Partially implemented at documentation-only proposal boundary** | **Blocked on ADR acceptance and implementation evidence** | The canonical specification, evidence ledger, proposed ADR 0032, owner/resource/privacy/accessibility/test contract, and DN0-DN6 sequence exist; no runtime is authorized. |
| Diagnostic navigation | DN1-DN6 | **Not implemented** | **Blocked on DN0; DN6 also needs a separate capability decision** | No failed-command action, scan, detector, section locator, cache, highlight, setting, user pattern, extension contribution, or native release evidence exists. |
| Production operations | PO0 | **Partially implemented at detailed proposal/checker boundary** | **Blocked on runtime schemas plus ADR/exact-digest and owner/dependency/capability acceptance** | The canonical specification, exact proposed PO0 contract/digest with 17 record payloads and 37 phase/effect/authority-bound actions, semantic checker/mutations/nonactivation scan, proposed ADR 0034, 2026 source baseline, limits, provider/rule/lifecycle/settings/traceability detail and future evidence ledger exist; runtime schemas/parsers, prototypes, first real-path failures and runtime behavior remain absent. |
| Production operations | PO1-PO8 | **Not implemented** | **Blocked on PO0 and phase-specific CP5/D6/D3/D7 gates** | No environment passport/lock; change/ownership/drift, resource/scheduling, cohort/environment, network/SLO, live-log/time/handoff or dependency view; situation-aware completion; rollout prioritization; preflight; GitOps/JIT/policy composition; Incident Mode; journal; managed operation or diagnostic session; runbook pack; provider adapter; local ranker; or release evidence exists. |
| Productivity | CP0 | **Fully implemented** | **Partial** | Accepted architecture, threat model, ceilings, fixtures, mutations, and nonactivation policy exist. |
| Productivity | CP1 | **Fully implemented** | **Partial** | Shell-native completion and bounded explicit refresh exist; hosted native evidence remains. |
| Productivity | CP2.0 | **Fully implemented** | **Partial** | Bounded typed Quick Action model and hostile corpus exist with no runtime authority. |
| Productivity | CP2.1 | **Fully implemented as internal library** | **Partial** | Private atomic persistence, CAS, recovery, watches, and benchmarks exist; no startup/UI activation. |
| Productivity | CP2.2 | **Implemented locally** | **Partial** | Layered search, placeholder/risk/conflict review, bounded import/export/CRUD/recovery, and explicit insert/copy UI are present. Hosted native shells, controlled screen readers, and 30-day performance/resource evidence remain release gates. |
| Productivity | CP3.0 | **Fully implemented at pure boundary** | **Partial** | Five pure serializers, bounded inventories, metadata/tamper verification, tests, fuzz, benchmark, and policy ratchets are complete; activation is disabled and hosted native evidence remains. |
| Productivity | CP3.1 | **Fully implemented locally** | **Partial** | Explicit opt-in persistence, crash-safe all-old/all-new publication, verified five-shell startup/reload, diagnostics, rollback, and exact uninstall are implemented; hosted native/macOS/WSL and controlled-baseline evidence remains. |
| Productivity | CP3.2 | **Fully implemented locally** | **Partial** | Eleven static provider packs, 33 disabled-by-default actions, health/update/alias-safety, CLI, tests, fuzz, benchmarks, and policy gates are complete; hosted evidence remains. |
| Productivity | CP3.3 | **Fully implemented locally** | **Partial** | Six explicit native inventory formats, exact just/Task/mise bridges, bounded dry-run/CAS import/workspace/trust/revocation/removal, path-free digest/revision receipts, background cache authorization, final insertion recheck, tests, mutation, fuzz, benchmark, ADR, CLI, and docs are complete; hosted native/accessibility and 30-day evidence remain. |
| Productivity | CP4 | **Fully implemented locally at product-integrated nonactivating boundary** | **Partial/external** | Seven cached provider projections, retained cached-product-to-route handoff, route/session/revision/generation isolation, idempotence/revocation, final revalidation, accessible state/risk UX, production confirmation, fuzz/benchmark/policy evidence are complete locally. Approved provider refresh/capsule production, exact execution, OpenBao, and real native/provider/accessibility/release evidence remain. |
| Productivity | CP5.0 | **Fully implemented at research boundary** | **Complete locally** | Seven-shell API matrix, pure insertion prototype, locked matcher/dependency evidence, privacy review, and retain-CP1/defer-P2 decision are machine-gated; no runtime surface exists. |
| Productivity | CP5.1-CP5.6 | **Partially implemented overall; CP5.1-CP5.4 fully implemented at source/local boundaries; CP5.5 inert bridge source complete; CP5.6 partial** | **Partial/external; preview disabled** | Accepted ADR 0025; bidirectional protocol; restrictive Windows/Unix endpoint sources; bounded broker/sources/ranking; pane UI/controller/renderer; helper target and four bidirectional shell adapters; fuzz/benchmark/mutation/lifecycle and local Windows PowerShell plus WSL Bash/Zsh/Fish evidence. Reviewed launcher/signing, WSL relay, interactive PowerShell replacement, live composition, native three-OS/accessibility/package/rollback/resource/30-day evidence remain. CP1 remains fallback. |
| Productivity | CP6 | **Fully implemented locally at accepted no-provider source boundary; partial overall** | **Blocked on provider/native/release evidence** | Verified capability-free packs map only into disabled typed actions; exact selected-input redaction, disclosure, single-use consent, typed response, independent risk, and review models exist. Provider/tool/workflow/MCP/Enter/execution remain hard-disabled; native UI, provider privacy/legal, lifecycle and release evidence remain. |
| Compatibility | G0 | **Partially done** | **Partially done** | Exact Ghostty 1.3.1 Linux/BSD provenance/fixtures/checksums, deterministic Windows adaptation, ADR 0026, generation, verification, classic golden, and properties exist. Native macOS fixture and Linux/macOS release evidence remain. |
| Compatibility | G1 | **Fully done** | **Partially done** | Pure typed registry, schemas, layers, direct/reverse indexed lookup, reserved sequence/table structures, classic bridge, palette derivation, and Windows latency evidence exist; external platform evidence remains. |
| Compatibility | G2 | **Fully done** | **Partially done** | Explicit profiles, bind/unbind/priority layers, strict diagnostics, immutable atomic reload, global/palette transaction, and dry-run/confirmed migration are implemented; native release matrices remain. |
| Compatibility | G3 | **Fully done** | **Partially done** | Structured outcomes, fallthrough, exact sequences/cancellation, bounded tables/catch-all/chains, stable all-surface execution, per-route state, and shell ownership are implemented; native IME/layout evidence remains. |
| Compatibility | G4 | **Fully done** | **Partially done** | Clear, selection/search, topology, zoom/equalize/resize, and bounded private export families are implemented; controlled native visual/resource checks remain. |
| Compatibility | G5 | **Partially done** | **Partially done** | CLI/migration, xtask generation/verification/tests, generated references, property/fuzz targets, and Windows Criterion benchmarks exist. Native three-platform, visual, accessibility, resource, and 30-day evidence remains. |
| Compatibility | G6 | **Partially done** | **Partially done** | ADRs 0027/0028, the redacted inspector, and bounded parked-PTY undo/redo for complete top-level tabs exist. Individual split/local-tab/native-window history and native lifecycle evidence remain. |

## SSH activation M2/F4/D3 status

| Requirement | Implementation status | Release evidence | Remaining exit work |
|---|---|---|---|
| M2 protected activation | **Partially done** | ADR 0012 is accepted; exact-scope policy, mutation gates, and the fail-closed application boundary pass locally. | Two independent exact-head approvals and server enforcement, green hosted S0/CodeQL gates, real loader/build attestation and live revocation, then enablement. |
| F4 process/PTY lifecycle | **Partially done; nonactivated** | One Router-owned runner, capacity-one current-executable review worker, 50-operation cap, 256-record redacted audit FIFO, exact executable guard, ContextManager PTY/route publication, bounded process-group/Job Object teardown, PTY-worker joining, rollback, cancellation, close, and shutdown pass locally. | Protected attestation/activation plus Windows/macOS/Linux and gated-WSL real OpenSSH descendant, resource, pixels, and accessibility evidence. |
| D3 exact-argument broker | **Partially done; nonactivated** | Exact typed package/capability/session/capsule/argv/environment/cwd policy, fresh executable-bound second review, and guarded lifecycle are production-compiled; the activation constant is false and the linked candidate is unverified. | Attested first-party grant, protected activation, and real native replacement-race and 1/10/50 lifecycle proof. |

The safe current outcome is an implemented but unavailable boundary: no managed
SSH child can start, and normal shell-owned `ssh` remains the recovery path.

## Release sequence

### v0.4 — standalone terminal stability

v0.4 freezes the terminal-core product boundary: standalone identity/migration, bounded hostile terminal input, transactional runtime configuration, panes/tabs/context/footer, shell integration, image handling, contributor automation, and multi-platform release policy. S0 source gates are complete locally; S1 controlled cross-platform/visual/accessibility evidence remains partial; S2 source enforcement is complete but its comparable baseline remains collecting and inactive.

Managed saved-host/cloud behavior is intentionally not added to v0.4. Users can always run normal `ssh` in their shell.

### Post-v0.4 semantic diagnostic navigation

The DN0 design may be reviewed before v0.4 closes, but DN1 product work follows
the stable release. DN1-DN4 may proceed independently alongside later v0.5 work;
they have no assigned release and do not block SSH, providers, Automation
Studio, or video. DN5 follows generic precision/resource proof. DN6 remains
blocked on a separate terminal-output privacy/capability decision and ADR 0029
for third-party delivery.

The [canonical specification](../SEMANTIC-DIAGNOSTIC-NAVIGATOR.md) and
[proposed ADR 0032](../adr/0032-bounded-semantic-diagnostic-navigation.md) own
the detailed interaction, architecture, limits, lifecycle, and evidence gates.

## Early DevOps and SSH delivery track

The first managed-connectivity milestones are deliberately ordered around trust and lifecycle boundaries rather than UI breadth.

### v0.5.0 — production SSH and command-productivity maturation

The dependency order is more important than the UI wish list:

1. Preserve D1/D2 provider-neutral contracts, session isolation, freshness, and bounded workers.
2. Complete protected review/activation of the D3 exact-argument launch broker.
3. Keep D4 OpenSSH inventory non-executing and last-known-good.
4. Preserve the locally complete read-only D5.1 product surface and obtain its remaining native/accessibility release evidence.
5. Enable reviewed system-OpenSSH launch/lifecycle in D5.2 with native security/resource evidence.
6. Promote the already-local CP1-CP3.3 completion/Quick Action/alias/import/workspace work only after its native/accessibility/performance release evidence passes.
7. Preserve accepted ADR 0023 and the locally complete review-only M6 CLI/Hub surface; obtain D3/M5 managed-execution and native evidence before enabling any recipe or multi-target action.

The first broad DevOps release should therefore be a secure terminal-first SSH product, not a provider SDK platform.

### v0.5.1 — multi-cloud and orchestrator adapters

After the generic process/capability/session boundary is proven, deliver separately enabled provider/context adapters. Official CLIs and local configuration remain preferred authority. AWS, Azure, GCP, Kubernetes, OpenShift, infrastructure, or organization-identity adapters must fail independently and publish only bounded public metadata/context into the core.

### Post-v0.5.1 — situation-aware production operations

Deliver `PO1-PO5` after the relevant CP5 and read-only D6 foundations. Begin with
the pane-local production passport/lock, then a bounded evidence and dependency
graph with change/provenance, resource explanation, healthy comparison, passive
network diagnosis and approved SLO summaries; then situation-aware insert-only
candidates, production impact preflight, and Incident Mode with hypotheses,
time/log navigation and content-minimized handoff. The requested Kubernetes
workflow may rank `status`, `history`, `undo`, `restart`, or diagnosis for the
real owning controller, but it must explain evidence quality, freshness,
permissions, GitOps, risk, blast radius, verification, and recovery. `PO6`
execution and managed port-forward/probe/debug sessions wait for D3 and provider
activation; `PO7` runbook packs wait for the ecosystem capability decision.
`PO8` may add bounded read-only cross-environment comparison. Deterministic rules
remain authoritative and no LLM is required.

### v0.6 and later — ecosystem, richer remote operations, and optional model features

The D7/CP6 package, provenance, revocation, capability and selected-input
model-suggestion source boundary is accepted and implemented without activation.
Local signed inspection, disabled installation, an optional no-WASI conformance
host, disabled typed action packs and exact selected-input consent exist. Public
extension distribution/SDK publication, component activation, provider APIs,
richer remote files/session memory/collaboration and the CP6 provider runtime
still require separately protected, independently proven slices.

LO0 separately documents an optional LLM Orchestration extension. LO1-LO5 are
not implemented. After explicit LO0 acceptance, a pure neutral workflow model
must precede any provider or plan runtime; core policy, review, one-run grants
and existing domain brokers remain authoritative. Local/self-hosted inference is
the first direction, no paid API is required, and the track does not block
Studio or video. Later data, automation, media and video domains still require
their own accepted boundaries and evidence. None is a current product claim or a
prerequisite for production SSH.

Automation Studio AS0 is likewise proposal-only. The target is an optional
embedded file editor separated from the DevOps/SRE domain extension and
language/tool add-ons, with core-owned documents, trust, native surface, LSP,
process, terminal, credential and audit brokers. AS1-AS6 implementation is not
done. See the [architecture](../AUTOMATION-STUDIO-ARCHITECTURE.md) and
[future evidence contract](../AUTOMATION-STUDIO-TESTING.md).

AS0 feasibility may continue before the first stable v0.4 release without
adding production scope. AS1-AS2 follow that release; the evidenced minimal
Studio should ship before a dedicated video-editing extension. Later Studio
phases and video research may proceed independently. Both products reuse
generic workspace and task services, while video owns its media-specific state
and does not depend on Studio's editor or language-server internals.


## Command-productivity sequence

- **CP0:** architecture/threat/compatibility baseline — complete at its source boundary.
- **CP1:** shell-native completion — complete locally; native release evidence remains.
- **CP2.0-CP2.2:** typed actions, private store, review/search/admin/insert-copy — complete locally.
- **CP3.0-CP3.3:** pure five-shell projection, opt-in aliases, reviewed packs, selected native import/trusted workspace bridges — complete locally with remaining release evidence.
- **CP4:** provider/capsule-aware actions — product-integrated and nonactivated; approved provider refresh/capsule production and exact execution remain gated.
- **CP5:** optional app-owned suggestions — CP5.1-CP5.4 are complete at their
  source/local boundaries, CP5.5 is complete as an inert helper/adapter source
  bridge, CP5.6 and live composition remain partial, preview is disabled, and
  native CP1 completion remains the fallback.
- **CP6:** signed ecosystem packs and selected-input model suggestions — accepted nonactivating source boundary complete locally; no tools/workflows/provider calls; activation, public distribution/SDK and native release proof remain blocked.

## LLM Orchestration sequence

- **LO0 — Partially done at documentation-only proposal boundary:** specification,
  proposed ADR, dependency direction, provisional limits and test ladder exist;
  acceptance and a strict machine contract remain.
- **LO1-LO5 — Not done:** no neutral workflow implementation, orchestrator,
  provider request, context-consent UI, reviewed execution, multi-extension
  workflow, storage, MCP mapping or release evidence exists.

## Semantic diagnostic navigation sequence

- **DN0 - Partially done at documentation-only proposal boundary:** specification,
  evidence ledger, proposed ADR, provisional ceilings, and test ladder exist;
  acceptance, machine limits, mutation owners, and failing tests remain.
- **DN1 - Not done:** exact separate failed-command actions using trusted prompt
  result metadata; navigate to the prompt/input anchor, not a fabricated region.
- **DN2 - Not done:** bounded on-demand logical-line batches, route generations,
  one continuation, optional content-free cache, reflow remap-or-clear, cleanup.
- **DN3 - Not done:** structured severity, conservative Error/Fatal headers, and
  bounded section reconstruction with false-positive/fuzz/performance evidence.
- **DN4 - Not done:** renderer-neutral highlight, accessibility, configuration
  recovery, native visual/resource/latency assurance, and experimental rollout.
- **DN5 - Not done:** independently evidenced specialized formats/user patterns.
- **DN6 - Not done/not authorized:** privacy-reviewed extension contributions.

## Situation-aware production operations sequence

| Phase | Outcome | Current state |
|---|---|---|
| PO0 | Decision, threats, owner map, limits, primary-source baseline, strict proposed machine contract/checker, tests, and nonactivation | Detailed proposal/checker only; partially done |
| PO1 | Production environment passport, pane lock, and context diff | Not done |
| PO2 | Bounded evidence, change/ownership/drift, resource explanation, healthy comparison, passive network diagnosis, SLO summaries and dependency/impact graph | Not done |
| PO3 | Situation-aware diagnostic completion and deterministic ranking | Not done |
| PO4 | Impact, permission, GitOps, JIT, policy, change-window, and approval preflight | Not done |
| PO5 | Incident Mode, hypotheses, bounded timeline, trusted/approximate time and log navigation, content-minimized journal and reviewed handoff/export | Not done |
| PO6 | Explicit execute-observe-stabilize-verify-recover lifecycle plus separately granted port-forward/probe/debug sessions through existing brokers | Not done; blocked on D3/provider activation |
| PO7 | Signed declarative organization packs and one-step cross-tool workflows | Not done; blocked on D7/capability decision |
| PO8 | Additional adapter slices, bounded read-only cross-environment comparison, optional small local tie-breaker, and release evidence | Not done |

### Production Operations experience and implementation checkpoints

| Phase | Simple operator experience | Implementation checkpoint before status may advance |
|---|---|---|
| PO0 | Review nonactive PO projections for the six reused surfaces and the read-only, reviewed-insertion, managed-action/session, stale, refused, cancel, failure, and recovery journeys. | Accepted/superseded ADR; frozen typed schemas, limits, wording, focus/input ownership, responsive projections, operation/session kinds, machine assurance contract, semantic checker/mutations, scenario corpus, dependency review, and nonactivation proof. |
| PO1 | Read environment, account, cluster/namespace, identity, expiry, GitOps owner, incident, and freshness from the existing line above the command; inspect an exact diff before adopting a changed context. | One immutable route-scoped passport, per-field provenance, production classification, lock/diff state machine, D2 cancellation/isolation, responsive/accessibility projection, logout/revoke/disable/uninstall cleanup, and native/resource evidence. |
| PO2 | Open one `Investigate` menu and choose only relevant Explain state, What changed, Compare with healthy, Diagnose connection, or Show user impact views; every result shows source, age, scope, coverage, evidence quality, unknowns and network vantage where relevant. | Bounded neutral knowledge/change/ownership/drift/explainer/cohort/network/SLO records; UID ownership/dependency graph; semantic normalization; cycle/cohort/fan-out limits; correlation-not-causation rules; separately reviewed quota/deadline/watch/poll/relist adapters; redaction; in-memory bounds; no active probe, raw log, time-series store or default evidence disk cache; provider/resource proof. |
| PO3 | Type a tool prefix, request completion, scan at most five two-line situation rows, inspect detail, and insert with Tab/Right Arrow and no Enter; ordinary CP5/CP1 stays available. | Immutable input generation, hard safety gates, stable lexicographic rank, explainable/refused candidates, CP5 projection/editor reuse, stable selection, stale cancellation, zero per-key provider work, scenario oracle, native editor bytes, latency, pixels, and accessibility. |
| PO4 | Review environment, exact target, reason, impact, authority, GitOps/policy/change window, verification, and recovery in one cancel-first surface; before PO6 only insert reviewed text. | Typed target/argv/UID and impact composer, live authorization/JIT/approval state, truthful plan/dry-run capability, revision binding/expiry, no execution/elevation/global mutation, boundary tables, provider/policy/GitOps oracles, and native review proof. |
| PO5 | Deliberately enter an Incident workspace with objective, locked environment, facts, hypotheses, contradictions, missing evidence, short timeline and one next action; navigate trusted or labelled approximate time anchors, optionally inspect source-separated pausable memory-only logs, and preview a redacted handoff. | Bounded incident/hypothesis/timeline state; coverage-qualified negative evidence; trusted/approximate anchors; separate bounded live-log controller with source-local order, backpressure and visible gaps; Error Navigation handoff; content-minimized receipts/journal; optional protected atomic persistence; migration/retention/corruption/disk-full/export/delete/disable/uninstall owners; zero-write session mode; privacy canaries, 24-hour resource and native collaboration proof. |
| PO6 | Choose `Execute reviewed action` only after activation, then follow one nonmodal prepare/run/observe/stabilize/verify/result monitor; cancel safely or review one recovery. Use the same explicit preflight/lifetime/stop flow for a Kubernetes port forward, controlled probe or safe debug session. | Independent D3/provider activation; final revalidation; one-use grant; exact broker request/argv; before-state; observation/stabilization/regression predicates; deadlines/children/listeners/temporary resources/cancellation/provider IDs; real verification; uncertainty/partial results; content-minimized receipt; separate recovery proposal; per-slice target/endpoint/vantage/image/profile/traffic/privilege/cleanup oracles; package/release gates. |
| PO7 | Review publisher, signature, rules, conflicts, and zero grants when installing a pack; a guided workflow stops after each reviewed step until the operator asks to continue. | Strict declarative schemas, D7 verification/store reuse, rejection of scripts/callbacks/credentials/capabilities/cycles/conflicts, signed malicious corpus, provenance/revoke/update/rollback, one-step boundary, policy, disable/uninstall, cross-tool and native package proof. |
| PO8 | Use the same words and surfaces across each supported provider; unsupported sources explain what is missing and preserve shell completion. Compare environments only after reviewing service equivalence, independent access/freshness, coverage and fan-out; optional local tie-breaking can be turned off instantly. | One adapter at a time behind independent capability/version/auth/quota/resource/rollback gates; read-only cross-environment comparison with no authority merging or multi-environment mutation; optional offline reorder-only model with signed provenance, hard CPU/memory/size limits, no raw logs/network/policy override/action creation, deterministic fallback, differential/adversarial proof, three-platform release/accessibility/soak/removal evidence. |

The detailed contract and honest limitations live in
[Situation-Aware Production Operations](../SITUATION-AWARE-PRODUCTION-OPERATIONS.md)
with [exact proposed PO0 contracts](../SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md),
the [UX and implementation blueprint](../SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md)
and [testing plan](../SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md).

## Ghostty compatibility track

Automexia keeps its own shortcut model as the implicit default. The explicit
pinned and moving Ghostty profiles never enable themselves and are not a v0.4
release criterion.

- **G0 — Partially done:** exact Linux/BSD source/binary/checksum provenance,
  fixtures, generator, Windows adaptation, ADR, and offline checks exist;
  native macOS fixture and Linux/macOS release evidence remain.
- **G1 — Fully done:** pure typed/compiled registry, indexed/reverse lookup,
  classic adapter, and registry-derived palette are implemented.
- **G2 — Fully done:** profiles, layers/unbinds, migration, diagnostics, and
  atomic last-known-good reload are implemented.
- **G3 — Fully done:** structured dispatch, fallthrough, sequences, tables,
  catch-all, chains, route isolation, and exact pending-byte behavior exist.
- **G4 — Fully done:** planned stateless compatibility actions and their bounded
  security/resource behavior are implemented.
- **G5 — Partially done:** host-independent verification, generated references,
  nightly fuzz/benchmark wiring, properties, repository mutation gates, and a
  strict private evidence validator exist; controlled native/release evidence
  and the activated 30-day baseline remain.
- **G6 — Partially done:** the modal redacted inspector has parked count/list,
  newest restore, two-step clear, and no-PTY key isolation; complete-top-level-
  tab history exists, while individual split/local-tab/native-window history and
  controlled native lifecycle evidence remain gated.

Compatibility work must not regress Automexia defaults or bypass the same
security, performance, accessibility, native, and release gates as other input
behavior.

## Release gates that cut across phases

A phase is not a stable release claim until its required native hosts, security depth, resource/performance baseline, accessibility evidence, packaging/signing/notarization, docs/assurance mapping, and rollback/recovery behavior pass. Windows-only or local-only proof cannot close a declared cross-platform gate.

## Status update rule

When status changes, update this phase matrix and the relevant user/developer canonical page in the same change. Do not create a second readiness document to restate the matrix. Detailed execution evidence belongs in CI/release artifacts and source history.
