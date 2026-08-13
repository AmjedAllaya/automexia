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

## v0.5 — internal modularization

Remove Rio environment fallbacks. Extract private Automexia app, extension API,
runtime, DevOps, UI-model, and keybinding crates. The keybinding crate will
begin as a behavior-preserving typed registry before it adds profiles or
sequences. Keep GPU drawing in the frontend adapter; keep UI model, extension,
and keybinding crates independent of renderer, PTY, and GPU. Group inherited
engine directories only after the split stabilizes.

## v0.6 — extension platform

Evaluate third-party extension distribution, a public SDK, capability UX, and a
Wasm sandbox. None of these are part of v0.4 or v0.5 commitments.

## Compatibility track

The existing Ghostty-style defaults are a tested subset, not an exact selectable
Ghostty profile. The separate
[full Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) records
every implemented, partial, planned, deferred, and excluded capability. Its
ordered work covers fixture provenance, the typed registry, atomic reload,
versioned profiles, dispatch semantics, missing actions, generated tooling,
and later high-lifecycle features. Complete compatibility is deliberately not
a v0.4 release criterion.
