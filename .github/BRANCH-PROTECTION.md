# Governance on GitHub Free private

This edition deliberately does not claim paid private branch/ruleset enforcement. The machine-readable local policy is `repository-protection.json`.

The strongest enforceable release ceremony available entirely inside a Free/private repository is:

```text
internal release/X.Y.Z PR -> main -> merged -> post-merge validation -> release gates -> tag -> release
```

The workflow verifies current-main identity, same-repository origin, stable SemVer, version consistency, optional independent approvals, optional distinct merger, release uniqueness, protected-path separation, action SHA pinning, security/dependency gates, final native package execution, reproducibility, checksums and SBOMs.

Because GitHub Free private does not provide unbypassable private rulesets/protected-environment reviewers, keep repository write/admin access limited to people who are trusted with release authority.

The separate `release/linux/X.Y.Z` lane uses the owner-only policy accepted in
[ADR 0040](../docs/adr/0040-owner-authorized-linux-releases.md). The pinned human
owner must author, merge, trigger and rerun publication, with a same-repository
merged PR and exact current-main identity. It does not require a second person.
The public binary archive retains its existing branch and immutable-tag rules;
release publication does not modify its protected metadata branch.
