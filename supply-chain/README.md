# Cargo Vet dependency trust baseline

`config.toml`, `audits.toml`, and `imports.lock` are Cargo Vet's
source-controlled trust records for the exact `Cargo.lock` graph. They are a
ratchet: adding or changing a dependency must produce an explicit review
decision instead of silently inheriting trust from this baseline.

The initial baseline is generated with:

```text
cargo xtask assurance initialize-vet
```

It records the current graph as compatibility exemptions because this project
does not yet have a complete independent crate-by-crate audit program. It does
not certify every dependency as safe. Before accepting a future change here:

1. Inspect the dependency source, maintainer status, license, advisories,
   build scripts, native code, and platform impact.
2. Prefer a published audited/imported criterion over a new broad exemption.
3. Keep an exemption version-specific, remove superseded entries, and update
   `Cargo.lock`, Cargo Audit, Cargo Deny, SBOM, and release documentation as
   applicable.
4. Run `cargo xtask assurance pre-push`, then record the review in the change
   description without copying local paths, identities, or tool output.

The directory contains no credentials, host paths, or generated binaries. It is
reviewed source, not an ignored local cache.
