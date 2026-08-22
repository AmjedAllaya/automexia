# Roadmap

This page is the single source of truth for **future sequencing and phase status**. It intentionally omits detailed test commands, benchmark logs, implementation-agent checklists, and dated readiness narratives; those belong in [Testing and release](../developer/testing-release.md) or version-control/release evidence.

A phase can be source-complete while release evidence is still partial. User-facing guides remain authoritative for what users can rely on today.

## Executive phase matrix

| Track | Phase | Implementation | Release evidence | Conclusion |
|---|---|---|---|---|
| Core | v0.4/S0 | **Fully implemented locally** | **Partial** | Identity, hostile-input bounds, atomic reload, and current terminal source gates are complete; stable release gates remain. |
| Assurance | v0.4/S1 | **Partial** | **Partial** | Strong deterministic, Windows-native, QA, fuzz, resource, and workflow machinery exists; controlled Linux/macOS visual/GPU/accessibility evidence and the full baseline remain. |
| Performance | S2 | **Not implemented** | **Not started** | The 30-day comparable baseline is incomplete, so the 5% latency/10% memory ratchet is inactive. |
| DevOps | D0 | **Partial** | **Partial** | Schema-2 threat/manual/fixture definitions are complete locally; ADR 0012 remains proposed and native execution is pending in F4/F5. |
| DevOps | D1 | **Fully implemented** | **Partial** | Four private provider-neutral crates and bounded contracts satisfy their source boundary. |
| DevOps | D2 | **Fully implemented** | **Partial** | Generic status, immutable history, capsule/cache/session isolation, cancellation, and truthful freshness exist. |
| DevOps | D3 | **Partial; nonactivated** | **Blocked** | A test-only exact-argv review model exists; production launch, capability UX, atomic spawn, and native lifecycle proof do not. |
| DevOps | D4 | **Fully implemented; read-only product adapter active** | **Partial** | Bounded OpenSSH inventory/persistence is connected only to D5.1 reviewed browsing; it retains no process/network/launch authority. |
| SSH UX | D5.0-D5.2 | **Partial overall; D5.1 complete locally; D5.2 review preparation complete locally** | **Partial/blocked** | Selected direct D4 records now produce canonical pending F2 plans and a responsive, accessible, disabled Connection Review. Protected D3/M2 activation, current executable/identity evidence, actual OpenSSH/PTY lifecycle, routes/tunnels/reconnect/receipts, and external native/accessibility evidence remain. |
| Multi-cloud | D6.0-D6.5 | **Not implemented** | **Blocked** | Provider auth, capsules, transports, and provider slices are planned only. |
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

The first broad DevOps release should therefore be a secure terminal-first SSH product, not a provider SDK platform.

### v0.5.1 — multi-cloud and orchestrator adapters

After the generic process/capability/session boundary is proven, deliver separately enabled provider/context adapters. Official CLIs and local configuration remain preferred authority. AWS, Azure, GCP, Kubernetes, OpenShift, infrastructure, or organization-identity adapters must fail independently and publish only bounded public metadata/context into the core.

### v0.6 and later — ecosystem, richer remote operations, AI

Public extension distribution, sandboxing, signed third-party packs, direct provider APIs where truly needed, richer remote files/session memory/collaboration, and AI execution wait until the first-party boundaries are stable. These are not prerequisites for production SSH.

## Command-productivity sequence

- **CP0:** architecture/threat/compatibility baseline — complete at its source boundary.
- **CP1:** shell-native completion — complete locally; native release evidence remains.
- **CP2.0-CP2.2:** typed actions, private store, review/search/admin/insert-copy — complete locally.
- **CP3.0-CP3.3:** pure five-shell projection, opt-in aliases, reviewed packs, selected native import/trusted workspace bridges — complete locally with remaining release evidence.
- **CP4:** provider/capsule-aware actions — waits for activated managed-session context.
- **CP5:** optional app-owned suggestions — planned; native CP1 completion remains fallback.
- **CP6:** signed ecosystem/AI action packs — deferred.

## Ghostty compatibility track

Automexia keeps its own shortcut model as the default. A complete Ghostty-compatible profile is not silently enabled and is not a v0.4 release criterion.

- **G0:** source/fixture provenance and shared safety — partial.
- **G1:** typed/compiled keybinding registry — not implemented.
- **G2:** versioned profiles, unbind/override layers, migration, atomic profile reload — partial only because generic reload exists.
- **G3:** structured dispatch outcomes, sequences/tables/chains — partial.
- **G4:** remaining stateless actions/parity work — partial.
- **G5:** generated tooling, migration CLI, compatibility fuzz/benchmarks/release verification — not implemented.
- **G6:** high-lifecycle inspector and parked-PTY undo/redo — deferred pending dedicated architecture/security/resource design.

Compatibility work must not regress Automexia defaults or bypass the same security/performance/accessibility gates as native bindings.

## Release gates that cut across phases

A phase is not a stable release claim until its required native hosts, security depth, resource/performance baseline, accessibility evidence, packaging/signing/notarization, docs/assurance mapping, and rollback/recovery behavior pass. Windows-only or local-only proof cannot close a declared cross-platform gate.

## Status update rule

When status changes, update this phase matrix and the relevant user/developer canonical page in the same change. Do not create a second readiness document to restate the matrix. Detailed execution evidence belongs in CI/release artifacts and source history.
