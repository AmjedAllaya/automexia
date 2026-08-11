# Roadmap

## v0.4 — standalone stability

Complete product rebranding, configuration coexistence/migration, contributor
automation, mandatory multi-platform CI, coverage/security policy, and signed
desktop artifact production. Only the newest v0.4 patch is supported.

## v0.5 — internal modularization

Remove Rio environment fallbacks. Extract private Automexia app, extension API,
runtime, DevOps, and UI-model crates. Keep GPU drawing in the frontend adapter;
keep UI model and extension crates independent of renderer, PTY, and GPU. Group
inherited engine directories only after the split stabilizes.

## v0.6 — extension platform

Evaluate third-party extension distribution, a public SDK, capability UX, and a
Wasm sandbox. None of these are part of v0.4 or v0.5 commitments.
