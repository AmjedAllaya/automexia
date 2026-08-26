
# Current Repository State — 2026-08-23

**Source:** supplied repository audit by the implementation agent
**Evidence type:** source/commit/contracts/tests inspection; not a fresh rerun of every native/runtime gate

## Audited committed implementation baseline

```text
20ff7928ea2d1eac5d26f13c62d0cda9b86bc078
```

Branch reported during the Ghostty work:

```text
agent/command-completion-cp1
```

## Current implementation snapshot

| Area | Repository-inspected state |
|---|---|
| Terminal/PTY ownership | Existing mature application architecture; not the proposed `termd → session-host → extension-host` design |
| Rust | 1.96.1 |
| Edition | 2021 |
| Ghostty G1–G4 | Substantially implemented |
| Ghostty G5 | Partial; visual/accessibility/resource/platform/long-run evidence remains |
| Ghostty G6 | Accepted and implemented for bounded top-level tab parked-PTY undo/redo via project ADR 0028; user-visible parked count/list/clear controls and broader split/pane/window history remain future |
| Scoped terminal search | Fully implemented locally for the selected pane and the active local tab in each visible pane; bindings, bounded query handling, route isolation, geometry snapshots, docs, and assurance metadata exist; controlled native visual, accessibility, and resource evidence remains |
| Provider/process activation | Significant source exists; important activation/native/security gates remain |
| CP5 | Research/proposal only; proposed project ADR 0025 is the owner |
| Public Wasmtime/WIT ecosystem | Non-activating proposal only under proposed project ADR 0029 |
| Current extensions | First-party/private linked contracts |
| Video extension | Not implemented |
| v0.4 release closure | Gated by signing, packaging, declared native/security/accessibility/visual evidence, and sustained resource evidence |
| v0.5 activation hardening | Separately gated for managed SSH, providers, credentials, ecosystem runtime, and AI authority |

## Interpretation

"Implemented" does not mean "release-qualified."

Use separate dimensions:

```text
source implemented
contract tested
native qualified
security qualified
performance qualified
accessibility qualified
packaging/signing qualified
release qualified
```

## Immediate engineering implication

Avoid architecture replacement while activation/readiness work is unfinished.

Prefer:

```text
existing owner
    ↓
measure concrete limitation
    ↓
extract/refactor only when justified
```

over:

```text
greenfield topology
    ↓
replace working ownership preemptively
```
