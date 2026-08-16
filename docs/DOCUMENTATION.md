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
- `docs/TESTING.md` owns evidence levels and commands.
- `docs/ROADMAP.md` owns release sequencing.
- `docs/TERMINAL-FIRST-OPERATIONS.md` owns the planned command-first remote
  operations vocabulary and its cross-feature D/CP phase mapping. It does not
  define shipped CLI behavior until the exact reference and feature ledger are
  updated.
- [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md) owns the
  evidence-based status reconciliation across the S, D, CP, and G roadmap
  tracks.
- root governance/support/security/release files own their named policies.

Other pages should link to these sources instead of copying large tables.

## Required feature documentation

Every feature entry in `tests/assurance/feature-matrix.json` declares:

- `guide`: at least one user/contributor task page;
- `reference`: at least one exact contract page or section;
- `explanation`: at least one architecture or ADR page.

The repository validator rejects missing files, anchors, non-Markdown targets,
unsupported documentation keys, empty categories, and feature entries without
all three forms. A new or materially changed feature must update its docs links,
quality evidence, platform evidence, tests, and changelog fragment together.

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
11. When a canonical roadmap adds, renames, or removes a phase, update
    `PHASE-IMPLEMENTATION-AUDIT.md` in the same change. Every phase needs an
    explicit status, source evidence, remaining work, and honest external
    validation limits.

## Change checklist

- Update the guide, reference, and explanation affected by the change.
- Update `docs/index.md` when adding a canonical page.
- Update `docs/FEATURES.md` and the feature assurance ledger for a new feature.
- Update `docs/PHASE-IMPLEMENTATION-AUDIT.md` when roadmap scope or phase
  status changes.
- Add or supersede an ADR for a durable boundary decision.
- Update `SUPPORT.md`, `SECURITY.md`, migration, or release docs when their
  contracts change.
- Add a `changes/` fragment unless the PR has an allowed docs-only label.
- Run `python tools/ci/check_phase_implementation_audit.py`,
  `python tools/ci/validate_repository.py`, and `cargo ready`.

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
