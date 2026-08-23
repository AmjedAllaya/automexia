
# Automexia Architecture Research and Repository-Integration Pack

**Last verified:** 2026-08-24; source snapshot: 2026-08-23
**Status:** Repository-aligned research/proposal pack — **not a replacement for the project's accepted ADRs, source, roadmap, or release evidence**

## Authority and evidence model

This pack is always non-authoritative. Within the project, authority is
contextual: accepted ADRs and architecture describe approved intent; source and
tests describe implemented state; native and release evidence qualify claims;
and roadmaps sequence remaining work. No one of those dimensions silently
overrides the others.

```text
approved intent    accepted ADRs + canonical architecture
implemented state source + contract tests
qualification     native, security, accessibility, performance, and release evidence
sequencing        canonical roadmap + phase audit
research input    this pack (never project authority)
```

The documents here are intended to:

- capture useful architecture research;
- explain recommended deltas;
- preserve hardening/testing ideas;
- separate current implementation from candidate/future architecture;
- prevent proposal documents from silently becoming project authority.

They must **not** be copied over real project ADRs wholesale.

## Repository snapshot used for this rewrite

The 2026-08-23 repository audit supplied by the implementation agent reports:

- audited committed implementation baseline:
  `20ff7928ea2d1eac5d26f13c62d0cda9b86bc078`;
- Rust `1.96.1`, edition `2021`;
- existing mature terminal/PTY ownership centered on the current application/`ContextManager`, not a greenfield `termd → session-host → extension-host` topology;
- Ghostty G1–G4 substantially implemented;
- Ghostty G5 partially qualified;
- accepted project ADR 0028 owns implemented bounded top-level tab parked-PTY undo/redo;
- split/pane/window history remains future work;
- scoped pane and visible-workspace search is implemented locally in this baseline; controlled native visual, accessibility, and resource qualification remains;
- proposed project ADR 0025 owns the unaccepted CP5 editor bridge proposal;
- proposed project ADR 0029 freezes a non-activating D7/CP6 ecosystem
  boundary; it adds no runtime;
- current first-party extensions use private linked contracts; public Wasmtime/WIT is future research;
- video automation is not implemented;
- v0.4 release closure remains gated by its native, security, accessibility,
  visual, packaging/signing, and sustained-resource evidence; v0.5 provider,
  credential, managed-SSH, ecosystem, and AI activation is a separate lane.

This pack treats those items as **repository-inspected claims from the supplied audit**. It does not independently certify native runtime/release evidence.

## Recommended near-term priority

```text
lane A — v0.4 release closure
  native/security/accessibility/visual evidence
  signing + packaging
  sustained resource evidence
  stable v0.4 decision

lane B — v0.5 activation hardening
  managed SSH + provider authority
  credential/capability review
  native lifecycle and recovery evidence

later, under separate accepted decisions
  public extension ecosystem
  major process-topology redesign
  video-platform implementation
```

## Folder map

- `repository_snapshot/` — source-audit state used to reconcile the proposals.
- `repository_integration/` — concrete delta guidance for the real project docs/ADRs.
- `research_proposals/` — architecture ideas and future RFDs; non-authoritative.
- `historical/` — superseded material retained only for provenance.
- `PROPOSAL_INDEX.md` — detailed index and implementation-state labels.
- `BUNDLE_VALIDATION.md` — internal bundle/link/hash validation only.
- `REFERENCE_SOURCES_2026.md` — dated upstream research references.

## Product constraint

This pack does not require or recommend large LLMs, paid inference APIs, or cloud AI services as a solution for Automexia. Narrow local specialized models may remain research candidates only when a concrete feature justifies them.
