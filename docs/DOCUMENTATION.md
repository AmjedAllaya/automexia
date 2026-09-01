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
- `docs/INSTALLATION.md` owns public availability, host preparation, the
  supported source-build path, first verified launch, source updates, build
  cleanup, source removal, and the boundary for future signed packages.
- `docs/GETTING-STARTED.md` owns the first-session tutorial after installation.
- `docs/EXTENSIONS.md` owns the plain-language user explanation of why
  extensions exist, the current first-party inventory, public availability,
  and the planned installation experience. It summarizes rather than replaces
  the exact ecosystem, provider, architecture, and roadmap authorities.
- `docs/FAQ.md` owns short onboarding answers and must link to exact authorities
  instead of creating new feature, platform, security, or release claims.
- `docs/PRODUCT-VISION.md` owns Automexia's purpose, audience, values,
  experience principles, and broader direction. It does not define feature
  availability.
- `docs/FEATURES.md` owns the human-readable capability catalog.
- `docs/MANUAL-FEATURE-TESTING.md` owns the end-to-end clean-machine manual
  acceptance workbook: setup, feature-to-scenario mapping, positive/negative/
  boundary workflows, expected results, controlled external evidence, and
  cleanup. It consumes rather than overrides exact references, feature status,
  machine contracts, specialized testing pages, and release policy.
- `docs/CONFIGURATION.md`, `docs/KEYBOARD.md`, and
  `docs/CLI-REFERENCE.md` own exact public reference.
- `docs/ARCHITECTURE.md` and `docs/adr/` own technical rationale.
- `docs/FEATURE-OWNERSHIP-AUDIT.md` owns the evidence-led map from implemented feature families to core, application, domain-package, UI-model, and extension owners; ADR 0035 owns the durable placement rule.
- `docs/BUILD-WRAP-ADOPT-ARCHITECTURE.md` owns the planned technology
  decision matrix and the core/first-party-extension/external-authority split.
- `docs/AUTOMATION-STUDIO-ARCHITECTURE.md` owns the public product, placement,
  and safety summary for the planned editor extension.
  `docs/AUTOMATION-STUDIO-TESTING.md` summarizes the public assurance
  expectations. They do not claim product availability, select a dependency,
  or expose the unpublished implementation plan; proposed ADR 0030 summarizes
  the possible durable boundary.
- `docs/COMMAND-PRODUCTIVITY.md` owns CP0-CP6 sequencing and
  `docs/DEVOPS-ALIASES.md` owns the CP2/CP3 typed-action, pure projection,
  collision/completion, metadata, CP3.1 private transaction/publication, native
  activation/reload, rollback, and uninstall boundaries.
- `docs/SEMANTIC-DIAGNOSTIC-NAVIGATOR.md` owns the public value, interaction,
  placement, resource, privacy, and broad delivery summary for planned error-
  section navigation. It does not define a shipped action, shortcut, setting,
  detector, exact algorithm, or extension capability; proposed ADR 0032
  summarizes the possible durable boundary.
- `docs/SITUATION-AWARE-PRODUCTION-OPERATIONS.md` owns the public value,
  experience principles, extension placement, safety model, and broad delivery
  direction for planned production guidance. Its contracts, UX, and testing
  companions publish stable principles and assurance categories without
  exposing private ranking formulas, schemas, provider playbooks, internal
  limits, state machines, or phase recipes. None of these pages claims a
  setting, UI, provider capability, watcher, incident controller, journal,
  model, execution path, or release; proposed ADR 0034 summarizes the possible
  durable boundary.
- `docs/ECOSYSTEM-PLATFORM.md` owns the current no-runtime behavior, fixed
  D7/CP6 package/sandbox/capability/selected-input suggestion boundary, planned
  review, and recovery/fallback contract. The detailed ordered work and evidence
  ledger live in `docs/research/D7-CP6-IMPLEMENTATION-AUDIT.md`; detailed
  evidence commands and the future matrix live in
  `docs/ECOSYSTEM-PLATFORM-TESTING.md`.
- `docs/LLM-ORCHESTRATION-EXTENSION.md` owns the public product position,
  optional-extension placement, consent, provider isolation, authority, and
  fallback summary. `docs/LLM-ORCHESTRATION-TESTING.md` summarizes public
  assurance expectations. They do not authorize a provider dependency, action
  registry, workflow executor, model download, or runtime activation; proposed
  ADR 0033 summarizes the possible durable boundary.
- `docs/UI-BRANDING-ROADMAP.md` owns renderer-surface U0-U10 status and
  `docs/research/U10-UI-BRANDING-ASSURANCE-AUDIT.md` records its evidence-led
  source/external reconciliation. The S1 policy remains the exact native,
  visual, resource, accessibility, and review authority.
- `docs/TESTING.md` owns evidence levels and commands.
- `docs/FEATURE-TEST-REINFORCEMENT.md` owns the human per-feature scenario,
  oracle, interaction, checker, and exit-criteria plan synchronized with
  `tests/assurance/feature-test-reinforcement-v1.json`; both must change when
  feature risks or required evidence change.
- `docs/ROADMAP.md` owns high-level public direction and broad release order. It
  deliberately omits exact internal sequencing and cannot override user guides,
  source, tests, or release evidence.
- `docs/CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md` summarizes the public
  direction for SSH, connectivity, provider context, command productivity, and
  later production guidance. It is not an implementation checklist.
- `docs/TERMINAL-FIRST-OPERATIONS.md` owns the public command-first interaction
  vocabulary and capability summary. It does not define shipped CLI behavior.
- [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md) is a compact
  public status summary. Feature-specific source, tests, and release evidence
  remain authoritative for exact behavior.
- root governance/support/security/release files own their named policies.
- `docs/PRIVATE-DOCUMENTATION-POLICY.md` owns the publication boundary. Exact
  unreleased designs, internal research, execution ledgers, and unpublished
  drafts live only in the ignored local workspace and must never be linked from
  public pages. They have no project authority until an approved public summary,
  ADR, implementation, and evidence establish it.

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

1. Lead with the user value and outcome before the technical approach, then
   state the supported version/platform scope. Keep the message consistent with
   [Product vision](PRODUCT-VISION.md).
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
11. When public direction or broad status changes, update both
    `docs/ROADMAP.md` and `PHASE-IMPLEMENTATION-AUDIT.md`. Exact phase graphs,
    next-task instructions, and unpublished delivery recipes stay in the
    ignored private workspace. Public feature pages still need honest source,
    remaining-work, and external-validation limits.
12. Documentation is part of implementation, not a later follow-up. Update the
    affected guide, reference, testing evidence, roadmap/audit status, and
    changelog together with the behavior they describe.
13. Follow the [public/private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).
    Public proposals describe value and safety boundaries at a high level.
    Unreleased algorithms, schemas, provider recipes, detailed UX flows,
    implementation maps, dependency evaluations, and internal evidence ledgers
    remain local until a deliberate publication review approves them.

## Change checklist

- Update the guide, reference, and explanation affected by the change.
- Confirm every behavior-affecting code, configuration, or test change updates
  its documentation in the same change.
- Update `docs/index.md` when adding a canonical page.
- Update `docs/FEATURES.md` and the feature assurance ledger for a new feature.
- Update `docs/ROADMAP.md` and `docs/PHASE-IMPLEMENTATION-AUDIT.md` together when
  public direction or broad status changes, without copying internal execution
  plans into them.
- Add or supersede an ADR for a durable boundary decision.
- Update `SUPPORT.md`, `SECURITY.md`, migration, or release docs when their
  contracts change.
- Add a `changes/` fragment unless the PR has an allowed docs-only label.
- Run `python tools/ci/test_repository_aligned_docs.py` and
  `python tools/ci/check_repository_aligned_docs.py` when the publication
  boundary, ignore rule, public policy, or private-workspace handling changes.
- Run `python tools/ci/test_pr_policy.py`,
  `python tools/ci/check_phase_implementation_audit.py`,
  `python tools/ci/test_phase_implementation_audit.py`,
  `python tools/ci/validate_repository.py`, and `cargo ready`.

Run the Markdown hygiene and documentation-boundary regression tests whenever
documentation tooling or publication policy changes:

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
content.

The primary public journey is:

1. `README.md` and `docs/index.md` explain the value and route the reader;
2. `docs/INSTALLATION.md` gets the application running;
3. `docs/GETTING-STARTED.md` and `docs/user-guide/index.md` teach the workspace;
4. task guides lead to exact CLI, keyboard, configuration, platform, security,
   and troubleshooting references.

Generate website search and navigation from `docs/index.md`, and use the
machine-readable feature ledger for capability/evidence views. Source code, not
a website copy, remains authoritative so offline and online documentation
cannot drift.
