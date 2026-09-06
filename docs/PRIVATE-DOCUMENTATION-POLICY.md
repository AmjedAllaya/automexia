# Public and private documentation boundary

Automexia is an open-source project, so users and contributors need clear
documentation about the software they can run, inspect, and improve. That does
not require publishing every unreleased implementation recipe or internal
planning document.

## Public documentation

Public documentation describes current implementation and release status only.
Future features and implementation plans remain private, including free and
open-source features. Current limitations and missing evidence may be documented
without describing a future design or announcing a product.

The repository should publish:

- product purpose, values, and intended users;
- installation, configuration, use, recovery, and uninstall guidance;
- behavior that exists in source, with honest release and platform status;
- stable architecture and security boundaries contributors must preserve;
- current implementation, known limitations, and release-evidence status;
- contribution, testing, release, support, and vulnerability-reporting rules;
- accepted decisions needed to understand or maintain public code.

## Local-only documentation

Keep the following outside version control until a deliberate publication
review approves them:

- every unreleased feature plan, including an ordinary or advanced free feature;
- every paid or commercial feature, product edition, package, service, or
  extension;
- pricing, monetization, revenue, market, customer, positioning, and business
  strategy;
- future specialist workflows, provider products, organization services,
  hosted services, and extension products;
- exact algorithms, ranking rules, heuristics, schemas, state machines, and
  provider-specific execution recipes for unreleased capabilities;
- detailed future UX flows, implementation maps, resource budgets, test plans,
  dependency evaluations, and phase-by-phase delivery instructions;
- competitive analysis, unpublished product research, commercial strategy,
  patent-sensitive concepts, and abandoned alternatives that reveal more than
  the public direction requires;
- internal evidence ledgers, private operational information, and unfinished
  drafts.

The local `.automexia-private/` directory is ignored for this purpose. Ignoring
a directory prevents normal Git staging; it does not encrypt the files or make
them safe for credentials.

Public pages must not tease, summarize, name, or link to these private plans.
When a source-owned validator requires a historical documentation path, that
path may contain only a minimal non-activating compatibility notice. It must not
contain the product idea, workflow, architecture, commercial placement, or
delivery plan.

## Never document in the repository

Public and private project documents must both avoid credentials, tokens,
private keys, customer or production data, personal identifiers, machine
names, and absolute local paths. Use clearly fictional examples and redacted
fixtures.

## Publication review

Before moving an internal document into the public repository:

1. confirm that the described behavior is implemented in public source and that
   users or contributors genuinely need the information;
2. remove private data and unnecessary implementation recipes;
3. verify security, licensing, patent, and commercial implications;
4. align the status with source and tests;
5. add it to the public documentation index only after review.

When detailed internal work becomes implemented public code, publish the
maintainer information contributors genuinely need and retain only unrelated
future strategy in the private workspace.

High-level proposals for advanced or commercial capabilities are not an
exception. They remain private until the same deliberate review.
