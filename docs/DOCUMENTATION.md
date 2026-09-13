# Documentation contribution guide

Documentation is a product surface. It must match current source, tests,
platform evidence, security boundaries, and release status.

## Information types

| Type | Purpose | Examples |
|---|---|---|
| Tutorial | Learn through a safe complete path | [Getting started](GETTING-STARTED.md) |
| How-to | Complete one task | [Troubleshooting](TROUBLESHOOTING.md) |
| Reference | Look up exact behavior | [Configuration](CONFIGURATION.md), [Keyboard](KEYBOARD.md), [CLI](CLI-REFERENCE.md) |
| Explanation | Understand current design and trade-offs | [Architecture](ARCHITECTURE.md), [ADRs](DECISIONS.md) |
| Evidence | Understand implementation and release status | [Features](FEATURES.md), [Testing](TESTING.md), [Readiness](READINESS-AUDIT.md) |

## Public scope

Public documentation may include:

- current source behavior and exact release/platform status;
- ordinary free terminal capabilities;
- installation, use, configuration, recovery, migration, and uninstall;
- contributor architecture needed to maintain existing code;
- security and trust boundaries;
- tests, native evidence, packaging, and release rules;
- accepted decisions governing current source;
- current implementation and release-evidence status for the open-source terminal.

Public documentation must not include:

- unreleased product ideas, including ordinary and advanced future free features;
- paid or commercial features;
- pricing, packaging, revenue, customer, or market strategy;
- future specialist extensions;
- hosted or organization-service designs;
- unreleased provider workflows;
- detailed future algorithms, schemas, state machines, UX flows, dependency
  choices, limits, tests, or delivery phases;
- links or references to ignored private documents.

The exact public/private rule is in
[Private documentation policy](PRIVATE-DOCUMENTATION-POLICY.md).

## Canonical public owners

- `README.md` introduces current open-source terminal value.
- `docs/PRODUCT-VISION.md` owns the public purpose and values.
- `docs/FEATURES.md` owns human-readable feature maturity.
- `docs/index.md` owns public navigation.
- `docs/GETTING-STARTED.md` and `docs/user-guide/` own user workflows.
- `docs/CONFIGURATION.md`, `docs/KEYBOARD.md`, and
  `docs/CLI-REFERENCE.md` own exact public reference.
- `docs/ARCHITECTURE.md` and accepted ADRs own current technical rationale.
- `docs/TESTING.md` owns evidence levels and contributor commands.
- `docs/ROADMAP.md` records current implementation and release-evidence status.
- `docs/PRIVATE-DOCUMENTATION-POLICY.md` owns confidentiality and publication.

Pages may summarize these owners for a specific reader but must not contradict
them.

## Status language

Use these distinctions:

- **Available:** current user path exists in the described build and has the
  required evidence for the claim.
- **Implemented in source:** code exists, but release or platform gates remain.
- **Release-gated:** one or more named evidence requirements remain open.
- **Disabled/nonactivated:** code or models exist, but no supported runtime path
  is enabled.
- **External evidence required:** hardware, account, platform, signing, or human
  validation has not run.
- **Not implemented:** no current user behavior exists.

Do not replace these with vague terms such as “supported,” “ready,” or “soon.”

## Required feature documentation

Every public feature entry referenced by the assurance matrix needs:

- a guide for the user or contributor task;
- a reference for exact behavior and limits;
- an explanation of ownership and design;
- current platform and release evidence;
- failure, recovery, disable, and cleanup behavior where applicable.

A source test or roadmap sentence cannot promote a feature to available.

## Writing rules

1. Lead with current outcome and supported scope.
2. Use short, direct sentences and concrete terminology.
3. Keep examples fictional and repository-relative.
4. Never include credentials, private identifiers, real hostnames, personal
   paths, customer data, or copied private output.
5. Treat terminal output, paths, imported files, providers, and model-generated
   text as untrusted.
6. Document exact executables and arguments; do not recommend shell evaluation
   for structured actions.
7. Explain platform differences explicitly.
8. Describe keyboard and accessibility behavior for visible features.
9. Link to one canonical owner instead of duplicating large specifications.
10. Do not mention the subject or contents of a private future plan.

## Change checklist

Before publishing documentation:

- inspect current source, tests, status, and local modifications;
- confirm the subject belongs in public scope;
- move any unreleased advanced/commercial material to the ignored private area
  before editing the public page;
- verify links and anchors;
- check UTF-8, LF endings, final newline, and balanced fences;
- scan for secrets, local identifiers, private plan terms, and stale claims;
- run repository documentation validators;
- review the complete diff and ensure only authorized files changed.

## Publishing a private subject later

Private material does not become public merely because code work begins. A
deliberate publication review must decide:

1. which behavior is implemented and necessary to document;
2. which details contributors need to maintain public code;
3. which product, commercial, security, patent, or competitive details remain
   private;
4. which guide/reference/architecture/evidence owners must change; and
5. whether the feature's release status supports any user-facing claim.
