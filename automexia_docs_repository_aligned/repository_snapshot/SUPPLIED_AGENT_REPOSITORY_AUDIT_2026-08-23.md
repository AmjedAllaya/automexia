# Historical supplied repository-audit receipt

> **Status:** Historical, non-authoritative response receipt preserved for
> provenance. Its `5333b7e778` “current HEAD” wording and path-specific
> references describe the earlier supplied audit, not the reconciled committed
> baseline. Use [the current snapshot](CURRENT_REPOSITORY_STATE_2026-08-23.md)
> and the repository's canonical ADRs/docs for current decisions.

## Decision

Treat the `suggestions` folder as a proposal archive, not as authoritative project documentation.

The strongest course is:

1. Adopt the activation-hardening direction.
2. Incorporate the Ghostty compatibility/migration distinctions into the existing roadmap.
3. Mine the CP5 and supply-chain proposals for improvements to the real project ADRs.
4. Do not replace the current architecture with the proposed greenfield multiprocess/Wasmtime platform.
5. Park the video platform until the terminal’s existing security, provider, native-platform, packaging, and activation gates are closed.

No repository files were changed during this audit.

## What the repository actually contains

The suggestions sometimes describe implemented work as merely “reported” or “planned.” Inspection of the current source and commit history gives this more accurate picture:

| AreaActual state                |                                                                                                                                             |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Core terminal and PTY ownership | Existing, mature application architecture; not the proposed `termd → session-host → extension-host` design.                                 |
| Rust baseline                   | Rust 1.96.1, edition 2021—not the suggested Rust 1.98/edition 2024 baseline.                                                                |
| Ghostty G1–G4                   | Substantially implemented in source.                                                                                                        |
| Ghostty G5                      | Partial; native visual, accessibility, resource, and platform evidence remains.                                                             |
| Ghostty G6                      | Bounded top-level tab parked-PTY undo/redo is accepted and implemented at current HEAD. Split/pane/window history remains future work.      |
| Process/provider activation     | Much source exists, but important brokers and providers remain disabled or protected pending native/security evidence.                      |
| CP5 editor bridge               | Proposal and research only; no activated runtime. The real authority is project ADR 0025.                                                   |
| Public Wasmtime/WIT ecosystem   | Proposal only. Current first-party extensions are linked behind private contracts.                                                          |
| Video extension                 | No implementation.                                                                                                                          |
| Stable release readiness        | Still blocked by signing, packaging, cross-platform native evidence, controlled activation, accessibility, and sustained resource evidence. |

The current HEAD is `5333b7e778`, containing the Ghostty compatibility work, including accepted project ADR 0028. I inspected the commit, sources, contracts, and tests, but did not rerun the full contributor test suite; therefore this is source evidence, not fresh native-runtime certification.

The technology snapshot in the suggestions is mostly accurate as a dated research snapshot: Rust 1.98 was released on August 20, 2026; Tokio 1.51 is an LTS line; Wasmtime 48 is an LTS release; and WASI 0.3 was ratified in 2026. That validates the research, not automatic adoption by this repository. [Rust releases](https://blog.rust-lang.org/releases/latest/), [Tokio support policy](https://github.com/tokio-rs/tokio/blob/master/README.md), [Wasmtime LTS policy](https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-lts.md), [WASI 0.3 announcement](https://bytecodealliance.org/articles/WASI-0.3).

## Which proposals are best

My ranking is:

1. **Current-version hardening plan** — best basis for the next execution phase.
2. **Ghostty compatibility strategy and migration roadmap** — best architectural correction to the current G roadmap.
3. **CP5 editor bridge specification** — strongest future feature design, but should be merged into actual ADR 0025 and remain disabled until accepted.
4. **Supply-chain policy** — should be incorporated into existing security and release policy.
5. **Master architecture** — valuable idea catalogue, but unsuitable as a replacement architecture.
6. **Revised video specification** — substantially better than the original, but premature for the current roadmap.

Among competing documents:

- Revised video proposal > original video proposal.
- Ghostty compatibility strategy > the master architecture’s earlier G6 treatment.
- Actual project ADR 0025 > suggestion ADR 0009.
- Actual project ADR 0028 > suggestion ADR 0015.
- Existing build/wrap/adopt architecture > the suggestion bundle’s immediate public-WIT platform direction.

## Per-file assessment

### Bundle governance and master architecture

1. [suggestions/README.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/README.md) — **Partially agree.**
   The bundle map and historical separation are useful. I disagree with its implied authority and stale implementation status. It also points to the original video proposal under `video/`, although that file is under `historical/video/`. Fix the link, call the folder “research proposals,” and remove authoritative wording.
2. [AUTHORITATIVE\_INDEX.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/AUTHORITATIVE\_INDEX.md) — **Disagree as an authority; agree as a catalogue.**
   A folder cannot self-promote above the repository’s accepted ADRs, roadmap, architecture, source, and tests. Rename this to something such as `PROPOSAL_INDEX.md` and point explicitly to the real authorities.
3. [DOCUMENTATION\_VALIDATION.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/DOCUMENTATION\_VALIDATION.md) — **Disagree with its conclusion.**
   The reported “zero blocking issues” does not survive repository integration analysis: the README has a broken path, both master documents are duplicates, ADR numbers collide, G6 status is stale, and the proposed baseline differs from the real workspace. Keep validation only if it becomes reproducible and checks repository-level authority and links.
4. [REFERENCE\_SOURCES\_2026.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/REFERENCE\_SOURCES\_2026.md) — **Partially agree.**
   It uses generally appropriate primary sources. Add access dates, exact release/tag links, license references for model assets, and distinguish mutable “latest” URLs from pinned evidence.
5. [MANIFEST.json]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/MANIFEST.json) — **Mostly agree.**
   Every listed file existed and matched its recorded hash. Its semantic metadata is stale: the Ghostty commit and G6 acceptance have now been inspected. Explicitly document whether excluding the manifest itself is intentional.
6. [technology-baseline.toml]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/architecture/technology-baseline.toml) — **Partially agree.**
   Useful as a dated candidate baseline. It is not the adopted implementation baseline: the repository uses Rust 1.96.1/edition 2021 and currently has no Tokio, Wasmtime, SQLite, AccessKit, ONNX Runtime, or FFmpeg platform dependency. Split it into `current`, `candidate`, and `rejected/deferred` sections.
7. [project\_docs/README.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/project\_docs/README.md) — **Partially agree.**
   Navigation is helpful, but it repeats the incorrect authority model and gives future video/platform documents equal standing with implemented terminal work. Add implementation-state labels and remove conflicting ADR claims.
8. [suggestions/PROJECT\_ARCHITECTURE.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/PROJECT\_ARCHITECTURE.md) — **Partially agree.**
   Strong principles include terminal excellence without extensions, semantic overlays, typed process plans, untrusted extension input, bounded resources, external secret custody, and keeping extensions off hot paths. I disagree with adopting its greenfield process topology, immediate Wasmtime/WIT posture, huge crate decomposition, or implied replacement of the existing architecture. Extract principles and candidate decisions; do not adopt the document wholesale.
9. [project\_docs/PROJECT\_ARCHITECTURE.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/project\_docs/PROJECT\_ARCHITECTURE.md) — **Disagree with keeping a second copy.**
   It is byte-for-byte identical to the root master document. One canonical archival copy is enough; two copies create guaranteed drift.

### Project design documents 00–20

10. `00_PRODUCT_AND_INTERACTION.md` — **Agree.**
    Keyboard-first, command-palette discoverability, renderer-native overlays, semantic actions, and accessibility are appropriate. Avoid turning “keyboard-first” into “pointer-hostile”; pointer input should invoke the same semantic actions.
11. `01_RUST_CORE_ARCHITECTURE.md` — **Partially agree.**
    Agree on safe Rust, typed ownership, bounded queues, cancellation, and thin adapters. Do not mandate edition 2024, Tokio, SQLite, Wasmtime, or a large crate split without individual migrations, measurements, and ADRs.
12. `02_PROCESS_SESSION_IPC.md` — **Partially agree.**
    Failure isolation, bounded framing, peer validation, shutdown ownership, and stale-generation rejection are good. A `termd` and multiple session-host processes would be a major redesign of the existing `ContextManager` PTY owner; prototype and benchmark it before changing authority.
13. `03_TERMINAL_RUNTIME.md` — **Mostly agree with adaptation.**
    Standards-first compatibility, fast-path isolation, bounded image protocols, and differential testing are sound. Continue with the existing Rio-derived parser/renderer unless measurements justify replacement. Ghostty or `libghostty` should remain an optional reference candidate.
14. `04_EXTENSION_PLATFORM.md` — **Partially agree.**
    Fine-grained capabilities and no raw PTY/GPU/credential access are correct for future third-party extensions. Wasmtime/WIT is not current architecture and should follow acceptance of the real ecosystem ADR, not precede it. Existing first-party linked contracts remain the present owner.
15. `05_SECURITY_MODEL.md` — **Mostly agree.**
    Its trust-boundary, input-validation, redaction, and fail-closed principles align with the project. Consolidate them into the existing security model; do not create a parallel policy authority.
16. `06_RESOURCE_ACCESS_MODEL.md` — **Partially agree.**
    Typed plans, previews, environment capsules, capability checks, and auditable decisions are valuable. A universal resource graph, Context Guardian, Policy Engine, and Explain system is too broad for immediate implementation. Extend existing brokers incrementally.
17. `07_CREDENTIAL_BROKER.md` — **Agree with the objective, defer the architecture.**
    Secret material should remain in platform/external stores and the app should persist opaque references. A new general credential service should wait until current provider activation and native keychain evidence identify requirements that existing tools cannot satisfy.
18. `08_DEVOPS_EXTENSION.md` — **Mostly agree.**
    Wrapping official OpenSSH, Kubernetes, Docker, and cloud tools is preferable to rebuilding mature protocols. I disagree with an absolute “no connect button” rule: an accessible button can dispatch the same reviewed semantic plan without creating a second authority.
19. `09_TESTING_RELEASE.md` — **Agree.**
    The evidence dimensions—correctness, resilience, performance, native platforms, accessibility, packaging, and cleanup—fit the project well. Grow these through existing `xtask`, fixtures, assurance matrices, and CI rather than establishing a separate testing authority.
20. `10_ROADMAP_ADRS.md` — **Disagree with the proposed ordering; agree with staged evidence.**
    Its greenfield ordering ignores substantial implemented D, CP, S, and G work. Keep the current roadmap, add activation hardening, reclassify Ghostty into compatibility/migration/session tracks, and defer video and public extension hosting.
21. `11_PLATFORM_CAPABILITIES.md` — **Partially agree.**
    It is a useful idea inventory. It mixes current requirements with speculative platform capabilities without adequate ownership, priority, or release gates. Retain only as a backlog catalogue.
22. `12_AUTOMEXIA_LAB.md` — **Partially agree.**
    Scenario-driven, reproducible evidence is valuable. Build a thin scenario/evidence layer over existing test owners rather than a parallel monolithic “Lab” framework.
23. `13_EDITOR_SUGGESTION_BRIDGE.md` — **Strongly agree with the threat model; partially agree with the design.**
    Same-user peer checks, explicit capabilities, no implicit Enter, bounded framing, stale-generation protection, auditability, and kill switches are excellent. Merge useful deltas into actual ADR 0025. Resolve overlapping shell-adapter ownership, avoid premature crate proliferation, and choose transcript authentication only after platform-specific design review.
24. `14_VIDEO_AUTOMATION_EXTENSION.md` — **Partially agree, future only.**
    The extension boundary and typed media plans are reasonable, but the feature depends on brokers, sandboxing, job management, model governance, and review UI that are not currently available. Do not put it on the active terminal roadmap.
25. `15_VIDEOLAB.md` — **Agree conditionally.**
    If video is approved later, reproducible corpora, timing fidelity, NLE round trips, model evaluation, human review, and resource limits are necessary. Corpus licensing and privacy must be first-class evidence.
26. `16_TECHNOLOGY_BASELINE_2026.md` — **Partially agree.**
    The upstream versions are broadly credible as of the document date, but “current upstream” is not “approved dependency.” Include compatibility, MSRV, advisory, license, binary-size, startup, and rollback evaluations before adoption. Wasmtime’s own current advisories illustrate why version freshness alone is insufficient. [Wasmtime advisories](https://github.com/bytecodealliance/wasmtime/security/advisories)
27. `17_SUPPLY_CHAIN_AND_DEPENDENCY_POLICY.md` — **Agree strongly.**
    Separate acquisition from builds, pin artifacts, record provenance, support offline/reproducible paths, audit build scripts, and constrain package authority. Integrate this into `SECURITY.md`, `RELEASING.md`, and dependency review. The August 2026 `arrayref` compromise demonstrates the relevance of build-script and provenance controls. [Rust arrayref incident report](https://blog.rust-lang.org/2026/08/20/supply-chain-attack-on-arrayref/)
28. `18_MODERN_TERMINAL_COMPATIBILITY.md` — **Agree.**
    Standards and application behavior should be the contract; Ghostty should be one differential reference, not the oracle. Add explicit protocol versions, fixtures, supported application versions, and native evidence requirements.
29. `19_GHOSTTY_MIGRATION_COMPATIBILITY.md` — **Mostly agree.**
    Pinned, versioned profiles and safe bounded migration match current implementation. Permit safety/correctness errata through a new profile revision and explicit migration rather than silently changing an existing pinned profile. Ghostty 1.3.1 is an appropriate dated reference, not a permanent target. [Ghostty 1.3.1 notes](https://ghostty.org/docs/install/release-notes/1-3-1)
30. `20_PERSISTENT_SESSION_HISTORY.md` — **Partially agree.**
    Separating lifecycle persistence from compatibility is correct. Update it to recognize the implemented bounded top-level tab history. Do not require credential revalidation merely to reveal a still-running PTY: display it with stale/risk status, then revalidate before reconnection or new authority-bearing operations.

### Suggestion ADRs 0001–0015

All fifteen ADR numbers conflict with the real project ADR namespace. None should be imported with its current number or “Accepted” status.

31. `ADR 0001 — Rust core` — **Agree with safe-Rust principles; disagree as an ADR import.** Existing architecture and toolchain decisions already own this territory.
32. `ADR 0002 — earlier language-neutral architecture` — **Historical only.** Useful provenance, but superseded and unsuitable as a current decision.
33. `ADR 0003 — Wasmtime/WIT extensions` — **Partially agree for future third-party extensions.** Too early and too thin to supersede the real proposed ecosystem ADR.
34. `ADR 0004 — credential broker` — **Partially agree.** Correct custody goal, but it lacks the native-provider, lifecycle, migration, and activation detail required for acceptance.
35. `ADR 0005 — semantic overlays` — **Agree with the substance; disagree with duplication.** Existing renderer, accessibility, and overlay ADRs already own this decision.
36. `ADR 0006 — session supervisor` — **Partially agree.** A broader persistent supervisor may be valuable, but it extends far beyond accepted top-level tab history and needs its own threat, storage, native-platform, and cleanup design.
37. `ADR 0007 — policy/context services` — **Partially agree.** The direction is sound; the authority graph, failure behavior, persistence, and enforcement points are not concrete enough.
38. `ADR 0008 — Automexia Lab` — **Partially agree.** Scenario evidence is useful, but the ADR should follow a minimal schema and ownership prototype.
39. `ADR 0009 — CP5 bridge` — **Agree with many safeguards; disagree with it as the owner.** Actual ADR 0025 is more repository-specific and should receive any useful deltas.
40. `ADR 0010 — video extension` — **Disagree for the active roadmap.** It is an unvalidated product expansion with substantial codec, licensing, model, sandbox, and distribution costs.
41. `ADR 0011 — technology/supply baseline` — **Partially agree.** Supply-chain policy belongs in current security/release authorities; individual dependency adoption needs separate evidence.
42. `ADR 0012 — fine-grained capabilities` — **Agree for a future public ecosystem.** Fold it into the real ecosystem proposal and distinguish third-party sandbox capabilities from current linked first-party contracts.
43. `ADR 0013 — accessibility` — **Agree with the content; reject the duplicate ADR.** The real project already has ADR 0013 and broader accessibility ownership.
44. `ADR 0014 — terminal/Ghostty compatibility` — **Mostly agree with the substance; reject the number.** Incorporate TC/GM terminology into the existing G roadmap or create a new project-numbered ADR.
45. `ADR 0015 — parked PTY history` — **Agree with bounded parking; reject as stale duplication.** Actual ADR 0028 is accepted, more precise, and implemented for top-level tabs.

### Hardening, Ghostty, video, and historical documents

46. [AUTOMEXIA\_CURRENT\_VERSION\_HARDENING\_PLAN.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/hardening/AUTOMEXIA\_CURRENT\_VERSION\_HARDENING\_PLAN.md) — **Best document; strongly agree with revisions.**
    Its central instruction—pause feature expansion and convert source-complete work into activated, native-tested, observable, releasable behavior—is correct. Rebase its checklist against current HEAD because several source-level items now exist. Avoid freezing an extension API prematurely and avoid splitting code into new crates without an ownership need.
47. [AUTOMEXIA\_GHOSTTY\_COMPATIBILITY\_STRATEGY.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/ghostty/AUTOMEXIA\_GHOSTTY\_COMPATIBILITY\_STRATEGY.md) — **Strongly agree.**
    Separating terminal compatibility, Ghostty migration, and persistent-session lifecycle is the correct model. “Core should not know Ghostty exists” should mean no Ghostty authority or dependency in core—not necessarily that profile identity cannot reach the composition boundary.
48. [GHOSTTY\_IMPLEMENTATION\_STATUS\_2026-08-23.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/ghostty/GHOSTTY\_IMPLEMENTATION\_STATUS\_2026-08-23.md) — **Partially agree; now stale.**
    Its caveats were appropriately conservative, but current HEAD contains the referenced work and accepted ADR 0028. Update it with source-inspected status while retaining the distinction between source evidence and fresh native validation.
49. [TERMINAL\_COMPATIBILITY\_AND\_GHOSTTY\_MIGRATION\_ROADMAP.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/ghostty/TERMINAL\_COMPATIBILITY\_AND\_GHOSTTY\_MIGRATION\_ROADMAP.md) — **Mostly agree.**
    TC/GM/PS separation is better than a single “Ghostty parity” track. Merge it into the existing roadmap rather than creating another roadmap, and describe PS top-level history as implemented/partial rather than unstarted.
50. [AUTOMEXIA\_VIDEO\_EXTENSION\_SPEC\_2026.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/video/AUTOMEXIA\_VIDEO\_EXTENSION\_SPEC\_2026.md) — **Best video proposal, but defer it.**
    The revisions improve process brokering, sandboxing, framing, backpressure, decision provenance, interchangeable models, human review, HDR/timing, and staged delivery. “No GUI dependency” should become “CLI-complete with optional renderer-native review UI.” Managed FFmpeg distribution carries major codec, security, signing, and redistribution obligations; FFmpeg itself distributes source and maintains release branches rather than providing a ready-made product distribution policy. [FFmpeg downloads and releases](https://www.ffmpeg.org/download.html)
51. [AUTOMEXIA\_PROJECT\_SPEC\_original.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/historical/video/AUTOMEXIA\_PROJECT\_SPEC\_original.md) — **Disagree as an implementation plan; retain for provenance.**
    It hardcodes early model choices, invokes FFmpeg too directly, and combines VAD, enhancement, scene detection, face detection, reframing, and rendering into an oversized first milestone. The revised specification supersedes it.
52. [keyboard\_first\_extensible\_terminal\_architecture.pre\_rust.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/historical/keyboard\_first\_extensible\_terminal\_architecture.pre\_rust.md) — **Disagree as current architecture.**
    It predates mandatory Rust and the current trust and ownership boundaries. Preserve it only as historical reasoning.
53. [keyboard\_first\_extensible\_terminal\_architecture.pre\_platform\_expansion.md]\(/D:/workstation/projects/business-project/custom\_terminal/automexia-terminal/standalone/suggestions/historical/keyboard\_first\_extensible\_terminal\_architecture.pre\_platform\_expansion.md) — **Disagree as current authority; partially agree with its simpler scope.**
    Its narrower keyboard-first terminal concept is healthier than immediate platform expansion, but it is still superseded by implemented architecture and accepted ADRs.

## Recommended execution sequence

1. Reclassify the entire folder as research/proposals.
2. Remove duplicate master authority and rename suggestion ADRs so they cannot collide with real ADRs.
3. Use the hardening plan to create a delta-only activation checklist against current HEAD.
4. Prioritize process/provider activation, executable identity, environment isolation, credential-reference handling, native SSH lifecycle, accessibility, signing, packaging, and sustained resource evidence.
5. Merge TC/GM/PS terminology into the existing Ghostty roadmap and update G6 truth.
6. Incorporate worthwhile CP5 safeguards into actual ADR 0025, then require explicit protected acceptance before implementation.
7. Integrate supply-chain controls into existing security and release documentation.
8. Revisit public Wasmtime/WIT extensions only through the real ecosystem ADR.
9. Keep video as a separate product-discovery proposal until the core activation phase is complete and its business value, licensing, distribution, privacy, and corpus requirements are validated.

The most important decision is to finish and prove the platform already built before expanding Automexia into a new extension ecosystem or video-automation product.
