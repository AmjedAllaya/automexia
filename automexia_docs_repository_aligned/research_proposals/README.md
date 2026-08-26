
# Research Proposals

These documents are an idea/research pack.

They do not outrank:
- accepted project ADRs;
- current source/contracts;
- the real roadmap;
- native/release evidence.

## Implementation-state labels


Long-form files preserve earlier hypothetical architecture language. Imperative
sentences inside them are candidate design statements, not repository mandates.
Dependency, process-topology, persistence, sandbox, video, or model choices
require their canonical project owner and applicable accepted ADR before
implementation.
- **Current-compatible principle** — can be integrated without changing current architecture materially.
- **Candidate** — requires a real project decision/measurement.
- **Deferred** — keep as future research only.
- **Historical** — provenance only.

## Canonical research architecture

Use `ARCHITECTURE_RESEARCH_AND_TARGET_PRINCIPLES.md`.

There is intentionally no duplicate second master copy.

## Key current-compatible documents

- `docs/00_PRODUCT_AND_INTERACTION.md`
- `docs/03_TERMINAL_RUNTIME.md`
- `docs/05_SECURITY_MODEL.md`
- `docs/08_DEVOPS_EXTENSION.md`
- `docs/09_TESTING_RELEASE.md`
- `docs/17_SUPPLY_CHAIN_AND_DEPENDENCY_POLICY.md`
- `docs/18_MODERN_TERMINAL_COMPATIBILITY.md`
- `docs/19_GHOSTTY_MIGRATION_COMPATIBILITY.md`

## Deferred documents

- public Wasmtime/WIT ecosystem;
- video implementation;
- large process-topology rewrite;
- universal policy/resource service refactor.
