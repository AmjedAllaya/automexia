# Notices and provenance

Automexia Terminal is derived from the Rio terminal project:

- Upstream repository: https://github.com/raphamorim/rio
- Audited fork point: `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`
- Fork-point tag: `rio-base-0.5.20-7d595af`
- Upstream release lineage: Rio 0.5.20
- License: MIT; the complete inherited notice is preserved in `LICENSE`

Private crates retaining inherited names in v0.4 include `rio-backend`,
`rio-fonts`, `rio-grapheme-width`, `rio-graphics`, `rio-notifier`, `rio-vt`,
`rio-window`, `librio`, `librio-wasm`, Sugarloaf, Corcovado, and
Teletypewriter. Those names identify engine lineage and are not Automexia's
public product identity. All workspace crates are non-publishable in v0.4.

Automexia-owned modifications include the desktop product identity, first-party
extension runtime and DevOps context model, prompt-context rendering, unified
theme, shell integration, configuration migration, contributor automation,
packaging, and release policy.

Upstream updates are reviewed on compatibility branches and selectively ported
with provenance. Moving upstream branches are never merged directly into stable
Automexia branches. See `UPSTREAM.md`.

Bundled fonts, shader sources, and MPL-derived filter-runtime files retain
separate licenses described in `THIRD_PARTY_NOTICES.md`. The MIT license for
Automexia-owned code does not replace those component licenses.
