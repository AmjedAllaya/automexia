# Phase implementation audit

Audit date: 2026-08-26

Audited source baseline: a8bcf6497a9a332cdda95dca88831e7b7411148b
(last committed baseline before this repository-protection change).

The versioned hosted-CI and repository-protection implementation recorded by
ADR 0031 is included in this audit. Unrelated uncommitted application, renderer,
and documentation changes were preserved as work in progress and are not
counted as shipped evidence.

DN0-DN6 was added on 2026-08-25 as a documentation-only design audit against the
then-current repository and working tree. Existing prompt/result/navigation
foundations are counted only as reusable evidence; uncommitted or proposed DN
behavior is not counted as implemented product capability.

LO0-LO5 was added on 2026-08-25 as a documentation-only architecture audit.
Existing extension, action, capability and runner foundations are reusable
evidence only; they are not counted as an implemented neutral workflow model,
LLM extension, provider path, plan review or execution capability.

PO0-PO8 was added on 2026-08-25 as a documentation-only production-operations
audit. Existing CP5, D2/D6, CP4/M13, D3, and DN foundations are reusable evidence
only; they are not counted as a production lock, change/provenance view,
resource-state explainer, cohort comparison, network diagnosis, SLO/log view,
dependency graph, situation-aware recommendation, preflight, Incident Mode,
managed diagnostic session, monitor, journal, runbook pack, model, provider
capability, or execution implementation.

Scope: every execution phase defined by the product, stabilization, DevOps/SSH,
Connection Hub, command-productivity, persistent-alias, Ghostty compatibility,
D7/CP6 ecosystem, Semantic Diagnostic Navigator DN0-DN6, and optional LLM
Orchestration LO0-LO5 and situation-aware Production Operations PO0-PO8
roadmaps.

## Purpose

This is the single evidence-oriented view of what Automexia has fully
implemented, partially implemented, or not implemented. It does not replace
the design roadmaps. It reconciles them against the current source tree, tests,
benchmarks, fuzz targets, CI policy, platform contracts, architecture rules,
documentation, and release prerequisites.

Authoritative design sources:

- [Product roadmap](ROADMAP.md)
- [Connectivity and command-productivity focus roadmap](CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md)
- [Detailed SSH, connectivity, multi-environment, and multi-cloud plan](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md)
- [Stabilization roadmap](STABILIZATION-ROADMAP.md)
- [Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md)
- [Automation Studio architecture](AUTOMATION-STUDIO-ARCHITECTURE.md)
- [Automation Studio testing and evidence contract](AUTOMATION-STUDIO-TESTING.md)
- [D7/CP6 ecosystem implementation audit](research/D7-CP6-IMPLEMENTATION-AUDIT.md)
- [Optional LLM Orchestration extension](LLM-ORCHESTRATION-EXTENSION.md)
- [Optional LLM Orchestration testing](LLM-ORCHESTRATION-TESTING.md)
- [Readiness audit](READINESS-AUDIT.md)
- [SSH, DevOps, and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md)
- [Connection Hub](CONNECTION-HUB.md)
- [Command Productivity](COMMAND-PRODUCTIVITY.md)
- [DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md)
- [Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md)
- [Semantic Diagnostic Navigator](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md)

## Status rules

| Status | Meaning |
|---|---|
| **Fully implemented** | Source behavior and automated evidence required by the stated boundary exist and pass. External release evidence is listed separately. |
| **Partially implemented** | Useful source or test infrastructure exists, but activation, required behavior, native-host evidence, controlled measurement, review, or an external prerequisite remains. |
| **Not implemented** | The phase is design-only, planned, or deliberately deferred; no production capability satisfies its exit gate. |
| **External gate** | Source cannot complete it: credentials, hosted policy, another OS, controlled hardware, elapsed time, rights, or private contact/configuration is required. |

The status-first register in [the main roadmap](ROADMAP.md#current-feature-status)
normalizes implementation to exactly **Fully done**, **Partially done**, or
**Not done**. Those labels map respectively to fully implemented, partial, and
not implemented in the executive matrix. Repository validation compares every
phase and normalized label in both tables; release evidence remains separate.

Implementation status and release-evidence status are separate. A disabled
package can be complete at its package boundary without being a shipped
feature. No Windows-only result proves Linux or macOS. No compile check counts
as an executed benchmark. No diagnostic retry converts a flaky failure into a
pass. No mock substitutes for required controlled native evidence.

## Audit method and current evidence

The audit cross-checked phase claims against workspace membership, source
ownership, architecture allowlists, feature assurance, workflows,
documentation coverage, tests, benchmarks, fuzz ownership, and release policy.

ADR 0020 now fixes the cross-phase technology boundary: core owns product
policy and the single ExternalToolRunner, first-party extensions own bounded
domain adapters, and mature protocol/authentication/custody systems stay
external. This planning decision does not advance a D, CP, or S implementation
status. In particular, the named AccessKit, nucleo, schema/generation, storage,
transfer, serial, policy, sandbox, provider-SDK, vet, and mutation additions
remain subject to their documented protected milestones and evidence.

| Check | Result on the audited commit |
|---|---|
| Feature assurance | Passed: 40 feature-assurance entries, 302 documented source/evidence entries, and 46 phase-audit entries. |
| Platform coverage policy | Passed: Windows/Linux/macOS, PowerShell/CMD/Unix shells, X11/Wayland, alternate architectures, nightly artifacts, deep Windows/WSL jobs, and release validators are machine-enforced. |
| Documentation coverage | Passed: 13 public pages, 175 configuration keys, 79 binding actions, 17 CLI flags, 5 CLI commands, and 25 xtask commands. |
| Repository validation | Passed: 52 TOML, 17 YAML, 54 JSON, 6 XML, one desktop file, 367 Markdown files, 83 pinned Actions, 2 repository-protection rulesets, 14 exact required checks, and release-trust/stable-source/assurance/roadmap contracts. |

The hosted GitHub state was authenticated again on 2026-08-26. Seven available
controls pass the versioned contract: repository merge/DCO/cleanup settings,
Actions and selected-publisher permissions, workflow-token authority,
Dependabot alerts and security updates, and immutable releases. Required hosted
workflows currently fail because controlled S1, S2, and F5 workflow files are on
the implementation branch but not remote `main`. Rulesets are unavailable on
the private Free plan, reviewer capacity is one human, and hosted jobs are
rejected before checkout by the account billing/spending state. Private
vulnerability reporting and Secret Protection are also unavailable in the
current private/entitlement state. The workflow failure and five external
conditions remain **unresolved for release** until reviewed integration reaches
`main`, the exact protected commit passes, and the authenticated audit is all-pass.

## Executive phase matrix

| Track | Phase | Implementation | Release evidence | Conclusion |
|---|---|---|---|---|
| Core | v0.4/S0 | **Fully implemented locally** | **Partial** | Identity, hostile-input bounds, atomic reload, and current terminal source gates are complete; stable release gates remain. |
| Assurance | v0.4/S1 | **Source fully implemented; release evidence partial/external** | **Partial/external** | A versioned 24-suite policy, strict commit-bound validator, deterministic test-only visual fixture, bounded comparator, Windows-native WGPU/CPU QA/resource evidence, separate AppVerifier Basics/low-resource phases, mutation coverage, controlled workflow, and fail-closed release dependency exist. Elevated/named-hardware, Linux/macOS native, assistive-technology, exact visual-matrix, and independent human-review evidence remains external. |
| Performance | S2 | **Source and automation fully implemented; collecting** | **External baseline pending** | Bounded evidence normalization/composition, sample/confidence quality, exact source/operator binding, native memory metrics, independent baseline/waiver review, protected activation validation, 90-day retention, and a fail-closed release ratchet exist. Activation awaits 30 reviewed consecutive controlled-runner days. |
| DevOps | D0 | **Partial** | **Partial** | ADR 0012 is accepted and active schema 5 ratchets immutable schemas 1/2/3/4 with exact M3-M5 route/tunnel, trust/status, lifecycle/receipt/reconnect, and native-manifest rules; protected approvals and real native execution remain. |
| DevOps | D1 | **Fully implemented** | **Partial** | Four private provider-neutral crates and bounded contracts satisfy their source boundary. |
| DevOps | D2 | **Fully implemented** | **Partial** | Generic status, immutable history, capsule/cache/session isolation, cancellation, and truthful freshness exist. |
| DevOps | D3 | **Partial; nonactivated** | **Blocked** | The fail-closed broker/runner owns bounded current-executable review, exact managed argv/identity binding, actual child status, redacted outcomes, process-group/Job Object teardown, PTY-worker joining, and bounded receipts through the guarded PTY/route owner. Protected approvals, real attestation/activation, and native OpenSSH descendant/resource/accessibility proof remain. |
| DevOps | D4 | **Fully implemented; read-only product adapter active** | **Partial** | Bounded OpenSSH inventory/persistence is connected only to D5.1 reviewed browsing; it retains no process/network/launch authority. |
| SSH UX | D5.0-D5.2 | **Partial overall; D5.1, nonactivated F5.1-F5.3, and the F5.4 local assurance path complete** | **Partial/blocked** | Routes/trust/tunnels/lifecycle plus exact native host/commit/OpenSSH/artifact binding, protected manual workflow, and path-free summaries pass locally. Protected activation/attestation, actual status/SSH execution, and controlled real native descendant/listener/resource/accessibility manifests remain. |
| Multi-environment | M6/F6 | **Partial overall; review/edit source and product surfaces fully implemented locally** | **Blocked on D3/M5 execution evidence** | Accepted ADR 0023; schema-2 editor/migration/recovery; public preview-first CLI; immutable worker publication; Connection Hub catalog/restore review; exact fingerprints; typed recipes/no-hooks; narrow remote initialization; armed broadcast; semantic projections; fuzz/mutation/integration and bounded benchmarks pass. Managed execution and controlled native/resource/accessibility/release evidence remain. |
| Multi-cloud | D6.0/M7 | **Fully implemented locally** | **Partial/external** | Strict provider-neutral schemas, immutable session capsules, 19-state lifecycle, exact allow-once review, isolation/redaction, passive-status migration, cached-only semantics, fuzz/mutation, 16×64 lifecycle, and a 64-capsule benchmark pass. No real provider CLI, cloud network, credential cache, browser/device flow, or Linux/macOS native provider evidence ran. |
| Multi-cloud | D6.1/M8 | **Partial overall; source and cached product review fully implemented locally, execution nonactivated** | **Blocked** | AWS adapter, strict capsule replacement, cached six-provider Hub review, and M11 private EKS ingestion pass locally. D3 activation/attestation and controlled real AWS/native/resource/accessibility/release evidence remain. |
| Multi-cloud | D6.2/M9 | **Partial overall; source and cached product review fully implemented locally, execution nonactivated** | **Blocked** | Azure adapter, cached Hub review, and M11 private AKS allocation/validation/revoke/cleanup pass locally. D3 activation/attestation and controlled real Azure/native/resource/accessibility/release evidence remain. |
| Multi-cloud | D6.3/M10 | **Partial overall; source and cached product review fully implemented locally, execution nonactivated** | **Blocked** | Google Cloud adapter, cached Hub review, and M11 private GKE allocation/validation/revoke/cleanup pass locally. D3 activation/attestation and controlled real Google/native/resource/accessibility/release evidence remain. |
| Multi-cloud | D6.4/M11 | **Partial overall; source, private transient lifecycle, and cached product review fully implemented locally, execution nonactivated** | **Blocked** | Kubernetes/OpenShift contracts plus app-owned 16-file/1 MiB private validate/revalidate/revoke/cleanup and responsive Hub review pass locally. D3 activation, real clients/clusters/plugins, Unix no-follow, controlled cleanup/resources/accessibility/release evidence remain. |
| Multi-cloud | D6.5/M12 | **Partial overall; Teleport source and cached product review fully implemented locally, execution nonactivated; OpenBao not done** | **Partial/blocked** | Teleport has bounded exact source plus cached Hub review; D3 activation and controlled real native/provider evidence remain. OpenBao is rejected and absent pending ADR 0024 acceptance. |
| Ecosystem | D7 | **Fully implemented locally at accepted source boundary; partially implemented overall** | **Release activation and external evidence open** | Accepted ADR 0029/digest; pure domain, signed local verification, protected atomic disabled store, custom-WIT/no-WASI Wasmtime conformance, exact grants/lifecycle, action-pack mapping, selected-input model policy, renderer-neutral review, app denial adapter, property/fuzz/mutation/benchmark evidence. Public SDK/download/distribution/provider calls/component activation/native release evidence remain false or external. |
| LLM orchestration | LO0 | **Partially implemented at documentation-only proposal boundary** | **Blocked on ADR acceptance and a versioned machine contract** | The canonical specification, proposed ADR 0033, neutral-workflow dependency direction, provider/context/approval/resource/lifecycle boundaries, provisional limits, LO0-LO5 sequence and future evidence ladder exist. No workflow-model crate, provider/model adapter, registry, executor, UI, MCP mapping, model download or runtime authority exists. |
| LLM orchestration | LO1-LO5 | **Not implemented** | **Blocked on LO0; later slices require independent protected and native gates** | No neutral action/workflow implementation, orchestrator extension, model request, context-consent UI, reviewed one-run execution, multi-extension workflow, provider delivery, storage or native/release evidence exists. |
| Automation Studio | AS0 | **Partially implemented at proposal-only boundary** | **Blocked on acceptance and native feasibility** | The architecture, proposed ADR 0030, build/wrap/adopt choices, AS0-AS6 sequence, and future evidence ledger exist. No dependency or runtime was added; numeric machine limits, exact dependency review, and native Windows/macOS/Linux X11/Wayland editor-host proof remain. |
| Automation Studio | AS1-AS6 | **Not implemented** | **Blocked on AS0 acceptance** | No document service, workspace-trust implementation, editor/webview host, LSP/DAP broker, language server, Studio/DevOps package integration, file-write grant, typed script-run product path, remote/mobile client, or advanced runtime exists. |
| Diagnostic navigation | DN0 | **Partially implemented at documentation-only proposal boundary** | **Blocked on ADR acceptance and implementation evidence** | The canonical specification, evidence ledger, proposed ADR 0032, owner split, provisional ceilings, privacy/accessibility/lifecycle contract, test ladder, and DN0-DN6 sequence exist. No runtime dependency or action is authorized; accepted ADR, frozen machine contract, mutation owners, and failing tests remain. |
| Diagnostic navigation | DN1-DN6 | **Not implemented** | **Blocked on DN0; DN6 also requires a separate extension capability decision** | No failed-command action, scan scheduler, normalized-line API, detector, section locator, cache, highlight, setting, user pattern, extension contribution, or native/resource/accessibility/release evidence exists. |
| Production operations | PO0 | **Partially implemented at detailed proposal/checker boundary** | **Blocked on runtime schemas plus ADR/exact-digest and owner/dependency/capability acceptance** | The canonical specification, exact proposed PO0 contract/digest with 17 record payloads and 37 phase/effect/authority-bound actions, semantic checker/mutations/source nonactivation scan, UX/build blueprint, 2026 source baseline, provider/rule/lifecycle/settings/traceability details and future evidence ledger exist. Runtime schemas/parsers, prototypes, first real-path failures, runtime capability, dependency, setting, watcher, investigation view, managed session, provider authority and execution remain absent. |
| Production operations | PO1-PO8 | **Not implemented** | **Blocked on PO0 and phase-specific CP5/D6/D3/D7 gates** | No production passport/lock; change/ownership/drift, resource/scheduling, cohort/environment, network/SLO, live-log/time/handoff or dependency view; situation-aware candidate or Kubernetes rollout prioritization; preflight; GitOps/JIT/policy composition; Incident Mode; managed operation/diagnostic session; journal; runbook pack; provider adapter; local ranker; execution path; or native/release evidence exists. |
| Productivity | CP0 | **Fully implemented** | **Partial** | Accepted architecture, threat model, ceilings, fixtures, mutations, and nonactivation policy exist. |
| Productivity | CP1 | **Fully implemented** | **Partial** | Shell-native completion and bounded explicit refresh exist; hosted native evidence remains. |
| Productivity | CP2.0 | **Fully implemented** | **Partial** | Bounded typed Quick Action model and hostile corpus exist with no runtime authority. |
| Productivity | CP2.1 | **Fully implemented as internal library** | **Partial** | Private atomic persistence, CAS, recovery, watches, and benchmarks exist; no startup/UI activation. |
| Productivity | CP2.2 | **Implemented locally** | **Partial** | Layered search, placeholder/risk/conflict review, bounded import/export/CRUD/recovery, and explicit insert/copy UI are present. Hosted native shells, controlled screen readers, and 30-day performance/resource evidence remain release gates. |
| Productivity | CP3.0 | **Fully implemented at pure boundary** | **Partial** | Five pure serializers, bounded inventories, metadata/tamper verification, tests, fuzz, benchmark, and policy ratchets are complete; activation is disabled and hosted native evidence remains. |
| Productivity | CP3.1 | **Fully implemented locally** | **Partial** | Explicit opt-in persistence, crash-safe all-old/all-new publication, verified five-shell startup/reload, diagnostics, rollback, and exact uninstall are implemented; hosted native/macOS/WSL and controlled-baseline evidence remains. |
| Productivity | CP3.2 | **Fully implemented locally** | **Partial** | Eleven static provider packs, 33 disabled-by-default actions, health/update/alias-safety, CLI, tests, fuzz, benchmarks, and policy gates are complete; hosted evidence remains. |
| Productivity | CP3.3 | **Fully implemented locally** | **Partial** | Six explicit native inventory formats, exact just/Task/mise bridges, bounded dry-run/CAS import/workspace/trust/revocation/removal, path-free digest/revision receipts, background cache authorization, final insertion recheck, tests, mutation, fuzz, benchmark, ADR, CLI, and docs are complete; hosted native/accessibility and 30-day evidence remain. |
| Productivity | CP4 | **Partially implemented overall; fully implemented locally at product-integrated nonactivating boundary** | **Partial/external** | Seven provider projections, retained cached-product handoff, exact route/session/revision publication, idempotence/revocation, cached search, final revalidation, accessible risk/state UX, production confirmation, fuzz/benchmark/mutation/policy evidence are complete locally. Approved provider refresh/capsule production, exact execution, OpenBao, and real native/provider/accessibility/release evidence remain. |
| Productivity | CP5.0 | **Fully implemented at research boundary** | **Complete locally** | Seven-shell API matrix, pure insertion prototype, locked matcher/dependency evidence, privacy review, and retain-CP1/defer-P2 decision are machine-gated; no runtime surface exists. |
| Productivity | CP5.1-CP5.6 | **Partially implemented overall; CP5.1-CP5.4 fully implemented at source/local boundaries; CP5.5 inert bridge source complete; CP5.6 partial** | **Partial/external; preview disabled** | Accepted ADR 0025; authenticated request/reply protocol; restrictive endpoints; bounded broker/sources/ranking; pane UI/publication/route exchange; helper binary target; four bidirectional adapters; real local Windows PowerShell and WSL Bash/Zsh/Fish evidence. Reviewed launcher, signing/attestation, WSL relay, live activation, successful three-OS/accessibility/package/rollback/resource/30-day evidence remain. CP1 remains fallback. |
| Productivity | CP6 | **Fully implemented locally at accepted no-provider source boundary; partially implemented overall** | **Provider/native/release evidence open** | Signed capability-free packs map into existing disabled typed actions; exact selected-input redaction/disclosure/consent/typed response/risk/review states exist with provider/tool/workflow/MCP/Enter/execution hard-disabled. Public product UI, provider transport, privacy/legal and three-platform native evidence remain. |
| Compatibility | G0 | **Partially implemented** | **Partially done** | Exact Ghostty 1.3.1 Linux/BSD provenance/fixtures/checksums, deterministic Windows adaptation, ADR 0026, generation, verification, classic golden, and properties exist. Native macOS fixture and Linux/macOS release evidence remain. |
| Compatibility | G1 | **Fully implemented locally** | **Partially done** | Pure typed registry, schemas, layers, direct/reverse indexed lookup, reserved sequence/table structures, classic bridge, palette derivation, and Windows latency evidence exist; external platform evidence remains. |
| Compatibility | G2 | **Fully implemented locally** | **Partially done** | Explicit profiles, bind/unbind/priority layers, strict diagnostics, immutable atomic reload, global/palette transaction, and dry-run/confirmed migration are implemented; native release matrices remain. |
| Compatibility | G3 | **Fully implemented locally** | **Partially done** | Structured outcomes, fallthrough, exact sequences/cancellation, bounded tables/catch-all/chains, stable all-surface execution, per-route state, and shell ownership are implemented; native IME/layout evidence remains. |
| Compatibility | G4 | **Fully implemented locally** | **Partially done** | Clear, extended selection, branded pane-footer search, bottom-centered visible-pane search, topology, zoom/equalize/resize, and bounded private export families are implemented; controlled cross-platform visual/accessibility/resource checks remain. |
| Compatibility | G5 | **Partially implemented** | **Partially done** | CLI/migration, xtask generation/verification/tests, generated references, property/fuzz targets, and Windows Criterion benchmarks exist. Native three-platform, visual, accessibility, resource, and 30-day evidence remains. |
| Compatibility | G6 | **Partially implemented** | **Partially done** | ADRs 0027/0028, the redacted inspector, and bounded parked-PTY undo/redo for complete top-level tabs exist. Individual split/local-tab/native-window history and native lifecycle evidence remain. |

### Architecture Phase 0-5 mapping

The architecture document uses broader Phase 0-5 names. They map to the
executable ledgers above as follows:

| Architecture phase | Executable phase mapping | Status |
|---|---|---|
| Phase 0: v0.4 security/stability | S0, S1, S2 | **Source gates complete; assurance partial; S2 enforcement implemented but baseline activation pending.** |
| Phase 1: provider-neutral APIs | D1, D2 | **Fully implemented at source boundary.** |
| Phase 2: production first-party SSH | D0, D3, D4, D5.0-D5.2 | **Partial foundation only; production SSH not implemented.** |
| Phase 3: provider auth/capsules | D6.0-D6.4 | **Partially implemented overall: M7 is fully implemented locally; M8-M11 source, cached product review, and private transient lifecycle are fully implemented locally while execution remains nonactivated.** |
| Phase 4: lazy inventory/infrastructure/enterprise adapters | Later D6/D6.5 work | **Partially implemented: M12 Teleport source and cached review are complete locally/nonactivated; OpenBao is not implemented pending ADR 0024.** |
| Phase 5: third-party ecosystem/selected-input suggestions | D7 and CP6 | **Fully implemented locally at the accepted nonactivating source boundary; partially implemented overall because public distribution, component/provider activation, native product UX, and release assurance remain gated.** |

LO0-LO5 is a separate cross-extension product track, not a retrospective Phase 6
of this older architecture mapping. LO0 is documentation-only and LO1-LO5 are
not implemented.

## Core stabilization phases

### S0.1 — product and protocol identity

**Fully implemented locally.**

- XTGETTCAP, executable/package IDs, URL/desktop metadata, terminfo, native
  dialogs, environment variables, config paths, and user-facing surfaces use
  Automexia identity.
- Rio names are constrained to attributed provenance and private inherited
  engine identifiers.
- Identity, provenance, metadata, migration, and coexistence regressions run in
  contributor validation.

Remaining external proof: native clean install/package results and public
repository administration. These do not reopen source implementation.

### S0.2 — bounded hostile terminal control strings

**Fully implemented locally; extended hosted evidence remains.**

- OSC retention is capped at 1 MiB, APC/graphics at 96 KiB, XTGETTCAP at 4 KiB,
  synchronized updates at 2 MiB, and Sixel uses bounded streaming.
- Synchronized-update storage is lazy rather than a 2 MiB reservation per
  pane. Completed large OSC/APC/synchronized-update allocations release their
  high-water capacity, while small common buffers remain reusable under
  explicit 64 KiB/8 KiB/64 KiB retention ceilings.
- Overflow discards to the correct terminator; CAN/SUB cancels without
  dispatch; diagnostics omit hostile payloads and are rate-limited.
- Tests cover exact limits, limit-plus-one, fragmentation, unterminated state,
  cancellation, recovery, repeated attack, lazy allocation, ordinary-buffer
  reuse, saturating size arithmetic, and retained-memory release. A dedicated
  parser-construction Criterion case protects the allocation fast path.
- Ten assurance-owned fuzz targets cover VT, OSC, control strings, images,
  migration, sanitization, semantic classification, and OpenSSH inventory.

Remaining: retained hosted fuzz/sanitizer/Miri, longer persisted campaigns, and
hostile remote-PTY evidence across release OSes.

### S0.3 — atomic last-known-good reload

**Fully implemented locally.**

- Config/theme/font/hotkey candidates validate completely before mutation.
- Invalid candidates retain the previous live generation.
- Hotkey additions precede removals; compensating rollback runs in reverse and
  any OS rollback failure is explicit.
- Reload is event-loop serialized and never recreates PTYs.
- Tests cover every load failure, duplicate/invalid hotkeys, partial add/remove,
  rollback, repeated reload, and recovery.

This is the required retry pattern: preserve last-known-good, report
degradation, and never silently retry into a false pass.

### S1.1 — native prompt, resize, session, and input assurance

**Source fully implemented; multi-platform native evidence partial/external.**

Implemented:

- deterministic 2,000-transition storms, resize deduplication/coalescing,
  input/shutdown barriers, final-resize repair, immutable prompt generations,
  Unicode path reflow, stale-cell cleanup, route isolation, session cloning,
  pane-local tabs, and terminal-owned keyboard selection;
- native Windows WGPU/CPU GUI and ConPTY storms, process/resource/frame
  sampling, PowerShell history budgets, WSL clone coverage, close isolation,
  and extreme-size recovery;
- Windows/Unix PTY lifecycle tests and PTY startup/output Criterion coverage;
- responsive footer/modal/tab/overlay/DPI geometry and the content-aware
  1.22-row context rhythm.

Missing or external:

- controlled Linux X11/Wayland and macOS Intel/Apple Silicon GUI storms;
- native repaint, minimize/restore, scale/HiDPI, and GPU artifacts on them;
- claims beyond representative tested distributions.

### S1.2 — resource lifetime and hardware evidence

**Source fully implemented; controlled hardware evidence partial/external.**

Implemented:

- Windows ceilings for handles, threads, private bytes, working set,
  descendants, PTYs, routes, image resources, and teardown;
- safe exact-binary Application Verifier/WPR wrappers with finally cleanup,
  separate Basics and bounded low-resource phases, verifier error/stop and log
  size rejection, redacted reports, and private ETL exclusion;
- worker restart/saturation/shutdown, PTY lifecycle, image WGPU/CPU
  open/dismiss, cache eviction, file release, and no-sidecar tests;
- separate ASan, TSan, Miri, Loom, fuzz, and native responsibilities.

Missing:

- elevated AppVerifier Basics/Heaps/Handles/Locks and reviewed WPR evidence;
- named Intel/AMD/NVIDIA, RDP/software, Linux, and macOS hardware runs;
- long process/GPU/resource soak evidence.

### S1.3 — visual quality and frame regression

**Source fully implemented; approved native visual evidence partial/external.**

Implemented: renderer-neutral geometry/state, structured snapshots, contrast,
hit-target/modal/cursor/footer/path/pane invariants, narrow-to-8K layout cases,
and a topmost Windows client capture that waits for presentation, rejects
blank/single-color output, restores z-order, and excludes live frames from
portable QA. The repository-owned comparator now bounds encoded/decoded size,
dimensions, output, and memory; rejects one changed channel in one pixel with
no default masks; and atomically writes an exact JSON verdict plus heatmap.
The exact test-only `s1-standard-v1` fixture freezes public content, clock, and
motion, while the versioned policy enumerates 1,600 cases per visual suite
and requires independent HTTPS human review.

Missing or external: approved expected/actual/diff golden matrices across
viewport/theme/font/scale/UI states, Linux/macOS native frames, and recorded
human aesthetic review. The comparator is source-complete tooling; it is not a
claim that unrecorded frames were reviewed.

### S1.4 — accessibility

**v0.4 source policy fully implemented; assistive-technology evidence external.**

Implemented: keyboard operation, focus visibility/order, non-color identity,
contrast, hidden targets, scaling, labels, reduced-motion requirements,
inventory/manual matrix, generic status summaries, exact Narrator/NVDA/
VoiceOver/Orca environment coverage, strict evidence validation, independent
HTTPS review, and ADR 0013.

External: recorded Narrator/NVDA, VoiceOver, and Orca evidence. The v0.5
AccessKit semantic tree remains a deliberately separate maturation phase.

### S1.5 — context freshness and provider recovery

**Partially implemented.**

- Provider snapshots carry source revision, observation time, availability,
  last error, and truthful current/refreshing/stale/disabled/unavailable/error
  state.
- Prompt, cwd, profile, session, and capsule changes request refresh through
  bounded session-scoped workers; external tools never run on render/input/VT/
  PTY threads.
- Bounded caches, exact operation/session/capsule identity, newest-generation
  publication, cancellation, periodic reconciliation, and last-known-good
  behavior prevent stale or failed discovery from erasing known production
  context.
- Deterministic provider, route-isolation, cache, saturation, failure, and
  accessibility-label tests cover the current local provider boundary.

Remaining: controlled provider/CLI cold/warm/slow/missing/disconnected latency
and resource evidence on Windows/Linux/macOS. Reliable file/event watchers may
be added only where they improve latency without becoming the correctness
source; bounded periodic reconciliation remains mandatory.

### S1.6 — deterministic orchestration and QA evidence

**Source tooling fully implemented; multi-host release evidence partial.**

- Pinned Nextest 0.9.137 profiles own timeouts, leaks, serialized native groups,
  JUnit, and explicit flaky failure; Cargo doctests remain separate.
- cargo xtask qa --full --bundle owns deadlines, process-tree cleanup, bounded
  logs/files/bundles, atomic summaries, redaction, and explicit skips.
- Private ETL, raw LCOV, live terminal frames, secrets, and private paths are
  excluded from portable evidence.
- cargo ready is the contributor gate; cargo automexia is the fast launch path.
- Diagnostic retries must still fail as flaky; silent retry success is banned.

Remaining: retain complete bundles from every controlled release host.

### S1.7 — property, model, fuzz, coverage, and mutation strength

**Partially implemented.**

Implemented: shrinking layout properties and persisted regressions, a 512-case
independent resize-queue model, generation-aware bounded atomic snapshot
publication with stale/failure/target-limit tests, fixed-seed storms, finite
Loom models, nightly sanitizers/Miri/fuzz ownership, global plus
80%-changed-owned-line coverage policy, and mutation tests for platform,
feature, productivity, release, documentation, and CI drift.

Missing or external: long weekly fuzz/corpus trends, an owned region/branch
baseline, scoped cargo-mutants survivor triage, and governed cargo-vet adoption.

### S1.8 and S2 — performance proof and enforcement

**Source and automation fully implemented; controlled baseline collecting.**

- Nine controlled Criterion targets cover application/DevOps services, image
  preview, Quick Action parsing/store, connection planning, PTY I/O, event
  polling, OpenSSH inventory, and VT. Each run uses a unique evidence target.
- Strict bounded no-follow JSON normalizes classified Criterion estimates and
  the existing native Windows private-bytes/working-set report. Criterion
  latency requires 50-10,000 finite repeated samples, exact 95% confidence, and
  a confidence interval no wider than 10% of the estimate.
- Composition requires a clean exact source commit plus exact time, runner,
  toolchain, profile, and operator identity. It rejects stale/future, duplicate,
  unknown, oversized, non-finite, linked, path-bearing, or mismatched evidence.
- A reviewed-policy digest freezes 5% latency, 10% memory, 30-90 consecutive
  days, required claims, privacy limits, exact expiring waivers, and same-runner
  comparability. Each accepted day is digest/operator traceable; the builder
  cannot activate incomplete history or let a collector accept the baseline.
- Nightly retains normalized evidence for 90 days. Tagged release preflight now
  requires fresh exact-commit evidence and an active baseline, and fails closed
  on an unwaived regression. The registered `S2 controlled activation` workflow
  uses the protected `stable-release` environment and emits a bounded digest
  summary before activation can merge.

External gate: the baseline fixture remains `collecting`. Thirty complete
consecutive runs on the named Windows GPU/benchmark runner, controlled hardware
metadata, independent review/HTTPS acceptance, and successful protected
activation are still required. The 2026-08-24 remote audit found no configured
runner variables and zero accepted S2 days. No local or Linux-only run is
reported as that missing evidence. See the
[S2 completion audit](research/S2-RELEASE-RATCHET-COMPLETION-AUDIT.md).

### v0.5 assurance maturation

**Partially implemented.**

Implemented foundations:

- renderer-independent extension API/runtime, DevOps, UI, cache, queue, and
  lifecycle state are extracted into private crates suitable for deterministic
  tests, bounded Loom models, and hosted Miri without platform/GPU FFI;
- the feature-assurance ledger, architecture rules, CI policy mutations,
  sanitizer/fuzz ownership, coverage policy, and redacted QA evidence establish
  the required proof vocabulary.

Not implemented:

- the AccessKit-backed renderer-independent accessibility tree and native
  Narrator/NVDA/VoiceOver/Orca validation;
- a pinned scoped cargo-mutants campaign, time budget, survivor triage, and
  reviewed threshold for Automexia-owned pure modules;
- cargo-vet ownership, imported-audit trust, criteria, exemptions, renewal, and
  pull-request workflow;
- an informational unused-dependency job with platform/feature false-positive
  review;
- post-baseline enforced performance/resource ratchets.

These tools must complement, not replace, Cargo deny, dependency review,
CodeQL, SBOMs, attestations, deterministic tests, fuzzing, sanitizers, Miri,
Loom, native resource tests, and human accessibility/visual review.

## DevOps, SSH, and multi-cloud phases

### D0 — decision, threats, and compatibility baseline

**Partially implemented.**

**Fully implemented locally:** active schema 5 preserves immutable schemas 1-4
and freezes manual PowerShell/CMD/Bash/Zsh/WSL SSH, missing-client behavior,
exact trusted-loader/package identity, M3 direct/M4 routed/M5 tunnel grammar,
grant/audit/trust/lifecycle rules, all-false authority, and fixed
Windows/macOS/Linux/disabled-WSL resolution.

Twenty-three required scenario rows add restricted-key agent session binding,
post-quantum negotiation, weak-crypto warning preservation, and tunnel bind
collision to the prior route/trust/forward/cancellation/cleanup/1/10/50 matrix.
The hermetic protocol and validator require current source plus application and
package hashes, exact private loopback fixture/config/seed hashes, bounded
latency/CPU/memory/handle/process/PTY/listener/tunnel/task/route/cache/log/storage
peaks, zero cleanup deltas, manual-client/disable/uninstall preservation, and
redaction canaries. Synthetic evidence cannot satisfy release.

**Partially done externally:** ADR 0012 is accepted by the project owner.
ADR 0003's two independent exact-head approvals/server enforcement and the F4/F5
native fixture matrix remain. D0 defines the contract and keeps production
launch disabled.

### D1 — private contracts and bounded stable types

**Fully implemented at source boundary.**

- automexia-extension-api, automexia-extension-runtime, automexia-devops, and
  automexia-ui-model are private and separated from frontend/GPU/PTY/provider
  implementation.
- Versioned types reject unknown/oversized/invalid/session-mismatched data;
  launch debug is redacted and environment values are absent.
- Exact-route wake, bounded cache/queues, registration ordering, cancellation,
  coalescing, last-known-good, shutdown, goldens, Loom, Miri ownership, and
  architecture rules exist.

Deferred without invalidating D1: automexia-app extraction, inherited engine
regrouping, and removal of v0.4 Rio fallbacks at the v0.5 transition.

### D2 — generic status and Environment Capsules

**Fully implemented at provider-neutral source boundary.**

- Generic ContextContribution to StatusSegment replaces renderer providers.
- Core owns ordering, compaction, grapheme safety, color/contrast,
  accessibility, hit testing, and details routing.
- Historical prompt context is immutable; each session/clone gets independent
  capsule/session/PTY identity.
- Rebind validates revisions, cancels stale work, invalidates exact cache keys,
  and requires a new session for environment-changing transitions.
- Late results are rejected; failures preserve last truthful freshness/error.

Production relaunch/login is a D3/D5/D6 dependency, not missing D2 work.

A capacity-one application worker now owns current executable observation away
from input, PTY, renderer, and startup hot paths. It publishes a
generation-bound result before waking the route; the controller installs only
an exact current preparation within 30 seconds and invalidates it on refresh or
exit. The first approval requests this observation; a second explicit approval
binds it, and spawn revalidates the guarded file identity.

### D3 — exact-argv first-party session launch

**Partially implemented and intentionally nonactivated.**

**Fully implemented locally; nonactivated:** the production-compiled broker and
one Router-owned ExternalToolRunner bind package policy, exact executable and
destination, operation/session/capsule scope, 60-second decision, monotonic
lease, bounded public environment, one captured trusted cwd, redacted audit, and
fixed platform resolution without PATH/cwd search. The broker opens and
re-compares an exact executable guard; ContextManager alone consumes it, creates
one PTY, publishes one route matching the reviewed session, and then records
publication. Completion, cancellation, revocation, stale leases, failed
publication, route close, capacity 50, audit FIFO 256, and application shutdown
are locally tested. The Connection Review exposes allow once, allow for session,
and deny through pointer, A/Enter, S, D, focus, and accessibility semantics.

**Partially or not done for activation:** ADR 0012 is accepted by the project
owner, but ADR 0003's two independent exact-head approvals and server
enforcement are external. Real package-loader/build attestation and revocation,
native proof of graceful-then-forced descendant cleanup, hostile real OpenSSH/
PTY/server fixtures, controlled screen-reader/pixels, and 1/10/50-session
resource evidence remain. The cleanup source itself is locally complete:
Windows terminates the owned Job Object, Unix retains the waitable leader while
signalling the owned process group, and PTY workers join with a deadline. The
linked package remains unverified and the activation constant remains false,
so no production child can start.

The fail-closed production state is correct and must not be called shipped.

### D4 — OpenSSH inventory and persistence

**Fully implemented as disabled, nonactivated package.**

- Only exact filesystem-read authority; architecture forbids process/network/
  launch/clipboard/environment/secret/UI/renderer/PTY authority.
- Limits: 1 MiB/file, 8 MiB total, 128 files, depth 8, 10,000 aliases,
  4 KiB/value, and 16 KiB/line.
- Lexical in-grant includes; cycle/link/reparse/permission/ownership/dynamic/
  executable syntax fails safely; no process, DNS, socket, or ssh -G path.
- Public-only schema 1 metadata uses stable no-follow reads, bounded durable
  replacement, Unix 0700/0600, and current-user Windows DACL.
- Exact nonrecursive watches are bounded, coalesced, cancellable, periodically
  reconciled, late-generation safe, and last-known-good.
- Windows/Unix tests, hostile/property cases, fuzz, 10,000-alias benchmark,
  architecture ratchets, and assurance ownership exist.

Remaining release proof: hosted native macOS and controlled longitudinal
benchmark evidence. D4 does not activate D3 or D5.

### D5.0 — Connection Hub contract and UX baseline

**Partially implemented overall; every local non-executing F2 deliverable is
fully implemented.**

Implemented evidence:

- `automexia-devops::connections` owns strict schema-1 definition, observation,
  intent, review, receipt, profile, recipe, step, tunnel, document, state, and
  resolved-plan records with fixed byte/item/depth/retry/time ceilings;
- validation rejects future/unknown schemas, unknown fields, controls/bidi,
  option-like targets, duplicates, missing dependencies, cycles, oversized
  input, secret-bearing variable/environment names, free-form commands, and
  inconsistent stage/risk/confirmation/failure/retry/reconnect policy;
- canonical fingerprints cover profile, source, target, identity, transport/
  route, tunnels, recipes, executable identities, requested capabilities, and
  ordered plan steps;
- exhaustive authentication/result transition tests cover every public state,
  illegal transitions, terminality, cancellation, stale/expiry, canonical IDs,
  active-operation correlation, rejection of late generations, and denial of
  background or denied-state authentication;
- `automexia-ui-model::connection_hub` owns pure wide/medium/narrow Hub,
  Connection Review, and recipe-planner projections with modal/inert behavior,
  managed grid focus including stale-selection fallback, focus restoration,
  route-aware modal keyboard cycles, reading order, live loading progress,
  value-redacted human action labels, high-contrast/reduced-motion preferences,
  100-400% scaling, every content/auth state, disabled primary actions, and no
  PTY resize;
- frozen ten-provider/all-auth/layout/accessibility fixtures, 30 required
  hostile/property/record/state/model/UX regressions, validation-bypass and
  planner-panic mutation checks, a fuzz target, and the 64-step Criterion
  benchmark are registered in CI/assurance; and
- architecture checks forbid filesystem, process, network, provider,
  credential, PTY, listener, renderer, GPU, or unsafe authority in this slice.

Not implemented externally: ADR 0012 is accepted by the project owner, while
ADR 0003 protected approvals/server enforcement, package attestation, and native
activation evidence remain. Those gates keep D5.0 **Partially done** without
invalidating the complete local F2 exit. D5.1 is fully implemented locally as a
capability-free read-only product; native macOS/Linux and controlled
accessibility evidence remains partial. D5.2's fail-closed runner, exact PTY,
and route-publication path is now local; production activation and later
connection lifecycle features remain open.

### D5.1 — read-only Connection Hub

**Fully implemented locally; external release evidence partially done.**

The Router owns one cloneable `ConnectionHubRuntime` for the application
lifetime. It opens D4 metadata and the Connection Library below the application
configuration root and owns one cancellable/joined worker with a capacity-two,
non-blocking inbox. Saturation fails with `connection-worker-busy`; obsolete
queued generations are skipped, scans are cancelled, immutable state publishes
before route wake, last-known-good data survives failure, and shutdown clears
memory-only grants and joins deterministically. Opening the Hub reads only
already-opened private state and performs no scan, process, network,
authentication, provider, listener, PTY, or credential work.

The command palette exposes a distinct read-only Connection Hub action. The
typed `OpenConnectionHub` registry action and `Ctrl+Shift+H` / `Cmd+Shift+H`
defaults are fully implemented locally, collision-tested across constructed
Windows, Linux/BSD, and macOS tables, and suppressed under Search, Vi, and
alternate-screen ownership. The real Sugarloaf modal supports an explicit
parented native multi-file picker,
canonical-path review, confirm/cancel, and a compact first-run setup with one
primary action. Search, tag/favorite/recent/source filters, grouping, and
virtualized navigation appear only with a usable catalog; contextual Clear
appears only with an active filter. Hidden controls are removed from visual and
accessibility order and reject pointer, keyboard, shortcut, and IME input.
Code-native vector icons, restrained semantic colors, and text labels provide
redundant meaning. The modal also preserves selection/inspector, focus restore,
responsive tiny-to-8K geometry, and truthful loading/error/recovery states.
Exact-file review tokens are visible and revocable only by the initiating
controller. Catalog projection and summary clones are cached across unchanged
frames, while IME preedit uses the query-only hostile-input validator. Connect,
Login, provider refresh, and recipe execution are visibly disabled. Profile/
recipe/preference data is a non-executing local snapshot.

Favorites and tags use only the D4 public metadata store. Every change shows a
before/after diff and requires the reviewed revision; conflicts reload instead
of overwriting, hostile control/bidi tags fail closed, and recent-use remains
read-only until a successful managed D5.2 connection exists. No selected path,
host, identity, query, or provider value is persisted in grants or logged.

Windows 11 evidence passed 10 runtime, 8 controller, 3 targeted worker/
controller/cache unit, 5 renderer, 33 D4, 33 UI-model, 5 Connection Library,
and 46
command-palette tests plus `cargo deny`. A feature-gated test control waited for prompt-active state before opening the
Hub; the complete 1600x950 frame at 125% scale was inspected for bounds,
hierarchy, icon/text redundancy, semantic color, and absence of setup-only
catalog controls. The 50-sample warm 10,000-record
projection measured 7.2849–7.5959 ms against the below-16-ms target. The first
immediate post-LTO run measured a noisier 8.5180–9.5607 ms and was investigated
rather than silently retried. The release executable remains 22,670,336 bytes,
650,752 bytes (2.96%) above the same-host pre-M1 baseline.

Remaining external evidence: native macOS/Linux picker and static permission/
recovery runs plus controlled Narrator/NVDA, VoiceOver, and Orca verification.
Local semantic/geometry tests are not reported as those native runs. Connection
actions remain disabled until D5.2.

### D5.2 — managed OpenSSH launch and lifecycle

**Partially implemented overall; F5.1-F5.3 and the F5.4 local assurance path are fully implemented at the nonactivated boundary; F5.4 real native results remain external.**

The pure owner accepts one current inventory route or bounded typed host/user/
port. Direct argv freezes 17 defensive options plus optional exact `-l`/`-p`;
config-routed argv freezes the 15-option subset plus one canonical bounded `-J`
chain and one alias. A launch binding exists only when a freshly rebuilt review
is exactly equal. Profile/source/capsule/plan, route/argv, capability, complete
host-key/public-identity evidence, observation freshness/generation, and
canonical executable identity invalidate stale work. Changed keys cannot bind;
full fingerprints are not truncated; `known_hosts` is never mutated; `C` copies
without execution/newline/Enter. The broker rejects all argv drift and leaves
OpenSSH `KexAlgorithms`/`WarnWeakCrypto` policy untouched.

The Router owns one fail-closed runner and attaches the existing bounded
Connection Hub worker as a nonblocking receipt sink. ContextManager remains the
only independent PTY/route owner and application child-exit events reconcile the
managed lease before normal close. Zero, nonzero, unavailable status,
cancellation, route close, revocation, and shutdown produce truthful redacted
outcomes and fixed notifications; route close no longer implies success.

Terminal outcomes create provider-neutral receipts without destination,
terminal text, credential, path, environment, process ID, or executable identity.
The private atomic store keeps at most 256 records and 2 MiB with a single writer,
primary/previous recovery, symlink/reparse/permission/size validation, and
shutdown drain. An opaque reconnect identity is persisted only for inventory
profiles. Reconnect rebuilds from current D4 inventory, rejects a missing or
changed source revision, and still requires a fresh executable/host-trust review
and explicit approval; it never automatically resumes a session or action.

Schema 5 freezes this source contract while retaining exact hashes for
historical schemas 1-4. Focused M3/M4 tests cover exact/PQ-preserving
direct/routed argv, ProxyJump bounds/hostility, typed fields, full trust/identity
evidence, changed-key denial, safe copy, bounded nonactivated `ssh-add`
parsing, binding/executable replacement, terminal outcomes, receipt recovery,
worker bounds, and reconnect.

M5 adds exact local, remote, and dynamic tunnel descriptors and a separate
configuration-free typed-direct OpenSSH grammar. Local/dynamic binds default to
`127.0.0.1`; remote, non-loopback, or production forwarding requires a fresh
endpoint-bound Allow-once decision and disables session grants. Config aliases
and jumps with tunnels fail closed. OpenSSH owns all prospective sockets. A
bounded session/generation lifecycle rejects stale events and terminal reversal,
separates ready/collision/failed/cancelled/closed, and closes nonterminal state
with its route. Compact icon/color/text and accessibility projections expose the
same exact public facts.

The active schema also binds a bounded redacted 23-scenario native-manifest
validator and synthetic mutation fixture. Controlled validation binds real
evidence to the executing OS/architecture, exact clean commit, fixed OpenSSH
versions, and freshly hashed binary/package/client; a protected manual workflow
uploads only path-free summary fields. Deterministic exact-argv/hostile/
collision/staleness/decision/UI/1/10/50 cleanup tests pass. The synthetic
fixture cannot satisfy a release; the explicit local probe found OpenSSH client
9.5 but no `sshd` on this Windows host and truthfully returned an external
prerequisite without installing or changing anything.

Still external or protected: ADR 0003's two independent exact-head approvals and
server enforcement, real loader/build attestation and revocation, product
activation of the locally complete current-executable review path, a real
managed OpenSSH child, native proof of graceful-then-forced descendant cleanup,
prompt/diagnostic and manual-SSH regression, controlled pixels/screen readers, and
native Windows/macOS/Linux plus separately gated WSL 1/10/50 resource evidence.
Actual public status execution and production activation/native proof remain
external. F5.3 tunnels are fully done locally and nonactivated; F5.4 still needs
validated real Windows/macOS/Linux OpenSSH and cleanup proof, 1/10/50 resource,
manual-SSH, enable/disable/uninstall, pixel, and accessibility manifests. WSL
remains separately denied by the current release contract.

### M6/F6 — typed automation and multi-environment workspaces

**Partially implemented overall; review-only source contracts are complete
locally.** `automexia-devops::connections::{automation,workspace}` owns no
process, PTY, network, credential, provider, filesystem, listener, renderer, or
clock capability. It provides immutable recipe reviews, exact stage ordering,
no-hooks recovery, lifecycle deadlines/retry/cancel/generation rules, typed
remote initialization, declarative layout/restore, and explicitly armed
broadcast with per-target results and digest-only audit.

The application Connection Library is schema 2 while retaining its private v1
filenames for atomic migration continuity. Schema 1 loads only as an in-memory
migration preview. Editor/import previews bind the full document and CAS base;
recipe edits atomically advance dependent profile/workspace revisions and exact
fingerprints, and every material edit clears approvals. Topology-only transfer
uses fresh IDs and strips connection bindings/private labels. The UI model adds
compact, focus-restoring, redundant icon/color/text semantics for restore and
armed/disarmed broadcast without requesting execution.

ADR 0023 is accepted. The application now binds every review to the worker-
published immutable current library, exposes a bounded preview-first
`automexia workspaces` CLI, and renders a responsive Workspaces catalog plus
restore review in the Connection Hub. CLI mutation requires explicit apply and
library/entity CAS; command review comes from a bounded regular file; controller
replacement invalidates stale review; keyboard/pointer routes are modal-local;
and all product projections keep process, PTY, network, Enter, reconnect, and
resume authority false.

Focused Windows x86_64 evidence passed 9 planner, 6 automation, 10 workspace, 15
Hub-model, and 10 Connection Library tests plus warning-denied focused Clippy,
architecture/mutation policy, fuzz entry-point compilation ownership, and a
1,000-generation × 50-target bounded-state test. The 2026-08-22 Criterion run measured the maximum
16-window/64-pane/128-connection validation at 26.044–28.009 µs and 50-target
broadcast review at 15.524–16.032 µs. The direct-SSH control rerun measured
7.5818–7.8347 µs and still reported a 1.64–7.72% stored-baseline regression;
the path is unchanged by M6, so this is recorded as unresolved controlled noise,
not silently treated as a pass or attributed to M6.

The final Windows x86_64 gate passed formatting, workspace warning-denied
Clippy, Nextest (1,847 passed, 7 skipped across 54 binaries), documentation tests
(64 passed, 3 ignored), full QA including resize/session-clone/Loom/dependency
policy, and `cargo ready`. Readiness ran from a clean temporary target after the
normal D: target correctly failed its 12 GiB free-space preflight; the isolated
verification and temporary target were removed after the fresh build and
version smoke passed. Its first post-change cold run exposed a timing-only
completion pipe-holder regression outside M6; the process owner now synchronously
reaps provider descendants, and focused, Nextest, full-QA, and final cold
readiness reruns passed. Native OpenSSH, GPU, screen-reader, elevated Windows,
and hosted cross-platform evidence remain explicitly external.

The 2026-08-25 product-boundary suite passed 9 integration and 6 focused
application interaction tests, including real temp-store preview/CAS/stale/
healthy-primary recovery, oversized command-file rejection, current binding,
route isolation, pointer/keyboard navigation, 320 px through 5K responsive
geometry, semantic accessibility projection, stale-review invalidation, and no
PTY input. The final revision passed workspace formatting and warning-denied
Clippy, 2,095 Nextest cases with 7 skipped, 64 documentation tests with 3
ignored, full QA, and clean C:-target `cargo ready` through fresh application
build/version smoke; the disposable target was removed. Criterion measured
maximum workspace validation at 28.873–33.842 µs (7/30 high outliers) and
50-target review at 16.454–16.915 µs (3/30 high outliers), with no controlled
release baseline.

Managed execution, real OpenSSH/PTY/process/handle/socket cleanup, native
Windows/macOS/Linux, controlled screen-reader/visual, hosted policy, and release
evidence remain external D3/M5 gates. M6 execution stays false and CP3.3 local
workspace tasks gain no remote/provider authority.

### D6.0-D6.5 — providers and multi-cloud

**Partially implemented overall: D6.0/M7 is fully implemented locally; M8-M11 and M12 Teleport now have fully implemented local source, cached product review, and M11 private transient lifecycle while all execution remains nonactivated; OpenBao is not implemented pending ADR 0024 acceptance.**

D6.0 now owns one authority-free provider-neutral source boundary:

- `provider_auth.rs` freezes strict bounded context, capsule, observation,
  exact operation/isolation/browser policy, review, receipt, and audit records;
  public JSON ingress rejects unknown fields, hostile values, malformed
  semantics, and documents above 16 MiB.
- `ProviderAuthCapsuleStore` owns at most 64 public-only capsules and binds
  every read/write to capsule ID, session, revision, provider, and generation.
  Rebind cancels active work and stale publication fails closed.
- The reducer and Hub model cover 19 states, truthful last-known-good context,
  refresh/auth/browser/device/MFA waits, expiry/offline/denial/cancel/error, and
  explicit recovery. Passive Hub/status paths cannot launch provider tools.
- Exact official-CLI operation requests require current visible `AllowOnce`
  process and applicable network decisions. The M7 module has no process,
  network, filesystem, browser, credential, token-cache, certificate, PTY,
  renderer, or provider-config-write authority.
- Global CLI context mutations and secret-bearing flags fail closed. Official
  CLIs remain the owners of browser/device/WAM/MFA, tokens, certificates, and
  caches; M7 stores only bounded public observations in memory.
- Twelve focused tests, six policy mutation cases, the registered fuzz entry,
  16 repeated maximum 64-capsule lifecycles, and a Windows x86_64 Criterion run
  at 94.317–97.215 µs provide local source evidence. The benchmark is not a
  controlled release ratchet.

D6.1/M8 now has an independent disabled package. It parses at most 1 MiB and
128 exact granted AWS profiles, retains only public region/account/role/source
hints, constructs capsule-bound IAM Identity Center PKCE/device and regional STS
operations, decodes only strict bounded public caller identity, names both AWS
CLI and Session Manager plugin in a nonactivated PTY/tree-cleanup plan, and
produces only EKS `--dry-run` output for D3-owned runtime ingestion through M11. Ten focused tests
plus locked warning-denied Clippy and formatting passed on Windows x86_64. No AWS
process, network, login, SSM session, credential cache, or EKS cluster ran.

D6.2/M9 now also has an independent disabled package. It parses only bounded
public `az account` JSON, rejects secret-bearing/hostile/duplicate/over-complex
records, constructs exact tenant-bound WAM/browser/device login and
subscription status operations without `az account set`, binds AAD-only Bastion to the capsule
subscription, and leaves AKS output behind an opaque M11 transient-file intent.
Eight focused tests plus app registration, locked warning-denied Clippy, and
formatting passed on Windows x86_64. No Azure process, network, authentication,
cache, Bastion connection, AKS cluster, PTY, or filesystem ran.

D6.3/M10 now has an independent disabled package. It parses one exact bounded
named gcloud configuration, rejects credential/external-account fields, uses
`--configuration` on exact user/project operations, retains federation files as
opaque references, binds IAP to capsule project/zone while leaving keys and OS
Login with gcloud, and leaves GKE behind an M11 private `KUBECONFIG` reference.
Eight focused tests, app registration, warning-denied Clippy, and a near-limit
444.00–460.66 µs benchmark passed on Windows x86_64. No gcloud process,
network, auth/federation, credential DB, IAP/SSH, GKE, PTY, or file ran.

D6.4/M11 now has independently disabled Kubernetes and OpenShift packages.
Kubernetes performs stable exact regular-file review or private transient ingestion,
typed 1 MiB YAML/JSON parsing with explicit structural budgets, deterministic
source-order merge with collision denial, public-only metadata, default-denied
exec review, production TLS checks, capsule pinning, and exact nonactivated
`kubectl auth whoami`/context/exec plans. OpenShift reuses only that public
kubeconfig model and owns private-output web login, project inspection, and rsh.
Eight Kubernetes and five OpenShift tests, app guards, warning-denied Clippy,
`cargo deny`, and a 50-sample 900 KiB 1.8280–1.8788 ms Windows benchmark
passed. No real client, cluster, network, credential, plugin, browser, PTY, or
user kubeconfig ran.

D6.5 Teleport now has an independently disabled source package. It parses only
bounded public `tsh status --client --format=json`, requires exact current
proxy/cluster/user and fresh RFC 3339 expiry, clears Teleport environment and
agent integration, and builds exact nonactivated 18.10+ version/login/status/
ssh/logout plans with capsule/session/revision revalidation. Eleven focused
tests, app registration, warning-denied Clippy, dependency policy, and a noisy
but statistically unchanged 100-sample Windows benchmark pass. No `tsh`, proxy,
network, browser/MFA, cache/certificate/agent, PTY, or native release fixture
ran.

D6.1-D6.5 still need D3 product activation/attestation, controlled real-tool
native tests, child cleanup/resource/accessibility, and release evidence. Cached
product review and M11 private transient allocation/validation/revalidation/
revoke/cleanup are complete locally. OpenBao remains intentionally absent until
ADR 0024 is accepted. Direct SDK inventory remains a later, explicit, lazy authority.

### D7 — public ecosystem, direct APIs, and selected-input model suggestions

**Fully implemented locally at the accepted source boundary; partially implemented overall.** ADR 0029 and the exact contract digest were accepted on
2026-08-25. The immutable proposal contract remains unchanged and a separate
acceptance receipt authorizes private source dependencies while leaving every
release authority false.

Implemented source owners are:

- `automexia-ecosystem`: strict JSON/manifest/path/compatibility, exact capability
  grants and diffs, bounded fair lifecycle, offline distribution policy,
  selected-input privacy/consent/response contracts, and renderer-neutral review
  states without filesystem/process/network/provider/PTY authority;
- `automexia-ecosystem-runtime`: real Ed25519/provenance/SBOM/license/revocation
  verification over bounded local ZIP packages, protected atomic disabled
  storage and recovery, exact cleanup, signed non-executing action-pack mapping,
  versioned WIT conformance, and optional Wasmtime Component Model tests with no
  default WASI, exact imports, fuel/epoch/memory/table/instance limits and joined
  cancellation;
- the terminal adapter: explicit inspect/install-disabled/review/disable/kill/
  uninstall composition plus hard denial of activation, downloads, provider
  calls and grants; marketplace status is truthfully accepted-source-disabled.

The accepted logical world is `automexia:ecosystem/suggestion@1`; checked-in WIT
uses source world `extension` because WIT package interfaces and worlds share an
item namespace and the contract also imports `suggestion`. The separate
acceptance receipt freezes this syntax mapping. The private conformance source
is unpublished and cannot activate a component.

The source checker, 17 hostile mutations, unit/property tests, real signed ZIP
fixtures, WIT parser, Wasmtime fixtures, Windows protected-ACL/recovery tests,
app denial tests, fuzz harness compile and Criterion benchmarks provide local
evidence. A live sanitizer fuzz campaign remains blocked without nightly; the
stable-toolchain rejection is recorded rather than called a pass.

Still external/release-gated: two exact-head protected approvals for activated
authority; named publisher/trust/repository/revocation/incident owners; public
SDK and downloader; a separate network ADR/update client; controlled malicious
package/component and compromised-key drills; native signed Windows/Linux/macOS
sandbox/package/uninstall evidence; actual visual/accessibility/IME/focus
assessment; privacy/legal provider approval; full resource baselines; 1,000
lifecycle cycles; the 30-day soak; and verified rollback/fallback. Until those
pass, component execution, public downloads, provider calls, tools, process,
credentials, PTY input and automatic execution remain disabled.

### LO0-LO5 — optional LLM Orchestration

**LO0 is partially implemented at the documentation-only proposal boundary;
LO1-LO5 are not implemented.** The canonical specification, proposed ADR 0033,
build/wrap/adopt decision, pure `automexia-workflow-model` dependency direction,
typed action/plan/digest/grant/result outline, explicit context-consent and
provider policy, one-run approval model, provisional limits, lifecycle and
future test ladder are documented.

The repository has no workflow-model crate, cross-domain action registry,
orchestrator package, model/provider adapter, context-consent UI, plan parser,
workflow grant, plan executor, model request, managed model, prompt store, MCP
mapping, shortcut, configuration or runtime surface. Existing Quick Actions,
extension contracts, capability review and ExternalToolRunner are reusable
foundations, not proof of orchestration.

LO0 requires explicit ADR acceptance, a strict versioned machine contract and
approved owners before LO1 production source work. LO1 must remain pure and
I/O-free. LO2 may produce draft plans only. LO3-LO5 add authority only through
app-owned registry/policy/review, one-run grants, existing domain brokers, final
revalidation, sensitive-step interruption, cancellation and receipts. Models
receive no executable handle. Local/self-hosted inference is the first direction;
no paid API is required and no remote/provider fallback is silent.

This track is independent of Studio and video release sequencing and does not
block either. Domain extensions contain no model/provider code and remain useful
when the orchestrator is absent, disabled, offline, crashed or uninstalled.
Third-party delivery additionally requires accepted D7.

### AS0 — Automation Studio decision and native feasibility

**Partially implemented at the proposal-only boundary; no product capability
exists.** The canonical architecture, proposed ADR 0030, conditional CodeMirror/
Monaco and Wry decisions, separate Studio/DevOps/SRE/add-on ownership, four-tier
execution model, saved-revision run intent, lightweight profiles, AS0-AS6 order,
and future security/native/accessibility/resource/lifecycle evidence contract are
documented.

The repository has no Studio production crate or package, editor dependency,
document/trust/surface service, webview host, typed IPC, LSP/DAP broker, language
server integration, file-write capability, Studio UI, or Studio script-run path.
AS0 remains blocked on explicit ADR acceptance, a versioned numeric machine
contract, exact license/advisory/provenance/MSRV/size/startup/resource review, and
inspected native Windows, macOS, Linux X11, and Linux Wayland proof on the actual
Automexia window/renderer stack. CP5 remains a separate shell-line proposal.
This non-production AS0 work may continue before the first stable v0.4 release
but cannot add a dependency, runtime, activation path, release claim, or v0.4
blocker.


### AS1-AS6 — Automation Studio implementation and advanced capabilities

**Not implemented.** AS1 core-owned document/trust/IPC/lifecycle contracts, AS2
minimal embedded editing, AS3 bounded LSP and first-party language packs, AS4
separate DevOps/SRE typed script/tool integration, AS5 provider/tool domain
packs, and AS6 debugging/remote/mobile/collaboration plus neutral workflow-
registry interoperability all remain future work. Studio contains no model or
provider behavior.

First-party slices may proceed independently of a public marketplace only after
ADR 0030 and exact existing capability gates are accepted. Third-party add-ons
also require ADR 0029/D7. DAP, remote/mobile, collaboration and orchestration
interoperability require separate authority decisions; none inherit AS0 or LO0
approval. The terminal-only
product, native shell editors, CP1 and current first-party extensions remain the
required fallback throughout.

The roadmap places AS1-AS2 after the first stable terminal release and requires
an evidenced minimal AS2 Studio before a dedicated video-editing extension is
released. This is a delivery dependency, not implementation evidence: no Studio
or video runtime exists. AS3-AS6 and video research may proceed independently,
and video must use generic workspace/task services rather than Studio-specific
editor or language-server internals.


## Command-productivity phases

### CP0 — decisions, threats, and nonactivation

**Fully implemented at policy boundary.**

ADR 0015 is accepted. Versioned fixtures define compatibility, native
precedence, 16 threats, seven boundaries, 14 ceilings, hostile cases, and
fingerprints. Workspace scanning prevents grid inference and early authority.
Mutation/repository/architecture gates own it. CP0 grants no runtime feature.

### CP1 — shell-native completion

**Fully implemented; hosted release evidence partial.**

- PowerShell, Bash, Zsh, Fish, CMD, and WSL retain native editors/history/
  cursor/quoting/completion/user definitions.
- Adapters are idempotent, fingerprinted, removable, repairable, and disable
  cleanly.
- Docker/Kubernetes/OpenShift/Helm refresh only explicitly. Other providers
  remain provider/package owned; mutating installers require manual consent.
- Exact argv/null stdin/750 ms deadline/capture caps/UTF-8 controls/process
  group or Job Object/descendant cleanup/stable executable validation/private
  atomic artifacts are implemented.
- Refresh validates its bounded destination before process launch, clears the
  child environment to an explicit secret-free allowlist, rejects relative PATH
  entries and Windows remote roots/redirects, and sanitizes hostile diagnostics.
- A bounded two-digest publication transition preserves a verifiable
  last-known-good artifact across every in-process interruption point; all
  adapters enforce its exact framing and the 4096-byte root ceiling.
- PowerShell override requires consent; CMD truthfully remains DOSKEY fallback.
- No provider runs at startup, keystroke, rendering, or doctor.
- Tests cover lifecycle, collisions, integrity, timeout/overflow/leader exit,
  Unicode/spaces, bidi/control output, secret-environment isolation,
  interrupted publication, and platform paths.

Remaining: exact hosted Windows/Linux/macOS and controlled hostile-provider
evidence. There is no custom popup or Quick Action UI.

### CP2.0 — typed Quick Action model

**Fully implemented at pure boundary.**

Schema 1 covers typed templates, scopes, shells, placeholders, cwd, risk,
execution, provenance, tags, and alias eligibility. A 1 MiB predecode cap,
unknown/version/count/string/duplicate/reference/control/bidi/secret/unsafe
checks, eleven hostile fixtures, boundary/round-trip/property tests, architecture
allowlists, and mutation tests exist. It has zero runtime authority.

### CP2.1 — private Quick Action store

**Fully implemented as internal nonstarted library.**

- Exact private root, no-follow stable reads, 1 MiB source, 8 MiB memory
  estimate, Windows DACL and Unix 0700/0600.
- Durable same-directory primary plus previous revision, nonblocking lock and
  revision CAS, immutable fingerprints, tamper/rollback rejection, explicit
  recovery, last-known-good, bounded 64-event watch, coalescing, reconciliation,
  CRUD, and cleanup.
- Permission/concurrency/recovery/Unicode/1,000-cycle/storage/storm/property
  tests and controlled 1/256/1,024-action benchmarks exist.

Remaining: hosted native matrix, crash/power-loss injection, and the 30-day
baseline. CP2.2 now consumes this foundation through a joined app worker.

### CP2.2 — search, editor, review, and insertion

**Implemented locally; release evidence partial.** The app owns one joined,
per-route latest-only worker and immutable last-known-good snapshot. A hard
32-route ceiling prevents cross-pane state growth while fair batched publication
prevents one busy pane from starving another. The pure index applies exact
shell-user/global-user precedence, revalidates every activation layer, and keeps
search stable and bounded; workspace activation,
secret reads, providers, network, shell evaluation, and exact launch stay
disabled. The pane-neutral Command Center flow provides responsive search,
placeholder entry, risk/conflict/exact-command review, and explicit insert or
copy. Secret and exact-launch actions fail before any placeholder input is
collected; visible rows include risk, source, conflicts, and degraded-store
health, while empty/unavailable states are explicit. Insert uses bracketed paste
and never Enter. The CLI provides bounded
list/show/doctor, dry-run-by-default put/import/remove/recover, digest-checked
export, CAS, explicit conflict replacement, and conservative machine-path
consent.

Automated evidence covers five shell serializers, Unicode and hostile text,
scope shadowing, workspace/secret/exact-launch denial, transfer tampering,
native file security, worker storms/shutdown, route isolation, palette keyboard
flow, 400% responsive layout, dry-run mutation safety, policy mutations, and
search/expansion benchmarks. Remaining release evidence is hosted native
Windows/Linux/macOS shell insertion, controlled Narrator/NVDA/VoiceOver/Orca,
and the 30-day named-hardware performance/resource baseline. D3 exact execution
remains blocked.

### CP3.0 — projection compiler

**Fully implemented at the pure source boundary; release evidence is partial.**
The 2026-08-17 follow-up audit reconciled the phase again against code, tests,
contracts, CI, and documentation. The exact capability-free model boundary
includes a deterministic projection compiler for PowerShell, Bash, Zsh, Fish,
and CMD. Validation rejects unsafe scope, exact launch, CWD, override provenance,
argument-policy mismatch, hostile tokens, and unsupported CMD typed shapes with
stable codes. Complete bounded caller inventories drive native ownership,
matching-fingerprint user override consent, completion health, and tool identity;
missing or ambiguous evidence fails closed.

The follow-up closed four residual gaps: the compiler now recomputes canonical
source identity instead of trusting a well-formed caller digest; tool observations
have an explicit completeness bit; a claimed same-action owner also needs the
deterministic owner fingerprint; and decisions retain missing/unsupported tool
detail for the UI. Existing completion collisions block even when generation is
disabled. The malformed CI step that overwrote one of the two mutation-suite
commands was repaired, and YAML parsing now validates both independent steps.

Generated artifacts are sorted, size-limited, nonactivated, and carry schema,
generator, source, shell, tool/version/file identity, owner fingerprint,
previous-artifact digest, binding/decision manifests, and verified BLAKE3 body
identity. Thirteen focused tests cover all serializers, typed/forward/fixed
arguments, ownership and override rules, inventory-order determinism, degraded
completion/tool state, source mismatch, invalid/duplicate/incomplete observations,
structured/text tampering, hostile quoting, the 256-binding ceiling, and available
native syntax/exact capture/exit status. The nightly fuzz target now generates
bounded valid cases across all five shells and degradation/collision/tamper states.
A 256-binding Criterion target, schema-1 contract, static capability checker, and
nine policy mutations prevent regression.
An optimized local diagnostic compiled 256 Bash bindings, including independent
source recomputation, in 2.7919-2.9846 ms per iteration. This is not a substitute
for the named-hardware 30-day release baseline.

No test or compiler path reads or writes a real profile, executes a provider, or
grants filesystem/process/environment/network/secret/exact-launch authority.
CMD native evidence loads the macro file and tests exact positional golden
semantics because DOSKEY expansion is interactive-only. Hosted platform runs and
the 30-day controlled performance baseline remain release evidence; managed
publication, reload, exact uninstall, and shell startup integration belong to CP3.1.

### CP3.1 — persistent opt-in aliases

**Fully done — fully implemented at the source/local boundary.**

- **Fully done** — The existing CP1 managed hook verifies the exact ten-line,
  versioned compiler manifest and loads exactly one bounded, private,
  SHA-256-authenticated artifact from an immutable content-addressed generation
  for PowerShell, Bash, Zsh, Fish, and CMD.
- **Fully done** — `list`, `preview`, `test`, `enable`, `disable`, `rename`,
  `regenerate`, `disable-all`, `rollback`, `doctor`, and `reload` are explicit.
  Mutations are dry-run by default; their JSON/text output supplies the current
  revision and generation values required to apply, plus source identity,
  bindings, collisions/owner fingerprints, completion, and tool health.
- **Fully done** — A private cross-process lock, durable transaction journal,
  source compare-and-swap, immutable generation, `current` pointer committed
  last, one `previous` generation, and deterministic crash recovery guarantee an
  all-old or all-new result. Disabling/uninstalling preserves canonical actions.
- **Fully done** — Startup and doctor reject links/reparse points, unsafe
  permissions/ACLs, unexpected directory entries, malformed metadata, digest or
  exact compiler/source/shell identity mismatch, active or retained-generation
  tampering, and late native collisions without executing a provider, action,
  network operation, or canonical rewrite. Malformed source and unsafe roots are
  returned as stable health states.
- **Fully done** — Native definitions win. An advanced exact override is reused
  only from an authenticated manifest and only while the same observable owner
  fingerprint is still present; shell aliases/functions that cannot be restored
  safely remain native winners.
- **Fully done** — Active PowerShell/Bash/Zsh/Fish reload removes only unchanged
  Automexia-owned definitions and retains the last-known-good set on failure.
  CMD truthfully requires a new session because reversible DOSKEY ownership
  cannot be proven. Executable and completion observations are hashed once per
  unique identity rather than once per shell.
- **Fully done** — Twenty-one owned Rust security/lifecycle cases across platform
  conditions, three CLI detail regressions, hostile-manifest properties,
  cross-process contention, native Windows plus WSL Bash/Zsh/Fish lifecycle and
  wrong-compiler tests, exact uninstall preservation/refusal, a 256-alias
  benchmark, the versioned contract, eight mutations, aggregate policy, and
  configured nightly/release WSL gates protect the phase.

Release evidence is **Partially done**: local Windows and WSL Bash/Zsh/Fish
suites pass, including the Unix-only unsafe-permission regression. Hosted CI now
owns the complete WSL lifecycle in nightly/release and the native Linux/macOS
matrix. Published hosted results and the named-hardware 30-day startup/resource
baseline remain release gates, not missing CP3.1 source.

### CP3.2 — first-party static DevOps packs

**Fully implemented locally.** The capability-free registry contains immutable
schema-1 Git, Docker/Compose, Kubernetes, OpenShift, Helm, Terraform, OpenTofu,
AWS, Azure, Google Cloud, and OpenSSH manifests. Each pack has exactly three
typed actions (33 total), a reviewed minimum tool version, exact version argv,
HTTPS documentation, completion policy, stable provenance, and effect/risk
classification. Every built-in is `BuiltinDisabled`, insert-only, unaliased,
and materializes only after explicit selection. A reviewed digest asserts the
complete serialized registry during initialization, freezing exact argv,
versions, URLs, completion, effects, risks, and provenance.

Pure health evaluation accepts only bounded caller-supplied Missing/Detected/
Unobserved observations and never starts a provider. The update planner rejects
version regression and stale overlay digests, preserves valid custom overlays,
and reports added, updated, unchanged, deprecated, and removed actions. It
normalizes only manifest provenance versions, so version-only upgrades are
unchanged while functional metadata changes remain updates. The generic
validator rechecks manifest identity before allowing a built-in alias;
context-changing, authentication, destructive, and privileged effects fail
closed. `automexia packs list/show/doctor/enable` is read-only by default;
registry doctor reports only registry readiness, while enable preview exposes
exact argv/effect/risk/documentation and the reusable CAS revision. Apply rejects
a stale revision before store creation, never overwrites an action, and never
enables an alias.

Evidence: 12 integration cases plus two registry unit/mutation cases and five
focused CLI parser/rendering/preflight cases; a 33-action registry/health
Criterion target; a nightly pack fuzzer; a schema-1 exact-payload contract; eight
mutation cases; aggregate repository/xtask
wiring, and synchronized architecture/product/testing/roadmap documentation.
Hosted cross-platform and the named-hardware baseline remain release evidence,
not missing source behavior.

### CP3.3 — import and trusted task bridges

**Fully implemented locally; release assurance partial.**

Capability-free parsers cover explicit PowerShell CSV, Bash/Zsh alias, Fish
abbreviation, CMD/DOSKEY, and Git inventories; reject controls/bidi, duplicates,
likely secrets, paths, substitutions, pipelines, metacharacters, and Git shell
aliases; and create only explicitly selected independent Mutating/Insert actions.
App-owned bounded no-follow import uses dry-run/CAS conflict and rename review.
Exact named just/Task/mise workspace bridges persist in `.automexia/actions.toml`
without recipe parsing, listing, discovery, providers, network, credentials, or
execution. Private path-free digest/revision receipts, trust/revoke/remove,
source-change invalidation, bounded ancestor/cache reconciliation, short route
authorization, and review/insertion rechecks fail closed with textual UX.

Fourteen named parser/import/trust/runtime/CLI regressions, Unix link cases,
contract mutations, aggregate capability ratchets, nightly fuzzing, Criterion
parser/trust targets, CI/xtask wiring, ADR 0021, and synchronized documentation
are present. Hosted native/accessibility and controlled 30-day evidence remain.

### CP4 — capsule/provider actions

**Fully implemented locally at the product-integrated nonactivating boundary; partially implemented overall.**
The capability-free action core validates immutable public provider candidates,
bindings, digests, nine freshness/availability decisions, production risk, and
redacted audit. SSH plus six first-party provider owners contribute exact typed
argv; the application composes an all-or-nothing snapshot and publishes a
prebuilt index for one of at most 32 routes. Search rejects stale route/session/
capsule/generation results, provider rows deterministically shadow same-ID
persisted actions, and route cleanup/revocation removes the snapshot.
The Connection Hub retains the already validated capsule through a redacted
immutable publication. Opening the Action Center binds it only to the exact
selected route/session/revision, treats identical publication as unchanged,
and clears mismatch, revocation, or absence before cached search.


Search/review show compact provider, exact target, state, and environment risk
with connection icon/color plus complete textual/accessibility redundancy. The
screen revalidates immediately before copy or bracketed insertion. Current
observations remain insert-without-Enter; production adds a second confirmation;
refreshing/stale/expired/offline/unavailable/error/replaced and broker-required
operations cannot copy or insert. No provider adapter is imported by the worker
or interactive UI, and no CP4 source refreshes, authenticates, connects, starts
a process, reads credentials/files, persists provider state, or gains PTY/
renderer authority.

The schema-1 contract fixes seven providers, three publication outcomes, nine
decisions, eleven denied authorities, five ceilings, 25 named regressions, a
hostile-capsule fuzz target, two cached benchmarks, and seven policy mutations.
Approved provider refresh/capsule production, exact provider execution,
OpenBao after ADR 0024, real accounts/CLIs/clusters,
Linux/macOS native runtime, controlled accessibility/resources, packaging,
signing, and release fixtures remain external.

### CP5.0 — autocomplete research

**Fully implemented at the research boundary.**

The seven-family shell/API matrix, bounded ephemeral editor-state/insertion
prototype, 32/128/512-candidate Unicode and stale-generation benchmark,
Nucleo/Reedline/Carapace dependency decisions, zero privacy/authority delta,
native fallback, external gates, and rollback are recorded in the CP5.0
research report and fixed by the machine contract. CP1 remains the complete
solution; P2 is deferred. No runtime dependency, editor transport, profile,
keybinding, process, PTY, history source, worker, cache, or product UI exists.

### CP5.1-CP5.6 — optional suggestion UI

**Partially implemented overall; preview and stable activation remain disabled.**

| Phase | Status | Current source/local evidence | Remaining exit evidence |
|---|---|---|---|
| CP5.1 | **Fully done at source boundary / partially done for release** | Accepted ADR; strict bidirectional schema-1 frames; route/capability/generation/span validation; joined latest-only service; restrictive Windows named pipe and filesystem Unix socket adapters; property/fuzz, fragmented-stream, replay, cleanup and native Windows fixtures. | Runtime composition remains disabled; native Linux/macOS endpoint/churn/crash/sleep evidence and the WSL relay remain. |
| CP5.2 | **Fully done at source boundary / partially done for release** | Six ordered typed sources; independent history/frequency opt-ins; memory-only counters; cached-public LKG/freshness; bounded deadlines/batches/cache; privacy/source-policy tests. | Activated native editor/provider fixtures, slow/offline/cancellation storms, and native resource evidence remain. |
| CP5.3 | **Fully done at source boundary / partially done for release** | Deterministic native-first ranking and stable ties; grapheme/control/bidi/span checks; one-latest slots; stale/capability replacement rejection; property/fuzz and 32/128/512 plus near-limit frame benchmarks. | Named-hardware latency/allocation/cache distributions, native shell quote/selection/multiline insertion, and sustained cleanup evidence remain. |
| CP5.4 | **Fully done at source boundary / partially done for release** | Pane-owned listbox/controller/renderer; source publication mailbox; exact candidate reconstruction; authenticated status/replacement; 64-route and deadline bounds; tiny-to-8K and 100-300% tests. | Live screen composition remains disabled; native GPU, IME/pointer/keyboard, and controlled NVDA/Narrator/VoiceOver/Orca evidence remain. |
| CP5.5 | **Fully done at inert source bridge / partially done for product composition** | Helper target plus bounded bootstrap/transport/session/endpoint runner; strict response envelope; bidirectional PowerShell/Bash/Zsh/Fish adapters; Windows handle/status cleanup and WSL Bash/Zsh/Fish Unicode replacement/stale/hostile native tests. | Reviewed launcher/restricted inheritance, signed artifact, WSL relay, interactive PowerShell insertion, live screen composition, profile/uninstall fixtures and activation remain. |
| CP5.6 | **Partially done** | Preview activation stays false; broker/mailbox kill/reset/disable/uninstall, LKG, exact route limits, mutation policy, real local shell harnesses, hosted OS-specific steps and CP1 fallback contracts exist. | Successful hosted three-OS runs, accessibility/GPU, signed package/update/rollback, 1,000 real endpoint cycles, fuzz/leak/storm campaigns, named-hardware latency/resource evidence, and the 30-day preview baseline remain external. |

ADR 0025 and the schema-1 CP5-T17 through CP5-T22 contract are accepted for
source implementation with `accepted: true` and `runtime_activation: false`.
The source checker prevents forbidden transports, per-key processes, implicit
Enter, default adapter sourcing, missing preview guards, and false completion
claims. CP1 remains the default and fallback; a popup alone cannot satisfy
privacy, insertion, accessibility, lifecycle, performance, or rollback gates.

### CP6 — ecosystem packs and selected-input model suggestions

**Fully implemented locally at the accepted no-provider source boundary; partially implemented overall.** A verified immutable package that requests no
capabilities can map a signed action document into the existing typed Quick
Action model. Actions are namespaced, disabled, insert-only, exact-argument,
non-alias, non-raw, and collision/revocation/final-state checked. The mapping
never executes or presses Enter.

The model slice accepts only explicit bounded selected text. It normalizes and
validates input, creates deterministic secret redactions, discloses exact preview,
redactions, provider, locality, model, destination, purpose, retention, byte
count and environment risk, and binds confirmation to a digest, route,
generation, expiry and one use. Responses are strict typed bounded explanations
or suggestions; Automexia independently classifies risk. Debug/receipt/persistent
state excludes selected and response content. Renderer-neutral surfaces are
cancel-first, responsive, reduced-motion aware and carry textual accessible
trust/risk meaning.

Provider calls, tool calls, workflow planning, MCP passthrough, background or
typing requests, ambient terminal/history/clipboard/file/environment/credential/
agent/provider/capsule/connection/other-pane data, Enter and automatic execution
are hard-disabled. Remaining work is external release evidence: native product
composition and UX, screen readers/IME/focus/visual matrix, provider timeout/
cancel/delete/retention behavior, privacy/legal approval, malicious prompt
corpus, signed three-platform packages, resources/soak and protected activation.
The separate LO track neither activates nor weakens CP6.

### DN0-DN6 - semantic diagnostic navigation

**DN0 is partially implemented at the documentation-only proposal boundary.
DN1-DN6 are not implemented.**

The
[Semantic Diagnostic Navigator](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md), proposed
[ADR 0032](adr/0032-bounded-semantic-diagnostic-navigation.md), main-roadmap
register, architecture link, feature non-claim, command-result relationship,
decision indices, and changelog fragment are present. This proves that the
recommendation is integrated into repository authorities; it does not prove
runtime behavior.

Reusable current foundations:

- semantic previous/next prompt movement exists;
- supported integrations attach OSC 133 result metadata to prompt rows;
- normal scrollback, bounded search, renderer-neutral UI contracts, route
  scheduling, and exact route cleanup patterns exist;
- prompt identity supports its declared prompt/reflow lifecycle.

Important missing evidence:

- current result metadata and its following-prompt boundary now survive
  viewport and complete source-prompt eviction, but remain content-free
  lifecycle identity rather than a durable complete command-output region;
- no generic stable logical-line identity exists, and the proposal intentionally
  defers adding one;
- no navigator owner, action, palette entry, binding, setting, continuation,
  line-batch API, detector, section locator, cache, highlight, metric, fuzz
  target, benchmark, native harness, or feature-assurance entry exists;
- no extension has terminal-history capability, and DN6 is not implied by the
  private first-party extension API.

| Phase | Audit status | Exit evidence still required |
|---|---|---|
| DN0 | **Partially implemented at documentation-only proposal boundary** | Explicit ADR acceptance or supersession; approved action semantics; exact module/dependency owners; machine-checked ceilings and mutations; hostile/false-positive corpus; tests that fail before DN1 code; experimental rollout decision. |
| DN1 | **Not implemented** | Separate previous/next failed-command typed actions and palette entries; exact prompt/input targeting; known non-zero/success/neutral shell cases; order/no-wrap/forward-live/manual reset; output/resize/eviction/route close/session isolation; accessible status; disabled baseline; no default shortcut without collision proof. |
| DN2 | **Not implemented** | Bounded normalized logical-line batches; one route-local request/continuation; generations and stale rejection; optional 256-entry content-free cache; viewport placement; reflow remap-or-clear; alternate-screen suspension; queue/lock/allocation/cleanup tests and benchmarks; cache-disabled equivalence. |
| DN3 | **Not implemented** | Borrowed structured severity; conservative anchored Error/Fatal headers; bounded section locator; separate class/severity/provenance/confidence; false-positive corpus; malformed/oversized/Unicode/wrap/overwrite/eviction/mixed-format property/fuzz/performance evidence. |
| DN4 | **Not implemented** | Renderer-neutral non-color highlight; selection/search precedence; reduced-motion, tiny-to-8K and 100-300% behavior; content-free accessibility announcements; experimental disable/reset/LKG; Windows/macOS/Linux X11/Wayland and assistive-technology evidence; S2 input/frame/memory/cleanup ratchets. |
| DN5 | **Not implemented** | Independent specialized-format slices; bounded config-load-only user patterns; exact version/fixtures/limits/provenance/disable/cache invalidation/fuzz/benchmark/native/reference/rollback evidence per detector. |
| DN6 | **Not implemented and not authorized** | Separate accepted privacy-sensitive terminal-output read/contribution capability; declarative-first design; consent/scope/isolation/quotas/revocation/cleanup/malicious-detector proof; no content persistence/forwarding; ADR 0029 for third-party delivery. |

The architecture decision rejects continuous scanning, per-pane workers, a
persistent history database, color classification, domain parsers in `rio-vt`,
global logical-line IDs as a DN1 prerequisite, unrestricted regular expressions,
non-zero-equals-error semantics, and network/AI detection.

The planned scanner is strictly on demand. One `Context` owns its state. Under
the terminal lock it creates only a small bounded transient normalized-text
batch plus content-free positions, then runs app-owned pure detectors outside
the lock. Publication revalidates route, session, buffer, layout, request,
detector, and filter generations. Transient text is discarded and excluded from
anchors, metrics, persistence, crash evidence, QA bundles, and renderer state.

Provisional review ceilings are one active request and one continuation per
context, 256 content-free anchors, 256 physical rows/64 KiB transient text per
batch, 16 KiB text and 64 KiB structured candidates, a one-millisecond yield
target checked after each line, 1,000 logical lines/512 KiB per reconstructed
section, one highlight, and-only in DN5-32 patterns of at most 1 KiB each. These
are not public settings or accepted guarantees. DN0 must freeze or lower them
through tests, a machine contract, lock/input/frame benchmarks, and native
resource evidence.

No current test command can honestly claim DN behavior. Before any status
advance, the phase must add the focused deterministic test first, then
unit/model/property/fuzz/concurrency/renderer-neutral/native/accessibility/
benchmark/leak evidence proportionate to that slice. The disabled path must show
zero scan work, zero per-pane worker, zero persistent storage, and unchanged
terminal behavior. Every late fix reruns all affected gates.

Release sequencing remains deliberately independent: DN1 production follows the
stable v0.4 release, DN1-DN4 may run alongside later v0.5 work with no assigned
release yet, DN5 follows generic precision/resource evidence, and DN6 waits for
its own capability decision. DN does not block SSH, providers, Automation
Studio, or video unless a future explicit release plan makes it a shared gate.


### PO0-PO8 - situation-aware production operations

**PO0 is partially implemented at the detailed proposal/checker boundary.
PO1-PO8 are not implemented.**

The
[Situation-Aware Production Operations specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md),
[exact proposed PO0 contracts](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md),
[UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md),
[future testing contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md),
[proposed ADR 0034](adr/0034-situation-aware-production-operations.md), roadmap
register, architecture boundary, feature non-claim, decision index, assurance
entry, changelog fragment, strict JSON planning contract, canonical digest,
semantic checker, mutation suite and current-source nonactivation scan are
present. They establish one repository-owned proposal; they do not establish
runtime behavior or accept ADR 0034.

Reusable current foundations:

- CP1 remains the complete shell-native fallback, while CP5 provides a
  preview-disabled authenticated editor/suggestion boundary;
- D2 owns route-scoped immutable Environment Capsules, freshness, cancellation,
  and isolation;
- D6/M7-M12 own bounded public provider sources and cached review without active
  provider execution;
- CP4/M13 owns typed route-bound provider actions with final revalidation,
  production confirmation, and insert-without-Enter behavior;
- D3 owns the nonactivated exact-argv broker and child lifecycle; and
- DN0 separately proposes navigation inside terminal diagnostic output.

Important missing evidence:

- no accepted PO decision or accepted machine-contract digest exists; the
  proposed numeric contract/checker exists, but protected owner/dependency/
  capability approval, reviewed prototypes, first failing real-path tests,
  independent threat review and packaged nonactivation evidence do not;
- no pure operation model, environment passport, pane lock, context-diff state,
  evidence-quality model, evidence snapshot, resource/dependency graph,
  change/provenance/field-owner view, semantic drift comparison, resource or
  scheduling explainer, healthy cohort/revision/environment comparison,
  network-path diagnosis, SLO summary, live-log controller, incident hypothesis,
  adapter, watcher, ranker, policy engine, candidate, preflight, Incident Mode,
  journal, monitor, recovery, port-forward/probe/debug session, runbook pack, UI,
  setting, shortcut, provider capability, or execution path exists;
- no `kubectl rollout` situation corpus, real-cluster oracle, authorization,
  GitOps, JIT, impact, blast-radius, event-retention, ownership, scheduler,
  cohort, network-vantage, SLO-window, log-gap, time-anchor, debug-privilege,
  listener-cleanup, resource, fuzz, native, visual, accessibility, package,
  rollback, uninstall, or soak evidence exists; and
- current provider caches do not imply fresh live health, permission, GitOps,
  policy, telemetry, or business-criticality evidence.

| Phase | Audit status | Exit evidence still required |
|---|---|---|
| PO0 | **Partially implemented at detailed proposal/checker boundary** | The proposed strict machine contract, 17 payload sets, 37 action profiles, digest, semantic checker, mutations, traceability and source nonactivation scan exist. Still required: add phase-owned runtime schemas/parsers; accept or supersede ADR 0034 and the exact digest; approve source/dependency/capability owners; review nonactive prototypes and threats; add first failing real-path and packaged nonactivation tests. |
| PO1 | **Not implemented** | Read-only route-scoped production passport/lock/context diff; provider/public identity freshness; production fail-closed; pane/tab/window/clone isolation; native, responsive, accessibility, resource and cleanup evidence. |
| PO2 | **Not implemented** | Separately reviewed provider adapters; bounded public evidence and UID-based resource graph; change/provenance/field ownership and semantic drift; resource/scheduling explanation; healthy cohort/revision comparison; passive network-path diagnosis; optional approved SLO summaries; explicit evidence quality and unknowns; no raw logs, active probes, time-series store, credentials or disk cache; stale/offline/relist/revoke/disable/uninstall and resource evidence. |
| PO3 | **Not implemented** | Deterministic Kubernetes/cross-tool scenario corpus; hard gates, refusal, reasons and stable ranking; exact CP5 replacement/no Enter; CP1 fallback; stale/cancel/performance; native shells, pixels and assistive technology. |
| PO4 | **Not implemented** | Exact target/argv impact preflight; independent authorization, risk, GitOps, JIT, policy, change-window, approval and recovery contracts; no false dry-run claim; fail-closed production and provider authority evidence. |
| PO5 | **Not implemented** | Incident Mode with hypotheses, contradictions, coverage-qualified negative evidence, bounded timeline, trusted/approximate time anchors, optional source-separated memory-only live logs, Error Navigation handoff and content-minimized session journal; optional protected persistence and reviewed handoff/export; log-gap/order/backpressure, privacy, corruption/disk-full/recovery/retention/disable/uninstall and long-session evidence. |
| PO6 | **Not implemented and blocked on independent execution gates** | D3 and adapter capability activation; exact one-action review/execute/observe/stabilize/verify/recover state machine; before-state, regression and real provider/process-tree oracles; separately granted Kubernetes port-forward, controlled-probe and safe-debug slices with listener/vantage/image/profile/privilege/cleanup proof; cancellation/timeouts/rollback/no-chaining; native/resource/package/release evidence. |
| PO7 | **Not implemented** | D7/capability decision; signed versioned declarative organization packs; malicious corpus, policy conflict, one-step workflow, provenance, atomic rollback, disable/uninstall and cross-tool native evidence. |
| PO8 | **Not implemented** | Independent adapter slices; read-only cross-region/cluster comparison with reviewed service equivalence, separate authority/freshness and bounded fan-out; optional local tie-breaker restricted to already-valid candidates; deterministic fallback; model safety/resource differential tests; three-platform native UX/accessibility/package and controlled provider/soak evidence. |

Future user-experience evidence is also phase-specific and currently missing:

| Phase | Exact future UX proof required before the audit may advance |
|---|---|
| PO0 | Reviewed nonactive PO projections for all six reused surfaces and read-only, reviewed-insertion, managed-action/session, responsive, keyboard, accessible, stale/refused, cancel/failure/recovery journeys; frozen copy, focus, input, ownership-path and operation/session-kind contracts; explicit nonactivation. |
| PO1 | Existing above-command context line extended as a nonfocusing passport; comfortable/compact/minimal field order; exact context diff, adopt/keep-lock flow, production text semantics, pane/clone isolation, native focus and screen-reader proof. |
| PO2 | One `Investigate` entry exposing only relevant Explain state, What changed, Compare with healthy, Diagnose connection and Show user impact views; source/age/scope/coverage/evidence-quality labels; correlation never presented as cause; semantic rather than secret-bearing/raw-YAML comparison; explicit network vantage; visible stale/offline/unknown state; responsive detail and source-accessibility proof. |
| PO3 | Existing CP5 surface with at most five two-line situation rows, ordinary results reachable, predictable stable selection, detail on request, refusal/review labels, exact Tab/Right Arrow insertion without Enter, CP1 fallback, native shell/IME/listbox proof. |
| PO4 | One cancel-first fixed-order preflight with exact target/impact/authority/GitOps/policy/verification/recovery; unknown blocking state; reviewed insertion before PO6; clearly separate managed action after PO6; JIT/approval never hidden. |
| PO5 | Explicit Incident workspace entry/exit, objective/passport/facts/hypotheses/contradictions/missing evidence/timeline/next-action hierarchy; trusted versus approximate time navigation; visibly source-separated pausable memory-only logs with gaps; session-only journal default; reviewed redacted handoff/export; restored focus, multiple-operator/accessibility/long-session proof. |
| PO6 | Nonmodal one-operation monitor with prepare/run/observe/stabilize/verify/result/uncertain/recovery states, terminal remains usable, safe cancellation, before-state and last verified observation, no process-exit-only success, no automatic second mutation; the same explicit review/lifetime/stop model for port forwards, probes and debug sessions with native failure/cleanup proof. |
| PO7 | Pack install review showing publisher/signature/version/data/conflicts/zero grants; guided workflow stops after each reviewed step; immediate disable/uninstall; conflict, provenance, keyboard/accessibility and cross-tool native proof. |
| PO8 | Identical words and surfaces per adapter, graceful unsupported/disconnected behavior, read-only Compare environments with visible equivalence/access/freshness/window/fan-out limits and no merged authority, optional reorder-only model setting and instant deterministic fallback, low-end performance, three-platform visual/accessibility/package/soak/removal proof. |

The audit must compare these contracts with real controller, input, projection,
renderer, accessibility, provider, and packaged behavior. Screenshots or prose
alone cannot satisfy a phase.

The proposal uses deterministic rules, resource ownership/dependency graphs,
current public evidence, and reviewed organization metadata. It rejects an LLM-
first autonomous agent, provider work per keystroke, one opaque priority score,
automatic execution, a competing completion/process owner, and raw observability
storage. Any later small local ranker can only reorder candidates that already
passed deterministic gates and can neither create nor execute a command.

No current test command can claim PO runtime or provider behavior. The PO0
checker can claim only that the reviewed planning fixture is strict and that
current named runtime sources do not contain the reserved activation markers.
Before a phase advances, its
machine contract, first failing real-path test, independent oracles, focused
unit/property/fuzz/model checks, renderer-neutral/native/accessibility evidence,
resource and storage measurements, rollback/disable/uninstall proof, and exact
documentation must exist. Missing providers, accounts, clusters, GitOps systems,
policies, credentials, hardware, signing, protected reviewers, or assistive
technology stay external.

## Terminal-first remote operations projection

The canonical product experience is now specified in
[Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md) and accepted
for the v0.5+ roadmap by
[ADR 0018](adr/0018-terminal-first-remote-operations.md). It does not introduce
another phase or capability system: D3-D7 own remote/process/provider/file/
collaboration authority, while CP2-CP6 own actions, aliases, completion,
suggestions, packs, and selected-input model insertion without execution.

**Specification complete; public product implementation mostly not started.**

Implemented foundations that the specification reuses:

- current independent PTYs, windows, splits, pane-local tabs, cloning,
  renderer-owned palette/modal composition, prompt context, and passive footer;
- D1-D2 typed bounded models, session isolation, immutable snapshots,
  cancellation, status/freshness, and extension non-blocking behavior;
- the non-activated D3 exact-argv/capability review boundary;
- D4 bounded non-executing OpenSSH inventory;
- CP1 native shell/provider completion;
- CP2.0 typed Quick Action model and CP2.1 private nonstarted store.

Planned work, with no shipped-command claim:

| Product surface | Owning gate | Audit status |
|---|---|---|
| Canonical operation registry, `automexia` domains, generated palette/help/accessibility, optional collision-safe `ax` | CP2.2-CP3/D5 | Not implemented |
| Host picker, recent/favorites/groups/tags/queries, Connection Review, connect/reconnect/destinations | D5.0-D5.2 | D5.1 picker/browse/metadata and D5.2 typed direct/config-jump preparation, full trust review, safe copy, fail-closed runner, exact PTY seam, and route publication are complete locally but nonactivated; protected connect and native evidence remain |
| Identity references, agent/certificate/hardware public state, known-host explanation, routes/jumps/proxies/tunnels | D5.2 | M4 references/status/trust and M5 exact local/remote/dynamic tunnels, loopback defaults, strong review, and lifecycle are fully implemented locally and nonactivated; actual status execution, broader proxies, protected activation, and real F5.4 native evidence remain |
| Quick Actions, aliases, lifecycle hooks, reviewed multi-target execution | CP2.2-CP4/D5E | CP2/CP3 actions and CP4 cached provider discovery/product handoff/final revalidation are product-integrated locally; exact provider and multi-target execution remain disabled |
| Declarative workspace persistence/restoration and visibly armed broadcast | M6/F6 | Accepted schema-2 preview/CAS manager, public CLI, immutable Hub catalog/restore review, armed broadcast projection, and bounded product tests are complete locally; managed execution and controlled native evidence remain |
| Per-pane multi-cloud/context commands and typed import/reconcile adapters | D6.0-D6.5/CP4 | Partially implemented: M7, M8-M11, M12 Teleport, cached Hub review, M11 private lifecycle, and CP4 cached action projection are locally complete/nonactivated; provider refresh/import/reconcile/execution, real provider/native evidence, and OpenBao remain gated |
| Structured files/SFTP, bounded logs/bookmarks, team state/policy, shared sessions | D7 protected slices | Not implemented |
| Mosh, Telnet, serial, public ecosystem packs, optional editor popup, selected-input model explain/suggest | D7/CP5-CP6 | D7/CP6 source boundary fully done locally but public distribution, component/provider activation and native release evidence remain deferred; Mosh/Telnet/serial remain not implemented |
| Optional cross-extension LLM workflow orchestration | LO0-LO5 | LO0 documentation exists; workflow model, extension, provider request, reviewed execution and release evidence are not implemented |

Before any row changes to shipped, it needs the exact CLI/configuration/
keyboard reference, feature-ledger entry, typed resource ceilings, threat and
capability review, deterministic model/property/hostile tests, native Windows/
Linux/macOS and applicable shell/provider evidence, accessibility/responsive
goldens, benchmarks, long-run leak/cleanup proof, recovery/uninstall behavior,
and changelog. `ax` remains optional and cannot shadow native user state.

## Ghostty compatibility phases

The implicit `automexia` profile remains unchanged. `ghostty-1.3` is explicit
and pinned; `ghostty` is an explicit moving alias. Cross-platform release claims
remain narrower than the locally implemented source/runtime surface.

### G0 — source lock and safety — Partially done

**Partially implemented:** native macOS fixture and native Linux/macOS release smoke evidence remain external.

Ghostty 1.3.1 tag `v1.3.1` and commit
`22efb0be2bbea73e5339f5426fa3b20edabcaa11` are pinned. Reviewed Linux/BSD
keybindings/actions, source and binary hashes, deterministic Windows adaptation,
classic default golden, schema/checksum manifest, generator, offline verifier,
property tests, and accepted ADR 0026 exist. Normal builds never execute
Ghostty. A native macOS fixture and native Linux/macOS release smoke evidence
remain external.

### G1 — typed registry — Fully done

**Fully implemented at the local source boundary.**

The private pure `automexia-keybindings` owner provides stable action schemas,
typed logical/physical/named triggers, predicates, scopes, origins, policies,
bounded compilation, allocation-free indexed direct lookup, reverse lookup,
sequence trie/table storage, deterministic diagnostics, classic bridging, and
registry-derived palette hints. It owns no IO or runtime effects.

### G2 — profiles, overrides, migration, and reload — Fully done

**Fully implemented at the local source boundary.**

Default `automexia`, moving `ghostty`, and pinned `ghostty-1.3` profiles, typed
bind/unbind layers, strict/permissive compilation, moving-alias disclosure,
legacy bridging, bounded dry-run/confirmed migration, and complete immutable
last-known-good reload are implemented. Profile, palette, and typed global
hotkey publication is transactional and never recreates a PTY.

### G3 — outcomes, sequences, tables, and chains — Fully done

**Fully implemented at the local source boundary.**

Structured action outcomes, performability, consumption/fallthrough, shell-owned
bare controls, stable all-surface route snapshots, coalesced damage, exact
pending bytes, cancellation/replacement flushing, bounded nested/one-shot
tables, catch-all, ordered chains, and route-isolated indicators are implemented
with deterministic/property/fuzz contracts.

### G4 — stateless actions — Fully done

**Fully implemented at the local source boundary.**

Ghostty-compatible configuration/raw input, exact primary clear behavior,
separate Automexia clear variants, extended selection, pane-footer local search,
bottom-centered visible-pane search, window/tab/split semantics, inherited
independent PTYs, geometric focus, logical resize, transactional zoom/equalize,
and restrictive bounded screen export/cleanup are implemented and discoverable
through the shared registry. The two search scopes have distinct typed actions,
deterministic visible-route traversal, 4 KiB query and existing scrollback
bounds, full-surface pointer capture, responsive geometry snapshots, and
feature-gated native state controls without query disclosure.

### G5 — tooling and release verification — Partially done

**Partially implemented:** controlled native, accessibility, resource, long-campaign, and release evidence remain external.

Pre-GUI action/keybinding/explain/JSON CLI, dry-run-first migration, host-
independent xtask generation/verification, generated fixture/reference
comparison, two nightly fuzz targets, properties, Criterion coverage, a
mutation-tested repository policy, and strict private exact-commit native-
evidence validation/QA summary wiring are implemented. Same-host Windows x64
timing was observed on 2026-08-23. Controlled Linux/BSD/macOS fixtures and
Windows/Linux/macOS keyboard, visual, assistive-technology, resource, lifecycle,
and package matrices plus an activated like-hardware 30-day baseline remain
release prerequisites.

### G6 — inspector and topology history — Partially done

**Partially implemented:** individual split, local-tab, and native-window history plus native lifecycle evidence remain external.

Accepted ADR 0027 protects a bounded renderer-owned modal inspector that
excludes terminal output, clipboard, environment, commands, paths, credentials,
and parked entry identifiers/content. It now shows redacted active/parked counts
and newest-first session/history/TTL summaries, offers pointer/keyboard newest
restore, and requires two-step confirmation before clearing all parked entries.
Every modal key press/release is consumed before PTY encoding, and the feature-
gated native snapshot exposes only redacted accessibility state. Accepted ADR
0028 keeps complete-top-level-tab PTYs memory-only and bounded to 8 entries,
5 minutes, and 250,000 retained history lines per window with redo invalidation
and owner-driven cleanup. Individual split, pane-local-tab, whole-native-window
history, and controlled native lifecycle/resource/visual/AT evidence remain
outside the activated scope.

## Version milestone assessment

### v0.4 stable

**Source substantially implemented; stable release blocked.**

Core rebrand/migration, contributor workflow, prompt/resize/UI/input/shell/image
features, policy, security bounds, packaging definitions, and Windows evidence
exist. Publication still requires:

- final editable brand assets and redistribution rights;
- private conduct-reporting contact;
- Windows Authenticode and Apple Developer/notarization credentials;
- plan-supported protected branch/tag rules, at least three eligible human
  reviewers, expanded CODEOWNERS, exact-head reviews and successful hosted jobs;
- public private-vulnerability reporting plus available secret scanning and push
  protection; DCO, least-authority Actions, Dependabot, and immutable releases
  are already enabled and remain continuously audited;
- signed/notarized clean install/upgrade/uninstall/coexistence evidence;
- Linux/macOS GPU/PTY/visual/accessibility and native alternate hardware;
- elevated AppVerifier/WPR and the 30-day performance baseline;
- exact annotated tag/default-branch/fork/DCO source provenance and an
  authenticated all-pass repository audit are now release-blocking; the fork
  tag is published, while controlled workflows have not reached `main` and six post-fork merge commits and the
  recorded old DCO exception remain unresolved.

Unsigned/nightly artifacts are not release candidates.

### v0.5.0 foundation and SSH

**Foundation partial; release not implemented.** D1/D2, D3 review model, D4,
CP0/CP1, and CP2.0/CP2.1 are real. D0 acceptance, D3 activation, D5,
AccessKit, native SSH gates, and v0.4 gates remain. Managed SSH, Connection Hub,
and user-facing persistent actions/aliases must not be claimed.

### v0.5.1 multi-cloud

**Not implemented.** D6 capsules/auth/providers/transports and mixed-provider
native/resource evidence remain.

### v0.6 ecosystem and selected-input model suggestions

**Fully implemented locally at the accepted nonactivating source boundary; partially implemented overall.** D7/CP6 has private policy/runtime crates, strict
signed local bundle verification, protected atomic disabled storage, versioned
custom WIT, optional no-default-WASI Wasmtime conformance, exact grants and
lifecycle, non-executing signed action packs, selected-input consent/response,
renderer-neutral review, a hard-denial app adapter, mutation/property/fuzz source
and benchmarks. Public distribution/SDK, component/provider activation, actual
native product UX, three-platform signed-package assurance, controlled security
campaigns, 1,000 cycles and 30-day evidence remain open release gates.

LO0 is separately partial only as documentation/research planning: ADR 0033,
the canonical architecture and future evidence ladder exist, while its machine
contract and acceptance remain open. LO1-LO5 implementation, model requests,
workflow execution, product UI, native/provider/privacy/resource/package evidence
and every new authority remain not implemented. LO is not a Studio or video
prerequisite.

Automation Studio AS0 is also partial only as architecture/research planning:
ADR 0030 and the future evidence ledger exist, while the dependency/native-host
decision and numeric contract remain unaccepted. AS1-AS6 implementation,
product activation, native UI/IME/accessibility/resource/package evidence, and
every file/process/provider/remote/debug authority remain not implemented.
AS0 may continue as non-production feasibility work before v0.4 release; AS1-
AS2 follow the stable terminal, and an evidenced AS2 minimal Studio precedes a
dedicated video-editing release. No current video runtime is implied.

## Cross-cutting quality assessment

| Dimension | Status | Strong current evidence | Remaining |
|---|---|---|---|
| Correctness | **Strong, incomplete globally** | Locked metadata, fmt, warning-denied Clippy, Nextest/doctests, conformance, migration, shell, PTY, UI, property, Windows native, architecture. | Hosted three-OS protected commit and tests for future phases. |
| Security | **Strong boundary; release partial** | Default deny, exact argv/capabilities, no shell evaluation, one bounded app runner, guarded spawn/publication seam, approval UI, no-follow, private permissions, redaction, cargo-deny, and hosted scanners/SBOM policy. | Protected exact-head approvals/server enforcement, real attestation, native cleanup/OpenSSH campaigns, signing, vet governance, and future sandbox. |
| Performance | **Source enforcement complete; controlled evidence collecting** | Fast paths, bounded queues/caches, cancellation, Criterion, deadlines, Windows budgets, and S2's exact 5%/10% fail-closed ratchet/activation workflow. | Named hardware for 30 consecutive days, full controlled matrix, independent review, and an approved active baseline. |
| Resource/storage | **Good focused; controlled partial** | PTY/worker/image/store cycles, descendant cleanup, cache/artifact limits, isolated targets. | Long soak, AppVerifier/WPR, Linux/macOS GPU/process, crash/power loss, future lifecycle tests. |
| Resilience/retry | **Good fail-closed design** | Last-known-good, CAS, reconciliation, stale rejection, coalescing, cancellation, rollback, no silent flaky retries. | SSH/provider reconnect states, UI recovery, cross-platform faults, power loss, future profile/sequence recovery. |
| Architecture | **Strong current boundary** | Private provider-neutral crates, GPU/PTY separation, one launch descriptor, exact allowlists, denied authority. | Deferred app/engine cleanup; preserve rules in future phases. |
| Code quality | **Machine-enforced** | fmt, Clippy, lock, dependency/license/source policy, mutation checks, bounded typed APIs/errors. | Scoped code mutation, unused-dependency review, future implementation reviews. |
| Documentation | **Broad and checked** | ADRs, guides, references, testing, threats, roadmaps, release/support, registries, link/anchor checks. | Generated future keybinding docs and product guides when UI exists. |
| Accessibility | **Partial** | Keyboard/focus/contrast/scaling/non-color and future model. | AccessKit tree and native assistive-technology evidence. |
| Visual quality | **Structural strong; release partial** | Responsive geometry, snapshots, Windows painted frame, icon/font/contrast. | Controlled raster diffs and human review on all desktop stacks. |
| Supply chain | **Definitions strong; delivery blocked** | Pinned Actions, deny, review, CodeQL, SBOM/checksum/attestation/reproducibility/signing workflows. | Credentials, rights, hosted execution, repository policy, signed native install. |

## Platform assurance

| Platform | Current proof | Limitation |
|---|---|---|
| Windows x86_64 | Strong native unit/integration/ConPTY/GUI/resource/PowerShell/CMD/WSL/package evidence. | Signing, elevated AppVerifier/WPR, final clean install remain external. |
| Windows ARM64 | Cross-check/package workflow. | Native hardware smoke remains. |
| Linux x86_64 | Hosted contract, Ubuntu/WSL Unix tests, Bash/Zsh/Fish, PTY, X11/Wayland builds, DEB/RPM checks. | Native controlled GPU/visual/screen reader missing; not every distro certified. |
| Linux ARM64 | Artifact/cross workflow. | Native hardware/package/runtime smoke remains. |
| macOS x64/ARM64 | Native job and both architecture/package/notarization contracts; library cross-check. | No local controlled GUI/PTY/VoiceOver/notarized-install claim. |
| BSD/Unix | Portable Unix contracts where supported. | No hosted BSD runner or release artifact. |

A workflow definition proves coverage intent, not execution. “Works on all OS”
is feature-specific and requires its declared native matrix.

## Test and benchmark tiers

### Pull requests

Implemented for current features: metadata/lock/fmt/formats/docs/identity/
provenance/architecture/package checks; warning-denied Clippy; Nextest/JUnit;
doctests; shell and native platform jobs; X11/Wayland; alternate architecture;
conformance/resize/session/layout/property; deny/dependency/CodeQL definitions;
coverage; DCO/protected paths; all benchmark/fuzz ownership.

### Nightly and deep

Defined: fuzz, ASan/TSan, Miri, Loom, controlled Criterion, Windows deep GUI/
ConPTY/AppVerifier/WPR, WSL fuzz/native, unsigned packages, and extended
resource/image/PTY/worker/inventory/shell campaigns.

Remaining maturity: longer corpora, controlled named hosts, ratified baselines,
and retained platform-qualified artifacts.

### Release

Incomplete: signed MSI/ZIP, notarized DMG, clean DEB/RPM/tar installs, checksum/
SBOM/attestation/reproducibility, migration/coexistence/URL/desktop/terminfo/
GPU/PTY/accessibility/uninstall evidence for the exact protected commit, plus
manual visual/hardware review.

## Retry, recovery, and failure policy

Current preferred patterns:

- tests: retries never hide flakes; timeout kills the owned process tree/group;
- config: validate then atomic swap, otherwise keep complete last-known-good;
- providers/cache: bounded queue, newest generation, cancel/discard stale,
  preserve truthful state and freshness;
- persistence: bounded private read, lock/CAS, durable replacement, explicit
  previous recovery, reconciliation;
- processes: deadline, output cap, identity revalidation, descendant cleanup,
  no detached capture threads;
- release: skipped/unsupported/external remains explicit, never passed.

Future network/provider work must also define eligible failures, bounded
attempts/time, backoff/jitter, idempotency, cancellation, offline state,
revocation, and nonretryable security denials such as changed host keys,
malformed input, expired grants, and policy failure.

## Recommended execution order

The maintained checklist and dependencies are in the
[connectivity and command-productivity focus roadmap](CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md).
At this audited baseline, the focused order is:

1. Preserve accepted ADR 0012 and obtain ADR 0003's two independent exact-head
   approvals plus non-bypassable server enforcement. Bind real package
   attestation/revocation while production launch remains disabled.
2. Preserve the locally complete D5.1 read-only Hub and obtain its external
   native/accessibility evidence. Preserve D5.2's nonactivated approval UI,
   application runner, guarded PTY, and exact route-publication path.
3. Activate D3 only after its protected, attestation, native cleanup proof, and three-OS
   native gates pass. Preserve the locally complete nonactivated F5.1-F5.3
   routes/trust/tunnel/lifecycle source, then collect and validate F5.4's
   controlled Windows/macOS/Linux real OpenSSH evidence without treating the
   synthetic fixture as release proof.

4. Complete connection profiles, typed recipes, and declarative remote
   workspaces before adding provider execution.
5. Preserve the complete local D6.0 boundary and implement each D6.1-D6.5 provider independently through official
   CLI/auth authorities and isolated immutable capsules.
6. Preserve the product-integrated nonactivated CP4 boundary; activate provider refresh or exact execution only after D3/D5/D6 expose approved product context and native evidence.
7. Preserve the accepted CP5.1-CP5.6 source boundary: CP5.1-CP5.4 are complete
   at their source/local model boundaries, CP5.5 is complete as an inert bridge
   source, CP5.6 and live product composition remain partial, activation
   stays false, and CP1 remains the default until every native release gate passes.
8. Keep v0.4 external release evidence and G0-G6 compatibility work as
   independent evidence tracks; defer D7/CP6/G6 until their protected designs
   pass.
9. After the stable v0.4 release, close DN0 through explicit ADR/limit/test
   acceptance, then implement DN1-DN4 independently in test-first bounded
   slices. Keep DN5 evidence-driven and DN6 blocked on its separate privacy and
   extension-capability decision.
10. Preserve PO0 as a non-activating proposal/checker only until ADR/exact-
    digest, owner, dependency, capability, prototype, threat and first failing
    real-path-test acceptance. Implement
    PO1-PO5 after applicable D6/CP5 read-only gates, PO6 only after D3/provider
    activation, PO7 only after the ecosystem decision, and PO8 per independent
    adapter/release slice. Keep PO2 read-only and PO5 memory-only by default;
    gate every PO6 operation or diagnostic session independently. Keep CP1
    available and all production changes human-reviewed.
11. Preserve LO0 as documentation only until explicit ADR and machine-contract
    acceptance. Then prove LO1's neutral I/O-free workflow model before any
    provider or plan runtime. Keep LO2-LO5 independent of Studio and video and
    retain a complete model-free fallback for every domain.

## Final assessment

Automexia is beyond a prototype in terminal correctness, UI behavior, bounded
input, provider-neutral architecture, native completion, and nonactivated
DevOps foundations. It has substantive tests, benchmarks, fuzz ownership,
security controls, resource limits, documentation, and platform policy.

The full roadmap is not complete. Stable v0.4 evidence is incomplete;
production SSH/Connection Hub, remote workspaces, multi-cloud/provider-action
activation, Automexia-owned suggestions, full Ghostty compatibility, public
extensions, and optional LLM orchestration remain.

> Current milestone: v0.4 source stabilization, D1/D2, disabled D4, the complete
> local non-executing D5.0/F2 model boundary, CP0-CP3.3, and the local D0/D3
> package/resolution/current-review/lifecycle contract are implemented at their stated local/
> source boundaries. D0/D3/D5.0 remain partial because protected acceptance,
> production authority, real loader binding, and native execution evidence are
> open. Stable release proof, D5.1-D6/CP4/CP5 activation, and the other phases
> remain partial or not implemented as listed above.

Semantic diagnostic navigation remains unimplemented beyond DN0's documentation-
only proposal boundary. Existing prompt navigation and command-result metadata
are reusable foundations, not evidence of failed-command or error-section
product behavior. LLM orchestration likewise remains unimplemented beyond LO0's
documentation-only proposal; existing actions and brokers are foundations, not a
model, registry, plan or workflow-runtime claim. Situation-aware production
operations likewise remains unimplemented beyond PO0's documentation-only
proposal; current context, provider caches, Quick Actions, CP5 and D3 do not
constitute a production lock, change/ownership/drift view, resource explainer,
cohort/environment comparison, network or SLO diagnosis, evidence graph,
recommendation, preflight, Incident Mode, live-log/time/handoff controller,
managed operation/diagnostic session, journal, runbook or execution claim.
