# Roadmap

## v0.4 — standalone stability

Complete product rebranding, configuration coexistence/migration, contributor
automation, mandatory multi-platform CI, coverage/security policy, and signed
desktop artifact production. Only the newest v0.4 patch is supported.

Before stable release, close the remaining source-level identity and security
hardening items: return the Automexia terminal name from XTGETTCAP, bound raw
OSC/APC/XTGETTCAP accumulation with safe discard/recovery behavior, and keep
the last known-good configuration when a runtime reload fails. These items
take priority over adding more compatibility shortcuts.

The detailed [stabilization roadmap](STABILIZATION-ROADMAP.md) records completed
regressions, partially proven areas, the exact S0 implementation/test gates,
native Linux/macOS and visual review, performance baselining, hosted security,
and external release requirements. Windows-only results never satisfy a
cross-platform gate.

The v0.4 assurance milestone also closes the gap between logical correctness
and what a user actually sees. Before stable release, Automexia will add a
pinned Nextest/JUnit execution profile with timeouts and process-leak reporting,
deterministic renderer-state snapshots plus controlled rendered-frame captures,
an executed (not compile-only) Criterion baseline, property/state-machine tests
for resize and pane lifecycle invariants, Windows resource verification, and a
redacted `cargo xtask qa --full --bundle` evidence archive. The existing
`cargo ready` contributor gate remains deterministic and `cargo automexia`
remains the fast launch path; native GPU, screen-reader, profiler, and
Application Verifier work belongs in explicit deep-test profiles rather than
ordinary application startup.

v0.4 also establishes an accessibility baseline: every custom chrome action
must remain keyboard-operable, focus-visible, contrast-checked, and usable at
200% scaling, and supported platforms receive recorded manual screen-reader
smoke evidence. A complete cross-platform accessibility tree is staged with
the v0.5 UI-model boundary below rather than being claimed from color and
keyboard tests alone.

## v0.5 — internal modularization

Remove Rio environment fallbacks. Extract private Automexia app, extension API,
runtime, DevOps, UI-model, and keybinding crates. The keybinding crate will
begin as a behavior-preserving typed registry before it adds profiles or
sequences. Keep GPU drawing in the frontend adapter; keep UI model, extension,
and keybinding crates independent of renderer, PTY, and GPU. Group inherited
engine directories only after the split stabilizes.

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

## v0.6 — extension platform

Evaluate third-party extension distribution, a public SDK, capability UX, and a
Wasm sandbox. None of these are part of v0.4 or v0.5 commitments.

Extension-platform work inherits the established QA profiles. Capability,
sandbox, migration, and distribution changes require property/fuzz corpora,
resource ceilings, mutation-tested policy code, accessibility semantics, and
redacted QA evidence before a public SDK or third-party download path ships.

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

| Capability | v0.4 | v0.5 | v0.6 |
|---|---|---|---|
| Test orchestration and evidence | Pinned Nextest/JUnit, timeouts, leak reporting, and redacted QA bundle | Component-specific profiles after crate extraction | Extension SDK/profile evidence |
| Visual verification | Deterministic renderer state, controlled offscreen/native frame capture, diff artifacts, and human platform review | Accessibility-aware UI-model goldens | Extension UI/capability visual contracts |
| Property/concurrency testing | Proptest resize/session invariants and initial bounded Loom models | Loom/Miri coverage moves into pure private crates | Capability/sandbox state machines |
| Performance | Execute Criterion, collect 30-day baselines, record startup/interaction/resource data | Enforce mature latency/memory ratchets | Add SDK/sandbox overhead budgets |
| Native assurance | Windows AppVerifier/WPR and controlled Windows/Linux/macOS GPU/shell matrices | Full accessibility adapter and assistive-technology matrix | Sandboxed third-party extension isolation |
| Test-strength/security ratchets | Longer fuzz corpora and Automexia-owned coverage baseline | Scoped mutation testing and maintainable cargo-vet adoption | Public extension supply-chain/capability audits |

No single host or test layer may claim complete assurance. Pull requests prove
deterministic contracts, nightly jobs explore expensive state and native
behavior, release jobs require controlled hardware and packaging evidence, and
maintainers record the remaining visual/accessibility decisions.
