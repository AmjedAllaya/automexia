# Public and private documentation boundary

Automexia is an open-source project, so users and contributors need clear
documentation about the software they can run, inspect, and improve. That does
not require publishing every unreleased implementation recipe or internal
planning document.

## Public documentation

The repository should publish:

- product purpose, values, and intended users;
- installation, configuration, use, recovery, and uninstall guidance;
- behavior that exists in source, with honest release and platform status;
- stable architecture and security boundaries contributors must preserve;
- a high-level roadmap that explains direction without promising dates;
- contribution, testing, release, support, and vulnerability-reporting rules;
- accepted decisions needed to understand or maintain public code.

## Local-only documentation

Keep the following outside version control until a deliberate publication
review approves them:

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

## Never document in the repository

Public and private project documents must both avoid credentials, tokens,
private keys, customer or production data, personal identifiers, machine
names, and absolute local paths. Use clearly fictional examples and redacted
fixtures.

## Publication review

Before moving an internal document into the public repository:

1. confirm that the described behavior is implemented or that publication is
   intentionally limited to a high-level proposal;
2. remove private data and unnecessary implementation recipes;
3. verify security, licensing, patent, and commercial implications;
4. align the status with source and tests;
5. add it to the public documentation index only after review.

When detailed internal work becomes implemented public code, publish the
maintainer information contributors genuinely need and retain only unrelated
future strategy in the private workspace.
