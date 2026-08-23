# Phase implementation audit

Audit date: 2026-08-22

Audited source baseline: 3f7c00ed8b7d0dee37853654e2db08cd3b324572 plus the current M3 UX/evidence re-audit

Scope: every execution phase defined by the product, stabilization, DevOps/SSH,
Connection Hub, command-productivity, persistent-alias, and Ghostty
compatibility roadmaps.

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
- [Readiness audit](READINESS-AUDIT.md)
- [SSH, DevOps, and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md)
- [Connection Hub](CONNECTION-HUB.md)
- [Command Productivity](COMMAND-PRODUCTIVITY.md)
- [DevOps Quick Actions and persistent aliases](DEVOPS-ALIASES.md)
- [Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md)

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
| Feature assurance | Passed: 27 feature-assurance entries, 276 documented source/evidence entries, and 46 phase-audit entries. |
| Platform coverage policy | Passed: Windows/Linux/macOS, PowerShell/CMD/Unix shells, X11/Wayland, alternate architectures, nightly artifacts, deep Windows/WSL jobs, and release validators are machine-enforced. |
| Documentation coverage | Passed: 12 public pages, 172 configuration keys, 73 binding actions, 6 application flags, and 21 xtask commands. |
| Repository validation | Passed: 42 TOML, 14 YAML, 26 JSON, 6 XML, one desktop file, 210 Markdown files, 69 pinned Actions, release trust, assurance, and roadmap policy contracts. |

The hosted GitHub run state was not independently queried because GitHub CLI is
not installed on this host. The latest readiness record says hosted jobs were
blocked before checkout by the account Actions billing/spending state. Treat
that condition as **unverified and unresolved for release** until this exact
protected commit passes GitHub-hosted Windows, Linux, and macOS jobs.

## Executive phase matrix

| Track | Phase | Implementation | Release evidence | Conclusion |
|---|---|---|---|---|
| Core | v0.4/S0 | **Fully implemented locally** | **Partial** | Identity, hostile-input bounds, atomic reload, and current terminal source gates are complete; stable release gates remain. |
| Assurance | v0.4/S1 | **Partial** | **Partial** | Strong deterministic, Windows-native, QA, fuzz, resource, and workflow machinery exists; controlled Linux/macOS visual/GPU/accessibility evidence and the full baseline remain. |
| Performance | S2 | **Not implemented** | **Not started** | The 30-day comparable baseline is incomplete, so the 5% latency/10% memory ratchet is inactive. |
| DevOps | D0 | **Partial** | **Partial** | ADR 0012 is accepted and active schema 5 ratchets immutable schemas 1/2/3/4 with exact M3-M5 route/tunnel, trust/status, lifecycle/receipt/reconnect, and native-manifest rules; protected approvals and real native execution remain. |
| DevOps | D1 | **Fully implemented** | **Partial** | Four private provider-neutral crates and bounded contracts satisfy their source boundary. |
| DevOps | D2 | **Fully implemented** | **Partial** | Generic status, immutable history, capsule/cache/session isolation, cancellation, and truthful freshness exist. |
| DevOps | D3 | **Partial; nonactivated** | **Blocked** | The fail-closed broker/runner now binds exact managed argv and current executable identity, reconciles actual child status, emits redacted outcomes, and queues bounded receipts through the guarded PTY/route owner. Protected approvals, real attestation/activation, forced descendant proof, and native OpenSSH/resource/accessibility evidence remain. |
| DevOps | D4 | **Fully implemented; read-only product adapter active** | **Partial** | Bounded OpenSSH inventory/persistence is connected only to D5.1 reviewed browsing; it retains no process/network/launch authority. |
| SSH UX | D5.0-D5.2 | **Partial overall; D5.1 and nonactivated F5.1-F5.3 source complete locally** | **Partial/blocked** | Typed direct/config-jump/tunnel argv, loopback defaults, strong tunnel review, complete trust evidence, guarded lifecycle, receipts, reconnect, and compact tunnel states pass locally. Protected activation/attestation, actual status/SSH execution, forced cleanup, and controlled real native F5.4 evidence remain. |
| Multi-environment | M6/F6 | **Partial overall; review-only source contracts complete locally** | **Blocked** | Schema-2 reviewed library editing/migration/transfer, exact dependent fingerprints, typed recipe lifecycle/no-hooks, narrow remote initialization, declarative restore, armed broadcast, semantic UI models, fuzz/mutation, and bounded benchmarks pass. Proposed ADR 0023 acceptance, product activation/execution, and controlled native/resource/accessibility evidence remain. |
| Multi-cloud | D6.0/M7 | **Fully implemented locally** | **Partial/external** | Strict provider-neutral schemas, immutable session capsules, 19-state lifecycle, exact allow-once review, isolation/redaction, passive-status migration, cached-only semantics, fuzz/mutation, 16×64 lifecycle, and a 64-capsule benchmark pass. No real provider CLI, cloud network, credential cache, browser/device flow, or Linux/macOS native provider evidence ran. |
| Multi-cloud | D6.1/M8 | **Partial overall; source-complete nonactivated** | **Blocked** | Independent bounded AWS profile parser plus exact SSO/STS/SSM/EKS-dry-run contracts and ten focused tests pass. D3 activation, M11 EKS ingestion, product UI, and controlled native/provider/resource/accessibility evidence remain. |
| Multi-cloud | D6.2/M9 | **Partial overall; source-complete nonactivated** | **Blocked** | Independent bounded Azure public-account parser plus exact tenant login/account, AAD-only Bastion, opaque transient AKS contracts, and eight focused tests pass. D3 activation, M11 AKS ingestion, product UI, and controlled native/provider/resource/accessibility evidence remain. |
| Multi-cloud | D6.3/M10 | **Partial overall; source-complete nonactivated** | **Blocked** | Independent bounded named gcloud parser plus exact per-command user/project, opaque federation, scope-bound IAP, private-environment GKE contracts, eight focused tests, and near-limit benchmark pass. D3 activation, M11 GKE ingestion, product UI, and controlled native/provider evidence remain. |
| Multi-cloud | D6.4-D6.5 | **Not implemented** | **Blocked** | Kubernetes/OpenShift, Teleport, and OpenBao adapters, product login UI, and controlled native/provider evidence remain independently gated; OpenBao additionally requires ADR 0024 acceptance. |
| Ecosystem | D7 | **Not implemented; deferred** | **Blocked by design** | Public SDK/downloads, sandboxing, direct APIs, and AI execution wait for v0.6 gates. |
| Productivity | CP0 | **Fully implemented** | **Partial** | Accepted architecture, threat model, ceilings, fixtures, mutations, and nonactivation policy exist. |
| Productivity | CP1 | **Fully implemented** | **Partial** | Shell-native completion and bounded explicit refresh exist; hosted native evidence remains. |
| Productivity | CP2.0 | **Fully implemented** | **Partial** | Bounded typed Quick Action model and hostile corpus exist with no runtime authority. |
| Productivity | CP2.1 | **Fully implemented as internal library** | **Partial** | Private atomic persistence, CAS, recovery, watches, and benchmarks exist; no startup/UI activation. |
| Productivity | CP2.2 | **Implemented locally** | **Partial** | Layered search, placeholder/risk/conflict review, bounded import/export/CRUD/recovery, and explicit insert/copy UI are present. Hosted native shells, controlled screen readers, and 30-day performance/resource evidence remain release gates. |
| Productivity | CP3.0 | **Fully implemented at pure boundary** | **Partial** | Five pure serializers, bounded inventories, metadata/tamper verification, tests, fuzz, benchmark, and policy ratchets are complete; activation is disabled and hosted native evidence remains. |
| Productivity | CP3.1 | **Fully implemented locally** | **Partial** | Explicit opt-in persistence, crash-safe all-old/all-new publication, verified five-shell startup/reload, diagnostics, rollback, and exact uninstall are implemented; hosted native/macOS/WSL and controlled-baseline evidence remains. |
| Productivity | CP3.2 | **Fully implemented locally** | **Partial** | Eleven static provider packs, 33 disabled-by-default actions, health/update/alias-safety, CLI, tests, fuzz, benchmarks, and policy gates are complete; hosted evidence remains. |
| Productivity | CP3.3 | **Fully implemented locally** | **Partial** | Six explicit native inventory formats, exact just/Task/mise bridges, bounded dry-run/CAS import/workspace/trust/revocation/removal, path-free digest/revision receipts, background cache authorization, final insertion recheck, tests, mutation, fuzz, benchmark, ADR, CLI, and docs are complete; hosted native/accessibility and 30-day evidence remain. |
| Productivity | CP4 | **Not implemented** | **Blocked** | Requires activated D3 and D5/D6 capsule/provider context. |
| Productivity | CP5.0 | **Fully implemented at research boundary** | **Complete locally** | Seven-shell API matrix, pure insertion prototype, locked matcher/dependency evidence, privacy review, and retain-CP1/defer-P2 decision are machine-gated; no runtime surface exists. |
| Productivity | CP5.1-CP5.6 | **Not implemented** | **Not started** | A separate accepted bridge ADR plus protocol, sources, ranking, UI, shell activation, and release proof are required; CP1 remains fallback. |
| Productivity | CP6 | **Not implemented; deferred** | **Blocked by design** | Signed ecosystem packs and AI tools require v0.6 gates. |
| Compatibility | G0 | **Partial** | **Partial** | Shared safety prerequisites pass; versioned Ghostty fixtures, generation, checksums, and replacement ADR are absent. |
| Compatibility | G1 | **Not implemented** | **Not started** | No private typed/compiled keybinding registry exists. |
| Compatibility | G2 | **Partial** | **Not started for profile release** | Generic last-known-good reload exists; profiles, unbind layers, migration, and profile compiler do not. |
| Compatibility | G3 | **Partial** | **Not started for profile release** | Some fallthrough behavior exists; structured outcomes, sequences, tables, and chains do not. |
| Compatibility | G4 | **Partial** | **Partial** | Some actions exist; clear variants, extended selection/search, zoom/equalize, and screen export remain. |
| Compatibility | G5 | **Not implemented** | **Not started** | Generated profiles, CLI/migration, xtask generation, compatibility fuzz, and registry benchmarks are absent. |
| Compatibility | G6 | **Not implemented; deferred** | **Blocked by design** | Inspector and parked-PTY undo/redo need separate ADR/security/resource design. |

### Architecture Phase 0-5 mapping

The architecture document uses broader Phase 0-5 names. They map to the
executable ledgers above as follows:

| Architecture phase | Executable phase mapping | Status |
|---|---|---|
| Phase 0: v0.4 security/stability | S0, S1, S2 | **Source gates complete; assurance partial; enforcement not started.** |
| Phase 1: provider-neutral APIs | D1, D2 | **Fully implemented at source boundary.** |
| Phase 2: production first-party SSH | D0, D3, D4, D5.0-D5.2 | **Partial foundation only; production SSH not implemented.** |
| Phase 3: provider auth/capsules | D6.0-D6.4 | **Partially implemented overall: D6.0 is fully implemented locally; D6.1 is source-complete nonactivated; D6.2-D6.4 are not implemented.** |
| Phase 4: lazy inventory/infrastructure/enterprise adapters | Later D6/D6.5 work | **Not implemented.** |
| Phase 5: third-party ecosystem/AI | D7 and CP6 | **Not implemented and deferred.** |

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

**Partially implemented.**

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

**Partially implemented.**

Implemented:

- Windows ceilings for handles, threads, private bytes, working set,
  descendants, PTYs, routes, image resources, and teardown;
- safe exact-binary Application Verifier/WPR wrappers with finally cleanup and
  private ETL exclusion;
- worker restart/saturation/shutdown, PTY lifecycle, image WGPU/CPU
  open/dismiss, cache eviction, file release, and no-sidecar tests;
- separate ASan, TSan, Miri, Loom, fuzz, and native responsibilities.

Missing:

- elevated AppVerifier Basics/Heaps/Handles/Locks and reviewed WPR evidence;
- named Intel/AMD/NVIDIA, RDP/software, Linux, and macOS hardware runs;
- long process/GPU/resource soak evidence.

### S1.3 — visual quality and frame regression

**Partially implemented.**

Implemented: renderer-neutral geometry/state, structured snapshots, contrast,
hit-target/modal/cursor/footer/path/pane invariants, narrow-to-8K layout cases,
and a topmost Windows client capture that waits for presentation, rejects
blank/single-color output, restores z-order, and excludes live frames from
portable QA.

Missing: pinned expected/actual/diff goldens across theme/font/scale/layout/UI
states, a reviewed tolerance baseline, Linux/macOS frames, and recorded human
aesthetic review.

### S1.4 — accessibility

**Partially implemented.**

Implemented: keyboard operation, focus visibility/order, non-color identity,
contrast, hidden targets, scaling, labels, reduced-motion requirements,
inventory/manual matrix, generic status summaries, and ADR 0013.

Missing: recorded Narrator/NVDA, VoiceOver, and Orca evidence plus the v0.5
AccessKit semantic tree and controlled 200% scale checks for future UI.

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

Implemented: shrinking layout properties and persisted regressions, fixed-seed
storms, finite Loom models, nightly sanitizers/Miri/fuzz ownership, global plus
80%-changed-owned-line coverage policy, and mutation tests for platform,
feature, productivity, release, documentation, and CI drift.

Missing: pure resize-queue and atomic snapshot-replacement model expansion,
long weekly fuzz/corpus trends, owned region/branch baseline, scoped
cargo-mutants survivor triage, and governed cargo-vet adoption.

### S1.8 and S2 — performance proof and enforcement

**Measurement partial; S2 not implemented.**

- Eight assurance targets cover application/DevOps services, image preview,
  Quick Action parsing/store, PTY I/O, event polling, OpenSSH inventory, and VT.
- Local smoke proves harness execution; workflow policy distinguishes
  compilation from actual execution.
- Security/resource ceilings already apply where statistical baselines are not
  sufficient.

Missing: a complete 30-day same-runner baseline for startup/input/prompt/
context/listing/parser/renderer/resize/memory/workers/images/actions/PTY/build
storage, retained comparisons, and active >5% latency or >10% memory waivers.

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
native graceful-then-forced descendant cleanup, hostile real OpenSSH/PTY/server
fixtures, controlled screen-reader/pixels, and 1/10/50-session resource evidence
remain. The linked package remains unverified and the activation constant
remains false, so no production child can start.

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

**Partially implemented overall; F5.1-F5.3 are fully implemented at the nonactivated source boundary; F5.4 real native evidence remains external.**

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
validator and synthetic mutation fixture. Deterministic exact-argv/hostile/
collision/staleness/decision/UI/1/10/50 cleanup tests pass. The synthetic
fixture cannot satisfy a release; the explicit local probe found OpenSSH client
9.5 but no `sshd` on this Windows host and truthfully returned an external
prerequisite without installing or changing anything.

Still external or protected: ADR 0003's two independent exact-head approvals and
server enforcement, real loader/build attestation and revocation, product
controller consumption of a current attested executable observation, a real
managed OpenSSH child, graceful-then-forced descendant cleanup, prompt/
diagnostic and manual-SSH regression, controlled pixels/screen readers, and
native Windows/macOS/Linux plus separately gated WSL 1/10/50 resource evidence.
Actual public status execution and production activation/native proof remain
external. F5.3 tunnels are fully done locally and nonactivated; F5.4 still needs
validated real Windows/macOS/Linux OpenSSH, forced cleanup, 1/10/50 resource,
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

Focused Windows x86_64 evidence passed 9 planner, 6 automation, 10 workspace, 15
Hub-model, and 10 Connection Library tests plus warning-denied focused Clippy,
architecture/mutation policy, fuzz entry-point compilation ownership, and a
1,000-generation × 50-target bounded-state test. Criterion measured the maximum
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

Proposed ADR 0023 acceptance, public editor/controller/renderer/CLI integration,
managed execution, real OpenSSH/PTY/process/handle/socket cleanup, native
Windows/macOS/Linux, controlled screen-reader/visual, hosted policy, and release
evidence remain. M6 activation stays false and CP3.3 local workspace tasks gain
no remote/provider authority.
### D6.0-D6.5 — providers and multi-cloud

**Partially implemented overall: D6.0/M7 is fully implemented locally; D6.1/M8 through D6.3/M10 are source-complete and nonactivated; D6.4-D6.5 are not implemented.**

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
produces only EKS `--dry-run` output for later M11 ingestion. Ten focused tests
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

The remaining provider slices are still not implemented:

1. D6.4 Kubernetes/OpenShift trusted sources, exec allowlists, and contexts.
2. D6.5 independently enabled Teleport, then OpenBao after proposed ADR 0024 is accepted.

D6.1-D6.3 still need D3 product activation/attestation, M11 EKS/AKS/GKE
ingestion, product review UI, controlled real-tool native tests, cleanup/
resource/accessibility, and release evidence. D6.4-D6.5 still need independent
operation construction, grants, parsing, lifecycle, exact CLI argv, offline/
expired/denied behavior, redaction, product UI, controlled real-tool native
tests, and multi-pane isolation. Direct SDK inventory remains a later, explicit,
lazy authority.

### D7 — public ecosystem, direct APIs, and AI

**Not implemented and deliberately deferred.** Public signing/revocation,
compatibility, sandboxing, quotas, migration, capability UX,
malicious-package/supply-chain tests, and exact data-flow controls must precede
it. AI receives no ambient PTY/history/agent/credential/cloud/capsule/process/
filesystem/network authority.

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

**Not implemented and dependency-blocked.** Requires D3/D5/D6, cached public
context only, freshness, explicit refresh, production risk, exact brokered
launch, isolation, cancellation/revocation/offline/expiry/redacted audit, and
multi-provider native/resource tests.

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

**Not implemented; detailed specification exists.**

| Phase | Required work |
|---|---|
| CP5.1 | Separate ADR; opt-in local pipe/socket, restrictive permissions, peer/session capability, bounded versioned framing, buffer/cursor/span/generation, replay/isolation/cleanup/native fallback. |
| CP5.2 | Native results, opt-in shell history, cwd/executables, frequency, cached public providers, and typed actions; no network/auth/secrets/history-file/remote-output/per-key process. |
| CP5.3 | Deterministic ranking, bounded cache/queue, stale cancellation, Unicode/graphemes, shell-returned spans/escaping, rapid-typing properties, Criterion gates. |
| CP5.4 | Pane-owned accessible listbox avoiding cursor/IME/footer/tabs/siblings/modals; type/source/freshness/risk; tiny-to-8K and 100-300% goldens; impossible-size fallback. |
| CP5.5 | Version-gated opt-in shell adapters, truthful CMD fallback, and preservation of profiles, bindings, completers, predictors, history, and native UI. |
| CP5.6 | Preview flag, kill switch, LKG/reset/disable/uninstall, three-OS native, fuzz/leak/resize/multi-pane/accessibility, and 30-day baseline. |

CP1 remains the fallback. A popup alone cannot satisfy privacy, insertion,
accessibility, lifecycle, performance, or rollback gates.

### CP6 — ecosystem packs and AI

**Not implemented and deferred.** Requires v0.6 sandbox, provenance,
signature/revocation, quotas, malicious-package, privacy, and least-authority
gates.

## Terminal-first remote operations projection

The canonical product experience is now specified in
[Terminal-first remote operations](TERMINAL-FIRST-OPERATIONS.md) and accepted
for the v0.5+ roadmap by
[ADR 0018](adr/0018-terminal-first-remote-operations.md). It does not introduce
another phase or capability system: D3-D7 own remote/process/provider/file/
collaboration authority, while CP2-CP6 own actions, aliases, completion,
suggestions, packs, and AI-assisted insertion.

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
| Quick Actions, aliases, lifecycle hooks, reviewed multi-target execution | CP2.2-CP4/D5E | CP2/CP3 actions are local/insert-only; M6 typed recipe lifecycle and armed broadcast review are source-complete, but multi-target execution remains disabled |
| Declarative workspace persistence/restoration and visibly armed broadcast | M6/F6 | Review-only schema-2 persistence, restore, armed broadcast, semantic projection, and bounded tests are complete locally; product controller/renderer/execution and native evidence remain |
| Per-pane multi-cloud/context commands and typed import/reconcile adapters | D6.0-D6.5/CP4 | Partially implemented: D6.0 immutable provider-context/capsule/auth lifecycle and review contracts are complete locally; all provider commands, import/reconcile adapters, product flows, and CP4 remain not implemented |
| Structured files/SFTP, bounded logs/bookmarks, team state/policy, shared sessions | D7 protected slices | Not implemented |
| Mosh, Telnet, serial, public ecosystem packs, optional editor popup, AI explain/suggest | D7/CP5-CP6 | Not implemented/deferred |

Before any row changes to shipped, it needs the exact CLI/configuration/
keyboard reference, feature-ledger entry, typed resource ceilings, threat and
capability review, deterministic model/property/hostile tests, native Windows/
Linux/macOS and applicable shell/provider evidence, accessibility/responsive
goldens, benchmarks, long-run leak/cleanup proof, recovery/uninstall behavior,
and changelog. `ax` remains optional and cannot shadow native user state.

## Ghostty compatibility phases

Automexia ships its classic shortcuts. It has no selectable Ghostty profile and
must not claim complete Ghostty action/keybinding-language parity.

### G0 — source lock and safety

**Partially implemented.**

Present: recorded audit commit, tested classic mappings, last-known-good reload,
Automexia identity, bounded control strings, Nextest/QA/property/fuzz
infrastructure, and a hand-maintained compatibility matrix.

Missing: verified Ghostty 1.3.1 tag generation; normalized Linux/macOS/
Windows-adapted/actions/provenance/deviations fixtures; checksums/generator;
Automexia default golden; replacement ADR; transform Proptest; QA fixture
identity. CI must stay offline and never require Ghostty.

### G1 — typed registry

**Not implemented.** No automexia-keybindings crate, stable action schemas,
typed trigger/predicate/scope/origin model, direct/reverse lookup, trie/table
registry, or palette generation exists. Current bindings remain a flat vector
with some ad hoc string parsing.

### G2 — profiles, overrides, migration, atomic profile reload

**Partially implemented through generic reload only.** Missing explicit
automexia/ghostty/versioned profiles, moving/pinned rules, unbind/priority/
origin layers, strict defaults, complete diagnostics, immutable off-thread
compile/swap, migration dry-run/apply, and transactional global/palette update.

### G3 — outcomes, sequences, tables, and chains

**Partially implemented.** Existing copy-or-ETX, search/Vi ownership, some edge
fallthrough, and passthrough can be reused. Missing ActionOutcome/can_perform,
lower-priority/PTY fallthrough, all-surface snapshots, prefix byte/cancel rules,
table stack/one-shot/catch-all, chains, IME/AltGr/dead-key/Kitty handling,
per-surface pending state/UI, and fuzz/model proof.

### G4 — stateless actions

**Partially implemented.** Present: many frontend actions, config/raw input,
selection/search/Vi, directional/word selection, tab/window/pane/split/clone/
close/move, and geometric focus.

Missing: distinct clear variants; page/home/end/line selection; scroll/search
from selection; exact profile wiring; split resize/zoom/equalize; secure bounded
screen export/open/copy/paste-path with restrictive temp cleanup. Each action
needs unit/dispatch/binding/palette/CLI/generated-doc/security/platform/resource
tests.

### G5 — generated tooling and release verification

**Not implemented.** Missing generated profiles; list-actions/list-keybinds/
explain/collision/JSON CLI; safe migration; xtask generate/verify/test; generated
docs; fixture parity; compatibility fuzz/Proptest/Loom/native layouts/goldens/
resources; Criterion registry/sequence/table/reload/reverse-index benchmarks;
and the 30-day lookup/dispatch baseline.

### G6 — inspector and undo/redo

**Not implemented and deferred.** Inspector needs a redacted model excluding
secrets/clipboard/hidden/private output. Undo/redo needs bounded parked
independent PTYs, topology transactions, count/time/memory/scrollback ceilings,
cleanup, recovery, redo invalidation, and separate ADRs.

## Version milestone assessment

### v0.4 stable

**Source substantially implemented; stable release blocked.**

Core rebrand/migration, contributor workflow, prompt/resize/UI/input/shell/image
features, policy, security bounds, packaging definitions, and Windows evidence
exist. Publication still requires:

- final editable brand assets and redistribution rights;
- private conduct-reporting contact;
- Windows Authenticode and Apple Developer/notarization credentials;
- protected public repository/rules, reviews, DCO/CODEOWNERS, vulnerability
  reporting, immutable releases, and successful hosted jobs;
- signed/notarized clean install/upgrade/uninstall/coexistence evidence;
- Linux/macOS GPU/PTY/visual/accessibility and native alternate hardware;
- elevated AppVerifier/WPR and the 30-day performance baseline;
- intentional fork tag publication and resolution of the recorded old DCO
  exception.

Unsigned/nightly artifacts are not release candidates.

### v0.5.0 foundation and SSH

**Foundation partial; release not implemented.** D1/D2, D3 review model, D4,
CP0/CP1, and CP2.0/CP2.1 are real. D0 acceptance, D3 activation, D5,
AccessKit, native SSH gates, and v0.4 gates remain. Managed SSH, Connection Hub,
and user-facing persistent actions/aliases must not be claimed.

### v0.5.1 multi-cloud

**Not implemented.** D6 capsules/auth/providers/transports and mixed-provider
native/resource evidence remain.

### v0.6 ecosystem and AI

**Not implemented and deferred.** D7/CP6 public SDK/distribution/sandbox/
signing/revocation/quotas/policy/AI remain future work.

## Cross-cutting quality assessment

| Dimension | Status | Strong current evidence | Remaining |
|---|---|---|---|
| Correctness | **Strong, incomplete globally** | Locked metadata, fmt, warning-denied Clippy, Nextest/doctests, conformance, migration, shell, PTY, UI, property, Windows native, architecture. | Hosted three-OS protected commit and tests for future phases. |
| Security | **Strong boundary; release partial** | Default deny, exact argv/capabilities, no shell evaluation, one bounded app runner, guarded spawn/publication seam, approval UI, no-follow, private permissions, redaction, cargo-deny, and hosted scanners/SBOM policy. | Protected exact-head approvals/server enforcement, real attestation, native cleanup/OpenSSH campaigns, signing, vet governance, and future sandbox. |
| Performance | **Local measurement; enforcement incomplete** | Fast paths, bounded queues/caches, cancellation, Criterion, deadlines, Windows budgets. | Named hardware for 30 days, full matrix, approved baseline, S2. |
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
3. Activate D3 only after its protected, attestation, cleanup, and three-OS
   native gates pass. Preserve the locally complete nonactivated F5.1-F5.3
   routes/trust/tunnel/lifecycle source, then collect and validate F5.4's
   controlled Windows/macOS/Linux real OpenSSH evidence without treating the
   synthetic fixture as release proof.

4. Complete connection profiles, typed recipes, and declarative remote
   workspaces before adding provider execution.
5. Preserve the complete local D6.0 boundary and implement each D6.1-D6.5 provider independently through official
   CLI/auth authorities and isolated immutable capsules.
6. Implement CP4 only after D3/D5/D6 expose bounded cached public context.
7. Preserve the fully completed CP5.0 retain-CP1 decision. CP5.1-CP5.6 remain
   not started and may proceed only after a new proposal's separate bridge ADR
   and gates pass.
8. Keep v0.4 external release evidence and G0-G6 compatibility work as
   independent evidence tracks; defer D7/CP6/G6 until their protected designs
   pass.

## Final assessment

Automexia is beyond a prototype in terminal correctness, UI behavior, bounded
input, provider-neutral architecture, native completion, and nonactivated
DevOps foundations. It has substantive tests, benchmarks, fuzz ownership,
security controls, resource limits, documentation, and platform policy.

The full roadmap is not complete. Stable v0.4 evidence is incomplete;
production SSH/Connection Hub, remote workspaces, multi-cloud, provider-aware
actions, Automexia-owned suggestions, full Ghostty compatibility, public
extensions, and AI remain.

> Current milestone: v0.4 source stabilization, D1/D2, disabled D4, the complete
> local non-executing D5.0/F2 model boundary, CP0-CP3.3, and the local D0/D3
> package/resolution/fixture contract are implemented at their stated local/
> source boundaries. D0/D3/D5.0 remain partial because protected acceptance,
> production authority, real loader binding, and native execution evidence are
> open. Stable release proof, D5.1-D6/CP4/CP5 activation, and the other phases
> remain partial or not implemented as listed above.
