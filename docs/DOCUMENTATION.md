# Documentation contribution guide

Documentation is a product surface. It follows the same review, ownership,
testing, accessibility, security, and release discipline as code.

## Information architecture

Use one reader intent per page:

| Type | Reader need | Automexia examples |
|---|---|---|
| Tutorial | Learn by completing a safe path | [Getting started](GETTING-STARTED.md) |
| How-to | Solve one concrete problem | [Troubleshooting](TROUBLESHOOTING.md), [WSL development](WSL-DEVELOPMENT.md) |
| Reference | Look up exact behavior, values, commands, or limits | [Configuration](CONFIGURATION.md), [Keyboard](KEYBOARD.md), [CLI](CLI-REFERENCE.md) |
| Explanation | Understand architecture and trade-offs | [Architecture](ARCHITECTURE.md), [decision index](DECISIONS.md), ADRs |

Do not put current user instructions only in a roadmap or readiness audit.
Roadmaps describe intended future state; guides/references define shipped
behavior; ADRs explain durable choices; tests and the assurance ledger prove
the claim.

## Canonical ownership

- `docs/index.md` owns navigation.
- `docs/FEATURES.md` owns the human-readable capability catalog.
- `docs/CONFIGURATION.md`, `docs/KEYBOARD.md`, and
  `docs/CLI-REFERENCE.md` own exact public reference.
- `docs/ARCHITECTURE.md` and `docs/adr/` own technical rationale.
- `docs/BUILD-WRAP-ADOPT-ARCHITECTURE.md` owns the planned technology
  decision matrix and the core/first-party-extension/external-authority split.
- `docs/COMMAND-PRODUCTIVITY.md` owns CP0-CP6 sequencing and
  `docs/DEVOPS-ALIASES.md` owns the CP2/CP3 typed-action, pure projection,
  collision/completion, metadata, CP3.1 private transaction/publication, native
  activation/reload, rollback, and uninstall boundaries.
- `docs/ECOSYSTEM-PLATFORM.md` owns the current no-runtime behavior, fixed
  D7/CP6 package/sandbox/capability/AI safety boundary, planned review, and
  recovery/fallback contract. The detailed ordered work and evidence ledger live
  in `docs/research/D7-CP6-IMPLEMENTATION-AUDIT.md`; detailed evidence commands
  and the future matrix live in `docs/ECOSYSTEM-PLATFORM-TESTING.md`.
- `docs/TESTING.md` owns evidence levels and commands.
- `docs/ROADMAP.md` owns release sequencing and the status-first feature/phase
  register.
- `docs/CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md` owns the ordered focused
  execution checklist for SSH, connectivity, remote workspaces, multi-cloud,
  Quick Actions, and autocomplete. It does not replace the main status register
  or exact shipped-behavior references.
- `docs/TERMINAL-FIRST-OPERATIONS.md` owns the planned command-first remote
  operations vocabulary and its cross-feature D/CP phase mapping. It does not
  define shipped CLI behavior until the exact reference and feature ledger are
  updated.
- [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md) owns the
  evidence-based status reconciliation across the S, D, CP, and G roadmap
  tracks.
- root governance/support/security/release files own their named policies.
- `automexia_docs_repository_aligned/` is a versioned research and proposal
  pack. Its manifest, snapshots, and RFDs preserve analysis and candidate
  decisions; they do not override accepted ADRs, canonical pages, source,
  tests, or release evidence. Integrate an accepted conclusion into its
  canonical owner and leave a backlink instead of making the pack a second
  authority.

Reader-oriented pages under [user-guide](user-guide/index.md), [guide](guide/),
[reference](reference/), [developer](developer/), and [project](project/) may
summarize or reorganize these detailed owners for a specific audience. They
must link back, preserve the same current behavior and status, and never
override a machine-enforced top-level contract.

The complete decision authority is [DECISIONS.md](DECISIONS.md) plus
[docs/adr](adr/). The older project decision index, project roadmap, and
path-adapted ADR copies 0001-0023 are retained as navigation and compatibility
summaries. New decisions live only in the complete ADR tree.

Other pages should link to these sources instead of copying large tables.

## Required feature documentation

Every feature entry in `tests/assurance/feature-matrix.json` declares:

- `guide`: at least one user/contributor task page;
- `reference`: at least one exact contract page or section;
- `explanation`: at least one architecture or ADR page.

The repository validator rejects missing files, anchors, non-Markdown targets,
unsupported documentation keys, empty categories, and feature entries without
all three forms. A new or materially changed feature must update its docs links,
quality evidence, platform evidence, tests, roadmap status, phase audit, and
changelog fragment together. Every behavior-affecting code, configuration, or
test change updates its affected guide/reference/testing text in the same change,
even when the feature's roadmap status does not change.

## Writing rules

1. Lead with the outcome and state the supported version/platform scope.
2. Separate current behavior from planned behavior and label external evidence
   honestly.
3. Give copyable commands, expected results, failure behavior, limits, and a
   safe recovery path.
4. Explain why when a choice is surprising; link the ADR instead of repeating
   its full history.
5. Never put secrets, private hostnames, personal paths, credentials, signing
   material, or unredacted logs in examples.
6. Use descriptive link text and relative repository links. Add headings for
   stable deep links; avoid line-number links in committed docs.
7. Define acronyms on first use, use platform-neutral terms where behavior is
   shared, and call out native differences where it is not.
8. Do not promise universal Linux/BSD/GPU/screen-reader behavior from a compile
   check. Use the evidence vocabulary in [Platform support](PLATFORMS.md).
9. For visible UI changes, update screenshots or renderer-neutral goldens and
   include meaningful alternative text in web assets.
10. Keep examples minimal and tested. Prefer a secure default and explain
    opt-outs rather than requiring configuration for normal use.
11. When a canonical roadmap adds, renames, removes, or changes a phase, update
    both the `docs/ROADMAP.md` status-first register and
    `PHASE-IMPLEMENTATION-AUDIT.md` in the same change. The register uses only
    **Fully done**, **Partially done**, or **Not done**; every phase also needs
    source evidence, remaining work, and honest external validation limits.
12. Documentation is part of implementation, not a later follow-up. Update the
    affected guide, reference, testing evidence, roadmap/audit status, and
    changelog together with the behavior they describe.
13. Research and proposal packs must identify an exact audited committed
    baseline, distinguish implemented state from desired state, preserve
    historical inputs explicitly, and avoid duplicate non-historical
    authorities. Dependency names and imperative architecture language remain
    candidates until an accepted ADR and implementation evidence say otherwise.

## Change checklist

- Update the guide, reference, and explanation affected by the change.
- Confirm every behavior-affecting code, configuration, or test change updates
  its documentation in the same change.
- Update `docs/index.md` when adding a canonical page.
- Update `docs/FEATURES.md` and the feature assurance ledger for a new feature.
- Update the `docs/ROADMAP.md` status-first register and
  `docs/PHASE-IMPLEMENTATION-AUDIT.md` together when roadmap scope or phase
  status changes; use only the three canonical roadmap labels.
- Add or supersede an ADR for a durable boundary decision.
- Update `SUPPORT.md`, `SECURITY.md`, migration, or release docs when their
  contracts change.
- Add a `changes/` fragment unless the PR has an allowed docs-only label.
- Run `python tools/ci/test_repository_aligned_docs.py` and
  `python tools/ci/check_repository_aligned_docs.py` when the aligned pack changes.
- Run `python tools/ci/test_pr_policy.py`,
  `python tools/ci/check_phase_implementation_audit.py`,
  `python tools/ci/test_phase_implementation_audit.py`,
  `python tools/ci/validate_repository.py`, and `cargo ready`.

Run the Markdown hygiene regression test and aligned-pack regression test
whenever documentation tooling or the proposal pack changes:

    python tools/ci/test_documentation_hygiene.py
    python tools/ci/test_repository_aligned_docs.py

The pull-request policy rejects source, configuration, test, workflow, asset,
or packaging changes that do not update at least one affected `docs/*.md` file
in the same pull request. A changelog fragment does not count as documentation;
documentation-only and changelog-only changes do not create a circular
requirement.

The policy job also runs an offline Markdown link check. External URLs should
be primary, authoritative sources and are reviewed for content relevance even
when CI cannot fetch them.

## Preparing a future documentation website

Markdown files are intentionally standalone, use a single H1, relative links,
stable headings, and no repository-specific rendering extensions. A site
generator can map the four page types into navigation without rewriting
content. Generate website search and navigation from `docs/index.md`, and use
the machine-readable feature ledger for capability/evidence views. Source code,
not a website copy, remains authoritative so offline and online documentation
cannot drift.
