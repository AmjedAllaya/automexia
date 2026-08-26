# Ghostty compatibility implementation ledger

## Outcome and authority

This ledger is the working G0-G6 implementation plan for the
[full compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md). The intended
user outcome is an explicit, versioned Ghostty 1.3 compatibility profile with
typed, explainable bindings while preserving Automexia's existing defaults.

The authoritative decision is
[ADR 0026](adr/0026-versioned-ghostty-keybinding-profiles.md). ADR 0011 remains
authoritative for the implicit `automexia` profile. The frontend owns platform
events and effects; the private keybinding crate owns only pure compilation and
resolution. Normal builds and tests are offline and never execute Ghostty.

## Acceptance criteria

1. Existing installations still resolve to `automexia`, and current classic
   defaults and user overrides remain equivalent.
2. `ghostty-1.3` is pinned to reviewed 1.3.1 fixture identity; `ghostty` is a
   visible moving alias and never selected implicitly.
3. Bindings compile to bounded immutable direct/reverse indexes, sequence tries,
   and per-table registries with deterministic diagnostics and no event-path IO.
4. Failed parse, compilation, global-hotkey preparation, or publication retains
   the complete last-known-good registry and never recreates a PTY.
5. Consumption, performability, fallthrough, sequences, tables, and chains are
   deterministic and surface-isolated.
6. Missing stateless actions have direct, dispatch, discoverability, hostile,
   resource, and platform tests appropriate to their authority.
7. CLI, palette, generated references, fixture verification, and compatibility
   checks read the same registry.
8. Inspector count/list, newest restore, two-step clear, and complete-top-level-
   tab undo/redo follow accepted redaction and PTY lifecycle decisions; broader
   topology history remains outside scope.
9. Roadmaps and references distinguish locally proven behavior from native
   Linux/BSD, macOS, controlled assistive-technology, and release evidence.

## Implementation result ledger — 2026-08-26

| Group | Status | Implemented owner and evidence | Remaining exit evidence |
|---|---|---|---|
| G0 source lock | **Partially done** | `automexia-keybindings` raw fixtures and provenance; generated assurance fixtures/manifests; ADR 0026; deterministic Windows transform; classic golden; exact checksum/regen/property tests | Native macOS fixture and native Linux/macOS release smoke |
| G1 typed registry | **Fully done** | Pure crate owns 85 upstream schemas plus explicit Automexia extensions, typed triggers/predicates/scopes/origins/policies, bounded compiler, allocation-free indexed direct lookup, reverse lookup, trie/table storage, diagnostics, and classic adapter | Native release evidence is carried by G5 |
| G2 profiles/reload | **Fully done** | `automexia` default, moving `ghostty`, pinned `ghostty-1.3`, bind/unbind layers, strict/permissive diagnostics, immutable last-known-good publication, global/palette transaction, dry-run/confirmed atomic migration | Native release evidence is carried by G5 |
| G3 dispatch language | **Fully done** | Structured status/damage, performable/unconsumed fallthrough, exact pending bytes, cancellation, bounded tables/one-shot/catch-all/chains, stable all-surface snapshots, route isolation and indicators | Native IME/layout evidence is carried by G5 |
| G4 stateless actions | **Fully done** | Frontend/VT/layout/export owners implement finite fractional font sizing with a documented 6–100 point adaptation, exact absolute/line/fractional scroll translation, Ghostty clear behavior, Automexia clear extensions, selection/search, independent inherited splits, exact resize, rollback-safe zoom/equalize, and private bounded export/cleanup | Controlled visual/resource evidence is carried by G5 |
| G5 tooling/release | **Partially done** | Pre-GUI CLI/explain/JSON; bounded migration; host-independent xtask generate/verify/test; generated references; properties; two nightly fuzz targets; Criterion coverage; fail-closed repository policy; private three-platform evidence validator and QA summary wiring | Controlled native three-platform layout/visual/AT/resource/package matrix and activated comparable 30-day baseline |
| G6 lifecycle | **Partially done** | ADRs 0027/0028; modal redacted inspector with active/parked count, bounded newest-first list, newest restore, two-step clear, no-PTY key isolation, and memory-only count/time/history-bounded undo/redo for a complete closed top-level tab | Individual split/local-tab/native-window history and controlled native lifecycle proof |

The implementation deliberately does not claim an upstream Windows profile or a
synthetic macOS profile. Unsupported actions remain discoverable and fail closed
rather than being mapped to unrelated behavior.

## Build, wrap, or adopt

The implementation builds a small Automexia-owned pure Rust crate using only
existing workspace serialization and test infrastructure. It does not adopt a
runtime Ghostty dependency or copy Ghostty's engine. Ghostty is wrapped only by
an explicit maintainer generation command whose normalized outputs and hashes
are checked in. This keeps source provenance reviewable while preserving the
frontend as the sole event/effect owner.

## State, limits, and trust boundaries

- Maximum 4 trigger atoms per chord, 8 chords per sequence, 32 active table
  frames, 32 actions per chain, 4,096 bindings per compiled profile, 256
  diagnostics, 128-byte identifiers, and 4 KiB action parameters.
- Compilation validates all input before publication. Duplicate, shadowed,
  unsupported, unavailable, global-sequence, recursive-table, and oversize
  entries receive bounded redacted diagnostics.
- Pending state contains only normalized triggers, exact encoded input bytes,
  table identifiers, and a cancellation code. It never retains clipboard,
  environment, terminal output, credentials, or arbitrary logs.
- Migration canonicalizes bounded include paths, detects cycles, never executes
  configuration, defaults to dry-run, and writes atomically with a backup only
  after explicit apply confirmation.
- Screen export snapshots visible UTF-8 only under a short terminal lock,
  serializes afterward, uses restrictive temporary permissions, and is cleaned
  by count, age, and shutdown. Exported text is never executed.
- G6 parks only complete closed top-level tabs. Their PTYs remain independently
  owned and count/time/history bounded; capacity refusal keeps the entry parked
  rather than duplicating it. The inspector receives only redacted aggregate
  summaries. Clear requires confirmation and drops entries through that owner.

## Test and evidence ladder

Tests are added with each owner: pure unit/property tests first, then frontend
adapter/dispatch tests, CLI/xtask parity, fuzz and Criterion targets, renderer-
neutral snapshots, resource cycles, and native checks. The final local ladder
is formatting, warning-denied workspace linting, CI-profile Nextest, workspace
doctests, full repository QA, and `cargo ready`. The local host can execute Windows unit, compilation, and same-host benchmark
checks, but controlled native Windows/Linux/macOS keyboard, GPU, resource,
packaging, and assistive-technology evidence is accepted only through the exact-
commit private manifest. Linux/BSD, macOS, and an activated named-hardware
30-day baseline remain explicit release prerequisites until those environments
actually run.

## 2026-08-26 local assurance evidence

- Focused inspector/key/lifecycle tests passed (7 inspector/modal tests and 3
  parked-topology tests), as did `cargo xtask test keybindings`, offline all-
  fuzz-target compilation, host-independent generation/verification, Python
  policy/native-evidence mutation suites, architecture, identity, and package
  checks.
- The feature-gated native GUI snapshot path compiled on Windows. It publishes
  only inspector active/confirmation state and the redacted accessibility
  summary; this is automation plumbing, not a native screen-reader pass.
- Windows x64 Criterion point estimates were: 1,000-binding compile 3.9541 ms,
  single lookup 33.261 ns, 1,000-binding lookup 62.279 ns, reverse lookup
  195.77 ns, four-level sequence 181.64 ns, invalid flush 132.92 ns, and active
  table 93.465 ns. These are same-host observations, not a formal saved-baseline
  comparison or the required activated 30-day controlled baseline.
- Native Windows cargo-fuzz 0.13.1 compiled `ghostty_keybindings`, but ASan could
  not start because its runtime DLL was absent (`STATUS_DLL_NOT_FOUND`). The
  supported sanitizer-free mode then failed MSVC linking on unresolved sancov
  section symbols. No cases ran and the migration target was not repeated into
  the same deterministic failure. Hosted Linux nightly owns both ASan campaigns.
- The initial workspace `target/debug` cache (about 29 GB), failed fuzz cache
  (5.72 GB), and later compile-only fuzz cache (1.88 GB) were removed after
  recording results. These are reproducible ignored build artifacts, not source
  or user data.
- Controlled native Windows/Linux/macOS evidence remains private and must use the
  exact-commit validator. Each platform reports fixture smoke, keyboard layouts,
  IME/AltGr/dead keys, reload, inspector accessibility, topology history,
  1/10/50 resource cycles with zero cleanup, package install/rollback, the visual
  matrix, and native assistive-technology review. QA retains only the bounded
  path-free summary; missing or malformed evidence fails closed.

## Delivery and rollback
The implementation is one coherent compatibility change: pure registry and
fixtures, frontend profile/dispatch/actions, tooling/evidence, bounded lifecycle,
and synchronized documentation. `automexia` remains the default, no persistent
migration runs automatically, screen exports are temporary, and parked history
is memory-only. The change can be reverted without rewriting user configuration;
a missing requested profile fails with an actionable diagnostic and preserves
the last-known-good active registry.
