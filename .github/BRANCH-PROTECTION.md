# Governance on GitHub Free private

This edition deliberately does not claim paid private branch/ruleset enforcement. The machine-readable local policy is `repository-protection.json`.

The strongest enforceable release ceremony available entirely inside a Free/private repository is:

```text
internal release/X.Y.Z PR -> main -> merged -> post-merge validation -> release gates -> tag -> release
```

The workflow verifies current-main identity, same-repository origin, stable SemVer, version consistency, optional independent approvals, optional distinct merger, release uniqueness, protected-path separation, action SHA pinning, security/dependency gates, final native package execution, reproducibility, checksums and SBOMs.

Because GitHub Free private does not provide unbypassable private rulesets/protected-environment reviewers, keep repository write/admin access limited to people who are trusted with release authority.
