# Documentation contribution guide

Documentation is a product surface. The goal is not to maximize page count; it is to make the authoritative answer easy to find and hard to contradict.

## Information architecture

Use one reader intent per page:

| Category | Reader need | Reader-facing pages |
|---|---|---|
| User Guide | Learn how to use the product, choose an approach, and follow end-to-end tasks | `user-guide/` |
| Guide | Understand deeper product behavior or a specialized workflow | `guide/` |
| Reference | Look up exact syntax/defaults/limits | `reference/` |
| Developer explanation | Understand architecture, security, testing, release | `developer/` |
| Project history/status | Understand future work or durable decisions | `project/` |

A roadmap is never the only documentation for current behavior. An ADR explains why a durable boundary exists; it does not duplicate the user guide. A testing page proves a claim; it does not become the feature specification.

## Authority and reader-facing ownership

The machine-enforced authority policy is
[Documentation contribution guide](../DOCUMENTATION.md). This page is its
concise contributor-oriented companion; if they conflict, the detailed policy
wins.

- [Documentation index](../index.md) owns navigation and the short product
  status summary.
- [Product vision](../PRODUCT-VISION.md) owns the purpose, audience, values,
  experience principles, and broader direction; it does not own feature status.
- User Guide and guide pages own task-oriented learning flows.
- [Configuration](../CONFIGURATION.md), [keyboard](../KEYBOARD.md), and
  [CLI](../CLI-REFERENCE.md) own exact machine-checked public contracts;
  reference pages present the same facts in reader-oriented locations.
- [Architecture](../ARCHITECTURE.md) owns the detailed technical contract;
  this developer section provides a concise orientation.
- [Testing](../TESTING.md) owns evidence levels, commands, and detailed release
  proof; the testing/release overview summarizes it.
- [Roadmap](../ROADMAP.md) and the
  [phase implementation audit](../PHASE-IMPLEMENTATION-AUDIT.md) own phase
  status. The project roadmap is a condensed compatibility summary.
- [Decision index](../DECISIONS.md) and [canonical ADRs](../adr/) own durable
  decisions. The project decision/ADR tree is a retained navigation mirror.

Reader-facing summaries must link to these owners instead of silently
forking exact tables, status, limits, or evidence.

## Writing rules

1. Lead with the user value and outcome before implementation detail, then
   state the supported scope. Keep public messaging consistent with the
   [Product vision](../PRODUCT-VISION.md).
2. Label **Available now**, **Implemented locally/release-gated**, **Implemented internally/not activated**, and **Planned** behavior explicitly.
3. Put exact commands/defaults/limits in reference; User Guide pages may repeat only the practical subset needed for a task and must link back to the reference owner.
4. Put detailed evidence in testing/release; summarize only the result where a guide needs it.
5. Put phase status in the roadmap once. Do not copy full phase ledgers into architecture or feature guides.
6. Explain surprising choices briefly and link the ADR for history/trade-offs.
7. Include safe failure/recovery behavior for operational instructions.
8. Never include secrets, private hostnames, personal paths, credentials, signing material, or unredacted logs in examples.
9. Use relative links and stable headings. Keep one H1 per page.
10. Prefer concise tables for matrices, but use prose for concepts and causal explanations.
11. Do not claim another OS/GPU/screen-reader passed because portable code compiled. Use the evidence vocabulary in [Testing and release](testing-release.md).
12. Delete stale duplicated prose when changing the canonical owner; do not "update all copies" indefinitely.

## Feature-change checklist

- Update the affected User Guide/guide and/or reference.
- Update architecture/ADR only if a durable boundary changed.
- Update tests/assurance mapping for the behavior.
- Update roadmap status only if phase status changed.
- Add/update a changelog fragment according to repository policy.
- Run the repository documentation/link/policy validators and `cargo ready`.

## Adding a new page

A new page is justified when it has a distinct reader intent that cannot be answered clearly by extending an existing canonical page. Before adding one, identify the existing owner and decide whether the new content is a subsection instead. When a new canonical page is necessary, add it to `index.md` and to this ownership list.

## Historical/audit documents

Point-in-time readiness audits and execution ledgers are useful during a focused release push, but they should not remain parallel canonical documentation. Preserve such evidence in version control, CI artifacts, issues, or release records. Promote only durable conclusions into `developer/testing-release.md` or `project/roadmap.md`.
