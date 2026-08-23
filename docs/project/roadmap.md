# Roadmap

This page is a condensed project overview retained from the documentation
reorganization. The [canonical roadmap](../ROADMAP.md) and
[phase implementation audit](../PHASE-IMPLEMENTATION-AUDIT.md) are the
machine-enforced sources for current sequencing and phase status. This summary
must not override them. Detailed test commands and evidence live in
[Testing](../TESTING.md).

A phase can be source-complete while release evidence is still partial. User-facing guides remain authoritative for what users can rely on today.

## Executive phase matrix

| Track | Phase | Implementation | Release evidence | Conclusion |
|---|---|---|---|---|
| Core | v0.4/S0 | **Fully implemented locally** | **Partial** | Identity, hostile-input bounds, atomic reload, and current terminal source gates are complete; stable release gates remain. |
| Assurance | v0.4/S1 | **Partial** | **Partial** | Deterministic and Windows-native assurance now includes resize-queue properties, generation-aware atomic snapshots, and a bounded reviewed visual comparator; controlled Linux/macOS GPU/visual/accessibility evidence and approved golden matrices remain. |
| Performance | S2 | **Partially implemented; collecting** | **External baseline pending** | Strict comparable evidence, native memory composition, baseline/waiver validation, 90-day retention, and fail-closed release enforcement exist; activation still requires 30 reviewed consecutive controlled-runner days. |
| DevOps | D0 | **Partial** | **Partial** | ADR 0012 is accepted; schema 5 ratchets immutable schemas 1/2/3/4 with exact M3-M5 routes/tunnels, trust/status, tunnel lifecycle, receipt/reconnect, and native-manifest rules. Protected approvals and real native execution remain. |
| DevOps | D1 | **Fully implemented** | **Partial** | Four private provider-neutral crates and bounded contracts satisfy their source boundary. |
| DevOps | D2 | **Fully implemented** | **Partial** | Generic status, immutable history, capsule/cache/session isolation, cancellation, and truthful freshness exist. |
| DevOps | D3 | **Partial; nonactivated** | **Blocked** | The hard-disabled broker/runner binds exact managed argv/current executable identity, reconciles actual child outcomes, and queues bounded redacted receipts through ContextManager; attestation, protected activation, forced descendant cleanup, and native proof remain. |
| DevOps | D4 | **Fully implemented; read-only product adapter active** | **Partial** | Bounded OpenSSH inventory/persistence is connected only to D5.1 reviewed browsing; it retains no process/network/launch authority. |
| SSH UX | D5.0-D5.2 | **Partial overall; D5.1 and nonactivated F5.1-F5.3 source complete locally** | **Partial/blocked** | Typed direct/config-jump/tunnel argv, loopback defaults, strong tunnel review, full trust evidence, guarded lifecycle, receipts, reconnect, and compact tunnel states pass locally. Protected activation/attestation, actual status/SSH execution, forced cleanup, and controlled real native/accessibility proof remain. |
| Multi-environment | M6/F6 | **Partial overall; review-only source contracts complete locally** | **Blocked** | Schema-2 reviewed persistence/migration/transfer, typed recipe lifecycle/no-hooks, narrow remote initialization, declarative restore, armed broadcast, and bounded tests pass. Proposed ADR 0023 acceptance, product activation/execution, and controlled native/resource/accessibility evidence remain. |
| Multi-cloud | D6.0/M7 | **Fully implemented locally** | **Partial/external** | Strict provider-neutral schemas, immutable session capsules, 19-state lifecycle, exact allow-once review, isolation/redaction, passive-status migration, cached-only semantics, fuzz/mutation, bounded lifecycle, and a 64-capsule benchmark pass. Real provider CLI/native evidence remains external. |
| Multi-cloud | D6.1-D6.5 | **Partial overall; M8-M11 and M12 Teleport source-complete/nonactivated** | **Partial/blocked** | AWS, Azure, Google Cloud, Kubernetes/OpenShift, and Teleport bounded exact adapters pass locally. D3 product activation and controlled provider/native evidence remain; OpenBao is absent pending ADR 0024 acceptance. |
| Ecosystem | D7 | **Partially implemented at proposal-only boundary** | **Blocked on acceptance** | Proposed ADR 0029, a strict nine-threat/28-limit machine contract, 15 mutations, nonactivation/dependency gate, and detailed audit exist. Runtime SDK/download/package/sandbox/distribution/UI/native work is not authorized. |
| Productivity | CP0 | **Fully implemented** | **Partial** | Accepted architecture, threat model, ceilings, fixtures, mutations, and nonactivation policy exist. |
| Productivity | CP1 | **Fully implemented** | **Partial** | Shell-native completion and bounded explicit refresh exist; hosted native evidence remains. |
| Productivity | CP2.0 | **Fully implemented** | **Partial** | Bounded typed Quick Action model and hostile corpus exist with no runtime authority. |
| Productivity | CP2.1 | **Fully implemented as internal library** | **Partial** | Private atomic persistence, CAS, recovery, watches, and benchmarks exist; no startup/UI activation. |
| Productivity | CP2.2 | **Implemented locally** | **Partial** | Layered search, placeholder/risk/conflict review, bounded import/export/CRUD/recovery, and explicit insert/copy UI are present. Hosted native shells, controlled screen readers, and 30-day performance/resource evidence remain release gates. |
| Productivity | CP3.0 | **Fully implemented at pure boundary** | **Partial** | Five pure serializers, bounded inventories, metadata/tamper verification, tests, fuzz, benchmark, and policy ratchets are complete; activation is disabled and hosted native evidence remains. |
| Productivity | CP3.1 | **Fully implemented locally** | **Partial** | Explicit opt-in persistence, crash-safe all-old/all-new publication, verified five-shell startup/reload, diagnostics, rollback, and exact uninstall are implemented; hosted native/macOS/WSL and controlled-baseline evidence remains. |
| Productivity | CP3.2 | **Fully implemented locally** | **Partial** | Eleven static provider packs, 33 disabled-by-default actions, health/update/alias-safety, CLI, tests, fuzz, benchmarks, and policy gates are complete; hosted evidence remains. |
| Productivity | CP3.3 | **Fully implemented locally** | **Partial** | Six explicit native inventory formats, exact just/Task/mise bridges, bounded dry-run/CAS import/workspace/trust/revocation/removal, path-free digest/revision receipts, background cache authorization, final insertion recheck, tests, mutation, fuzz, benchmark, ADR, CLI, and docs are complete; hosted native/accessibility and 30-day evidence remain. |
| Productivity | CP4 | **Partially implemented overall; source-complete nonactivated** | **Partial/external** | Seven cached provider projections, route/session/generation isolation, final revalidation, compact accessible state/risk UX, production confirmation, fuzz/benchmark/policy evidence are complete locally. Product provider publication/execution, OpenBao, and real native/provider/accessibility/release evidence remain. |
| Productivity | CP5.0 | **Fully implemented at research boundary** | **Complete locally** | Seven-shell API matrix, pure insertion prototype, locked matcher/dependency evidence, privacy review, and retain-CP1/defer-P2 decision are machine-gated; no runtime surface exists. |
| Productivity | CP5.1-CP5.6 | **Partially implemented at proposal-only boundary** | **Blocked on acceptance** | Proposed ADR 0025, schema-1 six-threat contract, 17 mutations/document tests, fixed ownership/limits/shell matrix, and detailed audit exist. No runtime code or UI is authorized; CP1 remains fallback. |
| Productivity | CP6 | **Partially implemented at proposal-only boundary** | **Blocked on acceptance** | Signed action-pack policy and explicit selected-input AI consent/no-tool/no-execution rules are machine-frozen. No pack or AI runtime exists pending ADR 0029 acceptance and protected evidence. |
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
| F4 process/PTY lifecycle | **Partially done; nonactivated** | One Router-owned runner, 50-operation cap, 256-record redacted audit FIFO, exact executable guard, ContextManager PTY/route publication, rollback, cancellation, close, and shutdown pass locally. | Fresh current-executable review after attestation; native graceful/forced descendant cleanup; Windows/macOS/Linux and gated-WSL OpenSSH, resource, pixels, and accessibility evidence. |
| D3 exact-argument broker | **Partially done; nonactivated** | Exact typed package/capability/session/capsule/argv/environment/cwd policy is production-compiled; the activation constant is false and the linked candidate is unverified. | Attested first-party grant, protected activation, real native replacement-race and 1/10/50 lifecycle proof. |

The safe current outcome is an implemented but unavailable boundary: no managed
SSH child can start, and normal shell-owned `ssh` remains the recovery path.

## Release sequence

### v0.4 — standalone terminal stability

v0.4 freezes the terminal-core product boundary: standalone identity/migration, bounded hostile terminal input, transactional runtime configuration, panes/tabs/context/footer, shell integration, image handling, contributor automation, and multi-platform release policy. S0 source gates are complete locally; S1 controlled cross-platform/visual/accessibility evidence remains partial; S2 comparable performance enforcement is not active.

Managed saved-host/cloud behavior is intentionally not added to v0.4. Users can always run normal `ssh` in their shell.

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
7. Accept proposed ADR 0023, wire the review-only M6 editor/workspace/broadcast surface, and obtain managed-execution/native evidence before enabling any recipe or multi-target action.

The first broad DevOps release should therefore be a secure terminal-first SSH product, not a provider SDK platform.

### v0.5.1 — multi-cloud and orchestrator adapters

After the generic process/capability/session boundary is proven, deliver separately enabled provider/context adapters. Official CLIs and local configuration remain preferred authority. AWS, Azure, GCP, Kubernetes, OpenShift, infrastructure, or organization-identity adapters must fail independently and publish only bounded public metadata/context into the core.

### v0.6 and later — ecosystem, richer remote operations, AI

The D7/CP6 proposal-only package/sandbox/provenance/revocation/capability/AI contract and execution audit are complete without activation. Public extension distribution, sandboxing, signed third-party packs, direct provider APIs where truly needed, richer remote files/session memory/collaboration, and AI runtime still wait for explicit protected acceptance and independently proven slices. These are not prerequisites for production SSH.

## Command-productivity sequence

- **CP0:** architecture/threat/compatibility baseline — complete at its source boundary.
- **CP1:** shell-native completion — complete locally; native release evidence remains.
- **CP2.0-CP2.2:** typed actions, private store, review/search/admin/insert-copy — complete locally.
- **CP3.0-CP3.3:** pure five-shell projection, opt-in aliases, reviewed packs, selected native import/trusted workspace bridges — complete locally with remaining release evidence.
- **CP4:** provider/capsule-aware actions — source-complete and nonactivated; product capsule publication and execution remain gated.
- **CP5:** optional app-owned suggestions — planned; native CP1 completion remains fallback.
- **CP6:** signed ecosystem/AI action packs — proposal and threat contract complete; runtime blocked on ADR 0029 acceptance.

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
- **G5 — Partially done:** tooling, generated references, fuzz/property tests,
  and Windows benchmarks exist; controlled native/release evidence remains.
- **G6 — Partially done:** redacted inspector and complete-top-level-tab
  parked-PTY history exist; individual split/local-tab/native-window history is
  not activated.

Compatibility work must not regress Automexia defaults or bypass the same
security, performance, accessibility, native, and release gates as other input
behavior.

## Release gates that cut across phases

A phase is not a stable release claim until its required native hosts, security depth, resource/performance baseline, accessibility evidence, packaging/signing/notarization, docs/assurance mapping, and rollback/recovery behavior pass. Windows-only or local-only proof cannot close a declared cross-platform gate.

## Status update rule

When status changes, update this phase matrix and the relevant user/developer canonical page in the same change. Do not create a second readiness document to restate the matrix. Detailed execution evidence belongs in CI/release artifacts and source history.
